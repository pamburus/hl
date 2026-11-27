#!/bin/bash

set -euo pipefail

# cargo-audit exits with an error when it finds vulnerabilities, so its status says nothing here.
# The workflow audits again after this script, which catches any failure to produce a report.
report=$(cargo audit --json || true)

jq -r '.vulnerabilities.list[].package | "\(.name)@\(.version)"' <<<"${report}" | sort -u | while read -r package; do
    cargo update --package "${package:?}"
done
