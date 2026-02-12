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
    '/crates/styled-help/src/'
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
        crates/*/${LLVM_PROFILE_PATTERN:?}
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
    $hl $fixture | check_hash "e8fa6e11738adb14ae4dbaa6bdc76778fac74c079cdb144da04bf2ab963ddd69"
    # --delimiter auto: same as default
    $hl $fixture --delimiter auto | check_hash "e8fa6e11738adb14ae4dbaa6bdc76778fac74c079cdb144da04bf2ab963ddd69"
    # --delimiter crlf: parses first jsonl entry only; everything else is raw
    $hl $fixture --delimiter crlf | check_hash "38597e9ed4d0b26d28eabc1f2f82006b76731fe56c9a3f9c301c1f1327d0d1d2"
    # --delimiter newline: parses jsonl, logfmt; shows prefixed, pretty, pretty-stripped raw
    $hl $fixture --delimiter newline | check_hash "b6b8befa4872f54ca321e3618e83ad46f6543f82cb378cf3c9051b0c85dca9fc"

    # --input-format json: parses only json entries (jsonl, pretty, pretty-stripped)
    $hl $fixture --input-format json | check_hash "3466aeccb479e6d925fdc96319db8de4343813c87fff6cf38c461dcb9dec47f0"
    # --input-format json --delimiter auto: same as above
    $hl $fixture --input-format json --delimiter auto | check_hash "3466aeccb479e6d925fdc96319db8de4343813c87fff6cf38c461dcb9dec47f0"
    # --input-format json --delimiter crlf: shows first jsonl entry only
    $hl $fixture --input-format json --delimiter crlf | check_hash "588779f9f3294429ea8c0b2132b81e862b53041f9dc85beddd40158a223171b8"
    # --input-format json --delimiter newline: shows first jsonl entry only
    $hl $fixture --input-format json --delimiter newline | check_hash "588779f9f3294429ea8c0b2132b81e862b53041f9dc85beddd40158a223171b8"
    # --input-format json --allow-prefix: parses jsonl and prefixed json entries
    $hl $fixture --input-format json --allow-prefix | check_hash "28c4157badd62d80e185ad48e04b263f8e3623ff354598e736976b408f8011bb"

    # --input-format logfmt: parses only logfmt entry
    $hl $fixture --input-format logfmt | check_hash "bef7a37564bcf3fbe1182a0877e1d2cc3d7bfc762f6d823e71c6bd2e6a744ad8"
    # --input-format logfmt --delimiter auto: same as above
    $hl $fixture --input-format logfmt --delimiter auto | check_hash "bef7a37564bcf3fbe1182a0877e1d2cc3d7bfc762f6d823e71c6bd2e6a744ad8"
    # --input-format logfmt --delimiter crlf: shows nothing
    $hl $fixture --input-format logfmt --delimiter crlf | check_hash "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    # --input-format logfmt --delimiter newline: same as 'auto' above
    $hl $fixture --input-format logfmt --delimiter newline | check_hash "bef7a37564bcf3fbe1182a0877e1d2cc3d7bfc762f6d823e71c6bd2e6a744ad8"
    # --input-format logfmt --allow-prefix: same as above
    $hl $fixture --input-format logfmt --allow-prefix | check_hash "bef7a37564bcf3fbe1182a0877e1d2cc3d7bfc762f6d823e71c6bd2e6a744ad8"

    # --allow-prefix: parses jsonl, logfmt, prefixed, space-prefixed; shows pretty raw
    $hl $fixture --allow-prefix | check_hash "87f593c84d77286a7accaafb2e546e476adbace3356c53e121600264ef965378"
    # --allow-prefix --delimiter auto: same as above
    $hl $fixture --allow-prefix --delimiter auto | check_hash "87f593c84d77286a7accaafb2e546e476adbace3356c53e121600264ef965378"
    # --allow-prefix --delimiter crlf: parses first jsonl entry only; everything else is raw
    $hl $fixture --allow-prefix --delimiter crlf | check_hash "38597e9ed4d0b26d28eabc1f2f82006b76731fe56c9a3f9c301c1f1327d0d1d2"
    # --allow-prefix --delimiter newline: same as 'auto' above
    $hl $fixture --allow-prefix --delimiter newline | check_hash "87f593c84d77286a7accaafb2e546e476adbace3356c53e121600264ef965378"

    # Special delimiters: single test each
    $hl $fixture --delimiter lf | check_hash "b6b8befa4872f54ca321e3618e83ad46f6543f82cb378cf3c9051b0c85dca9fc"
    $hl $fixture --delimiter cr | check_hash "38597e9ed4d0b26d28eabc1f2f82006b76731fe56c9a3f9c301c1f1327d0d1d2"
    $hl $fixture --delimiter nul | check_hash "38597e9ed4d0b26d28eabc1f2f82006b76731fe56c9a3f9c301c1f1327d0d1d2"
}

function merge() {
    "${LLVM_BIN:?}/llvm-profdata" merge \
        -o ${PROFDATA_FILE:?} \
        -sparse \
        ${LLVM_PROFILE_PATTERN:?} \
        crates/*/${LLVM_PROFILE_PATTERN:?}
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
