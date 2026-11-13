#!/usr/bin/env python3
"""Compares the number of instructions two builds of hl execute on the same inputs.

Usage:
  perf-gate.py [--base REV] [--threshold RATIO] [--repeat N] [--report-only] [--input REGEX] [--mode REGEX]
  perf-gate.py --base-hl PATH --hl PATH [...]

Without --hl, builds the working tree and the commit where it forked from the base revision (origin/develop by
default) in release mode, the base in a worktree it keeps for the next run. Each build runs on the same inputs in each
mode under cachegrind, with one processing thread and an empty cache unless the mode sets otherwise, as many runs at a
time as there are CPUs. The counts repeat to within a fraction of a percent, so one run of each is enough by default.
With more runs, the medians are compared.
The check fails when the head exceeds the base by more than the threshold in any input and mode.

Valgrind runs on Linux only. Elsewhere, the script runs itself in a Docker container.
"""

import argparse
import gzip
import json
import os
import re
import statistics
import shutil
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SAMPLES = ["kitchen-sink.log", "prometheus.log", "cafe.log"]
IMAGE = "rust:1-bookworm"
BASE_WORKTREE = ROOT / ".worktrees" / "perf-gate-base"

# Arguments of each mode, and how it takes the input:
#   file: the input file
#   gz: the input file compressed with gzip
#   stdin: the input file on stdin
#   files: the input split in two files, with the records alternating between them
#   indexed: the input file, indexed by an earlier run without valgrind
MODES = {
    "cat": ([], "file"),
    "cat-threads": (["-C", "4"], "file"),
    "sort-file": (["-s"], "file"),
    "sort-files": (["-s"], "files"),
    "sort-indexed": (["-s"], "indexed"),
    "sort-gz": (["-s"], "gz"),
    "sort-stdin": (["-s"], "stdin"),
    "sort-threads": (["-s", "-C", "4"], "file"),
    "follow-stdin": (["-F"], "stdin"),
}

# One processing thread unless the mode asks for more: valgrind runs threads one at a time anyway, and the counts repeat
# more closely.
BASE_ARGS = ["--config", "-", "-P", "--color", "always", "-C", "1"]


# --- inputs
#
# Inputs are made from the records in the sample directory, repeated with new timestamps until they reach the size.
# The timestamps go slightly out of order, so that sorting has work to do.


def sample_records():
    for name in SAMPLES:
        for line in (ROOT / "sample" / name).read_text().splitlines():
            if line.strip():
                yield json.loads(line)


def timestamp(i):
    ms = i * 10 + (i * 7919) % 13
    s, ms = divmod(ms, 1000)
    m, s = divmod(s, 60)
    h, m = divmod(m, 60)
    return f"2026-01-01T{h:02}:{m:02}:{s:02}.{ms:03}Z"


def logfmt_value(v):
    if isinstance(v, str):
        return v if v and re.fullmatch(r'[^\s"=]+', v) else json.dumps(v)
    return json.dumps(v)


def logfmt(record, prefix=""):
    for k, v in record.items():
        if isinstance(v, dict):
            yield from logfmt(v, f"{prefix}{k}.")
        else:
            yield f"{prefix}{k}={logfmt_value(v)}"


def json_line(record):
    return json.dumps(record, ensure_ascii=False)


def logfmt_line(record):
    return " ".join(logfmt(record))


FORMATS = {"json": json_line, "logfmt": logfmt_line}


def write_input(path, fmt, size):
    records = list(sample_records())
    lines = []
    n = 0
    while n < size:
        i = len(lines)
        record = dict(records[i % len(records)])
        for key in ("ts", "time", "timestamp"):
            if key in record:
                record[key] = timestamp(i)
        lines.append(FORMATS[fmt](record) + "\n")
        n += len(lines[-1].encode())
    path.write_text("".join(lines))
    with open(path, "rb") as src, gzip.open(path.with_suffix(".log.gz"), "wb") as dst:
        shutil.copyfileobj(src, dst)
    for i, part in enumerate(input_parts(path)):
        part.write_text("".join(lines[i::2]))


def input_parts(path):
    return [path.with_suffix(f".{i}.log") for i in range(2)]


# --- runs


