#!/usr/bin/env python3
"""
Coverage Diff Analysis Tool

This script compares two Git commits to find:
1. All changed line ranges across the entire workspace
2. All uncovered line ranges from coverage analysis
3. The intersection (changed but uncovered lines)

Usage:
    ./coverage-diff-analysis.py <old_commit>
    ./coverage-diff-analysis.py <old_commit> --quiet
    ./coverage-diff-analysis.py bcce60036a98e08987669d0339c362dc893cae75

Note: Analyzes changes from old_commit to current working directory (including staged/unstaged changes)

Requirements:
- Git repository with the commits
- Coverage tool that generates target/lcov.info (e.g., 'just coverage')
- Python 3.6+
"""

import argparse
import subprocess
import sys
import os
import re
from typing import Dict, Set, List, Optional
from pathlib import Path


def run_command(cmd: List[str], capture_output: bool = True, cwd: Optional[str] = None) -> subprocess.CompletedProcess[str]:
    """Run a shell command and return the result."""
    try:
        result = subprocess.run(cmd, capture_output=capture_output, text=True, cwd=cwd, check=True)
        return result
    except subprocess.CalledProcessError as e:
        if not hasattr(run_command, 'quiet') or not run_command.quiet:  # type: ignore
            print(f"Error running command {' '.join(cmd)}: {e}")
            if e.stdout:
                print(f"stdout: {e.stdout}")
            if e.stderr:
                print(f"stderr: {e.stderr}")
        sys.exit(1)


def parse_ranges(range_str: str) -> Set[int]:
    """Parse range string like '25-27 30 35-40' into set of line numbers."""
    lines: Set[int] = set()
    if not range_str.strip():
        return lines

    for part in range_str.split():
        if '-' in part:
            try:
                start, end = map(int, part.split('-'))
                lines.update(range(start, end + 1))
            except ValueError:
                continue
        else:
            try:
                lines.add(int(part))
            except ValueError:
                continue
    return lines


def ranges_to_string(lines: Set[int]) -> str:
    """Convert set of line numbers back to range string."""
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


def get_changed_lines(old_commit: str) -> Dict[str, str]:
    """Get all changed line ranges between old commit and working directory."""
    if not hasattr(get_changed_lines, 'quiet') or not get_changed_lines.quiet:  # type: ignore
        print(f"📝 Analyzing changes between {old_commit[:8]}..working directory")

    # Get unified diff with no context lines (includes staged and unstaged changes)
    cmd = ["git", "diff", "--unified=0", old_commit]
    result = run_command(cmd)

    changed_ranges: Dict[str, str] = {}
    current_file = None

    for line in result.stdout.split('\n'):
        # Match file headers like "+++ b/src/app.rs"
        if line.startswith('+++ b/'):
            current_file = line[6:]  # Remove "+++ b/"
            continue

        # Match hunk headers like "@@ -142 +143 @@" or "@@ -321,10 +322,10 @@"
        if line.startswith('@@'):
            if not current_file:
                continue

            # Extract the new side (+ part) of the hunk header
            match = re.search(r'\+(\d+)(?:,(\d+))?', line)
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
    """Run the coverage analysis command."""
    if os.path.exists("justfile"):
        run_command(["just", "coverage"], capture_output=quiet)
    elif os.path.exists("Makefile"):
        run_command(["make", "coverage"], capture_output=quiet)
    else:
        if not quiet:
            print("Error: No justfile or Makefile found. Cannot run coverage.")
        sys.exit(1)


def _clean_file_path(file_path: str) -> str:
    """Clean the file path by removing project root prefix."""
    clean_file = file_path
    if clean_file.startswith('/'):
        # Find the project root marker (usually contains 'hl')
        if '/hl/' in clean_file:
            # Extract everything after the project root
            parts = clean_file.split('/hl/')
            if len(parts) > 1:
                clean_file = parts[-1]
    return clean_file


def _convert_lines_to_ranges(line_numbers: List[int]) -> str:
    """Convert list of line numbers to range string format."""
    if not line_numbers:
        return ""

    line_numbers.sort()
    ranges: List[str] = []
    start = line_numbers[0]
    end = start

    for line_num in line_numbers[1:]:
        if line_num == end + 1:
            end = line_num
        else:
            if start == end:
                ranges.append(str(start))
            else:
                ranges.append(f"{start}-{end}")
            start = line_num
            end = line_num

    # Add the last range
    if start == end:
        ranges.append(str(start))
    else:
        ranges.append(f"{start}-{end}")

    return " ".join(ranges)


