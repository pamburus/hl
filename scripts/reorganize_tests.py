#!/usr/bin/env python3
"""
Reorganize inline unit tests in Rust files.

This script finds all inline test modules (marked with #[cfg(test)]) in Rust files
and moves them to separate tests.rs files in subdirectories matching the module structure.

Usage:
    python3 reorganize_tests.py [--dry-run] [src_dir]

Example:
    python3 reorganize_tests.py --dry-run src/
    python3 reorganize_tests.py src/
"""

import argparse
import os
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import List


@dataclass
class TestModule:
    """Represents a test module found in a source file."""

    file_path: str
    module_name: str
    start_line: int
    end_line: int
    content: str
    indent: str


def find_test_modules(file_path: str) -> List[TestModule]:
    """
    Find all #[cfg(test)] mod tests blocks in a Rust file.

    Returns a list of TestModule objects representing each test module found.
    """
    with open(file_path, "r", encoding="utf-8") as f:
        lines = f.readlines()

    test_modules = []
    i = 0

    while i < len(lines):
        line = lines[i]

        # Look for #[cfg(test)] pattern
        if re.search(r"#\[cfg\(test\)\]", line):
            # Check if the next non-empty line is a mod declaration
            j = i + 1
            while j < len(lines) and lines[j].strip() == "":
                j += 1

            if j < len(lines):
                next_line = lines[j]
                mod_match = re.match(r"^(\s*)mod\s+(\w+)\s*\{", next_line)

                if mod_match:
                    indent = mod_match.group(1)
                    module_name = mod_match.group(2)
                    start_line = i

                    # Find the closing brace for this module
                    brace_count = 0
                    in_string = False
                    escape_next = False
                    quote_char = None

                    k = j
                    found_open = False

                    while k < len(lines):
                        current_line = lines[k]

                        # Simple brace matching (not perfect but works for most cases)
                        for char in current_line:
                            if escape_next:
                                escape_next = False
                                continue

                            if char == "\\":
                                escape_next = True
                                continue

                            if char == '"':
                                if not in_string:
                                    in_string = True
                                    quote_char = char
                                elif char == quote_char:
                                    in_string = False
                                    quote_char = None
                            elif not in_string:
                                if char == "{":
                                    brace_count += 1
                                    found_open = True
                                elif char == "}":
                                    brace_count -= 1
                                    if found_open and brace_count == 0:
                                        # Found the closing brace
                                        end_line = k + 1
                                        content = "".join(
                                            lines[start_line:end_line]
                                        )

                                        test_modules.append(
                                            TestModule(
                                                file_path=file_path,
                                                module_name=module_name,
                                                start_line=start_line,
                                                end_line=end_line,
                                                content=content,
                                                indent=indent,
                                            )
                                        )

                                        i = k
                                        break
                        else:
                            k += 1
                            continue
                        break

        i += 1

    return test_modules


def extract_test_body(test_module: TestModule) -> str:
    """
    Extract just the body of the test module (without the module declaration).

    Returns the content that should go into tests.rs file.
    """
    content = test_module.content

    # Find the opening brace of "mod tests {"
    open_brace_idx = content.find("{")
    if open_brace_idx == -1:
        return ""

    # Find the matching closing brace by counting braces
    # Start counting from after the opening brace
    brace_count = 1
    in_string = False
    quote_char = None
    escape_next = False
    close_brace_idx = -1

    for i in range(open_brace_idx + 1, len(content)):
        char = content[i]

        if escape_next:
            escape_next = False
            continue
        if char == "\\":
            escape_next = True
            continue

        # Handle double quotes for strings - ignore single quotes to avoid Rust lifetime false positives ('a, 'b, etc.)
        if char == '"':
            if not in_string:
                in_string = True
                quote_char = '"'
            elif in_string and char == quote_char:
                in_string = False
        elif not in_string:
            if char == "{":
                brace_count += 1
            elif char == "}":
                brace_count -= 1
                if brace_count == 0:
                    close_brace_idx = i
                    break

    if close_brace_idx == -1:
        return ""

    # Extract content between the braces
    body = content[open_brace_idx + 1 : close_brace_idx]

    # Split into lines and remove module declaration lines and adjust indentation
    lines = body.split("\n")

    # Remove leading blank lines
    while lines and not lines[0].strip():
        lines.pop(0)

    # Remove trailing blank lines
    while lines and not lines[-1].strip():
        lines.pop()

    # Adjust indentation - remove the module's indent
    if lines and test_module.indent:
        indent_len = len(test_module.indent)
        adjusted_lines = []
        for line in lines:
            if line.strip():  # Non-empty line
                if line.startswith(test_module.indent):
                    adjusted_lines.append(line[indent_len:])
                else:
                    adjusted_lines.append(line)
            else:  # Empty line
                adjusted_lines.append(line)
        lines = adjusted_lines

    return "\n".join(lines)


