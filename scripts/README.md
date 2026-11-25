# Coverage Analysis Scripts

This directory contains tools for analyzing test coverage in the HL project.

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
