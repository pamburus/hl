#!/usr/bin/env python3
"""
Coverage Diff Analysis Tool

This script compares two Git commits to find:
1. All changed line ranges across the entire workspace
2. All uncovered line ranges from coverage analysis
3. The intersection (changed but uncovered lines)

Usage:
    ./coverage-diff-analysis.py <base>
    ./coverage-diff-analysis.py <base> --quiet
    ./coverage-diff-analysis.py bcce60036a98e08987669d0339c362dc893cae75

Note: Analyzes changes from base to current working directory (including staged/unstaged changes)

Requirements:
- Git repository with the commits
- Coverage tool that generates target/lcov.info (e.g., 'just coverage')
- Python 3.8+
"""

import argparse
import os
import re
import subprocess
import sys
import textwrap
from pathlib import Path
from typing import Dict, List, Optional, Set, Tuple


class CoverageDiffError(Exception):
    """Base exception for coverage diff analysis errors."""

    pass


def horizontal_rule() -> str:
    """Return a horizontal rule string."""
    return "─" * 72


def thick_horizontal_rule() -> str:
    """Return a horizontal rule string."""
    return "━" * 80


def run_command(
    cmd: List[str],
    capture_output: bool = True,
    cwd: Optional[str] = None,
    quiet: bool = False,
) -> subprocess.CompletedProcess[str]:
    """Run a shell command and return the result.

    Args:
        cmd: Command and arguments as a list.
        capture_output: Whether to capture stdout/stderr.
        cwd: Working directory for the command.
        quiet: Whether to suppress error output.

    Returns:
        CompletedProcess with the command results.

    Raises:
        subprocess.CalledProcessError: If the command fails.
    """
    try:
        result = subprocess.run(
            cmd, capture_output=capture_output, text=True, cwd=cwd, check=True
        )
        return result
    except subprocess.CalledProcessError as e:
        if not quiet:
            print(
                f"Error running command {' '.join(cmd)}: {e}", file=sys.stderr
            )
            if e.stdout:
                print(f"stdout: {e.stdout}", file=sys.stderr)
            if e.stderr:
                print(f"stderr: {e.stderr}", file=sys.stderr)
        raise


def parse_ranges(range_str: str) -> Set[int]:
    """Parse range string like '25-27 30 35-40' into set of line numbers.

    Args:
        range_str: String containing space-separated ranges and individual line numbers.

    Returns:
        Set of line numbers.
    """
    lines: Set[int] = set()
    if not range_str.strip():
        return lines

    for part in range_str.split():
        if "-" in part:
            try:
                start, end = map(int, part.split("-"))
                lines.update(range(start, end + 1))
            except ValueError:
                # Invalid range format, skip it
                continue
        else:
            try:
                lines.add(int(part))
            except ValueError:
                # Invalid line number, skip it
                continue
    return lines


def ranges_to_string(lines: Set[int]) -> str:
    """Convert set of line numbers back to range string.

    Args:
        lines: Set of line numbers.

    Returns:
        Space-separated range string (e.g., "1-3 5 7-9").
    """
    if not lines:
        return ""

    sorted_lines = sorted(lines)
    ranges: List[str] = []
    start = sorted_lines[0]
    end = start

    for line in sorted_lines[1:]:
        if line == end + 1:
            end = line
        else:
            if start == end:
                ranges.append(str(start))
            else:
                ranges.append(f"{start}-{end}")
            start = line
            end = line

    # Add the last range
    if start == end:
        ranges.append(str(start))
    else:
        ranges.append(f"{start}-{end}")

    return " ".join(ranges)


