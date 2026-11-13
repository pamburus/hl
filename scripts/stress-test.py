#!/usr/bin/env python3
"""Runs hl on large inputs and checks its output, CPU time and peak memory.

Usage:
  stress-test.py --hl PATH [--profile quick|full] [--case REGEX] [--mode REGEX] [--keep]

Cases and profiles are defined in stress-test.toml next to this script, which also explains the checks.
Each case runs in each of its modes at two sizes and compares the two runs, so a check catches time or
memory that grows faster than it should without depending on how fast the machine is.
"""

import argparse
import itertools
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import tomllib
from pathlib import Path

CASES = Path(__file__).with_suffix(".toml")
CHUNK = 1 << 20
BATCH = 1 << 16
# A run may exceed its limit by this much CPU time, a few scheduler ticks that do not depend on the input size.
CPU_NOISE = 0.02

MODES = {
    "cat": [],
    "sort-file": ["-s"],
    "sort-stdin": ["-s"],
    "sort-gz": ["-s"],
    "follow-stdin": ["-F"],
    "follow-file": ["-F"],
}


def parse_size(s):
    m = re.fullmatch(r"(\d+)\s*(|KiB|MiB|GiB)", s)
    if not m:
        raise ValueError(f"invalid size: {s}")
    return int(m[1]) << {"": 0, "KiB": 10, "MiB": 20, "GiB": 30}[m[2]]


def format_size(n):
    for unit, shift in (("GiB", 30), ("MiB", 20), ("KiB", 10)):
        if n >= 1 << shift and n % (1 << shift) == 0:
            return f"{n >> shift}{unit}"
    return str(n)


# --- inputs
#
# An input is a list of writes, each a list of items. An item is a short record, given by its number, or a text
# that is not a record, given by its length and whether it ends with a delimiter. Writes matter only when following
# a file, where each write is appended separately after a pause.


class Text:
    def __init__(self, size, terminated=True):
        self.size = size
        self.terminated = terminated


def record_ts(i):
    s, ms = divmod(i, 1000)
    m, s = divmod(s, 60)
    h, m = divmod(m, 60)
    return f"2026-01-01T{h:02}:{m:02}:{s:02}.{ms:03}Z", f"2026-01-01 {h:02}:{m:02}:{s:02}.{ms:03}"


def record_msg(i):
    return f"record {i} " + "y" * 100


def record(i):
    return f'{{"ts":"{record_ts(i)[0]}","msg":"{record_msg(i)}"}}\n'.encode()


def text_chunks(text, i):
    # A record too long to be parsed, so it is shown as it is.
    head = f'{{"ts":"{record_ts(i)[0]}","msg":"'.encode()
    tail = b'"}\n' if text.terminated else b'"}'
    left = text.size - len(head) - len(tail)
    yield head
    while left > 0:
        n = min(left, CHUNK)
        yield b"x" * n
        left -= n
    yield tail


def batched(chunks, size=BATCH):
    # Joins short chunks so that files are written and compared in fewer, larger pieces.
    batch, n = [], 0
    for chunk in chunks:
        if len(chunk) >= size:
            if batch:
                yield b"".join(batch)
                batch, n = [], 0
            yield chunk
            continue
        batch.append(chunk)
        n += len(chunk)
        if n >= size:
            yield b"".join(batch)
            batch, n = [], 0
    if batch:
        yield b"".join(batch)


def item_chunks(items):
    def chunks():
        for i, item in enumerate(items):
            if isinstance(item, Text):
                yield from text_chunks(item, i)
            elif isinstance(item, bytes):
                yield item
            else:
                yield record(item)

    return batched(chunks())