def _process_file_coverage(file_path: str, uncovered_lines: List[int]) -> Optional[str]:
    """Process coverage data for a single file and return range string."""
    if not file_path or not uncovered_lines:
        return None

    ranges_str = _convert_lines_to_ranges(uncovered_lines)

    return ranges_str if ranges_str else None


def _parse_lcov_line(line: str, current_file: Optional[str], uncovered_lines: List[int]) -> tuple[Optional[str], List[int]]:
    """Parse a single LCOV line and update state."""
    line = line.strip()

    # Source file marker
    if line.startswith('SF:'):
        return line[3:], []  # Remove "SF:", reset uncovered_lines

    # Line coverage data
    elif line.startswith('DA:'):
        parts = line.split(',')
        if len(parts) >= 2:
            try:
                line_num = int(parts[0][3:])  # Remove "DA:"
                hit_count = int(parts[1])
                if hit_count == 0:
                    uncovered_lines.append(line_num)
            except ValueError:
                pass

    return current_file, uncovered_lines


def get_uncovered_lines() -> Dict[str, str]:
    """Get all uncovered line ranges from coverage report."""
    quiet = hasattr(get_uncovered_lines, 'quiet') and get_uncovered_lines.quiet  # type: ignore
    if not quiet:
        print("🔍 Running coverage analysis...")

    # Run coverage analysis
    _run_coverage_analysis(quiet)

    # Parse lcov.info file
    lcov_path = Path("target/lcov.info")
    if not lcov_path.exists():
        if not quiet:
            print("Error: target/lcov.info not found. Coverage analysis may have failed.")
        sys.exit(1)

    uncovered_ranges: Dict[str, str] = {}
    current_file: Optional[str] = None
    uncovered_lines: List[int] = []

    with open(lcov_path, 'r') as f:
        for line in f:
            # Check for file transition
            if line.strip().startswith('SF:'):
                # Process previous file if any
                if current_file:
                    ranges_str = _process_file_coverage(current_file, uncovered_lines)
                    if ranges_str:
                        clean_file = _clean_file_path(current_file)
                        uncovered_ranges[clean_file] = ranges_str

                # Start new file
                current_file, uncovered_lines = _parse_lcov_line(line, current_file, uncovered_lines)
            else:
                # Process line coverage data
                current_file, uncovered_lines = _parse_lcov_line(line, current_file, uncovered_lines)

        # Process the last file
        if current_file:
            ranges_str = _process_file_coverage(current_file, uncovered_lines)
            if ranges_str:
                clean_file = _clean_file_path(current_file)
                uncovered_ranges[clean_file] = ranges_str

    return uncovered_ranges


def find_intersections(changed: Dict[str, str], uncovered: Dict[str, str]) -> Dict[str, str]:
    """Find intersection of changed and uncovered lines."""
    intersections: Dict[str, str] = {}

    for file_path in set(changed.keys()) | set(uncovered.keys()):
        changed_lines = parse_ranges(changed.get(file_path, ""))
        uncovered_lines = parse_ranges(uncovered.get(file_path, ""))
        intersection = changed_lines & uncovered_lines

        if intersection:
            intersections[file_path] = ranges_to_string(intersection)

    return intersections


def _print_ide_links(ranges_dict: Dict[str, str]) -> None:
    """Print ranges in IDE-friendly clickable format."""
    for file_path in sorted(ranges_dict.keys()):
        ranges_str = ranges_dict[file_path]
        if not ranges_str.strip():
            continue

        # Parse each range and print as separate clickable links
        for range_part in ranges_str.split():
            if '-' in range_part:
                start, end = range_part.split('-')
                print(f"{file_path}:{start} – {file_path}:{end}")
            else:
                print(f"{file_path}:{range_part}")


