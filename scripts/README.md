# Scripts

This directory contains development tools for the HL project.

## stress-test.py

Runs a built `hl` on large inputs in each of its modes (`cat`, sorting a file, stdin or a gzip file, following stdin or a file). Each case runs at two sizes, the second 4 times the first, and checks three things:

- **Output**: exactly what the input should give
- **CPU time**: the ratio of the two runs stays within what the case allows, so a change from linear to quadratic fails
- **Peak memory**: the ratio of the two runs stays within what the case allows, so data that cannot be parsed does not pile up

Comparing two runs of the same build makes the checks independent of how fast the machine is.

Cases and size profiles live in [`stress-test.toml`](stress-test.toml). A check that fails because of an open issue is listed under the case's `known` table; it does not fail the run, and once it passes the run fails until the entry is removed. An entry for a check that fails only at times, depending on timing, is marked as intermittent, and passing does not fail the run.

### Running

```bash
# Quick profile, under a minute (also part of `just ci` and the CI workflow)
just stress-test

# Full profile, with larger lines, limits and inputs; takes several minutes
just stress-test full

# Narrow down to one case or mode, or override a profile setting
just stress-test full --case long-line --mode follow-file --set limit=256MiB
```

The script itself takes the binary with `--hl`, so it can also check another build, for example a release:

```bash
./scripts/stress-test.py --hl ~/.cargo/bin/hl --profile quick
```

Requires Python 3.11+ and a Unix-like system.

## perf-gate.py

Compares the number of instructions two builds of `hl` execute on the same inputs, to catch changes that make it slower by a constant factor, which the stress test does not see. By default it compares the working tree with the commit where it forked from `origin/develop`.

- **Inputs**: the records from [`sample`](../sample), repeated with new timestamps, as JSON and as logfmt
- **Modes**:
  - `cat`
  - sorting a file, a gzip file and stdin
  - sorting two files together
  - sorting a file that is already indexed
  - `cat` and sorting a file with 4 processing threads
  - following stdin
- **Counts**: from cachegrind, with one processing thread unless the mode sets more and an empty cache unless the mode needs an index, so repeated runs differ by well under 1%
- **Runs**: once per input and mode on each build by default, as many at a time as there are CPUs; with `--repeat N`, the medians are compared
- **Check**: fails when the head exceeds the base by more than 2% in any input and mode

Following a file is left out, because how much `hl` reads per wakeup depends on timing.

In pull requests, the `perf-gate` job of the CI workflow runs it on the release build of the stress-test job, against the tip of the base branch, and shows the counts in the job summary and in a comment on the pull request, updated on each run. Pull requests from forks get the job summary only, because their token cannot comment. The release build of each push to a release line is cached for the pull requests based on it, so usually nothing else is built. For a change that adds cost on purpose, add the `perf-regression-accepted` label and re-run the job: it then reports the counts without failing.

### Comparing builds

```bash
# Compare with where the branch forked from origin/develop
just perf-gate

# Compare with another revision, in some modes only
just perf-gate v0.36.5 --mode sort

# Compare two builds directly
./scripts/perf-gate.py --base-hl path/to/base/hl --hl path/to/head/hl
```

The base is checked out in the `.worktrees/perf-gate-base` worktree, which is kept for the next run so that the base builds incrementally. Local changes in it are discarded.

Requires Python 3 and valgrind, which runs on Linux only. Elsewhere, the script runs itself in a Docker container, keeping the Linux builds and toolchains in Docker volumes.

## coverage-diff-analysis.py

A comprehensive tool that compares test coverage between two Git commits to identify exactly which changed lines lack test coverage.

### Features

- **Full Workspace Support**: Analyzes coverage across the entire Rust workspace, including all crates
- **Precise Line Analysis**: Shows exact line numbers and ranges for changed and uncovered code
- **Intersection Detection**: Identifies the specific lines that were changed but remain uncovered
- **Automatic Coverage Generation**: Runs `just coverage` or `make coverage` automatically
- **Git Integration**: Safely switches between commits and restores original HEAD
- **Actionable Reports**: Provides clear summary with coverage gap percentages

### Usage

```bash
./scripts/coverage-diff-analysis.py <base>
```

**Note**: The script analyzes changes from the old commit to your current working directory, including both staged and unstaged changes. This means you can analyze coverage impact before committing!

### Example

```bash
# Compare current working directory (including uncommitted changes) with a specific commit
./scripts/coverage-diff-analysis.py bcce60036a98e08987669d0339c362dc893cae75

# Compare current working directory with main branch
./scripts/coverage-diff-analysis.py main

# Analyze your feature branch changes before committing
git checkout feature-branch
./scripts/coverage-diff-analysis.py main
```

### Output

The script generates a comprehensive report with three main sections:

1. **Changed Ranges**: All lines modified between the old commit and working directory (including uncommitted changes)
2. **Uncovered Ranges**: All lines not covered by tests in the current working directory
3. **Intersection**: Lines that were changed but remain uncovered (these need tests!)

### Example Output

```
📝 CHANGED RANGES (by file)
src/app.rs: 8 23 143 225 257-258 (5 lines)
src/model.rs: 479 576-579 601-603 (7 lines)

🔍 UNCOVERED RANGES (by file)
src/app.rs: 239 350-362 447 501 (15 lines)
src/model.rs: 479 577 601 (3 lines)

🎯 INTERSECTION: CHANGED BUT UNCOVERED
src/app.rs: 257-258 (2 lines)
src/model.rs: 479 577 601 (3 lines)

📋 SUMMARY
Coverage gap in changed code: 5/12 lines (41.7%)
```

### Requirements

- Python 3.6+
- Git repository with the specified old commit
- Current working directory with code you want to analyze for coverage (including any uncommitted changes)
- `just coverage` or `make coverage` command that generates `target/lcov.info`

### Use Cases

- **Pre-commit Analysis**: Check coverage gaps in your uncommitted changes before committing
- **Development Workflow**: Identify exactly which lines in your working changes need test coverage
- **Interactive Development**: Run analysis while coding to see real-time coverage impact
- **Avoiding Test Duplication**: Focus testing efforts only on uncovered changed lines
- **Coverage Quality**: Distinguish between meaningful coverage improvements and "test profanation"

### Integration

This tool is designed to be part of your development workflow:

1. **During Development**: Run analysis on your current branch to see what needs testing
2. **Before Committing**: Ensure your changes have adequate test coverage
3. **Code Review**: Include coverage analysis in review process
4. **CI/CD**: Compare feature branches against main branch for coverage validation

### Benefits of Working Directory Analysis

- **No Git State Changes**: Analyzes your current working directory without any checkouts
- **Includes Uncommitted Changes**: See coverage impact of your work-in-progress code
- **Simple Usage**: Just specify the base commit to compare against
- **Safe Operation**: No risk of losing uncommitted changes or getting into detached HEAD state
- **Interactive Development**: Perfect for iterative development and testing
- **Fast Analysis**: Uses current coverage data without rebuilding/retesting
