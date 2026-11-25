# Contributing to hl

Thank you for your interest in contributing to hl! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

- [Issues vs Discussions](#issues-vs-discussions)
- [Reporting Bugs](#reporting-bugs)
- [Requesting Features](#requesting-features)
- [Contributing Code](#contributing-code)
- [Development Setup](#development-setup)
- [Code Style](#code-style)

## Issues vs Discussions

To keep the issue tracker focused and organized, we use **Issues** for actionable items and **Discussions** for questions and general conversation.

### Quick Reference

| Type                           | Use Issues | Use Discussions |
|--------------------------------|------------|-----------------|
| 🐛 Bug reports                 | ✅ Yes     | ❌ No           |
| ✨ Feature requests            | ✅ Yes     | ❌ No           |
| ❓ Questions ("How do I...?")  | ❌ No      | ✅ Yes          |
| 💡 Ideas & brainstorming       | ❌ No      | ✅ Yes          |
| 💬 General discussions         | ❌ No      | ✅ Yes          |
| 🆘 Help & support              | ❌ No      | ✅ Yes          |

### When to Use Issues

Use **Issues** for:

- **Bug reports**: Something is broken or not working as documented
- **Feature requests**: Concrete proposals for new features or enhancements

### When to Use Discussions

Use **[Discussions](https://github.com/pamburus/hl/discussions)** for:

- **Questions**: How to use hl, configuration help, troubleshooting
- **Ideas**: Brainstorming and discussing potential features before creating a formal request
- **General feedback**: Sharing thoughts, experiences, or use cases
- **Community support**: Getting help from other users

## Reporting Bugs

If you've found a bug, please [create a bug report](https://github.com/pamburus/hl/issues/new?template=bug.yml) using our bug report template.

### Before Reporting a Bug

1. **Search existing issues** to see if the bug has already been reported
2. **Check Discussions** to see if others have encountered the same problem
3. **Update to the latest version** of hl to see if the bug has been fixed
4. **Try to reproduce** the bug with minimal steps

### What to Include in a Bug Report

- **Clear description**: What is the bug?
- **Steps to reproduce**: Detailed steps to reproduce the behavior
- **Expected behavior**: What you expected to happen
- **Actual behavior**: What actually happened
- **Environment details**:
  - hl version (`hl --version`)
  - Operating system and version
  - Terminal emulator
  - Shell (bash, zsh, fish, etc.)
- **Logs or error messages**: Any relevant output or error messages
- **Sample data**: If possible, provide a minimal log file that reproduces the issue

## Requesting Features

If you have an idea for a new feature or enhancement, please [create a feature request](https://github.com/pamburus/hl/issues/new?template=feature.yml) using our feature request template.

### Before Requesting a Feature

1. **Search existing issues and discussions** to see if the feature has been requested
2. **Consider starting a discussion first** if you want feedback on your idea
3. **Think about the use case**: How would this feature benefit you and other users?

### What to Include in a Feature Request

- **Feature description**: Clear and concise description of the feature
- **Motivation and use case**: Why you need this feature and what problem it solves
- **Example usage**: Show how you would use the feature (commands, configuration, output)
- **Alternatives considered**: Any workarounds or alternative solutions you've thought about

## Contributing Code

We welcome code contributions! The rules in this section apply to every change, whether it is made by a person or by an AI agent.

### Rules to Follow

The project [constitution](.specify/memory/constitution.md) is mandatory and takes precedence over this document. Among other things, it defines where tests go, how test data is stored, and when JSON schemas must be updated. These guidelines describe the process in detail:

- [Branches and Versioning](docs/guidelines/branches-and-versioning.md): which branch to target and how to bump the version
- [Commits](docs/guidelines/commits.md): validation before committing, commit message content and issue references

### Getting Started

1. **Fork the repository** and create a new branch off `develop` for your changes
2. **Set up your development environment** (see [Development Setup](#development-setup))
3. **Make your changes** with clear, focused commits
4. **Validate your changes** (see [Before Committing](#before-committing))
5. **Submit a pull request** against `develop` with a clear description of your changes

The `master` branch holds only the latest release and does not accept pull requests. Pull requests opened against it are moved to `develop` automatically.

### Pull Request Guidelines

- **Keep changes focused**: One feature or bug fix per pull request
- **Follow existing code style**: Match the style of the existing codebase
- **Write commit messages about user-visible behavior**: They follow Conventional Commits and generate the release notes, see [Commits](docs/guidelines/commits.md#commit-message-content)
- **Close issues from the commit message**: End the message with a `Closes: #N` footer when the commit resolves issue N. The issue closes when the release containing the commit reaches `master`; a `Closes` line in the pull request description has no effect, because pull requests target `develop`
- **Bump the version** when the change can affect the built program, see [Versioning of pull requests](docs/guidelines/branches-and-versioning.md#versioning-of-pull-requests)
- **Update documentation**: If your changes affect user-facing behavior
- **Add tests**: For new features or bug fixes when applicable
- **Keep the PR description clear**: Explain the problem and your solution

### Unrelated Issues

If you find an issue that is unrelated to the current change (a bug, a flaky test, outdated documentation, a missing check), file it as a GitHub issue instead of fixing it in the current change:

1. Search the existing issues first and comment on a matching issue instead of opening a duplicate.
2. Otherwise, open a new issue. Describe what you observed, how to reproduce it, and where in the code it happens (file and line), so it can be picked up without the context of the current change.

The unrelated issue gets its own branch and pull request.

### Bugs Found by New Tests

When a test your change adds fails, and the bug it shows may be older than your change, check whether the bug is already on the branch your pull request targets: apply only the test on top of the commit your branch is based on, and run it there. If the test uses code that your change adds, rewrite it without that code for the check; if that is not possible, treat the bug as introduced by your change.

- If the test passes there, your change introduces the bug, so fix it in the same pull request.
- If the test fails there as well, the bug is pre-existing. Search the existing issues for it and, if it is not reported yet, [report it](#reporting-bugs) with the test as the steps to reproduce it. Mention the issue in your pull request, and mark the test as expected to fail, with a comment that links the issue, so that it does not block your pull request (see below).

A test marked as expected to fail passes while it panics, and fails once it stops panicking, so the change that fixes the bug has to remove the mark, whether or not it refers to the issue:

```rust
#[test]
// Fails because of https://github.com/pamburus/hl/issues/N.
#[should_panic(expected = "the message of the failing assertion")]
fn test_name() {
    // ...
}
```

- Narrow `expected` down to the message of the failing assertion, so that an unrelated panic does not count as the expected failure. If another assertion of the test could fail with the same message, give the failing one a message of its own.
- A test that returns `Result` cannot have `#[should_panic]`; make it panic on the error instead, for example with `unwrap`.
- For a bug on some platforms only, apply the mark on those platforms, for example `#[cfg_attr(windows, should_panic(expected = "..."))]`.
- A bug that shows only at times, depending on timing, would make such a test flaky, and a bug that aborts the test process instead of panicking cannot be caught at all. Mark such a test with `#[ignore = "https://github.com/pamburus/hl/issues/N"]` instead, and remove the mark with the fix.

### Performance

The `perf-gate` CI job compares the instruction counts of the pull request with those of its base, and fails when they grow by more than 1%. Run `just perf-gate` to get the same comparison locally. For a change that adds cost on purpose, a maintainer adds the `perf-regression-accepted` label. See [perf-gate.py](scripts/README.md#perf-gatepy) for details.

Instruction counts do not show effects of caches, memory access or threads, so wall-clock time is measured with the benchmarks, locally only, because timings on shared CI runners vary too much. Run `just bench-compare` to compare the benchmarks of the working tree with where it forked from `origin/develop`. A criterion filter narrows it down, for example `just bench-compare origin/develop timestamp`. It skips the reference benchmarks, which measure third-party code and language or library primitives. Run it on an otherwise idle machine. See [bench-compare.py](scripts/README.md#bench-comparepy) for details.

For a performance-sensitive change, run `just bench-compare` once the review has settled and the pull request is ready to merge, and post its summary in the pull request.

### Code Review Process

- Maintainers will review your pull request and may suggest changes
- Address review feedback by pushing additional commits to your branch
- Once approved, a maintainer will merge your pull request

## Development Setup

hl is written in Rust. The common tasks are [just](https://just.systems) recipes, which install the tools they need on first use. Run `just` to list them.

### Prerequisites

- [Rust](https://rustup.rs/) via rustup, which picks the toolchain from `rust-toolchain.toml`; formatting additionally uses the nightly toolchain
- [just](https://just.systems/man/en/packages.html)
- [uv](https://docs.astral.sh/uv/getting-started/installation/), which runs the Python scripts under `scripts/`
- [GitHub CLI](https://cli.github.com) (`gh`), authenticated with `gh auth login`
- A Rust-compatible IDE or editor (VS Code with rust-analyzer, IntelliJ IDEA, etc.)

### Building from Source

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/hl.git
cd hl

# Build the project
just build

# Run hl
just run --help
```

### Running Tests

```bash
# Run all tests
just test

# Run the tests whose names contain the given text, with output
just test test-name -- --nocapture
```

### Running Benchmarks

```bash
just bench
```

## Code Style

- Format the code with `just fmt`. Rust code is formatted with nightly `rustfmt`, because `rustfmt.toml` uses unstable options; stable `cargo fmt` ignores them, including the exclusion of generated code
- Resolve all Clippy warnings; `just lint` runs Clippy on all targets with warnings denied
- Write clear, descriptive variable and function names
- Add comments for complex logic or non-obvious behavior

### Before Committing

Run all checks of the CI workflow locally and resolve everything they report:

```bash
just ci
```

After a change to the code, also check that the changed lines are covered by tests:

```bash
just uncovered
```

See [Validation](docs/guidelines/commits.md#validation) for what `just ci` runs.

## Questions?

If you have any questions about contributing, please:

- Check the [Discussions](https://github.com/pamburus/hl/discussions) to see if your question has been answered
- Start a new discussion in the [Q&A category](https://github.com/pamburus/hl/discussions/categories/q-a)

Thank you for contributing to hl! 🎉
