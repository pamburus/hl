# Quick Start: Reorganizing Inline Tests

## 30-Second Overview

This tool automatically moves inline `#[cfg(test)] mod tests` blocks from your Rust source files into separate `tests.rs` files organized by module.

## TL;DR

```bash
# See what would happen (ALWAYS DO THIS FIRST!)
./scripts/reorganize_tests.sh --dry-run

# If it looks good, apply the changes
./scripts/reorganize_tests.sh

# Verify tests still work
cargo test
```

## What Happens

### Before
```
src/app.rs (529 lines)
  ├── public functions
  ├── private functions
  └── #[cfg(test)] mod tests { ... }  (400 lines of tests)
```

### After
```
src/app.rs (129 lines)
  ├── public functions
  ├── private functions
  └── #[cfg(test)] mod tests {
        include!("../app/tests.rs");
      }

src/app/tests.rs (NEW - 400 lines)
  └── all test code
```

## Commands

### Option 1: Using the Shell Wrapper (Recommended)

```bash
# Preview changes
./scripts/reorganize_tests.sh --dry-run

# Apply changes
./scripts/reorganize_tests.sh

# For a custom source directory
./scripts/reorganize_tests.sh --dry-run src/other/path
./scripts/reorganize_tests.sh src/other/path
```

### Option 2: Using Python Directly

```bash
# Preview changes
python3 scripts/reorganize_tests.py --dry-run src/

# Apply changes
python3 scripts/reorganize_tests.py src/

# Custom directory
python3 scripts/reorganize_tests.py --dry-run /path/to/src
python3 scripts/reorganize_tests.py /path/to/src
```

## Example Output

```
$ ./scripts/reorganize_tests.sh --dry-run

╔════════════════════════════════════════════════════════════╗
║  Reorganizing Inline Unit Tests                        ║
╚════════════════════════════════════════════════════════════╝

[DRY RUN] Changes will NOT be applied
Source directory: src

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

... more files ...

Summary: 23 test module(s) processed
(This was a dry run - no files were modified)
```

## Important: Safety First

1. **Always run with `--dry-run` first** - Review the output before applying changes
2. **Commit your changes before running** - This gives you a checkpoint to revert to
3. **Run tests after reorganization** - Verify everything still works

## Recommended Workflow

```bash
# Step 1: Check current state
git status

# Step 2: Commit current work
git add -A
git commit -m "Work in progress before test reorganization"

# Step 3: Preview changes
./scripts/reorganize_tests.sh --dry-run

# Step 4: If preview looks good, apply changes
./scripts/reorganize_tests.sh

# Step 5: Verify tests pass
cargo test

# Step 6: Review what changed
git diff

# Step 7: Commit reorganization
git add -A
git commit -m "Reorganize inline tests to separate files"
```

## What Gets Moved

✓ All test code inside `#[cfg(test)] mod tests { }` blocks  
✓ Test functions (`#[test]`)  
✓ Helper functions used by tests  
✓ Test imports and use statements  

## What Stays

✓ Non-test code (functions, structs, etc.)  
✓ Comments (preserved in moved tests)  
✓ `#[cfg(test)]` on individual items outside test modules  

## How It Works

1. **Finds** all Rust files with inline test modules
2. **Extracts** the test module body
3. **Creates** a new `{module}/tests.rs` file
4. **Replaces** the test module with an `include!` macro reference

The `include!` macro means tests have full access to private module items, exactly like before.

## Troubleshooting

### Nothing happens
- Ensure you're in the project root directory
- Check that `src/` directory exists (or provide correct path)
- Verify files have `#[cfg(test)] mod tests { }` pattern

### Tests don't compile after running
- Check the error message from `cargo test`
- Review generated files in `src/*/tests.rs`
- Ensure imports in test files are correct

### Want to undo?
```bash
git checkout src/
rm -rf src/*/tests.rs
```

## See More Details

For comprehensive documentation, see:
- `REORGANIZE_TESTS_README.md` - Full documentation
- Python script has inline help: `python3 scripts/reorganize_tests.py --help`
- Shell script has help: `./scripts/reorganize_tests.sh --help`

## Questions?

Run the script with `--help` to see all available options:

```bash
./scripts/reorganize_tests.sh --help
python3 scripts/reorganize_tests.py --help
```
