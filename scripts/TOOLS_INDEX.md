# Test Reorganization Tools - Index

This document serves as an index and guide to the test reorganization tools in this project.

## Overview

The test reorganization tools help you move inline unit tests from Rust source files into separate `tests.rs` files organized by module structure. This improves code organization and readability.

## Files in This Directory

### Executable Scripts

1. **reorganize_tests.sh** - Bash wrapper script
   - Easy-to-use interface
   - Colored output and nice formatting
   - Recommended for most users
   - Run with: `./scripts/reorganize_tests.sh --help`

2. **reorganize_tests.py** - Python implementation
   - Core tool that does the actual work
   - Fully documented with comments
   - Can be used directly or via the wrapper
   - Run with: `python3 scripts/reorganize_tests.py --help`

### Documentation

1. **TOOLS_INDEX.md** (this file)
   - Overview of all tools and documents
   - Roadmap for different use cases

2. **QUICK_START.md**
   - 5-minute quick reference
   - Essential commands and workflow
   - Good for getting started quickly
   - Read if: You want to get started NOW

3. **TEST_REORGANIZATION.md**
   - Comprehensive usage guide
   - Examples and transformations
   - Troubleshooting guide
   - Read if: You want a complete overview

4. **REORGANIZE_TESTS_README.md**
   - Technical documentation
   - Implementation details
   - Advanced usage and edge cases
   - Read if: You want to understand how it works

## Quick Start

If you have 2 minutes:
```bash
./scripts/reorganize_tests.sh --dry-run
./scripts/reorganize_tests.sh
cargo test
```

If you have 5 minutes:
- Read: QUICK_START.md
- Then run the commands above

If you have 15 minutes:
- Read: TEST_REORGANIZATION.md
- Review the transformation examples
- Run with --dry-run to see the actual impact

## Choosing the Right Document

### "I just want to use this tool"
→ Read: **QUICK_START.md**

### "I need a comprehensive guide"
→ Read: **TEST_REORGANIZATION.md**

### "I want to understand the implementation"
→ Read: **REORGANIZE_TESTS_README.md**

### "I want to know what all the files are"
→ Read: **TOOLS_INDEX.md** (this file)

## Common Workflows

### Workflow 1: Just Run It (5 minutes)

```bash
# Preview changes
./scripts/reorganize_tests.sh --dry-run

# Apply if satisfied
./scripts/reorganize_tests.sh

# Verify tests work
cargo test
```

### Workflow 2: Conservative Approach (15 minutes)

```bash
# Create checkpoint
git add -A && git commit -m "checkpoint"

# Preview changes
./scripts/reorganize_tests.sh --dry-run

# Review carefully, then apply
./scripts/reorganize_tests.sh

# Verify and review
cargo test
git diff

# Commit if satisfied
git add -A && git commit -m "reorganize tests"
```

### Workflow 3: Custom Directory (10 minutes)

```bash
# For non-standard source locations
./scripts/reorganize_tests.sh --dry-run /path/to/src

# Apply changes
./scripts/reorganize_tests.sh /path/to/src

# Verify
cargo test
```

## What the Tool Does

### Input
- Scans Rust source files for `#[cfg(test)] mod tests { }` blocks
- Finds all inline test modules in the project

### Processing
- Extracts test module contents
- Creates appropriate subdirectory structures
- Replaces original test modules with `include!()` references

### Output
- New `{module}/tests.rs` files with extracted tests
- Modified source files with `include!()` macros
- Preserved test functionality and access levels

## Example Transformation

### Before
```rust
// src/app.rs - 600 lines (includes 300 lines of tests)
pub fn process() { }

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_process() { }
}
```

### After
```rust
// src/app.rs - 300 lines (tests removed)
pub fn process() { }

#[cfg(test)]
mod tests {
    include!("../app/tests.rs");
}
```

```rust
// src/app/tests.rs - NEW FILE - 300 lines
use super::*;

#[test]
fn test_process() { }
```

## Key Features

✓ **Preview Mode** - Use `--dry-run` to see changes before applying  
✓ **Automatic** - Finds and reorganizes all test modules  
✓ **Safe** - Uses `include!()` macro to preserve test functionality  
✓ **Reversible** - Easy to undo with git  
✓ **No Compilation** - Fast, doesn't require building  
✓ **Recursive** - Handles nested modules and subdirectories  

## Prerequisites

- Python 3.6+
- Bash shell
- A Rust project with inline test modules
- Git (for safety and reverting)

## Getting Started

1. **Read** QUICK_START.md (5 minutes)
2. **Run** `./scripts/reorganize_tests.sh --dry-run`
3. **Review** the output
4. **Run** `./scripts/reorganize_tests.sh` (if preview looks good)
5. **Verify** with `cargo test`
6. **Commit** your changes

## Help and Support

### Built-in Help

```bash
./scripts/reorganize_tests.sh --help
python3 scripts/reorganize_tests.py --help
```

### Troubleshooting

1. **Nothing found?** - Check that `.rs` files exist and have test modules
2. **Tests fail?** - This shouldn't happen, but run `cargo test` for details
3. **Want to undo?** - Run `git checkout src/`

### Documentation

- QUICK_START.md - Quick reference
- TEST_REORGANIZATION.md - Complete guide
- REORGANIZE_TESTS_README.md - Technical details

## Project Statistics

When applied to this project:

- **42 Rust files** scanned
- **23 test modules** found
- **~10,000 lines** of test code to reorganize
- **Estimated time** to run: <1 second (dry-run), ~2-5 seconds (actual)