def get_changed_lines(base: str, quiet: bool = False) -> Dict[str, str]:
    """Get all changed line ranges between old commit and working directory.

    Args:
        base: Git commit reference to compare against.
        quiet: Whether to suppress output.

    Returns:
        Dictionary mapping file paths to range strings.

    Raises:
        subprocess.CalledProcessError: If git diff command fails.
    """
    if not quiet:
        print(f"📝 Analyzing changes between {base[:8]}..working directory")

    # Get unified diff with no context lines (includes staged and unstaged changes)
    cmd = ["git", "diff", "--unified=0", base]
    result = run_command(cmd, quiet=quiet)

    changed_ranges: Dict[str, str] = {}
    current_file: Optional[str] = None

    for line in result.stdout.split("\n"):
        # Match file headers like "+++ b/src/app.rs"
        if line.startswith("+++ b/"):
            current_file = line[6:]  # Remove "+++ b/"
            continue

        # Match hunk headers like "@@ -142 +143 @@" or "@@ -321,10 +322,10 @@"
        if line.startswith("@@"):
            if not current_file:
                continue

            # Extract the new side (+ part) of the hunk header
            match = re.search(r"\+(\d+)(?:,(\d+))?", line)
            if match:
                start = int(match.group(1))
                count = int(match.group(2)) if match.group(2) else 1

                if count > 0:
                    if count == 1:
                        range_str = str(start)
                    else:
                        end = start + count - 1
                        range_str = f"{start}-{end}"

                    if current_file in changed_ranges:
                        changed_ranges[current_file] += f" {range_str}"
                    else:
                        changed_ranges[current_file] = range_str

    return changed_ranges


def _run_coverage_analysis(quiet: bool) -> None:
    """Run the coverage analysis command.

    Args:
        quiet: Whether to suppress output.

    Raises:
        CoverageDiffError: If no build tool is found or coverage command fails.
    """
    try:
        if os.path.exists("justfile"):
            run_command(["just", "coverage"], capture_output=quiet, quiet=quiet)
        elif os.path.exists("Makefile"):
            run_command(["make", "coverage"], capture_output=quiet, quiet=quiet)
        else:
            raise CoverageDiffError(
                "No justfile or Makefile found. Cannot run coverage."
            )
    except subprocess.CalledProcessError as e:
        raise CoverageDiffError(f"Coverage analysis failed: {e}") from e


def _is_project_file(file_path: str) -> bool:
    """Check if a file path is part of the project (not an external library).

    Args:
        file_path: File path to check.

    Returns:
        True if the file is part of the project, False if it's external.
    """
    # Only include files under the project root
    project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    return project_root in file_path


def _clean_file_path(file_path: str) -> str:
    """Clean the file path by removing project root prefix.

    Args:
        file_path: Absolute or relative file path.

    Returns:
        Cleaned file path relative to project root.
    """
    clean_file = file_path
    if clean_file.startswith("/"):
        # Get the project root (parent directory of this script)
        project_root = os.path.dirname(
            os.path.dirname(os.path.abspath(__file__))
        )

        # Extract everything after the project root
        if project_root in clean_file:
            parts = clean_file.split(project_root + "/")
            if len(parts) > 1:
                clean_file = parts[-1]
    return clean_file


def _parse_lcov_line(
    line: str, current_file: Optional[str], uncovered_lines: List[int]
) -> Tuple[Optional[str], List[int]]:
    """Parse a single LCOV line and update state.

    Args:
        line: LCOV line to parse.
        current_file: Current file being processed.
        uncovered_lines: List of uncovered line numbers for current file.

    Returns:
        Tuple of (current_file, uncovered_lines) after processing the line.
    """
    line = line.strip()

    # Source file marker
    if line.startswith("SF:"):
        return line[3:], []  # Remove "SF:", reset uncovered_lines

    # Line coverage data
    if line.startswith("DA:"):
        parts = line.split(",")
        if len(parts) >= 2:
            try:
                line_num = int(parts[0][3:])  # Remove "DA:"
                hit_count = int(parts[1])
                if hit_count == 0:
                    uncovered_lines.append(line_num)
            except (ValueError, IndexError):
                # Malformed DA line, skip it
                pass

    return current_file, uncovered_lines


