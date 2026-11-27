#!/usr/bin/env python3

"""Compute and check the release-line version for routine dependency updates."""

import argparse
from pathlib import Path
import re
import subprocess
import tempfile
import tomllib


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], text=True)


def changed(base: str, *paths: str) -> bool:
    return bool(git("diff", "--name-only", base, "HEAD", "--", *paths))


def manifest_version(contents: str) -> str:
    return tomllib.loads(contents)["workspace"]["package"]["version"]


def base_version(base: str) -> str:
    return manifest_version(git("show", f"{base}:Cargo.toml"))


def next_version(version: str) -> str:
    match = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)(?:-alpha\.(\d+))?", version)
    if not match:
        raise ValueError(f"Unsupported release-line version: {version}")
    major, minor, patch, alpha = match.groups()
    if alpha is None:
        return f"{major}.{minor}.{int(patch) + 1}-alpha.1"
    return f"{major}.{minor}.{patch}-alpha.{int(alpha) + 1}"


def release_dependencies(root: Path) -> str:
    tree = subprocess.check_output(
        [
            "cargo", "tree", "--locked", "--package", "hl", "--edges", "normal,build",
            "--target", "all", "--all-features", "--prefix", "depth", "--format", "{p} {f}",
            "--color", "never",
        ],
        cwd=root,
        text=True,
    )
    # Skip the root's own version and use relative identities for workspace crates.
    return tree.partition("\n")[2].replace(f" ({root.resolve()}/", " (./")


def requires_bump(base: str) -> bool:
    if changed(base, "rust-toolchain.toml", "flake.lock", "Cross.toml"):
        return True
    if not changed(base, ":(glob)**/Cargo.toml", "Cargo.lock"):
        return False

    with tempfile.TemporaryDirectory() as temp:
        baseline = Path(temp) / "base"
        git("worktree", "add", "--quiet", "--detach", str(baseline), base)
        try:
            return release_dependencies(baseline) != release_dependencies(Path.cwd())
        finally:
            git("worktree", "remove", "--force", str(baseline))


def write_version(previous: str, updated: str) -> None:
    files = {
        "Cargo.toml": r'(?m)(^\[workspace\.package\]\n[^\[]*?^version = ")',
        "Cargo.lock": r'(?m)(^\[\[package\]\]\nname = "hl"\nversion = ")',
    }
    result = {}
    for filename, prefix in files.items():
        path = Path(filename)
        if not path.is_file() or path.is_symlink():
            raise ValueError(f"Expected a regular file at {path}")
        contents = path.read_bytes().decode()
        pattern = prefix + re.escape(previous) + r'(")$'
        updated_contents, count = re.subn(pattern, rf"\g<1>{updated}\2", contents)
        if count != 1:
            raise ValueError(f"Expected one hl version entry in {path}")
        result[path] = updated_contents.encode()
    for path, contents in result.items():
        path.write_bytes(contents)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("prepare", "check", "apply"))
    parser.add_argument("base", help="Commit on the PR target branch")
    args = parser.parse_args()

    base = base_version(args.base)
    current = manifest_version(Path("Cargo.toml").read_text())
    if args.mode == "apply":
        # The publisher receives only a request to bump. Derive the value here.
        if current != base:
            raise SystemExit(f"Expected hl version {base} before applying the bump, found {current}")
        write_version(base, next_version(base))
        return

    expected = next_version(base) if requires_bump(args.base) else base
    if args.mode == "check":
        if current != expected:
            raise SystemExit(f"::error::Expected hl version {expected} for this dependency update, found {current}.")
    elif current == base and expected != base:
        print(args.base)


if __name__ == "__main__":
    main()
