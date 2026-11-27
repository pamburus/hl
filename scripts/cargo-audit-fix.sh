#!/bin/bash

set -euo pipefail

# cargo-audit exits with an error when it finds vulnerabilities, so its status says nothing here.
# The workflow audits again after this script, which catches any failure to produce a report.
report=$(cargo audit --json || true)

packages=$(jq -r '.vulnerabilities.list[].package | "\(.name)@\(.version)"' <<<"${report}" | sort -u)
if [[ -n "$packages" ]]; then
    update_args=()
    while IFS= read -r package; do
        update_args+=(--package "${package:?}")
    done <<< "$packages"
    # Resolve all specs against the same lockfile before any transitive versions change.
    cargo update "${update_args[@]}"
fi