Example reorganizations:
- `src/app.rs`: 1654 lines → ~400 lines + app/tests.rs
- `src/model.rs`: 2640 lines → ~800 lines + model/tests.rs
- `src/scanning.rs`: 1264 lines → ~400 lines + scanning/tests.rs

## Advanced Usage

### Custom Source Directory
```bash
./scripts/reorganize_tests.sh --dry-run /custom/path
./scripts/reorganize_tests.sh /custom/path
```

### Verbose Output
```bash
./scripts/reorganize_tests.sh -v --dry-run
```

### Using Python Directly
```bash
python3 scripts/reorganize_tests.py --dry-run src/
python3 scripts/reorganize_tests.py src/
```

### Batch Processing
```bash
for project in ~/projects/*/; do
    cd "$project"
    ./scripts/reorganize_tests.sh --dry-run
    ./scripts/reorganize_tests.sh
    cargo test
done
```

## Decision Tree

### Should I run this tool?

- Do you have inline test modules in source files? **→ Yes, run it**
- Do you want to organize tests separately? **→ Yes, run it**
- Do you have integration tests in tests/ directory? **→ This tool doesn't affect them**
- Do you have doc tests? **→ This tool doesn't affect them**

### How to run it?

- First time? **→ Use --dry-run**
- Sure about changes? **→ Run without --dry-run**
- Multiple projects? **→ Run for each project**
- Custom path? **→ Specify the path as argument**

### After running?

- Tests pass? **→ Commit the changes**
- Tests fail? **→ Revert with git checkout**
- Want more changes? **→ Run on different directory**

## Safety Summary

| Step | Safety | Reversible |
|------|--------|-----------|
| `--dry-run` | Very safe | N/A |
| Apply changes | Safe (uses include!) | Yes (git checkout) |
| Manual changes | Depends | Check git status |

**Always:** Use --dry-run first, commit before applying, verify with cargo test

## File Organization After Running

### Before
```
src/
├── app.rs (contains tests)
├── cli.rs (contains tests)
└── model.rs (contains tests)
```

### After
```
src/
├── app.rs (references tests)
├── app/
│   └── tests.rs
├── cli.rs (references tests)
├── cli/
│   └── tests.rs
├── model.rs (references tests)
└── model/
    └── tests.rs
```

## Integration with Development

### Before Committing
```bash
./scripts/reorganize_tests.sh --dry-run
./scripts/reorganize_tests.sh
cargo test
git diff
```

### Before Merging
- Ensure all tests pass after reorganization
- Review the diff to ensure tests moved correctly
- Verify no test functionality changed

### In CI/CD
- Run normally after merge: `cargo test`
- No special configuration needed
- Include! macro preserves test discovery

## Common Questions

**Q: Do I need to rewrite my tests?**
A: No, tests move exactly as they are.

**Q: Will my tests still work?**
A: Yes, `include!()` preserves all functionality.

**Q: Can I undo this?**
A: Yes, `git checkout src/` restores everything.

**Q: Can I run this multiple times?**
A: Yes, but it's designed to run once. Running again will try to reorganize already-reorganized tests.

**Q: What if my source directory isn't `src/`?**
A: Specify the correct path: `./scripts/reorganize_tests.sh /your/path`

## Next Steps

1. **Read:** QUICK_START.md (5 minutes)
2. **Preview:** `./scripts/reorganize_tests.sh --dry-run`
3. **Review:** Output carefully
4. **Apply:** `./scripts/reorganize_tests.sh`
5. **Test:** `cargo test`
6. **Commit:** `git add -A && git commit -m "reorganize tests"`

## Document Summaries

### QUICK_START.md
- **Length:** ~5 minutes
- **Level:** Beginner
- **Contains:** Essential commands, workflow, TL;DR
- **Best for:** Getting started quickly

### TEST_REORGANIZATION.md
- **Length:** ~15 minutes
- **Level:** Intermediate
- **Contains:** Examples, transformations, troubleshooting
- **Best for:** Comprehensive overview

### REORGANIZE_TESTS_README.md
- **Length:** ~20 minutes
- **Level:** Advanced
- **Contains:** Technical details, edge cases, implementation
- **Best for:** Understanding how it works

### TOOLS_INDEX.md (this file)
- **Length:** ~10 minutes
- **Level:** All levels
- **Contains:** Overview, navigation, decision trees
- **Best for:** Finding the right document

## Ready to Get Started?

Pick your path:

**Path 1: Quick Start (2 minutes)**
```bash
./scripts/reorganize_tests.sh --dry-run
./scripts/reorganize_tests.sh
```

**Path 2: Safe Approach (10 minutes)**
- Read QUICK_START.md
- Follow the workflow section
- Run with --dry-run
- Review output
- Apply changes

**Path 3: Complete Understanding (20 minutes)**
- Read TEST_REORGANIZATION.md
- Review examples
- Understand transformations
- Run with --dry-run
- Apply changes carefully

**Path 4: Deep Dive (30 minutes)**
- Read all documentation
- Study implementation details
- Review source code
- Test with --dry-run on test project
- Apply to main project

## Support

- **Quick questions:** See QUICK_START.md
- **How-to questions:** See TEST_REORGANIZATION.md
- **Technical questions:** See REORGANIZE_TESTS_README.md
- **Getting help:** Run `./scripts/reorganize_tests.sh --help`

---

**Start here:** `./scripts/reorganize_tests.sh --dry-run`

Good luck! 🚀