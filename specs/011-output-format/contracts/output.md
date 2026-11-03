# Contract: Record Output and Diagnostics

**Feature**: [../spec.md](../spec.md) | **Data model**: [../data-model.md](../data-model.md)

## Stream shape

- **Stdout** (or the `-o` file): a sequence of records, each followed by the output delimiter (`\n` by default, `\0` with `--output-delimiter=nul`). Nothing else is ever written to it.
- **Partial records**: never visible. In follow mode, output is flushed after each batch.
- **Stderr**: diagnostic entries, only when `HL_DEBUG_LOG` enables them.

## JSON record

A single-line JSON object (NDJSON) with no insignificant whitespace.

```text
record      = "{" [ entry *( "," entry ) ] "}"
entry       = json-string ":" json-value
```

**Key order**: predefined entries, then the input entry, then regular entries (data model, NormalizedRecord).

**Examples**, with default settings:

```json
{"timestamp":"2026-07-06T10:00:00.123Z","level":"warning","logger":"api","message":"slow request","caller":"handler.go:42","http":{"status":500},"latency":1.25}
{"timestamp":"2023-11-14T22:13:20.000Z","level":"info","message":"started"}
{"level":"error","message":{"code":7,"text":"boom"}}
```

With `--flatten=always`:

```json
{"level":"info","message":"done","http.status":200,"items":[{"id":1,"tags":{"a":1}}]}
```

With the ECS preset, whose output keys are dotted, under the default flatten mode for JSON:

```json
{"@timestamp":"2026-07-06T10:00:00.000Z","log":{"level":"info","logger":"app","origin":{"function":"main","file":{"name":"main.go","line":12}}},"message":"hello","service":{"name":"api"}}
```

## Logfmt record

One line of `key=value` pairs separated by single spaces.

```text
record      = [ pair *( " " pair ) ]
pair        = key "=" value
key         = bare-key / json-string
value       = bare-value / json-string / json-number / "true" / "false" / "null"
```

- **`bare-key`**: accepted by `is_bare_key` in `serde-logfmt`, the same set hl's parser accepts.
- **`bare-value`**: accepted by `is_bare_value`, does not start with `"`, and does not read back as a number, boolean or null.
- **`json-string`**: JSON string syntax with non-ASCII characters left as UTF-8.

**Examples**, with default settings:

```text
timestamp=2026-07-06T10:00:00.123Z level=warning logger=api message="slow request" caller=handler.go:42 http.status=500 latency=1.25
level=info message=started id="42" ok="true" empty="" "user name"=alice tags="[\"a\",\"b\"]"
```

With `--flatten=never`:

```text
level=info message=done http="{\"status\":200}"
```

## Reading the output back

Every record line parses with a standard parser for its format (SC-001).

hl reads its own output back: running `hl --output-format=json` on hl's logfmt output, or `--output-format=logfmt` on hl's JSON output, preserves keys, values and value types (SC-003).

## Diagnostic entries

Diagnostic entries are written through hl's internal logger to stderr, in the logger's usual line format. Their exact wording is not part of the contract, but each one contains the listed information.

| Level | When | Contains |
|---|---|---|
| debug | each dropped entry, remainder, prefix or part of an oversized entry | source name, byte range, kind, preview (up to 100 characters) |
| debug | a repeated key was resolved (FR-009a) | source name, byte range of the record, key |
| debug | a regular field collided with a predefined or input output key (FR-027) | source name, byte range of the record, key |
| info | at exit, per source with drops | source name, number of dropped pieces |

Example with `HL_DEBUG_LOG=debug`:

```text
[2026-10-21T10:00:00.000000Z DEBUG hl::app] dropped unparsed entry: source=app.log range=1024..1064 preview="panic: runtime error: index out of range"
[2026-10-21T10:00:00.000100Z INFO  hl::app] dropped 3 pieces from app.log
```
