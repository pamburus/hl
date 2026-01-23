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

We welcome code contributions! Here's how to get started:

### Getting Started

1. **Fork the repository** and create a new branch for your changes
2. **Set up your development environment** (see [Development Setup](#development-setup))
3. **Make your changes** with clear, focused commits
4. **Test your changes** thoroughly
5. **Submit a pull request** with a clear description of your changes

### Pull Request Guidelines

- **Keep changes focused**: One feature or bug fix per pull request
- **Follow existing code style**: Match the style of the existing codebase
- **Write clear commit messages**: Describe what and why, not just how
- **Update documentation**: If your changes affect user-facing behavior
- **Add tests**: For new features or bug fixes when applicable
- **Keep the PR description clear**: Explain the problem and your solution

### Code Review Process

- Maintainers will review your pull request and may suggest changes
- Address review feedback by pushing additional commits to your branch
- Once approved, a maintainer will merge your pull request

## Development Setup

hl is written in Rust. Here's how to set up your development environment:

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable version recommended)
- A Rust-compatible IDE or editor (VS Code with rust-analyzer, IntelliJ IDEA, etc.)

### Building from Source

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/hl.git
cd hl

# Build the project
cargo build

# Run tests
cargo test

# Run hl
cargo run -- --help
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run a specific test
cargo test test_name
```

### Running Benchmarks

```bash
cargo bench
```

## Code Style

- Follow Rust's official [style guidelines](https://doc.rust-lang.org/1.0.0/style/)
- Use `cargo fmt` to format your code before committing
- Use `cargo clippy` to catch common mistakes and improve your code
- Write clear, descriptive variable and function names
- Add comments for complex logic or non-obvious behavior

### Before Committing

Run these commands to ensure your code meets the project's standards:

```bash
# Format code
cargo fmt

# Check for common mistakes
cargo clippy

# Run tests
cargo test
```

## Questions?

If you have any questions about contributing, please:

- Check the [Discussions](https://github.com/pamburus/hl/discussions) to see if your question has been answered
- Start a new discussion in the [Q&A category](https://github.com/pamburus/hl/discussions/categories/q-a)

Thank you for contributing to hl! 🎉
