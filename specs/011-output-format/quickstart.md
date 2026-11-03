# Quickstart: Validating Machine-Readable Output Formats

**Feature**: [spec.md](./spec.md) | **Contracts**: [contracts/cli.md](./contracts/cli.md), [contracts/output.md](./contracts/output.md)

These scenarios prove the feature end to end on a local build. The expected outputs follow the contracts; this guide doesn't restate the formatting rules.

## Prerequisites

- A build from this branch: `cargo build`, with the binary at `target/debug/hl`.
- `jq` on `PATH`, for scenarios 1 and 6.
- A scratch directory with the sample files below.

```sh
mkdir -p /tmp/hl-of && cd /tmp/hl-of
cat > app.log <<'EOF'
{"ts":"2026-07-06T10:00:00.123Z","level":"info","msg":"started","http":{"status":200}}
panic: something went wrong
{"ts":"2026-07-06T10:00:01Z","level":"WARN","msg":"slow","latency":1.25,"secret":"x"}
{"ts":"2026-07-06T10:00:02Z","level":"error","msg":"failed","tags":["a","b"]}
EOF
cat > app.logfmt <<'EOF'
ts=1700000000000 level=info msg="hello world" n=42 s="42" ok=true nothing=null plus=+5 "user name"=alice
ts=1700000001000 level=debug msg=again n=.5
EOF
```

## Scenario 1: filtered JSON for a pipeline (User Story 1)

```sh
hl --output-format=json -l warning app.log | jq -c '{level, message}'
```

**Expected**: two lines, `{"level":"warning","message":"slow"}` and `{"level":"error","message":"failed"}`. `jq` reports no parse error, and the `panic` line is absent.

## Scenario 2: types from logfmt (US1, scenario 2; FR-010, FR-010a)

```sh
hl --output-format=json app.logfmt
```

**Expected**:
- `"n":42`, `"s":"42"`, `"ok":true`, `"nothing":null`, `"plus":5`, and `"n":0.5` on the second line;
- `"timestamp":"2023-11-14T22:13:20.000Z"`, with three fraction digits from the millisecond unit;
- `"user name":"alice"`.

## Scenario 3: JSON to logfmt and nesting (User Story 2)

```sh
hl --output-format=logfmt app.log
hl --output-format=json app.log | head -1
hl --output-format=json --flatten=always app.log | head -1
```

**Expected**:
- logfmt has `http.status=200`, a bare `message=started`, and `tags="[\"a\",\"b\"]"`;
- JSON by default has `"http":{"status":200}`;
- with `--flatten=always`, it has `"http.status":200`.

## Scenario 4: round trip (SC-003)

```sh
hl --output-format=json app.logfmt | hl --output-format=logfmt
```

**Expected**: the same keys, values and types as `app.logfmt`, with predefined fields renamed to their output keys, normalized level names, the RFC 3339 timestamp, and `plus=+5` becoming `plus=5`. `s="42"` stays quoted, and `"user name"=alice` keeps its quotes.

## Scenario 5: hiding, sorting, raw precedence (FR-014, FR-015, FR-016)

```sh
hl --output-format=json -h secret app.log | grep -c secret        # expected: 0
hl --output-format=json -s app.log app.log | wc -l                 # expected: 6
HL_OUTPUT_FORMAT=json hl -r app.log | diff - app.log && echo same  # expected: same
```

## Scenario 6: clean stdout and dropped-line diagnostics (User Story 4)

```sh
hl --output-format=json app.log 2>/dev/null | jq -e . > /dev/null && echo clean
HL_DEBUG_LOG=debug hl --output-format=json app.log 2>&1 >/dev/null | grep -i dropped
HL_DEBUG_LOG=info hl --output-format=json app.log 2>&1 >/dev/null | grep -i dropped
```

**Expected**:
1. `clean`.
2. A debug entry for `app.log` with the byte range `87..114` and the `panic:` preview, followed by an info summary of 1 dropped piece of unparsed text.
3. Only the info summary.

## Scenario 7: follow mode (User Story 3)

```sh
hl -F --output-format=json app.log > out.ndjson &
echo '{"ts":"2026-07-06T10:00:03Z","level":"info","msg":"live"}' >> app.log
sleep 1; tail -1 out.ndjson; kill %1
```

**Expected**: the last line of `out.ndjson` is the complete `live` record as JSON, with no badges, sync indicators or colors.

## Scenario 8: presets (FR-028)

```sh
hl --config etc/defaults/config-ecs.toml --output-format=json <ecs-sample.log>
```

**Expected**: `@timestamp` comes first, then a single `log` object containing `level`, `logger` and `origin.function`, `origin.file.name` and `origin.file.line`. `log.level` doesn't appear a second time.

## Automated checks

```sh
cargo test                      # unit, app-level and property tests
cargo bench --bench bench -- combined   # compare format-json / format-logfmt with parse-and-format
just uncovered                  # changed lines without coverage
just ci                         # full gate before each commit
```
