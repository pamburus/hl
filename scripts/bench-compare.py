#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# ///
"""Compares the benchmarks of the working tree with those of the commit where it forked from a base revision.

Usage:
  bench-compare.py [--base REV] [-- CRITERION-ARGS...]

Runs the benchmarks of the base revision (origin/develop by default) in its worktree, which the worktrees of the clone
share (see base_worktree.py), saving them as a criterion baseline. Then runs those of the working tree, compared with
that baseline. Benchmarks the base does not have are measured without a comparison. The reference benchmarks, which
measure third-party code and language or library primitives, are skipped. Criterion arguments, such as a benchmark
filter, apply to both runs.

One comparison runs at a time in the clone, another one fails right away.
"""

import argparse
import fcntl
import os
import shutil
import subprocess
import sys
from pathlib import Path

import base_worktree

ROOT = Path(__file__).resolve().parent.parent
LOCK = "hl-bench-compare.lock"
BASELINE = "bench-compare-base"


def target_dir():
    return Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")).resolve()


def bench(src, env, *args):
    proc = subprocess.run(["cargo", "bench", "--locked", "--bench", "bench", "--", *args], cwd=src, env=env)
    if proc.returncode != 0:
        sys.exit(proc.returncode)


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--base", default="origin/develop", help="compare with where HEAD forked from this revision")
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
        for path in sorted(criterion.rglob(BASELINE)) if criterion.exists() else []:
            shutil.rmtree(path)
        env = dict(base_worktree.env(), CRITERION_HOME=str(criterion), HL_BENCH_NO_REFERENCE="1")

        # The benchmarks of the base run in the target directory the bases share, so under its lock, while no other
        # base builds there.
        def run_base(src, target):
            bench(src, dict(env, CARGO_TARGET_DIR=str(target)), "--save-baseline", BASELINE, *opts.args)

        print(f"running the benchmarks of the base, {sha}", flush=True)
        base_worktree.build(src, sha, run_base)
        print("running the benchmarks of the working tree, compared with the base", flush=True)
        bench(ROOT, env, "--baseline-lenient", BASELINE, *opts.args)


if __name__ == "__main__":
    main()
