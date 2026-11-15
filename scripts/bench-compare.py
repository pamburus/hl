#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# ///
"""Compares the benchmarks of the working tree with those of the commit where it forked from a base revision.

Usage:
  bench-compare.py [--base REV] [--reuse-base] [-- CRITERION-ARGS...]

Runs the benchmarks of the base revision (origin/develop by default) in its worktree, which the worktrees of the clone
share (see base_worktree.py), saving them as a criterion baseline. Then runs those of the working tree, compared with
that baseline. Benchmarks the base does not have are measured without a comparison. The reference benchmarks, which
measure third-party code and language or library primitives, are skipped. Criterion arguments, such as a benchmark
filter, apply to both runs.

Each run of the base benchmarks is also stored in the git common directory, keyed by the base commit, a hash of the
benchmark executable and the criterion arguments. With --reuse-base, or with HL_BENCH_REUSE_BASE set, a stored run with
the same key is used instead of running the base benchmarks again. Stored runs older than 14 days are removed.

One comparison runs at a time in the clone, another one fails right away.
"""

import argparse
import fcntl
import hashlib
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

import base_worktree

ROOT = Path(__file__).resolve().parent.parent
LOCK = "hl-bench-compare.lock"
BASELINE = "bench-compare-base"
STORE = "hl-bench-compare"


def target_dir():
    return Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")).resolve()


def bench(src, env, *args):
    proc = subprocess.run(["cargo", "bench", "--locked", "--bench", "bench", "--", *args], cwd=src, env=env)
    if proc.returncode != 0:
        sys.exit(proc.returncode)


def executable(src, env):
    cmd = ["cargo", "bench", "--locked", "--bench", "bench", "--no-run", "--message-format=json-render-diagnostics"]
    proc = subprocess.run(cmd, cwd=src, env=env, stdout=subprocess.PIPE, text=True)
    if proc.returncode != 0:
        sys.exit(proc.returncode)
    for line in proc.stdout.splitlines():
        msg = json.loads(line)
        if msg.get("reason") == "compiler-artifact" and msg["target"]["name"] == "bench" and msg.get("executable"):
            return Path(msg["executable"])
    sys.exit("cargo did not report the benchmark executable")


def key(sha, exe, args):
    # The executable stands for everything the build depends on, such as the toolchain, the flags and the features, and
    # the arguments for which benchmarks run and how they are measured.
    h = hashlib.sha256()
    with open(exe, "rb") as f:
        while chunk := f.read(1 << 20):
            h.update(chunk)
    h.update(b"\0".join(arg.encode() for arg in args))
    return f"{sha[:12]}-{h.hexdigest()[:12]}"


def baselines(root):
    return sorted(root.rglob(BASELINE)) if root.exists() else []


def save(criterion, entry):
    tmp = entry.with_name(entry.name + ".tmp")
    shutil.rmtree(tmp, ignore_errors=True)
    tmp.mkdir(parents=True)
    for path in baselines(criterion):
        shutil.copytree(path, tmp / path.relative_to(criterion))
    shutil.rmtree(entry, ignore_errors=True)
    tmp.rename(entry)


def expire(store):
    for entry in store.iterdir() if store.exists() else []:
        if time.time() - entry.stat().st_mtime > base_worktree.EXPIRY:
            shutil.rmtree(entry, ignore_errors=True)


def age(entry):
    hours = (time.time() - entry.stat().st_mtime) / 3600
    if hours < 1:
        n, unit = round(hours * 60), "minute"
    elif hours < 48:
        n, unit = round(hours), "hour"
    else:
        n, unit = round(hours / 24), "day"
    return f"{n} {unit}" if n == 1 else f"{n} {unit}s"


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--base", default="origin/develop", help="compare with where HEAD forked from this revision")
    ap.add_argument(
        "--reuse-base",
        action="store_true",
        default=bool(os.environ.get("HL_BENCH_REUSE_BASE")),
        help="use a stored run of the base benchmarks with the same key, if any",
    )
    ap.add_argument("args", nargs="*", help="criterion arguments for both runs, after --, such as a benchmark filter")
    opts = ap.parse_args()

    # Runs in worktrees of one clone would skew each other's timings, and runs in one worktree would share its results.
    with open(base_worktree.common_dir() / LOCK, "w") as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            sys.exit("another bench-compare is running in this clone")

        sha = base_worktree.git("merge-base", "HEAD", opts.base)
        src = base_worktree.checkout(sha)

        # Both runs keep their results with those of the working tree, so that it is compared with the baseline the base
        # saved. The baseline of an earlier run is removed, so that benchmarks this run leaves out are not compared with
        # another base.
        criterion = target_dir() / "criterion"
        for path in baselines(criterion):
            shutil.rmtree(path)
        env = dict(base_worktree.env(), CRITERION_HOME=str(criterion), HL_BENCH_NO_REFERENCE="1")
        store = base_worktree.common_dir() / STORE
        expire(store)

        # The benchmarks of the base run in the target directory the bases share, so under its lock, while no other
        # base builds there.
        def run_base(src, target):
            base_env = dict(env, CARGO_TARGET_DIR=str(target))
            entry = store / key(sha, executable(src, base_env), opts.args)
            if opts.reuse_base and entry.exists():
                print(f"reusing the benchmarks of the base, {sha}, run {age(entry)} ago", flush=True)
                shutil.copytree(entry, criterion, dirs_exist_ok=True)
                return
            print(f"running the benchmarks of the base, {sha}", flush=True)
            bench(src, base_env, "--save-baseline", BASELINE, *opts.args)
            save(criterion, entry)

        base_worktree.build(src, sha, run_base)
        print("running the benchmarks of the working tree, compared with the base", flush=True)
        bench(ROOT, env, "--baseline-lenient", BASELINE, *opts.args)


if __name__ == "__main__":
    main()