def print_analysis(changed: Dict[str, str], uncovered: Dict[str, str], intersections: Dict[str, str], old_commit: str, quiet: bool = False, ide_links: bool = False) -> None:
    """Print the comprehensive analysis report."""
    if quiet:
        # Only print intersection results in quiet mode
        if intersections:
            if ide_links:
                _print_ide_links(intersections)
            else:
                for file_path in sorted(intersections.keys()):
                    lines = parse_ranges(intersections[file_path])
                    print(f"{file_path}: {intersections[file_path]} ({len(lines)} lines)")
        return

    print("\n" + "="*80)
    print("COMPREHENSIVE COVERAGE ANALYSIS REPORT")
    print("="*80)
    print(f"Comparing commits: {old_commit[:8]} → working directory")
    print()

    # Changed ranges
    print("📝 CHANGED RANGES (by file)")
    print("-" * 50)
    total_changed_lines = 0
    for file_path in sorted(changed.keys()):
        lines = parse_ranges(changed[file_path])
        total_changed_lines += len(lines)
        print(f"{file_path}: {changed[file_path]} ({len(lines)} lines)")
    print(f"\n📊 TOTAL CHANGED LINES: {total_changed_lines}")
    print()

    # Uncovered ranges
    print("🔍 UNCOVERED RANGES (by file)")
    print("-" * 50)
    total_uncovered_lines = 0
    for file_path in sorted(uncovered.keys()):
        lines = parse_ranges(uncovered[file_path])
        total_uncovered_lines += len(lines)
        print(f"{file_path}: {uncovered[file_path]} ({len(lines)} lines)")
    print(f"\n📊 TOTAL UNCOVERED LINES: {total_uncovered_lines}")
    print()

    # Intersections
    print("🎯 INTERSECTION: CHANGED BUT UNCOVERED")
    print("-" * 50)
    total_intersection_lines = 0

    if intersections:
        for file_path in sorted(intersections.keys()):
            lines = parse_ranges(intersections[file_path])
            total_intersection_lines += len(lines)
            print(f"{file_path}: {intersections[file_path]} ({len(lines)} lines)")
        print(f"\n📊 TOTAL INTERSECTION LINES: {total_intersection_lines}")
    else:
        print("✅ No intersection found - all changed lines are covered!")

    print()

    # Summary
    print("📋 SUMMARY")
    print("-" * 50)
    print(f"Files with changes: {len(changed)}")
    print(f"Files with uncovered lines: {len(uncovered)}")
    print(f"Files with changed-but-uncovered lines: {len(intersections)}")

    if total_changed_lines > 0:
        coverage_gap_pct = (total_intersection_lines / total_changed_lines) * 100
        print(f"Coverage gap in changed code: {total_intersection_lines}/{total_changed_lines} lines ({coverage_gap_pct:.1f}%)")

        if coverage_gap_pct < 5:
            print("✅ Excellent coverage! Less than 5% gap.")
        elif coverage_gap_pct < 15:
            print("👍 Good coverage. Consider adding tests for critical uncovered lines.")
        else:
            print("⚠️  Significant coverage gap. Testing recommended for important code paths.")

    print("\n" + "="*80)


def main():
    parser = argparse.ArgumentParser(
        description="Coverage Diff Analysis Tool - Compare Git commits to find changed but uncovered lines",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python3 coverage-diff-analysis.py master
  python3 coverage-diff-analysis.py bcce60036a98e08987669d0339c362dc893cae75 --quiet
  python3 coverage-diff-analysis.py HEAD~5 -q
  python3 coverage-diff-analysis.py master -q --ide-links

Note: Analyzes changes from old_commit to current working directory
      (including staged/unstaged changes)
        """
    )

    parser.add_argument(
        'old_commit',
        help='Git commit to compare against (e.g., master, HEAD~1, commit hash)'
    )

    parser.add_argument(
        '-q', '--quiet',
        action='store_true',
        help='Suppress verbose output and show only intersection results'
    )

    parser.add_argument(
        '--ide-links',
        action='store_true',
        help='Output in IDE-friendly clickable format (filename:line or filename:line-line)'
    )

    args = parser.parse_args()
    old_commit = args.old_commit
    quiet = args.quiet
    ide_links = args.ide_links

    # Verify we're in a git repository
    try:
        run_command(["git", "rev-parse", "--git-dir"])
    except:
        print("Error: Not in a Git repository.")
        sys.exit(1)

    # Verify old commit exists
    try:
        run_command(["git", "rev-parse", "--verify", old_commit])
    except:
        if not quiet:
            print(f"Error: Commit doesn't exist: {old_commit}")
        sys.exit(1)

    if not quiet:
        print("🚀 Starting coverage diff analysis...")

    # Set quiet mode for other functions
    get_changed_lines.quiet = quiet  # type: ignore
    get_uncovered_lines.quiet = quiet  # type: ignore
    run_command.quiet = quiet  # type: ignore

    # Get coverage data from current HEAD
    uncovered = get_uncovered_lines()

    # Get changed lines between old commit and HEAD
    changed = get_changed_lines(old_commit)

    # Find intersections
    intersections = find_intersections(changed, uncovered)

    # Print analysis
    print_analysis(changed, uncovered, intersections, old_commit, quiet, ide_links)

    if not quiet:
        print("✅ Analysis complete!")


if __name__ == "__main__":
    main()