def get_uncovered_lines(quiet: bool = False) -> Dict[str, str]:
    """Get all uncovered line ranges from coverage report.

    Args:
        quiet: Whether to suppress output.

    Returns:
        Dictionary mapping file paths to range strings.

    Raises:
        CoverageDiffError: If coverage analysis fails or lcov file is not found.
    """
    if not quiet:
        print("🔍 Running coverage analysis...")

    # Run coverage analysis
    try:
        _run_coverage_analysis(quiet)
    except CoverageDiffError as e:
        raise CoverageDiffError(f"Coverage analysis failed: {e}") from e

    # Parse lcov.info file
    lcov_path = Path("target/lcov.info")
    if not lcov_path.exists():
        raise CoverageDiffError(
            "target/lcov.info not found. Coverage analysis may have failed."
        )

    uncovered_ranges: Dict[str, str] = {}
    current_file: Optional[str] = None
    uncovered_lines: List[int] = []

    try:
        with open(lcov_path, "r", encoding="utf-8") as f:
            for line in f:
                # Check for file transition
                if line.strip().startswith("SF:"):
                    # Process previous file if any
                    if (
                        current_file
                        and uncovered_lines
                        and _is_project_file(current_file)
                    ):
                        ranges_str = ranges_to_string(set(uncovered_lines))
                        if ranges_str:
                            clean_file = _clean_file_path(current_file)
                            uncovered_ranges[clean_file] = ranges_str

                    # Start new file
                    current_file, uncovered_lines = _parse_lcov_line(
                        line, current_file, uncovered_lines
                    )
                else:
                    # Process line coverage data
                    current_file, uncovered_lines = _parse_lcov_line(
                        line, current_file, uncovered_lines
                    )

            # Process the last file
            if (
                current_file
                and uncovered_lines
                and _is_project_file(current_file)
            ):
                ranges_str = ranges_to_string(set(uncovered_lines))
                if ranges_str:
                    clean_file = _clean_file_path(current_file)
                    uncovered_ranges[clean_file] = ranges_str
    except IOError as e:
        raise CoverageDiffError(
            f"Failed to read coverage file {lcov_path}: {e}"
        ) from e

    return uncovered_ranges


def find_intersections(
    changed: Dict[str, str], uncovered: Dict[str, str]
) -> Dict[str, str]:
    """Find intersection of changed and uncovered lines.

    Args:
        changed: Dictionary of changed line ranges by file.
        uncovered: Dictionary of uncovered line ranges by file.

    Returns:
        Dictionary of intersections (changed but uncovered lines) by file.
    """
    intersections: Dict[str, str] = {}

    for file_path in set(changed.keys()) | set(uncovered.keys()):
        changed_lines = parse_ranges(changed.get(file_path, ""))
        uncovered_lines = parse_ranges(uncovered.get(file_path, ""))
        intersection = changed_lines & uncovered_lines

        if intersection:
            intersections[file_path] = ranges_to_string(intersection)

    return intersections


def _print_ide_links(ranges_dict: Dict[str, str]) -> None:
    """Print ranges in IDE-friendly clickable format.

    Args:
        ranges_dict: Dictionary of file paths to range strings.
    """
    for file_path in sorted(ranges_dict.keys()):
        ranges_str = ranges_dict[file_path]
        if not ranges_str.strip():
            continue

        # Parse each range and print as separate clickable links
        for range_part in ranges_str.split():
            if "-" in range_part:
                start, end = range_part.split("-")
                print(f"{file_path}:{start} – {file_path}:{end}")
            else:
                print(f"{file_path}:{range_part}")


