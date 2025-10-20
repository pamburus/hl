# Test Reorganization Script

This script reorganizes inline unit tests in Rust projects by moving `#[cfg(test)] mod tests` blocks from source files into separate `tests.rs` files in module-specific subdirectories.

## Overview

The script automates the common pattern of extracting test modules from Rust source files and organizing them in a cleaner directory structure. Instead of having large test blocks at the end of `app.rs`, tests are moved to `app/tests.rs`.

## Features

- **Automatic Discovery**: Finds all inline test modules (`#[cfg(test)] mod tests`) in Rust files
- **Intelligent Organization**: Creates appropriate subdirectory structures matching your module hierarchy
- **Safe Operation**: Includes `--dry-run` mode to preview changes before applying them
- **Preserves References**: Replaces test module bodies with `include!` macros for compatibility
- **Recursive Processing**: Handles nested modules and subdirectories

## Directory Structure Changes

### Before
```
src/
├── app.rs          (contains test module at end)
├── cli.rs          (contains test module at end)
└── model.rs        (contains test module at end)
```

### After
```
src/
├── app.rs          (references test module via include!)
├── app/
│   └── tests.rs    (test content moved here)
├── cli.rs          (references test module via include!)
├── cli/
│   └── tests.rs    (test content moved here)
├── model.rs        (references test module via include!)
└── model/
    └── tests.rs    (test content moved here)
```

## Usage

### Basic Usage

Run with default settings (processes `src/` directory):
```bash
python3 scripts/reorganize_tests.py
```

### Dry Run (Recommended First Step)

Preview what changes would be made without modifying any files:
```bash
python3 scripts/reorganize_tests.py --dry-run
```

### Custom Source Directory

Process a specific directory:
```bash
python3 scripts/reorganize_tests.py /path/to/source
```

### Options

- `--dry-run`: Show what would be done without making changes
- `src_dir`: Path to the source directory to process (default: `src`)

## Example Output

```
$ python3 scripts/reorganize_tests.py --dry-run src/

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

## How It Works

1. **Scans all `.rs` files** in the specified source directory recursively
2. **Detects test modules** by looking for `#[cfg(test)] mod tests` patterns
3. **Extracts test content** including all test functions and helper code
4. **Creates module directories** (e.g., `app/` for `app.rs`)
5. **Writes `tests.rs` files** containing the extracted test code
6. **Replaces original modules** with `include!` macro references

## What Gets Moved

Only test-related code is moved:
- `#[test]` functions
- Helper functions used by tests
- Test imports and use statements
- Comments within test modules

## What Stays Behind

Non-test code remains in the source file:
- `#[cfg(test)]` attributes on individual functions or types
- Inline test helpers outside of `mod tests` blocks
- The module reference itself (replaced with `include!` macro)

## Handling Existing Directories

If a module directory already exists, the script will create or overwrite the `tests.rs` file within it. This is safe as tests should be kept separate.

## Limitations and Notes

### Important Considerations

1. **include! macro**: The script uses `include!` to reference test files. This preserves the namespace and allows tests to access private items from the parent module.

2. **String literals**: The brace-counting logic handles strings carefully to avoid counting braces inside string literals as code structure.

3. **Comments**: Rust comments are properly handled during parsing.

4. **Indentation**: The script preserves relative indentation of test code.

### When to Use include!

The `include!` macro is used to include test files inline, which means:
- Tests have full access to private module items
- Tests maintain the same visibility as before
- Tests appear as if they were defined in the original file

Example of what the source file looks like after reorganization:

**Before:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // test code
    }
}
```

**After:**
```rust
#[cfg(test)]
mod tests {
    include!("../app/tests.rs");
}
```

**In `app/tests.rs`:**
```rust
use super::*;

#[test]
fn test_something() {
    // test code
}
```

## Running the Reorganization

### Step 1: Dry Run
```bash
python3 scripts/reorganize_tests.py --dry-run src/
```
Review the output to ensure the changes look correct.

### Step 2: Actual Run
Once you're satisfied with the dry run:
```bash
python3 scripts/reorganize_tests.py src/
```

### Step 3: Verify
After the script completes:
1. Review the created test files
2. Run your test suite to ensure everything works:
   ```bash
   cargo test
   ```
3. Commit the changes to version control

## Troubleshooting

### Script doesn't find any files
- Ensure the path you provide exists and contains `.rs` files
- Check that you're not accidentally excluding directories

### Tests don't compile after reorganization
- Run `cargo test` to see the specific error
- The test files may need additional imports
- Check that relative paths in `include!` statements are correct

### Some test modules aren't being processed
- The script skips files already named `tests.rs`
- Ensure test modules follow the `#[cfg(test)] mod <name>` pattern
- Comments between `#[cfg(test)]` and `mod` are allowed

### The include! path is wrong
- Check that the generated path correctly points to the tests.rs file
- The path is relative to the module file location

## Reverting Changes

If you need to revert the changes:

1. Use version control to restore original files:
   ```bash
   git checkout src/
   ```

2. Remove the created test directories:
   ```bash
   rm -rf src/*/tests.rs src/*/
   ```

## Best Practices

1. **Always run with `--dry-run` first** to verify changes
2. **Commit before running** so you have a checkpoint
3. **Run tests after reorganization** to ensure everything works
4. **Review generated files** to ensure formatting is acceptable
5. **Update documentation** that references file locations or test organization

## Example: Full Workflow

```bash
# Navigate to project root
cd /path/to/rust/project

# First, see what would happen
python3 scripts/reorganize_tests.py --dry-run src/

# If satisfied, commit current state
git add -A && git commit -m "Pre-test reorganization checkpoint"

# Run the actual reorganization
python3 scripts/reorganize_tests.py src/

# Test that everything still works
cargo test

# Review the changes
git diff

# Commit the reorganization
git add -A && git commit -m "Reorganize tests to separate files"
```

## Requirements

- Python 3.6+
- A Rust project with inline test modules
- Write access to the source directory

## Technical Details

### Parsing Strategy

The script uses:
- Regular expressions to identify test module patterns
- Brace-counting algorithm to find module boundaries
- String literal detection to avoid false positives
- Escape sequence handling for accurate parsing

### Edge Cases Handled

- Strings containing braces or comment markers
- Escape sequences within strings
- Multiple test modules in a single file (if any)
- Nested braces in test code
- Indented modules

## Contributing

To improve the script:
1. Test against various Rust project structures
2. Report issues with specific patterns that don't parse correctly
3. Suggest improvements for edge cases

## License

Same as the parent project.