#!/bin/bash

set -e

export RUSTFLAGS="-C instrument-coverage"
export CARGO_TARGET_DIR="target/coverage"
export LLVM_PROFILE_FILE="target/coverage/test-%m-%p.profraw"

LLVM_BIN=$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's|host: ||p')/bin

LLVM_PROFILE_PATTERN="target/coverage/test-*.profraw"
PROFDATA_FILE="target/coverage.profdata"
IGNORE=(
    '/.cargo/git/checkouts/'
    '/.cargo/registry/'
    '/target/coverage/debug/'
    'rustc/.*/library/'
    '_capnp.rs$'
)

function executables() {
    cargo test --tests --no-run --message-format=json \
    | jq -r 'select(.profile.test == true) | .filenames[]' \
    | grep -v dSYM -
}

LLVM_COV_FLAGS=(
    "${IGNORE[@]/#/--ignore-filename-regex=}"
    "--instr-profile=${PROFDATA_FILE:?}"
    $(executables | xargs -I {} echo -object {})
)

function clean() {
    rm -f ${LLVM_PROFILE_PATTERN:?}
}

function test() {
    cargo test --tests
} 

function merge() {
    ${LLVM_BIN:?}/llvm-profdata merge \
        -sparse ${LLVM_PROFILE_PATTERN:?} \
        -o ${PROFDATA_FILE:?}
}

function report() {
    ${LLVM_BIN:?}/llvm-cov \
        report \
        --summary-only \
        "${LLVM_COV_FLAGS[@]}"
}

function export() {
    ${LLVM_BIN:?}/llvm-cov \
        export \
        --format="lcov" \
        "${LLVM_COV_FLAGS[@]}" \
    > target/coverage.lcov
}

clean; test; merge; report; export
