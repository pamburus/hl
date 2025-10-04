#!/bin/bash

set -euo pipefail

# Run cargo update --dry-run and capture stderr (where cargo update writes its output)
input=$(cat - | ${SHELL:?} "$(dirname $0)/cargo-update-filter.sh")

output=$(echo "$input" | \
    sed -E '
        s/^[[:space:]]*Updating[[:space:]]+([a-zA-Z0-9_-]+)[[:space:]]+v([^[:space:]]+)[[:space:]]+->[[:space:]]+v([^[:space:]]+)/| \1 | v\2 | v\3 |/
        s/^[[:space:]]*Removing[[:space:]]+([a-zA-Z0-9_-]+)[[:space:]]+v([^[:space:]]+)/| \1 | v\2 | ❌ |/
    '\
)

if [ -n "$output" ]; then
    echo "| Package | From | To |"
    echo "| --- | --- | --- |"
    echo "$output"
fi
