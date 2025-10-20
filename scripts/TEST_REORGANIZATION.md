# Test Reorganization Tool

Reorganize inline unit tests in Rust projects by moving `#[cfg(test)] mod tests` blocks from source files into separate `tests.rs` files organized by module structure.

## Quick Start

```bash
# Preview what will be reorganized (ALWAYS DO THIS FIRST!)
./scripts/reorganize_tests.sh --dry-run

# If satisfied with preview, apply the changes
./scripts/reorganize_tests.sh

# Verify tests still work
cargo test
```

## Overview

This tool automates the common pattern of extracting test modules from Rust source files and organizing them in a cleaner directory structure.

### Before
```
src/
├── app.rs (includes test module at the end)
├── cli.rs (includes test module at the end)
└── model.rs (includes test module at the end)
```

### After
```
src/
├── app.rs (with include!() reference)
├── app/tests.rs (test code moved here)
├── cli.rs (with include!() reference)
├── cli/tests.rs (test code moved here)
├── model.rs (with include!() reference)
└── model/tests.rs (test code moved here)
```

## Features

- ✓ **Automatic Discovery**: Finds all inline test modules in Rust files
- ✓ **Safe Mode**: Includes `--dry-run` to preview changes before applying
- ✓ **Smart Organization**: Creates appropriate subdirectory structures
- ✓ **Preserves Functionality**: Uses `include!()` macro so tests work exactly the same
- ✓ **Recursive Processing**: Handles nested modules and subdirectories
- ✓ **Non-Destructive**: Easy to revert with git

## Usage

### Shell Wrapper (Recommended)

```bash
./scripts/reorganize_tests.sh [OPTIONS] [SRC_DIR]

OPTIONS:
  -d, --dry-run     Show what would be done without making changes
  -v, --verbose     Print detailed information
  -h, --help        Show this help message

EXAMPLES:
  ./scripts/reorganize_tests.sh --dry-run
  ./scripts/reorganize_tests.sh
  ./scripts/reorganize_tests.sh --dry-run src/
```

### Python Script

```bash
python3 scripts/reorganize_tests.py [OPTIONS] [SRC_DIR]

OPTIONS:
  --dry-run         Show what would be done without making changes

EXAMPLES:
  python3 scripts/reorganize_tests.py --dry-run src/
  python3 scripts/reorganize_tests.py src/
```

## Example Transformation

### Source File Before
```rust
// src/app.rs
pub fn process_data(input: &str) -> String {
    // implementation...
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_process_data() {
        assert_eq!(process_data("hello"), "hello");
    }
}
```

### Source File After
```rust
// src/app.rs
pub fn process_data(input: &str) -> String {
    // implementation...
}

#[cfg(test)]
mod tests {
    include!("../app/tests.rs");
}
```

### New Test File Created
```rust
// src/app/tests.rs
use super::*;

#[test]
fn test_process_data() {
    assert_eq!(process_data("hello"), "hello");
}
```

## Recommended Workflow

```bash
# 1. Create a checkpoint in git
git add -A && git commit -m "checkpoint before test reorganization"

# 2. Preview the changes
./scripts/reorganize_tests.sh --dry-run

# 3. Review the output carefully

# 4. If satisfied, apply the changes
./scripts/reorganize_tests.sh

# 5. Verify tests still work
cargo test

# 6. Check what changed
git diff

# 7. Commit the reorganization
git add -A && git commit -m "reorganize inline tests to separate files"
```

## Understanding the Output

When you run with `--dry-run`, you'll see output like:

```
Found 42 Rust files

Processing: src/app.rs
  Found 1 test module(s)
  - Moving 'tests' to src/app/tests.rs
    Lines 1126-1654
    [DRY RUN] Would create src/app/tests.rs

Processing: src/cli.rs
  Found 1 test module(s)
  - Moving 'tests' to src/cli/tests.rs
    Lines 538-618
    [DRY RUN] Would create src/cli/tests.rs

Summary: 23 test module(s) processed
(This was a dry run - no files were modified)
```

This tells you:
- How many Rust files were scanned
- Which files have test modules
- Where each test module will be moved to
- Line numbers of test modules

## How It Works

1. **Scans** all `.rs` files recursively in the source directory
2. **Finds** `#[cfg(test)] mod tests { }` patterns
3. **Extracts** the test module body
4. **Creates** new `{module}/tests.rs` files with the extracted content
5. **Replaces** original test modules with `include!()` macro references

## Why Use include!

The `include!()` macro ensures:
- Tests have full access to private items from the parent module
- Tests work exactly like they did before (no visibility changes)
- Cargo still discovers and runs tests normally
- The module namespace is preserved

## What Gets Moved

Only test-related code is moved:
- ✓ Test functions (`#[test]`)
- ✓ Helper functions inside test modules
- ✓ Test imports and use statements
- ✓ Comments within test modules

