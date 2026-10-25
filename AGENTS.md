# hl Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-02-04

## Constitution

Before starting any work, load and follow the project constitution at
[`.specify/memory/constitution.md`](.specify/memory/constitution.md).
Its principles are mandatory and take precedence over the guidelines below.

Also follow [`docs/guidelines/branches-and-versioning.md`](docs/guidelines/branches-and-versioning.md) for branches and version bumps, and [`docs/guidelines/commits.md`](docs/guidelines/commits.md) for commits.

## Unrelated Issues

If you find an issue that is unrelated to the current task while implementing or fixing something (a bug, a flaky test, outdated documentation, a missing check), file it as a GitHub issue instead of fixing it in the current change:

1. Search the existing issues first (`gh issue list --search "<keywords>" --state all`) and comment on a matching issue instead of opening a duplicate.
2. Otherwise, open a new issue with `gh issue create`. Describe what you observed, how to reproduce it, and where in the code it happens (file and line), so it can be picked up without the context of the current task.
3. Mention the filed issue in your report to the user.

Keep the current change focused on its own task. The unrelated issue gets its own branch and pull request.

## Active Technologies

- Rust (stable, edition 2024) + serde (config deserialization), clap (CLI argument parsing) (009-key-prettification)

## Project Structure

```text
src/
tests/
```

## Commands

- cargo test
- cargo clippy

## Code Style

Rust (stable, edition 2024): Follow standard conventions

## Recent Changes

- 009-key-prettification: Added Rust (stable, edition 2024) + serde (config deserialization), clap (CLI argument parsing)

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
