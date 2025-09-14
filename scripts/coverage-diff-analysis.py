#!/usr/bin/env python3
"""
Coverage Diff Analysis Tool

This script compares two Git commits to find:
1. All changed line ranges across the entire workspace
2. All uncovered line ranges from coverage analysis
3. The intersection (changed but uncovered lines)

Usage:
    ./coverage-diff-analysis.py <old_commit>
    ./coverage-diff-analysis.py bcce60036a98e08987669d0339c362dc893cae75

Note: Analyzes changes from old_commit to current working directory (including staged/unstaged changes)

Requirements:
- Git repository with the commits
- Coverage tool that generates target/lcov.info (e.g., 'just coverage')
- Python 3.6+
"""

import subprocess
import sys
import os
import re
from typing import Dict, Set, List, Tuple
from pathlib import Path


def run_command(cmd: List[str], capture_output=True, cwd=None) -> subprocess.CompletedProcess:
    """Run a shell command and return the result."""
    try:
        result = subprocess.run(cmd, capture_output=capture_output, text=True, cwd=cwd, check=True)
        return result
    except subprocess.CalledProcessError as e:
        print(f"Error running command {' '.join(cmd)}: {e}")
        if e.stdout:
            print(f"stdout: {e.stdout}")
        if e.stderr:
            print(f"stderr: {e.stderr}")
        sys.exit(1)


def parse_ranges(range_str: str) -> Set[int]:
    """Parse range string like '25-27 30 35-40' into set of line numbers."""
    lines = set()
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
    ranges = []
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
    print(f"📝 Analyzing changes between {old_commit[:8]}..working directory")

    # Get unified diff with no context lines (includes staged and unstaged changes)
    cmd = ["git", "diff", "--unified=0", old_commit]
    result = run_command(cmd)

    changed_ranges = {}
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


def get_uncovered_lines() -> Dict[str, str]:
    """Get all uncovered line ranges from coverage report."""
    print("🔍 Running coverage analysis...")

    # Run coverage analysis
    if os.path.exists("justfile"):
        run_command(["just", "coverage"], capture_output=False)
    elif os.path.exists("Makefile"):
        run_command(["make", "coverage"], capture_output=False)
    else:
        print("Error: No justfile or Makefile found. Cannot run coverage.")
        sys.exit(1)

    # Parse lcov.info file
    lcov_path = Path("target/lcov.info")
    if not lcov_path.exists():
        print("Error: target/lcov.info not found. Coverage analysis may have failed.")
        sys.exit(1)

    uncovered_ranges = {}
    current_file = None
    uncovered_lines = []

    with open(lcov_path, 'r') as f:
        for line in f:
            line = line.strip()

            # Source file marker
            if line.startswith('SF:'):
                # Process previous file if any
                if current_file and uncovered_lines:
                    # Remove project root path prefix for consistency
                    clean_file = current_file
                    if clean_file.startswith('/'):
                        # Find the project root marker (usually contains 'hl')
                        if '/hl/' in clean_file:
                            # Extract everything after the project root
                            parts = clean_file.split('/hl/')
                            if len(parts) > 1:
                                clean_file = parts[-1]

                    # Sort and convert to ranges
                    uncovered_lines.sort()
                    ranges = []
                    if uncovered_lines:
                        start = uncovered_lines[0]
                        end = start

                        for line_num in uncovered_lines[1:]:
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

                    if ranges:
                        uncovered_ranges[clean_file] = " ".join(ranges)

                # Start new file
                current_file = line[3:]  # Remove "SF:"
                uncovered_lines = []

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
                        continue

        # Process the last file
        if current_file and uncovered_lines:
            clean_file = current_file
            if clean_file.startswith('/'):
                # Find the project root marker (usually contains 'hl')
                if '/hl/' in clean_file:
                    # Extract everything after the project root
                    parts = clean_file.split('/hl/')
                    if len(parts) > 1:
                        clean_file = parts[-1]

            uncovered_lines.sort()
            ranges = []
            if uncovered_lines:
                start = uncovered_lines[0]
                end = start

                for line_num in uncovered_lines[1:]:
                    if line_num == end + 1:
                        end = line_num
                    else:
                        if start == end:
                            ranges.append(str(start))
                        else:
                            ranges.append(f"{start}-{end}")
                        start = line_num
                        end = line_num

                if start == end:
                    ranges.append(str(start))
                else:
                    ranges.append(f"{start}-{end}")

            if ranges:
                uncovered_ranges[clean_file] = " ".join(ranges)

    return uncovered_ranges


def find_intersections(changed: Dict[str, str], uncovered: Dict[str, str]) -> Dict[str, str]:
    """Find intersection of changed and uncovered lines."""
    intersections = {}

    for file_path in set(changed.keys()) | set(uncovered.keys()):
        changed_lines = parse_ranges(changed.get(file_path, ""))
        uncovered_lines = parse_ranges(uncovered.get(file_path, ""))
        intersection = changed_lines & uncovered_lines

        if intersection:
            intersections[file_path] = ranges_to_string(intersection)

    return intersections


def print_analysis(changed: Dict[str, str], uncovered: Dict[str, str], intersections: Dict[str, str], old_commit: str):
    """Print the comprehensive analysis report."""
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
    """Main function."""
    if len(sys.argv) != 2:
        print("Usage: python3 coverage-diff-analysis.py <old_commit>")
        print("Example: python3 coverage-diff-analysis.py bcce60036a98e08987669d0339c362dc893cae75")
        sys.exit(1)

    old_commit = sys.argv[1]

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
        print(f"Error: Commit doesn't exist: {old_commit}")
        sys.exit(1)

    print("🚀 Starting coverage diff analysis...")

    # Get coverage data from current HEAD
    uncovered = get_uncovered_lines()

    # Get changed lines between old commit and HEAD
    changed = get_changed_lines(old_commit)

    # Find intersections
    intersections = find_intersections(changed, uncovered)

    # Print analysis
    print_analysis(changed, uncovered, intersections, old_commit)

    print("✅ Analysis complete!")


if __name__ == "__main__":
    main()