def create_tests_file(
    module_dir: str, test_body: str, original_file_dir: str = None
) -> str:
    """
    Create a tests.rs file in the module directory.

    Adjusts relative paths in include_str!/include_bytes! if moving to a subdirectory.

    Returns the path to the created file.
    """
    os.makedirs(module_dir, exist_ok=True)
    tests_file = os.path.join(module_dir, "tests.rs")

    # Adjust relative paths if we're moving tests to a subdirectory
    adjusted_body = test_body
    if original_file_dir and original_file_dir != module_dir:
        # The tests are being moved to a subdirectory, so relative paths need "../"
        # Look for include_str!("path") and include_bytes!("path") patterns
        import re

        def adjust_include_path(match):
            prefix = match.group(1)  # 'include_str!' or 'include_bytes!'
            quote = match.group(2)  # '"' or "'"
            path = match.group(3)  # the path

            # Only adjust if path doesn't start with ../ or /
            if not path.startswith("../") and not path.startswith("/"):
                path = "../" + path

            return f"{prefix}({quote}{path}{quote})"

        # Match include_str!("path") and include_bytes!("path")
        adjusted_body = re.sub(
            r'(include_(?:str|bytes)!)\((["\'])([^"\']+)\2\)',
            adjust_include_path,
            adjusted_body,
        )

    with open(tests_file, "w", encoding="utf-8") as f:
        f.write(adjusted_body)
        if not adjusted_body.endswith("\n"):
            f.write("\n")

    return tests_file


def replace_test_module_with_mod_reference(
    file_path: str, test_module: TestModule
) -> str:
    """
    Replace the test module in the source file with a mod tests; reference.

    Returns the modified content.
    """
    with open(file_path, "r", encoding="utf-8") as f:
        content = f.read()

    # Create the replacement text - just a mod reference
    indent = test_module.indent
    module_name = test_module.module_name
    replacement = f"{indent}#[cfg(test)]\n{indent}mod {module_name};\n"

    # Replace the old module with the new reference
    new_content = content.replace(test_module.content, replacement)

    with open(file_path, "w", encoding="utf-8") as f:
        f.write(new_content)

    return new_content


def process_workspace(src_dir: str, dry_run: bool = False) -> None:
    """
    Process all Rust files in the workspace and reorganize tests.
    """
    src_path = Path(src_dir)

    if not src_path.exists():
        print(f"Error: Directory '{src_dir}' does not exist")
        sys.exit(1)

    if not src_path.is_dir():
        print(f"Error: '{src_dir}' is not a directory")
        sys.exit(1)

    # Find all .rs files
    rs_files = list(src_path.rglob("*.rs"))

    if not rs_files:
        print(f"No Rust files found in '{src_dir}'")
        return

    print(f"Found {len(rs_files)} Rust files")
    print()

    total_tests_moved = 0

    for rs_file in sorted(rs_files):
        # Skip tests.rs and test files
        if rs_file.name.endswith("tests.rs") or rs_file.name == "tests.rs":
            continue

        test_modules = find_test_modules(str(rs_file))

        if not test_modules:
            continue

        print(f"Processing: {rs_file}")
        print(f"  Found {len(test_modules)} test module(s)")

        for test_module in test_modules:
            module_name = test_module.module_name

            # Create the module directory
            module_dir = rs_file.parent / rs_file.stem

            print(f"  - Moving '{module_name}' to {module_dir}/tests.rs")
            print(
                f"    Lines {test_module.start_line + 1}-{test_module.end_line}"
            )

            if not dry_run:
                # Extract test body
                test_body = extract_test_body(test_module)

                # Create tests.rs file
                tests_file = create_tests_file(
                    str(module_dir), test_body, str(rs_file.parent)
                )

                # Replace in source file with mod reference
                replace_test_module_with_mod_reference(
                    str(rs_file), test_module
                )

                print(f"    ✓ Created {tests_file}")
            else:
                print(f"    [DRY RUN] Would create {module_dir}/tests.rs")

            total_tests_moved += 1

        print()

    print(f"Summary: {total_tests_moved} test module(s) processed")
    if dry_run:
        print("(This was a dry run - no files were modified)")


def main() -> None:
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="Reorganize inline unit tests to separate tests.rs files"
    )
    parser.add_argument(
        "src_dir",
        nargs="?",
        default="src",
        help="Source directory to process (default: src)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would be done without making changes",
    )

    args = parser.parse_args()

    process_workspace(args.src_dir, args.dry_run)


if __name__ == "__main__":
    main()
