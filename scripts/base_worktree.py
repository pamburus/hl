"""Worktrees of base revisions, for the scripts that compare a build of the working tree with a build of a base.

Each base revision has one worktree, `.worktrees/base-<sha>` in the main worktree of the clone, created on first use and
never moved to another commit. Every script and every worktree of the clone that compares with that commit shares it,
together with the builds in it. Worktrees unused for 14 days are removed.

Creating, preparing and removing the worktrees runs under a lock in the git common directory, so that worktrees of one
clone do not race on them.
"""

import fcntl
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PREFIX = "base-"
EXPIRY = 14 * 24 * 60 * 60
# The time of the last use, in the administrative directory of the worktree, so that git removes it with the worktree.
# It also marks the worktrees made here, so that others with a similar name are left alone.
STAMP = "hl-base-last-used"


def git(*args, cwd=ROOT):
    return subprocess.run(["git", "-C", str(cwd), *args], check=True, capture_output=True, text=True).stdout.strip()


def worktrees():
    out = git("worktree", "list", "--porcelain")
    return [Path(line.removeprefix("worktree ")).resolve() for line in out.splitlines() if line.startswith("worktree ")]


def stamp(path):
    return Path(git("rev-parse", "--absolute-git-dir", cwd=path)) / STAMP


def remove(path):
    proc = subprocess.run(["git", "-C", str(ROOT), "worktree", "remove", "--force", str(path)], capture_output=True)
    if proc.returncode != 0:
        print(f"cannot remove {path}: {proc.stderr.decode().strip()}", file=sys.stderr)


def expire(registered, keep):
    for path in registered:
        if path == keep or path.parent != keep.parent or not path.name.startswith(PREFIX):
            continue
        try:
            used = stamp(path).stat().st_mtime
        except FileNotFoundError:
            continue
        if time.time() - used > EXPIRY:
            print(f"removing {path}, unused for 14 days", flush=True)
            remove(path)


def checkout(sha):
    """Returns the worktree of the commit sha, with any local changes in it discarded."""
    common = Path(git("rev-parse", "--path-format=absolute", "--git-common-dir")).resolve()
    path = common.parent / ".worktrees" / f"{PREFIX}{sha[:12]}"
    with open(common / "hl-base-worktrees.lock", "w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        git("worktree", "prune")
        registered = worktrees()
        # Earlier versions of perf-gate kept one worktree for every base, inside the worktree they ran from.
        old = ROOT / ".worktrees" / "perf-gate-base"
        if old in registered:
            print(f"removing {old}, replaced by {path}", flush=True)
            remove(old)
        if path in registered:
            git("checkout", "-q", "--force", "--detach", sha, cwd=path)
            git("clean", "-q", "-d", "--force", cwd=path)
        else:
            path.parent.mkdir(exist_ok=True)
            git("worktree", "add", "-q", "--detach", str(path), sha)
        stamp(path).touch()
        expire(registered, path)
    return path