def instructions(hl, mode, src, workdir):
    cache = workdir / "cache"
    shutil.rmtree(cache, ignore_errors=True)
    env = {k: v for k, v in os.environ.items() if not k.startswith("HL_")}
    env["XDG_CACHE_HOME"] = str(cache)
    log = workdir / "valgrind.log"
    cmd = ["valgrind", "--tool=cachegrind", "--cache-sim=no", "--cachegrind-out-file=/dev/null", f"--log-file={log}"]
    args, how = MODES[mode]
    hl_cmd = [hl, *BASE_ARGS, *args]
    stdin = subprocess.DEVNULL
    if how == "gz":
        hl_cmd.append(str(src.with_suffix(".log.gz")))
    elif how == "stdin":
        stdin = open(src, "rb")
    elif how == "files":
        hl_cmd += map(str, input_parts(src))
    else:
        hl_cmd.append(str(src))
    if how == "indexed":
        subprocess.run(hl_cmd, stdout=subprocess.DEVNULL, env=env, check=True)
    cmd += hl_cmd
    try:
        proc = subprocess.run(cmd, stdin=stdin, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, env=env)
    finally:
        if stdin is not subprocess.DEVNULL:
            stdin.close()
    if proc.returncode != 0:
        raise RuntimeError(f"{hl} {mode}: exit code {proc.returncode}: {proc.stderr.decode().strip()}")
    m = re.search(r"I\s+refs:\s+([\d,]+)", log.read_text())
    if not m:
        raise RuntimeError(f"{hl} {mode}: no instruction count in {log.read_text()!r}")
    return int(m[1].replace(",", ""))


# --- builds


def git(*args):
    return subprocess.run(["git", "-C", str(ROOT), *args], check=True, capture_output=True, text=True).stdout.strip()


def target_dir():
    return Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))


def checkout_base(sha):
    # A worktree kept from run to run, so that moving it to another base rewrites only the files that differ and the
    # base builds incrementally. It belongs to this script, so local changes in it are discarded.
    git("worktree", "prune")
    if (BASE_WORKTREE / ".git").exists():
        subprocess.run(["git", "-C", str(BASE_WORKTREE), "checkout", "-q", "--force", "--detach", sha], check=True)
        subprocess.run(["git", "-C", str(BASE_WORKTREE), "clean", "-q", "-d", "--force"], check=True)
    else:
        git("worktree", "add", "-q", "--detach", str(BASE_WORKTREE), sha)
    return BASE_WORKTREE


def build(src, target):
    print(f"building {src}", flush=True)
    env = dict(os.environ, CARGO_TARGET_DIR=str(target))
    subprocess.run(["cargo", "build", "--locked", "--release", "--bin", "hl"], cwd=src, env=env, check=True)
    return target / "release" / "hl"


def build_base(src, sha):
    # A target directory of its own: cargo gives a workspace crate the same identity wherever its source is, so in a
    # shared one the head build would take the crates built for the base. The binary is kept by revision.
    hl = target_dir() / "perf-gate" / f"hl-{sha}"
    if not hl.exists():
        hl.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(build(src, target_dir() / "perf-gate" / "base"), hl)
    return hl


def in_docker(opts, base_src):
    # Linux build directories and the toolchains are kept in volumes, so later runs build incrementally.
    cmd = ["docker", "run", "--rm", "-v", f"{ROOT}:/src:ro", "-v", f"{base_src}:/base:ro"]
    cmd += ["-v", "hl-cargo-registry:/usr/local/cargo/registry", "-v", "hl-cargo-git:/usr/local/cargo/git"]
    cmd += ["-v", "hl-rustup:/usr/local/rustup"]
    cmd += ["-v", "hl-perf-gate-target:/target", "-e", "CARGO_TARGET_DIR=/target/linux", "-w", "/src", IMAGE]
    script = "apt-get update -qq >/dev/null && apt-get install -y -qq valgrind >/dev/null && exec scripts/perf-gate.py"
    cmd += ["bash", "-c", script + ' "$@"', "perf-gate.py", "--base-src", "/base", "--base", opts.base]
    cmd += [f"--threshold={opts.threshold}", f"--repeat={opts.repeat}", f"--size={opts.size}"]
    cmd += [f"--input={opts.input}", f"--mode={opts.mode}"]
    cmd += ["--report-only"] if opts.report_only else []
    return subprocess.run(cmd).returncode


# --- report


class Counts:
    def __init__(self, counts):
        self.median = statistics.median(counts)
        self.spread = (max(counts) - min(counts)) / self.median if len(counts) > 1 else None

    def __str__(self):
        # The spread from the lowest to the highest run, when there are several.
        return f"{self.median:,.0f}" + (f" ({self.spread:.2%})" if self.spread is not None else "")