def make_input(kind, line, input_size):
    if kind == "long-line":
        return [[0, Text(line), 2]]
    if kind == "unterminated-long-line":
        return [[0, 1, Text(line, terminated=False)]]
    if kind == "short-lines":
        # A range, not a list: on Linux the peak memory of hl includes the memory this script has when starting it.
        return [range(input_size // len(record(0)))]
    if kind == "split-record":
        return None
    raise ValueError(f"unknown input: {kind}")


def split_record_input():
    # Complete records that reach the limit together, with the start of one more record in the same write.
    rec = record(60)
    return [[*range(60), rec[:30]], [rec[30:]]], range(61)


# --- expected output


class Formatter:
    """Formats a short record the way the tested build does, learned from the build itself on one sample record."""

    def __init__(self, hl, env):
        ts_in, ts_out = record_ts(0)
        sample = f'{{"ts":"{ts_in}","msg":"MSG"}}\n'.encode()
        out = subprocess.run([hl, *BASE_ARGS], input=sample, capture_output=True, env=env, check=True).stdout.decode()
        if out.count(ts_out) != 1 or out.count("MSG") != 1:
            raise RuntimeError(f"cannot learn the output format from {out!r}")
        self.template = out.replace(ts_out, "\0ts\0").replace("MSG", "\0msg\0")

    def __call__(self, i):
        return self.template.replace("\0ts\0", record_ts(i)[1]).replace("\0msg\0", record_msg(i)).encode()


def expected_chunks(writes, mode, fmt):
    def chunks():
        for i, item in enumerate(itertools.chain.from_iterable(writes)):
            if isinstance(item, Text):
                if not mode.startswith("sort"):
                    yield from text_chunks(item, i)
                    if not item.terminated:
                        yield b"\n"
            else:
                yield fmt(item)

    return batched(chunks())


def expected_file(workdir, writes, mode, fmt):
    # Sorting drops what is not a record, the other modes show it, so the modes share one of two files.
    path = workdir / ("expected-sorted" if mode.startswith("sort") else "expected-shown")
    if not path.exists():
        with open(path, "wb") as f:
            for chunk in expected_chunks(writes, mode, fmt):
                f.write(chunk)
    return path


def first_difference(path, expected):
    with open(path, "rb") as f, open(expected, "rb") as e:
        offset = 0
        while True:
            want, got = e.read(CHUNK), f.read(CHUNK)
            if want != got:
                n = next((k for k in range(min(len(want), len(got))) if got[k] != want[k]), min(len(want), len(got)))
                return offset + n, want[n : n + 40], got[n : n + 40]
            if not want:
                return None
            offset += len(want)


# --- runs

BASE_ARGS = ["--config", "-", "-P", "--color", "never"]


class Run:
    def __init__(self, cpu, rss, error=None):
        self.cpu, self.rss, self.error = cpu, rss, error


def write_file(path, writes, gz=False):
    import gzip

    with (gzip.open(path, "wb", compresslevel=1) if gz else open(path, "wb")) as f:
        for write in writes:
            for chunk in item_chunks(write):
                f.write(chunk)


def wait(proc, deadline):
    while True:
        pid, status, usage = os.wait4(proc.pid, os.WNOHANG)
        if pid:
            proc.returncode = os.waitstatus_to_exitcode(status)
            return usage
        if time.monotonic() > deadline:
            proc.kill()
            pid, status, usage = os.wait4(proc.pid, 0)
            proc.returncode = None
            return usage
        time.sleep(0.01)


def run_hl(hl, mode, args, workdir, writes, expected_size, timeout):
    # The input files are written once per directory and shared by the modes, only the followed file is written anew.
    # The cache starts empty in each run, so that sorting a file builds its index every time.
    cache = workdir / "cache"
    shutil.rmtree(cache, ignore_errors=True)
    env = clean_env(cache)
    out_path = workdir / "out"
    cmd = [hl, *BASE_ARGS, *MODES[mode], *args]
    stdin = subprocess.DEVNULL
    if mode == "sort-gz":
        src = workdir / "in.log.gz"
        if not src.exists():
            write_file(src, writes, gz=True)
        cmd.append(str(src))
    elif mode == "follow-file":
        src = workdir / "followed.log"
        src.write_bytes(b"")
        cmd.append(str(src))
    else:
        src = workdir / "in.log"
        if not src.exists():
            write_file(src, writes)
        if mode.endswith("stdin"):
            stdin = open(src, "rb")
        else:
            cmd.append(str(src))

    with open(out_path, "wb") as out:
        start = time.monotonic()
        deadline = start + timeout
        proc = subprocess.Popen(cmd, stdin=stdin, stdout=out, stderr=subprocess.PIPE, env=env)
        if mode == "follow-file":
            # Appends each write to the followed file, then waits for all of the expected output.
            time.sleep(0.5)
            with open(src, "ab") as f:
                for n, write in enumerate(writes):
                    if n:
                        time.sleep(1.5)
                    for chunk in item_chunks(write):
                        f.write(chunk)
                    f.flush()
            while out_path.stat().st_size < expected_size and time.monotonic() < deadline and proc.poll() is None:
                time.sleep(0.02)
            time.sleep(0.2)
            if proc.poll() is None:
                proc.send_signal(signal.SIGINT)
            deadline = time.monotonic() + 10
        usage = wait(proc, deadline)
    if stdin is not subprocess.DEVNULL:
        stdin.close()

    rss = usage.ru_maxrss * (1 if sys.platform == "darwin" else 1024)
    error = None
    if proc.returncode is None:
        error = f"timed out after {timeout} s"
    elif proc.returncode != 0 and mode != "follow-file":
        error = f"exit code {proc.returncode}: {proc.stderr.read().decode().strip()}"
    proc.stderr.close()
    # CPU time counts both user and system time, it does not depend on what else the machine is doing as wall time does.
    return Run(usage.ru_utime + usage.ru_stime, rss, error)


def clean_env(cache):
    env = {k: v for k, v in os.environ.items() if not k.startswith("HL_")}
    env["XDG_CACHE_HOME"] = str(cache)
    return env


# --- checks


def growth_limit(growth, factor, slack):
    return {"constant": slack, "linear": factor * slack}[growth]


def known_issue(case, key):
    # An entry is an issue, or a table with the issue, the platforms it fails on, as named by `sys.platform`, and whether
    # it fails only sometimes. Returns the issue and whether it is intermittent, or None.
    entry = case.get("known", {}).get(key)
    if isinstance(entry, dict):
        if sys.platform not in entry.get("os", [sys.platform]):
            return None
        return entry["issue"], entry.get("intermittent", False)
    return (entry, False) if entry else None


def mode_growth(growth, mode):
    # A case can give a mode its own growth.
    if isinstance(growth, dict):
        return growth.get(mode, growth["default"])
    return growth


def ratio_check(a, b, growth, factor, slack, shown, noise=0):
    limit = growth_limit(growth, factor, slack)
    ratio = b / a
    ok = b <= a * limit + noise
    return ok, f"{shown} ×{ratio:.2f} ≤ {limit:.2f}" + (" within noise" if ok and ratio > limit else "")


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--hl", required=True, help="hl binary to test")
    ap.add_argument("--profile", default="quick", help="profile from stress-test.toml (default: quick)")
    ap.add_argument("--case", default="", help="run only cases whose name matches this regex")
    ap.add_argument("--mode", default="", help="run only modes whose name matches this regex")
    ap.add_argument("--set", action="append", default=[], metavar="KEY=VALUE", help="override a profile value")
    ap.add_argument("--keep", action="store_true", help="keep the work directory")
    opts = ap.parse_args()

    config = tomllib.loads(CASES.read_text())
    profile = config["profiles"][opts.profile]
    for item in opts.set:
        key, value = item.split("=", 1)
        if key not in profile:
            ap.error(f"unknown profile key: {key}")
        profile[key] = type(profile[key])(value)
    sizes = {k: parse_size(profile[k]) for k in ("line", "limit", "input")}
    factor, slack, timeout = profile["factor"], profile["slack"], profile["timeout"]
    hl = shutil.which(opts.hl) or opts.hl

    root = Path(tempfile.mkdtemp(prefix="hl-stress-"))
    fmt = Formatter(hl, clean_env(root / "cache"))
    failures, known, fixed = [], [], []

    def report(name, mode, check, ok, line):
        key = f"{mode}.{check}"
        known_entry = known_issue(case, key)
        issue, intermittent = known_entry or (None, False)
        if ok and issue and intermittent:
            status = f"ok, known to fail at times {issue}"
        elif ok and issue:
            status = f"FIXED? {issue} is listed as known"
            fixed.append(f"{name} {key}")
        elif ok:
            status = "ok"
        elif issue:
            status = f"known {issue}"
            known.append(f"{name} {key}")
        else:
            status = "FAILED"
            failures.append(f"{name} {key}")
        print(f"  {mode:13} {check:6} {line:44} {status}", flush=True)

    try:
        for case in config["cases"]:
            name = case["name"]
            if not re.search(opts.case, name):
                continue
            modes = [m for m in case["modes"] if re.search(opts.mode, m)]
            if not modes:
                continue
            print(f"{name}: {case['about']}", flush=True)
            if case["input"] == "split-record":
                for mode in modes:
                    workdir = root / name / mode
                    workdir.mkdir(parents=True)
                    writes, records = split_record_input()
                    expected = workdir / "expected"
                    expected.write_bytes(b"".join(fmt(i) for i in records))
                    args = ["--buffer-size", "1KiB", "--max-message-size", "2KiB"]
                    r = run_hl(hl, mode, args, workdir, writes, expected.stat().st_size, timeout)
                    diff = r.error or first_difference(workdir / "out", expected)
                    report(name, mode, "output", not diff, "" if not diff else str(diff))
                continue

            dim = case["scale"]
            runs, diffs = {m: [] for m in modes}, {m: [] for m in modes}
            for step, mul in enumerate((1, factor)):
                s = dict(sizes)
                s[dim] *= mul
                if dim == "limit":
                    s["line"] = max(s["line"], sizes["limit"] * factor * 2)
                workdir = root / name / str(step)
                workdir.mkdir(parents=True)
                writes = make_input(case["input"], s["line"], s["input"])
                args = ["--max-message-size", str(s["limit"])]
                for mode in modes:
                    expected = expected_file(workdir, writes, mode, fmt)
                    # The fastest of several runs is the least disturbed by the rest of the machine.
                    best = None
                    for _ in range(profile["repeat"]):
                        r = run_hl(hl, mode, args, workdir, writes, expected.stat().st_size, timeout)
                        if r.error:
                            diffs[mode].append(f"{dim}={format_size(s[dim])}: {r.error}")
                            best = r
                            break
                        diff = first_difference(workdir / "out", expected)
                        if diff and not diffs[mode]:
                            diffs[mode].append(f"{dim}={format_size(s[dim])}: at {diff[0]} expected {diff[1]!r}, got {diff[2]!r}")
                        if best is None or r.cpu < best.cpu:
                            best = Run(r.cpu, min(r.rss, best.rss) if best else r.rss)
                        else:
                            best.rss = min(best.rss, r.rss)
                    runs[mode].append((s[dim], best))
                if not opts.keep:
                    shutil.rmtree(workdir)

            for mode in modes:
                report(name, mode, "output", not diffs[mode], diffs[mode][0] if diffs[mode] else "")
                if any(r.error for _, r in runs[mode]):
                    continue
                (n0, a), (n1, b) = runs[mode]
                sizes_text = f"{dim} {format_size(n0)}→{format_size(n1)}"
                shown = f"{a.cpu:.2f}→{b.cpu:.2f} s"
                ok, text = ratio_check(max(a.cpu, 1e-3), b.cpu, case["cpu"], factor, slack, shown, CPU_NOISE)
                report(name, mode, "cpu", ok, f"{sizes_text} {text}")
                growth = mode_growth(case["rss"], mode)
                ok, text = ratio_check(a.rss, b.rss, growth, factor, slack, f"{a.rss >> 20}→{b.rss >> 20} MiB")
                report(name, mode, "rss", ok, f"{sizes_text} {text}")
    finally:
        if opts.keep:
            print(f"work directory: {root}")
        else:
            shutil.rmtree(root, ignore_errors=True)

    print()
    if known:
        print(f"known failures: {len(known)}")
    if fixed:
        print("passing checks still listed as known, remove them from stress-test.toml:")
        for f in fixed:
            print(f"  {f}")
    if failures:
        print("failed:")
        for f in failures:
            print(f"  {f}")
    if failures or fixed:
        sys.exit(1)
    print("✅ stress test passed")


if __name__ == "__main__":
    main()
