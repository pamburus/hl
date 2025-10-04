#!/bin/bash

set -euo pipefail

sed 's/\x1b\[[0-9;]*m//g' | grep -E "^[[:space:]]*(Updating|Removing)[[:space:]]+[a-zA-Z0-9_-]+[[:space:]]+v" || true