def report(rows, threshold, report_only, base, markdown):
    failed = [r for r in rows if r[4] > threshold]
    lines = [
        "| input | mode | base | head | ratio |",
        "| --- | --- | ---: | ---: | ---: |",
    ]
    for name, mode, a, b, ratio in rows:
        mark = " ❌" if ratio > threshold else ""
        lines.append(f"| {name} | {mode} | {a} | {b} | {ratio:.3f}{mark} |")
    if failed:
        verdict = f"{len(failed)} of {len(rows)} inputs and modes exceed ×{threshold}"
        verdict += ", reported only" if report_only else ""
    else:
        verdict = f"all {len(rows)} inputs and modes within ×{threshold}"
    # The results are printed as they come, the table goes to the markdown report.
    print(f"\n{verdict}")
    if markdown:
        # In a pull request, HEAD is the merge commit, and the commit of the pull request is its second parent.
        head = (
            git("rev-parse", "HEAD^2")
            if len(git("rev-list", "--parents", "-n1", "HEAD").split()) > 2
            else git("rev-parse", "HEAD")
        )
        title = f"Base {base}, head {head}." if base else ""
        hint = "Add the `perf-regression-accepted` label and re-run the job if the cost is intended." if failed else ""
        # The marker lets CI find the comment it posted before, to update it.
        lines = ["<!-- perf-gate -->", "## Instruction counts", "", title, "", *lines, "", f"**{verdict}**", "", hint]
        Path(markdown).write_text("\n".join(lines).strip() + "\n")
    return bool(failed) and not report_only


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--base", default="origin/develop", help="compare with where HEAD forked from this revision")
    ap.add_argument("--hl", help="head build, instead of building the working tree")
    ap.add_argument("--base-hl", help="base build, instead of building the base revision")
    ap.add_argument("--base-src", help=argparse.SUPPRESS)
    ap.add_argument("--threshold", type=float, default=1.02, help="largest allowed ratio of head to base medians")
    ap.add_argument("--repeat", type=int, default=1, help="runs of each input and mode on each build")
    ap.add_argument("--size", type=int, default=2 << 20, help="size of each input in bytes")
    ap.add_argument("--input", default="", help="run only inputs whose name matches this regex")
    ap.add_argument("--mode", default="", help="run only modes whose name matches this regex")
    ap.add_argument("--report-only", action="store_true", help="report runs over the threshold without failing")
    ap.add_argument("--markdown", help="write the results as a markdown report to this file")
    opts = ap.parse_args()

    base_hl, hl = opts.base_hl, opts.hl
    with tempfile.TemporaryDirectory(prefix="hl-perf-gate-") as tmp:
        workdir = Path(tmp)
        if not (base_hl and hl):
            if not opts.base_src:
                opts.base = git("merge-base", "HEAD", opts.base)
                base_src = checkout_base(opts.base)
                if not shutil.which("valgrind"):
                    if sys.platform == "linux" or not shutil.which("docker"):
                        sys.exit("valgrind is not installed")
                    sys.exit(in_docker(opts, base_src))
            else:
                base_src = Path(opts.base_src)
            print(f"base: {opts.base}", flush=True)
            base_hl = base_hl or build_base(base_src, opts.base)
            hl = hl or build(ROOT, target_dir())
        elif not shutil.which("valgrind"):
            sys.exit("valgrind is not installed")

        cases = []
        for name in FORMATS:
            if not re.search(opts.input, name):
                continue
            src = workdir / f"{name}.log"
            write_input(src, name, opts.size)
            cases += [(name, mode, src) for mode in MODES if re.search(opts.mode, mode)]

        # Valgrind runs the threads of a program one at a time, so each run takes one core, and the counts do not
        # depend on what else runs on the machine. The runs are started in turns between the builds.
        def run(hl, name, mode, src, i):
            rundir = workdir / f"{name}-{mode}-{i}"
            rundir.mkdir()
            return instructions(hl, mode, src, rundir)

        with ThreadPoolExecutor(os.cpu_count()) as pool:
            runs = []
            for name, mode, src in cases:
                a, b = [], []
                for i in range(opts.repeat):
                    a.append(pool.submit(run, base_hl, name, mode, src, f"base-{i}"))
                    b.append(pool.submit(run, hl, name, mode, src, f"head-{i}"))
                runs.append((name, mode, a, b))
            rows = []
            for name, mode, a, b in runs:
                a, b = Counts([f.result() for f in a]), Counts([f.result() for f in b])
                ratio = b.median / a.median
                rows.append((name, mode, a, b, ratio))
                print(f"  {name:7} {mode:13} {str(a):>24} {str(b):>24} ×{ratio:.3f}", flush=True)

    if report(rows, opts.threshold, opts.report_only, opts.base if not opts.base_hl else None, opts.markdown):
        sys.exit(1)


if __name__ == "__main__":
    main()