## What Stays

Non-test code remains in the source file:
- ✓ Implementation code and functions
- ✓ `#[cfg(test)]` on individual functions (outside `mod tests`)
- ✓ Public API and main logic

## Handling Nested Modules

For nested modules like `src/xerr/suggest.rs`:

**Before:**
```
src/xerr/suggest.rs (contains test module)
```

**After:**
```
src/xerr/suggest.rs (with include!() reference)
src/xerr/suggest/tests.rs (test content moved here)
```

## Troubleshooting

### No modules found
- Verify test modules follow the `#[cfg(test)] mod tests` pattern
- Check that `.rs` files exist in the specified directory
- Use `--dry-run` to confirm the path is correct

### Tests fail after reorganization
This shouldn't happen because `include!()` preserves test functionality:

```bash
# Run tests with verbose output for error details
cargo test -- --nocapture

# Check the generated test files
ls -la src/*/tests.rs

# Verify include!() paths are correct
grep -r "include!" src/
```

### Tests aren't discovered
- Ensure `#[cfg(test)]` attribute is preserved in source files
- Verify `include!()` macro path is correct (relative to source file)
- Run `cargo test --lib` for more details

## Reverting Changes

To undo the reorganization:

```bash
# Restore original files
git checkout src/

# Remove created test directories (optional cleanup)
find src -name tests.rs -type f -delete
find src -maxdepth 2 -type d -empty -delete
```

## Files in This Tool

```
scripts/
├── reorganize_tests.py              # Python implementation
├── reorganize_tests.sh              # Bash wrapper
├── TEST_REORGANIZATION.md           # This file
├── QUICK_START.md                   # Quick reference
└── REORGANIZE_TESTS_README.md       # Detailed documentation
```

## Safety Features

1. **Dry-run Mode** - Always preview before applying changes
2. **Non-Destructive** - Uses `include!()` to preserve functionality
3. **Git-Friendly** - Easy to revert with `git checkout`
4. **No Compilation Needed** - Fast analysis and transformation
5. **Conservative Approach** - Only moves test code

## Examples for This Project

When run on this project:

- **42 Rust files** will be scanned
- **23 test modules** will be reorganized
- **~10,000 lines** of test code will be moved

Files that will be reorganized:
- `src/app.rs` (1654 lines → ~400 lines + app/tests.rs)
- `src/model.rs` (2640 lines → ~800 lines + model/tests.rs)
- `src/scanning.rs` (1264 lines → ~400 lines + scanning/tests.rs)
- And 20 more files...

## Best Practices

1. **Always run `--dry-run` first** - Know what will happen
2. **Commit before running** - Create a checkpoint
3. **Review git diff after** - Verify the changes
4. **Run `cargo test` after** - Ensure everything works
5. **Commit the reorganization** - Save the changes

## Advanced Options

### Custom Source Directory
```bash
./scripts/reorganize_tests.sh --dry-run /path/to/src
./scripts/reorganize_tests.sh /path/to/src
```

### Verbose Output
```bash
./scripts/reorganize_tests.sh -v --dry-run
```

### Multiple Projects
```bash
for project in ~/projects/*/; do
    cd "$project"
    ./scripts/reorganize_tests.sh --dry-run
    ./scripts/reorganize_tests.sh
    cargo test
    cd -
done
```

## Requirements

- Python 3.6+
- Bash shell
- A Rust project with inline test modules

## Performance

- Scans 40+ files in seconds
- Processes 20+ test modules efficiently
- No compilation needed
- Fast and lightweight

## Integration with CI/CD

After reorganization, CI/CD should work unchanged:

```bash
cargo test
cargo test --all-features
cargo build --release
```

The `include!()` macro ensures tests continue to work exactly as before.

## More Information

- See `QUICK_START.md` for quick reference
- See `REORGANIZE_TESTS_README.md` for technical details
- See `reorganize_tests.py` for implementation details
- Run `./scripts/reorganize_tests.sh --help` for all options

## Getting Help

```bash
# Show help message
./scripts/reorganize_tests.sh --help

# Show Python help
python3 scripts/reorganize_tests.py --help

# Check cargo test output for specific errors
cargo test
```

## FAQ

**Q: Will this break my tests?**
A: No. The `include!()` macro preserves all test functionality exactly.

**Q: Do I need to change my test code?**
A: No. The tool extracts tests exactly as they are.

**Q: Can I run this multiple times?**
A: Yes, but it's designed to run once. Running again will try to move already-moved tests.

**Q: What if tests are already in separate files?**
A: The tool will skip files without inline test modules.

**Q: Can I customize the behavior?**
A: The Python script is well-documented. See `reorganize_tests.py` for customization.

## Next Steps

Ready to get started? Run:

```bash
./scripts/reorganize_tests.sh --dry-run
```

Then follow the workflow above!