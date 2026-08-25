#/bin/bash

set -euo pipefail

# Regenerates the checked-in Cap'n Proto bindings.
#
# The build script does the actual work; this only has to trigger it with the pinned compiler on
# PATH. The build directory is kept apart from the main one because this runs under Nix, targeting a
# platform that need not match the host, and sharing a directory would invalidate the host's cache
# on every switch.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-.build/cargo-capnp}"

capnp --version
cargo check --quiet --all-targets
