"""Worktrees and builds of base revisions, for the scripts that compare a build of the working tree with one of a base.

Each base revision has one worktree, `.worktrees/base-<sha>` in the main worktree of the clone, created on first use and
never moved to another commit. Every script and every worktree of the clone that compares with that commit shares it.
Worktrees unused for 14 days are removed.

The base worktrees take turns in one target directory, `target/base` in the main worktree, so that dependencies are
built once, not once per base. It stays separate from the target directories of the heads, because cargo gives a
workspace crate the same identity wherever its source is: a head build would take the crates built for a base.

Creating, preparing, building and removing the worktrees runs under a lock in the git common directory, so that
worktrees of one clone do not race on them.
"""

import fcntl
import os
import subprocess
import sys
import time
from contextlib import contextmanager
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PREFIX = "base-"
EXPIRY = 14 * 24 * 60 * 60
LOCK = "hl-base-worktrees.lock"
# The time of the last use, in the administrative directory of the worktree, so that git removes it with the worktree.
# It also marks the worktrees made here, so that others with a similar name are left alone.
USED = "hl-base-last-used"
# The revision built last in a base target directory.
BUILT = "hl-base-revision"


def git(*args, cwd=ROOT):
    return subprocess.run(["git", "-C", str(cwd), *args], check=True, capture_output=True, text=True).stdout.strip()


def common_dir():
    return Path(git("rev-parse", "--path-format=absolute", "--git-common-dir")).resolve()


def target_dir():
    return common_dir().parent / "target" / "base"


@contextmanager
def locked(path):
    with open(path, "w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        yield


def worktrees():
    out = git("worktree", "list", "--porcelain")
    return [Path(line.removeprefix("worktree ")).resolve() for line in out.splitlines() if line.startswith("worktree ")]


def used(path):
    return Path(git("rev-parse", "--absolute-git-dir", cwd=path)) / USED


def remove(path):
    proc = subprocess.run(["git", "-C", str(ROOT), "worktree", "remove", "--force", str(path)], capture_output=True)
    if proc.returncode != 0:
        print(f"cannot remove {path}: {proc.stderr.decode().strip()}", file=sys.stderr)


def expire(registered, keep):
    for path in registered:
        if path == keep or path.parent != keep.parent or not path.name.startswith(PREFIX):
            continue
        try:
            last = used(path).stat().st_mtime
        except FileNotFoundError:
            continue
        if time.time() - last > EXPIRY:
            print(f"removing {path}, unused for 14 days", flush=True)
            remove(path)


def checkout(sha):
    """Returns the worktree of the commit sha, with any local changes in it discarded."""
    common = common_dir()
    path = common.parent / ".worktrees" / f"{PREFIX}{sha[:12]}"
    with locked(common / LOCK):
        git("worktree", "prune")
        registered = worktrees()
        # Earlier versions of perf-gate kept one worktree for every base, inside the worktree they ran from.
        old = ROOT / ".worktrees" / "perf-gate-base"
        if old in registered:
            print(f"removing {old}, replaced by {path}", flush=True)
            remove(old)
        if path in registered:
            # Otherwise the checkout rewrites the files touched for a build, and cargo rebuilds them. Files that differ
            # from the index make it fail, the checkout restores them.
            subprocess.run(["git", "-C", str(path), "update-index", "-q", "--refresh"], capture_output=True)
            git("checkout", "-q", "--force", "--detach", sha, cwd=path)
            git("clean", "-q", "-d", "--force", cwd=path)
        else:
            path.parent.mkdir(exist_ok=True)
            git("worktree", "add", "-q", "--detach", str(path), sha)
        used(path).touch()
        expire(registered, path)
    return path


def changed(git_dir, last, sha):
    # The files that differ between the revision built last and this one. Files that exist only in the last one need
    # nothing, because cargo treats a missing source file as changed. Without a last revision, or with one this clone
    # does not have, all files count as changed.
    cmd = ["git", "-c", "safe.directory=*", "--git-dir", str(git_dir)]
    if last:
        proc = subprocess.run([*cmd, "diff", "--name-only", "--no-renames", "-z", last, sha], capture_output=True)
        if proc.returncode == 0:
            return proc.stdout.decode().split("\0")
    proc = subprocess.run([*cmd, "ls-tree", "-r", "--name-only", "-z", sha], check=True, capture_output=True)
    return proc.stdout.decode().split("\0")


def build(src, sha, run, target=None, git_dir=None, lock=None):
    """Runs run(src, target) for the worktree src of the commit sha, under the lock, and returns what it returns.

    Cargo checks the source files of a workspace crate by modification time only, relative to the package root. A base
    worktree whose files are older than the artifacts built from another one would silently reuse those artifacts, even
    where the content differs. So before a build of another revision, the files that differ from the revision built
    last are touched, and cargo rebuilds only the crates whose source changed and the crates that depend on them.

    The target directory, the git directory to compare the revisions in and the lock default to those of this clone.
    """
    target = target or target_dir()
    git_dir = git_dir or common_dir()
    with locked(lock or git_dir / LOCK):
        stamp = target / BUILT
        last = stamp.read_text().strip() if stamp.exists() else None
        if last != sha:
            if last or target.exists():
                for name in changed(git_dir, last, sha):
                    if name and (src / name).is_file():
                        os.utime(src / name)
            # Until the build completes, the target directory holds crates of both revisions.
            stamp.unlink(missing_ok=True)
        result = run(src, target)
        stamp.write_text(sha)
        return result