def print_analysis(
    changed: Dict[str, str],
    uncovered: Dict[str, str],
    intersections: Dict[str, str],
    base: str,
    quiet: bool = False,
    ide_links: bool = False,
) -> None:
    """Print the comprehensive analysis report.

    Args:
        changed: Dictionary of changed line ranges by file.
        uncovered: Dictionary of uncovered line ranges by file.
        intersections: Dictionary of changed-but-uncovered lines by file.
        base: Original commit that was compared.
        quiet: Whether to suppress verbose output.
        ide_links: Whether to use IDE-friendly format.
    """
    if quiet:
        # Only print intersection results in quiet mode
        if intersections:
            if ide_links:
                _print_ide_links(intersections)
            else:
                for file_path in sorted(intersections.keys()):
                    lines = parse_ranges(intersections[file_path])
                    print(
                        f"{file_path}: {intersections[file_path]} ({len(lines)} lines)"
                    )
        return

    print("\n" + thick_horizontal_rule())
    print("COMPREHENSIVE COVERAGE ANALYSIS REPORT")
    print(thick_horizontal_rule())
    print(f"Comparing commits: {base[:8]} → working directory")
    print()

    # Changed ranges
    print("📝 CHANGED RANGES (by file)")
    print(horizontal_rule())
    total_changed_lines = 0
    for file_path in sorted(changed.keys()):
        lines = parse_ranges(changed[file_path])
        total_changed_lines += len(lines)
        print(f"{file_path}: {changed[file_path]} ({len(lines)} lines)")
    print(f"\n📊 TOTAL CHANGED LINES: {total_changed_lines}")
    print()

    # Uncovered ranges
    print("🔍 UNCOVERED RANGES (by file)")
    print(horizontal_rule())
    total_uncovered_lines = 0
    for file_path in sorted(uncovered.keys()):
        lines = parse_ranges(uncovered[file_path])
        total_uncovered_lines += len(lines)
        print(f"{file_path}: {uncovered[file_path]} ({len(lines)} lines)")
    print(f"\n📊 TOTAL UNCOVERED LINES: {total_uncovered_lines}")
    print()

    # Intersections
    print("🎯 INTERSECTION: CHANGED BUT UNCOVERED")
    print(horizontal_rule())
    total_intersection_lines = 0

    if intersections:
        for file_path in sorted(intersections.keys()):
            lines = parse_ranges(intersections[file_path])
            total_intersection_lines += len(lines)
            print(
                f"{file_path}: {intersections[file_path]} ({len(lines)} lines)"
            )
        print(f"\n📊 TOTAL INTERSECTION LINES: {total_intersection_lines}")
    else:
        print("✅ No intersection found - all changed lines are covered!")

    print()

    # Summary
    print("📋 SUMMARY")
    print(horizontal_rule())
    print(f"Files with changes: {len(changed)}")
    print(f"Files with uncovered lines: {len(uncovered)}")
    print(f"Files with changed-but-uncovered lines: {len(intersections)}")

    if total_changed_lines > 0:
        coverage_gap_pct = (
            total_intersection_lines / total_changed_lines
        ) * 100
        print(
            f"Coverage gap in changed code: {total_intersection_lines}/{total_changed_lines} lines ({coverage_gap_pct:.1f}%)"
        )

        if coverage_gap_pct < 5:
            print("✅ Excellent coverage! Less than 5% gap.")
        elif coverage_gap_pct < 15:
            print(
                "👍 Good coverage. Consider adding tests for critical uncovered lines."
            )
        else:
            print(
                "⚠️  Significant coverage gap. Testing recommended for important code paths."
            )

    print("\n" + thick_horizontal_rule())


def main() -> None:
    """Main entry point for the coverage diff analysis tool."""
    parser = argparse.ArgumentParser(
        description="Coverage Diff Analysis Tool - Compare Git commits to find changed but uncovered lines",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=textwrap.dedent(
            """
            Examples:
              python3 coverage-diff-analysis.py master
              python3 coverage-diff-analysis.py bcce60036a98e08987669d0339c362dc893cae75 --quiet
              python3 coverage-diff-analysis.py HEAD~5 -q
              python3 coverage-diff-analysis.py master -q --ide-links

            Note: Analyzes changes from base to current working directory
                  (including staged/unstaged changes)
            """
        ),
    )

    parser.add_argument(
        "base",
        help="Git commit to compare against (e.g., master, HEAD~1, commit hash)",
    )

    parser.add_argument(
        "-q",
        "--quiet",
        action="store_true",
        help="Suppress verbose output and show only intersection results",
    )

    parser.add_argument(
        "--ide-links",
        action="store_true",
        help="Output in IDE-friendly clickable format (filename:line or filename:line-line)",
    )

    args = parser.parse_args()
    base = args.base
    quiet = args.quiet
    ide_links = args.ide_links

    # Verify we're in a git repository
    try:
        run_command(["git", "rev-parse", "--git-dir"], quiet=quiet)
    except subprocess.CalledProcessError:
        print("Error: Not in a Git repository.", file=sys.stderr)
        sys.exit(1)

    # Verify old commit exists
    try:
        run_command(["git", "rev-parse", "--verify", base], quiet=quiet)
    except subprocess.CalledProcessError:
        if not quiet:
            print(f"Error: Commit doesn't exist: {base}", file=sys.stderr)
        sys.exit(1)

    if not quiet:
        print("🚀 Starting coverage diff analysis...")

    # Get coverage data from current HEAD
    try:
        uncovered = get_uncovered_lines(quiet=quiet)
    except CoverageDiffError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    # Get changed lines between old commit and HEAD
    try:
        changed = get_changed_lines(base, quiet=quiet)
    except subprocess.CalledProcessError as e:
        print(f"Error: Failed to get changed lines: {e}", file=sys.stderr)
        sys.exit(1)

    # Find intersections
    intersections = find_intersections(changed, uncovered)

    # Print analysis
    print_analysis(changed, uncovered, intersections, base, quiet, ide_links)

    if not quiet:
        print("✅ Analysis complete!")


if __name__ == "__main__":
    main()
