#!/bin/bash
# Wrapper script for reorganizing Rust inline tests
# This script provides a convenient interface to the Python reorganization tool

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PYTHON_SCRIPT="$SCRIPT_DIR/reorganize_tests.py"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
DRY_RUN=false
SRC_DIR="src"

# Help text
usage() {
    cat << EOF
${BLUE}Reorganize Inline Unit Tests${NC}

This script reorganizes inline test modules in Rust source files by moving them
to separate tests.rs files in module-specific subdirectories.

${BLUE}Usage:${NC}
    $(basename "$0") [OPTIONS] [SRC_DIR]

${BLUE}Arguments:${NC}
    SRC_DIR                 Path to source directory (default: src)

${BLUE}Options:${NC}
    -d, --dry-run          Show what would be done without making changes
    -v, --verbose          Print detailed information
    -h, --help             Show this help message

${BLUE}Examples:${NC}
    # Preview changes in default src directory
    $(basename "$0") --dry-run

    # Apply reorganization to src directory
    $(basename "$0")

    # Preview changes in custom directory
    $(basename "$0") --dry-run path/to/src

    # Apply to custom directory with verbose output
    $(basename "$0") -v path/to/src

${BLUE}Workflow:${NC}
    1. Run with --dry-run to preview changes
    2. Review the output and ensure it looks correct
    3. Commit your current state to version control
    4. Run without --dry-run to apply changes
    5. Run 'cargo test' to verify tests still work
    6. Commit the reorganized code

${YELLOW}Note:${NC} Always run with --dry-run first to verify the changes!

EOF
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -d|--dry-run)
            DRY_RUN=true
            shift
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        -*)
            echo -e "${RED}Error: Unknown option: $1${NC}" >&2
            usage
            exit 1
            ;;
        *)
            SRC_DIR="$1"
            shift
            ;;
    esac
done

# Verify Python script exists
if [ ! -f "$PYTHON_SCRIPT" ]; then
    echo -e "${RED}Error: Python script not found at $PYTHON_SCRIPT${NC}" >&2
    exit 1
fi

# Verify source directory exists
if [ ! -d "$PROJECT_ROOT/$SRC_DIR" ]; then
    echo -e "${RED}Error: Source directory not found: $PROJECT_ROOT/$SRC_DIR${NC}" >&2
    exit 1
fi

# Show what we're about to do
echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║${NC}  Reorganizing Inline Unit Tests                        ${BLUE}║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo

if [ "$DRY_RUN" = true ]; then
    echo -e "${YELLOW}[DRY RUN]${NC} Changes will NOT be applied"
else
    echo -e "${GREEN}[LIVE RUN]${NC} Changes WILL be applied"
fi

echo "Source directory: $SRC_DIR"
echo

# Run the Python script
if cd "$PROJECT_ROOT"; then
    if [ "$DRY_RUN" = true ]; then
        python3 "$PYTHON_SCRIPT" --dry-run "$SRC_DIR"
    else
        python3 "$PYTHON_SCRIPT" "$SRC_DIR"
    fi

    RESULT=$?

    if [ $RESULT -eq 0 ]; then
        echo
        echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
        if [ "$DRY_RUN" = true ]; then
            echo -e "${BLUE}║${NC}  Dry Run Complete                                       ${BLUE}║${NC}"
            echo -e "${BLUE}║${NC}  Review the output above and run without --dry-run      ${BLUE}║${NC}"
            echo -e "${BLUE}║${NC}  to apply these changes.                               ${BLUE}║${NC}"
        else
            echo -e "${BLUE}║${NC}  Reorganization Complete                                ${BLUE}║${NC}"
            echo -e "${BLUE}║${NC}  ✓ Test modules have been moved to separate files      ${BLUE}║${NC}"
            echo -e "${BLUE}║${NC}  Next: Run 'cargo test' to verify everything works     ${BLUE}║${NC}"
        fi
        echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
        exit 0
    else
        echo -e "${RED}Error: Python script failed${NC}" >&2
        exit 1
    fi
else
    echo -e "${RED}Error: Could not change to project directory${NC}" >&2
    exit 1
fi
