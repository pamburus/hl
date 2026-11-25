# hl Agent Guidelines

## Contributing Rules

Before every change, read [`CONTRIBUTING.md`](CONTRIBUTING.md) and every document it lists under [Rules to Follow](CONTRIBUTING.md#rules-to-follow), and follow them. They apply to agents exactly as to human contributors. Read them again for each new change, even within the same session, because they may have changed since you last read them.

## Agent-Specific Rules

- When you file an unrelated issue as described in [Unrelated Issues](CONTRIBUTING.md#unrelated-issues), search with `gh issue list --search "<keywords>" --state all`, create it with `gh issue create`, and mention it in your report to the user.

### Pre-existing Bugs Found by New Tests

This procedure is for maintainers only, who push branches to this repository: `gh repo view --json viewerPermission` shows `ADMIN`, `MAINTAIN` or `WRITE`. When working from a fork, skip it and follow only [Bugs Found by New Tests](CONTRIBUTING.md#bugs-found-by-new-tests).

When the check in [Bugs Found by New Tests](CONTRIBUTING.md#bugs-found-by-new-tests) shows that the bug is on the base of the pull request, run the test on the latest release tag reachable from that base as well, leaving out pre-releases, which version sort puts above their release (`git tag -l "v*.*.*" --merged <base> --sort=-version:refname | grep -v -e - | head -1`). The base can be `develop`, a `release/*` branch or a feature branch; the procedure is the same.

- **Released**, the test fails on that tag too: start a sub-agent that reports the bug, and continue the current work without waiting for it. Once the issue exists, mark the test as expected to fail with a link to it, as described in [Bugs Found by New Tests](CONTRIBUTING.md#bugs-found-by-new-tests). The sub-agent searches the existing issues, then files the bug with the test as the steps to reproduce it, noting which of `master` and the `release/*` branches have it. It may use a temporary worktree to collect that evidence and removes the worktree at the end. It neither looks for the cause nor fixes the bug.
- **Unreleased**, the test passes on that tag: the bug came with a pull request merged after the release, and is fixed before the current work continues:
  1. Start a sub-agent that reports the bug the same way, then finds the commit that introduced it with `git bisect start --first-parent <base> <tag>`, so that each step is one pull request. If bisect ends at a merge commit, such as a merge of `develop` into a feature branch, it continues inside the merged branch, between the merge base and the second parent.
  2. The sub-agent fixes the bug, with the test, on a new branch from the target branch, and opens a pull request against that branch. The commit closes the reported issue and has a `Regressed-by: #N` footer with the pull request that landed the introducing commit, see [Issue references](docs/guidelines/commits.md#issue-references). The pull request description links that pull request too.
  3. The sub-agent handles all reviews, comments and failed checks of its pull request until it is merged.
  4. Meanwhile, run `scripts/wait-pr.py <fix-pr> <target-branch>` in the background, so that you are woken only when it exits, instead of polling yourself. It exits with:
     - 3 on new activity or a merge conflict: resume the sub-agent to handle it, for example with a message to it. Once it has finished, run the script again with `--since` and the time it printed, or, after a conflict, with the `--since` it last had. Running it before then would report the conflict, or activity that is already being handled, again at once.
     - 0 when the pull request is merged: rebase the current branch onto the target branch, drop the copy of the test that the fix already added, and continue the current work.
     - 1 or 2 when the pull request is closed, retargeted or not merged within 24 hours, or another code when the script fails: stop and report to the user.
