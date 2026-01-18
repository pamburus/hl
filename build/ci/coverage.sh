#!/bin/bash

set -e

export RUSTFLAGS="-C instrument-coverage"
export CARGO_TARGET_DIR="target/coverage"
export LLVM_PROFILE_FILE="target/coverage/test-%m-%p.profraw"
export MAIN_EXECUTABLE="target/coverage/debug/hl"

LLVM_BIN=$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's|host: ||p')/bin

LLVM_PROFILE_PATTERN="target/coverage/test-*.profraw"
PROFDATA_FILE="target/coverage.profdata"
IGNORE=(
    '/.cargo/'
    '/.rustup/'
    '/target/coverage/debug/'
    'rustc/.*/library/'
    '_capnp\.rs$'
    '/tests\.rs$'
    '/crate/styled-help/src/'
)

function executables() {
    echo ${MAIN_EXECUTABLE:?}
    cargo test --workspace --tests --no-run --message-format=json \
    | jq -r 'select(.profile.test == true) | .filenames[]' \
    | grep -v dSYM -
}

LLVM_COV_FLAGS=(
    "${IGNORE[@]/#/--ignore-filename-regex=}"
    "--instr-profile=${PROFDATA_FILE:?}"
    $(executables | xargs -I {} echo -object {})
)

function clean() {
    rm -f \
        ${LLVM_PROFILE_PATTERN:?} \
        crate/*/${LLVM_PROFILE_PATTERN:?}
}

function check_hash() {
    local expected="$1"
    local actual
    actual=$(shasum -a 256 | cut -d' ' -f1)
    if [ "$actual" != "$expected" ]; then
        echo "${BASH_SOURCE[1]}:${BASH_LINENO[0]}: Hash mismatch: expected $expected, got $actual" >&2
        return 1
    fi
}

function test() {
    cargo test --tests --workspace
    cargo build
    ${MAIN_EXECUTABLE:?} --config - > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --help > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --help=short --color never > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --help=long -c --paging never > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --config=etc/defaults/config-k8s.toml > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --config=etc/defaults/config-ecs.toml > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --shell-completions bash > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --man-page > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --list-themes > /dev/null
    ${MAIN_EXECUTABLE:?} --config - --list-themes=dark,16color > /dev/null
    ${MAIN_EXECUTABLE:?} --config - sample/prometheus.log -P --theme frostline --color always > /dev/null
    HL_DEBUG_LOG=info ${MAIN_EXECUTABLE:?} --config - sample/prometheus.log -P -o /dev/null
    echo "" | ${MAIN_EXECUTABLE:?} --config - --concurrency 4 > /dev/null
    echo "level=info" | ${MAIN_EXECUTABLE:?} --config - sample/prometheus.log -P -q 'level=x' 2> /dev/null > /dev/null || true

    # Test delimiter options with combined fixture containing all log formats
    local fixture="src/testing/assets/fixtures/delimiter/combined.log"
    local hl="${MAIN_EXECUTABLE:?} --config - -P --color never"

    # Default (no options): parses jsonl, logfmt, pretty; shows prefixed and pretty-stripped raw
    $hl $fixture | check_hash "81e53d5944f93cfc5f26f9eca2c0a4cd99ec7ae48cfccf6f92011b66c6aa584d"
    # --delimiter auto: same as default
    $hl $fixture --delimiter auto | check_hash "81e53d5944f93cfc5f26f9eca2c0a4cd99ec7ae48cfccf6f92011b66c6aa584d"
    # --delimiter crlf: parses jsonl, logfmt; shows prefixed, pretty, pretty-stripped raw
    $hl $fixture --delimiter crlf | check_hash "1776b8f301b8809374bdb6fad936c5649f52d2a29c230c13db93db7b86a79e93"

    # --input-format json: parses only json entries (jsonl, pretty, pretty-stripped)
    $hl $fixture --input-format json | check_hash "32250d3814dd03f2c5692b31daa73e5273c80914a0c1935acc318ceeb16e9401"
    # --input-format json --delimiter auto: same as above
    $hl $fixture --input-format json --delimiter auto | check_hash "32250d3814dd03f2c5692b31daa73e5273c80914a0c1935acc318ceeb16e9401"
    # --input-format json --delimiter crlf: parses only jsonl (single-line json)
    $hl $fixture --input-format json --delimiter crlf | check_hash "0185e71755887aff184accfd23a8d3778d7d849d1b10a8ac7f0d828ddcbb8e80"
    # --input-format json --allow-prefix: parses jsonl and prefixed json entries
    $hl $fixture --input-format json --allow-prefix | check_hash "f6267579b4e3c4233af9f8dbd09110d34a5fc7556378ae134da3a2959e892e9d"

    # --input-format logfmt: parses only logfmt entry
    $hl $fixture --input-format logfmt | check_hash "604d0912654c2375a9487234de91b3204c2f70de3652e175596d7028f94fb3d1"
    # --input-format logfmt --delimiter auto: same as above
    $hl $fixture --input-format logfmt --delimiter auto | check_hash "604d0912654c2375a9487234de91b3204c2f70de3652e175596d7028f94fb3d1"
    # --input-format logfmt --delimiter crlf: same as above
    $hl $fixture --input-format logfmt --delimiter crlf | check_hash "604d0912654c2375a9487234de91b3204c2f70de3652e175596d7028f94fb3d1"
    # --input-format logfmt --allow-prefix: same as above
    $hl $fixture --input-format logfmt --allow-prefix | check_hash "604d0912654c2375a9487234de91b3204c2f70de3652e175596d7028f94fb3d1"

    # --allow-prefix: parses jsonl, logfmt, prefixed, space-prefixed; shows pretty raw
    $hl $fixture --allow-prefix | check_hash "9e56c27e1a2389eec5dfd37ea9e13310454686ce706e7a247bde66803bf1c17c"
    # --allow-prefix --delimiter auto: same as above
    $hl $fixture --allow-prefix --delimiter auto | check_hash "9e56c27e1a2389eec5dfd37ea9e13310454686ce706e7a247bde66803bf1c17c"
    # --allow-prefix --delimiter crlf: same as above
    $hl $fixture --allow-prefix --delimiter crlf | check_hash "9e56c27e1a2389eec5dfd37ea9e13310454686ce706e7a247bde66803bf1c17c"

    # Special delimiters: single test each
    $hl $fixture --delimiter lf | check_hash "1776b8f301b8809374bdb6fad936c5649f52d2a29c230c13db93db7b86a79e93"
    $hl $fixture --delimiter cr | check_hash "4fbecc426cb22b575ce64146a5229dde3923051366f4b3e6ffd24b2205ea6c18"
    $hl $fixture --delimiter nul | check_hash "4fbecc426cb22b575ce64146a5229dde3923051366f4b3e6ffd24b2205ea6c18"
}

function merge() {
    "${LLVM_BIN:?}/llvm-profdata" merge \
        -o ${PROFDATA_FILE:?} \
        -sparse \
        ${LLVM_PROFILE_PATTERN:?} \
        crate/*/${LLVM_PROFILE_PATTERN:?}
}

function report() {
    "${LLVM_BIN:?}/llvm-cov" \
        report \
        --show-region-summary=false \
        --show-branch-summary=false \
        --summary-only \
        "${LLVM_COV_FLAGS[@]}"
}

function publish() {
    "${LLVM_BIN:?}/llvm-cov" \
        export \
        --format="lcov" \
        "${LLVM_COV_FLAGS[@]}" \
    > target/lcov.info
}

clean; test; merge; report; publish
