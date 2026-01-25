# Input Formats

`hl` supports multiple structured log formats, with automatic detection and explicit format specification options.

## Supported Formats

`hl` can parse two main structured log formats:

- **JSON** — JavaScript Object Notation
- **Logfmt** — Key-value pair format popularized by Heroku

## JSON Format

JSON is the most common structured log format. `hl` expects one JSON object per line (newline-delimited JSON, also called JSONL or NDJSON).

### Basic JSON Logs

```json
{"timestamp":"2024-01-15T10:30:45.123Z","level":"info","message":"request processed"}
{"timestamp":"2024-01-15T10:30:46.456Z","level":"error","message":"database connection failed"}
```

Each line is a complete JSON object representing a single log entry.

### Nested JSON

`hl` handles nested objects and arrays:

```json
{"timestamp":"2024-01-15T10:30:45Z","user":{"id":123,"name":"Alice"},"tags":["important","auth"]}
```

By default, nested objects are flattened to dot notation in the output:
```
user.id: 123, user.name: "Alice", tags: ["important", "auth"]
```

### Pretty-Printed JSON

While `hl` prefers single-line JSON, it can handle pretty-printed JSON entries if they're properly delimited. However, performance is better with single-line entries.

### JSON Field Requirements

`hl` doesn't require specific fields – any valid JSON object is accepted. However, certain field names are recognized for special handling:

**Timestamp fields:**
- `timestamp`, `@timestamp`, `time`, `ts`, `t`
- `date`, `datetime`
- `_time`, `syslog_timestamp`

**Level fields:**
- `level`, `severity`, `loglevel`, `log_level`
- `PRIORITY` (systemd)

**Message fields:**
- `message`, `msg`, `MESSAGE`

See [Timestamp Handling](./timestamps.md) for timestamp format details.

### Common JSON Log Examples

**Standard application log:**
```json
{"timestamp":"2024-01-15T10:30:45.123Z","level":"info","service":"api","message":"request completed","duration_ms":145}
```

**ELK/Elasticsearch format:**
```json
{"@timestamp":"2024-01-15T10:30:45.123Z","level":"INFO","logger":"com.example.Service","message":"processing started"}
```

**Bunyan format (Node.js):**
```json
{"name":"myapp","hostname":"server1","pid":1234,"level":30,"msg":"request received","time":"2024-01-15T10:30:45.123Z","v":0}
```

**Winston format (Node.js):**
```json
{"level":"info","message":"user logged in","timestamp":"2024-01-15T10:30:45.123Z","user_id":123}
```

**Pino format (Node.js):**
```json
{"level":30,"time":1705315845123,"pid":1234,"hostname":"server1","msg":"request processed"}
```

**Logrus format (Go):**
```json
{"level":"info","msg":"starting server","time":"2024-01-15T10:30:45Z","port":8080}
```

**Zerolog format (Go):**
```json
{"level":"info","time":"2024-01-15T10:30:45Z","message":"server started","port":8080}
```

**Python structlog:**
```json
{"event":"user_login","level":"info","timestamp":"2024-01-15T10:30:45.123Z","user_id":123}
```

All these formats work with `hl` out of the box.

## Logfmt Format

Logfmt is a key-value pair format where each entry is a single line with space-separated key=value pairs.

### Basic Logfmt

```
timestamp=2024-01-15T10:30:45Z level=info message="request processed" duration_ms=145
timestamp=2024-01-15T10:30:46Z level=error message="connection failed" error="timeout"
```

### Logfmt Syntax Rules

- **Key-value pairs:** `key=value` separated by spaces
- **Quoted values:** Values with spaces must be quoted: `message="hello world"`
- **Unquoted values:** Simple values don't need quotes: `level=info`, `count=42`
- **Boolean values:** `enabled=true`, `debug=false`
- **Numeric values:** `count=123`, `duration=1.5`

### Logfmt Examples

**Simple log entry:**
```
time=2024-01-15T10:30:45Z level=info msg="server started" port=8080
```

**With quoted strings:**
```
ts=2024-01-15T10:30:45Z level=error msg="database error" error="connection timeout" query="SELECT * FROM users"
```

**With nested context (flat representation):**
```
timestamp=2024-01-15T10:30:45Z level=info user.id=123 user.name=Alice action=login
```

**Heroku-style logs:**
```
at=info method=GET path=/api/users status=200 duration=145ms
```

### Logfmt Field Names

Field names in logfmt:
- Can contain letters, numbers, underscores, hyphens, and dots
- Are case-sensitive
- Should not contain spaces or special characters

Common logfmt conventions:
- `time`, `ts`, `timestamp` for timestamps
- `level`, `lvl` for log levels
- `msg`, `message` for messages
- Dot notation for nested context: `user.id`, `request.method`

## Format Auto-Detection

By default, `hl` automatically detects the input format:

```sh
hl app.log
```

### How Auto-Detection Works

1. **Reads initial lines** from the input (typically first 10-100 lines)
2. **Analyzes patterns:**
   - Lines starting with `{` → likely JSON
   - Lines with `key=value` patterns → likely logfmt
   - Mixed patterns → processes line by line
3. **Applies detected format** for the rest of the input
4. **Falls back** to line-by-line detection for mixed inputs

### Auto-Detection Accuracy

Auto-detection is very reliable for:
- Pure JSON logs
- Pure logfmt logs
- Consistent formats throughout the file

Auto-detection may be ambiguous for:
- Files with very few entries
- Highly irregular formats
- Custom formats that resemble but aren't valid JSON/logfmt

### Mixed Format Handling

When files contain both JSON and logfmt entries, `hl` processes each line according to its format:

```
{"timestamp":"2024-01-15T10:30:45Z","level":"info","message":"json entry"}
timestamp=2024-01-15T10:30:46Z level=info message="logfmt entry"
{"timestamp":"2024-01-15T10:30:47Z","level":"info","message":"another json entry"}
```

All entries are parsed and displayed correctly.

## Explicit Format Specification

Force a specific format when auto-detection isn't appropriate:

```sh
# Force JSON parsing
hl --input-format json app.log

# Force logfmt parsing
hl --input-format logfmt app.log

# Explicit auto-detection
hl --input-format auto app.log
```

### When to Use Explicit Format

**Use explicit JSON (`--input-format json`):**
- Strict validation required
- Want to fail on non-JSON entries
- Performance optimization for large files
- Input is known to be pure JSON

**Use explicit logfmt (`--input-format logfmt`):**
- Processing logfmt-only sources
- Want to fail on invalid logfmt
- Disambiguation in edge cases

**Use auto (`--input-format auto`):**
- Mixed or unknown formats
- Maximum flexibility
- Default behavior

## Format-Specific Behavior

### JSON Parsing

When parsing JSON, `hl`:
- Requires valid JSON objects (one per line or delimited)
- Skips lines that aren't valid JSON (with warnings)
- Preserves all fields and their types (strings, numbers, booleans, nulls, arrays, objects)
- Handles Unicode and escaped characters

### Logfmt Parsing

When parsing logfmt, `hl`:
- Parses key=value pairs according to logfmt specification
- Handles quoted and unquoted values
- Preserves value types when possible (numbers, booleans)
- Treats unrecognized content as unparseable (skips with warnings)

## Performance Considerations

### JSON vs Logfmt

- **JSON parsing** is slightly faster due to more mature parsing libraries
- **Logfmt parsing** is also efficient, with minimal overhead
- **Auto-detection** adds negligible overhead for most use cases

### Optimization Tips

For maximum performance with known formats:

```sh
# Skip auto-detection for large JSON files
hl --input-format json huge-file.json

# Process compressed files (format detection still works)
hl --input-format json large-file.json.gz
```

## Examples

### Pure JSON Logs

```sh
# Auto-detect and process JSON logs
hl application.log

# Explicit JSON with filtering
hl --input-format json --level error app.log
```

### Pure Logfmt Logs

```sh
# Process Heroku-style logfmt logs
hl --input-format logfmt heroku.log

# Logfmt with queries
hl --input-format logfmt --query 'status >= 400' access.log
```

### Mixed Format Logs

```sh
# Auto-detect handles mixed formats
hl mixed.log

# Sort mixed-format entries chronologically
hl --sort mixed.log
```

### Converting Between Formats

```sh
# Read logfmt, output as JSON (using --raw)
hl --input-format logfmt --raw app.log > app.json

# Read JSON, view formatted (default)
hl --input-format json app.json
```

### Docker Logs with JSON

```sh
# Docker adds prefixes before JSON
hl --input-format json --allow-prefix /var/lib/docker/containers/*/container.log
```

## Format Validation

### Strict Validation

Use explicit format specification for strict validation:

```sh
# Fail if any line is not valid JSON
hl --input-format json strict-app.log
```

Non-parseable lines will generate warnings and be skipped.

### Lenient Processing

Use auto-detection for lenient processing:

```sh
# Process whatever can be parsed, skip the rest
hl --input-format auto mixed-quality.log
```

## Configuration

Set default format in configuration:

```toml
# ~/.config/hl/config.toml
input-format = "json"
```

Or via environment variable:

```sh
export HL_INPUT_FORMAT=json
hl app.log
```

## Troubleshooting

### Entries Not Parsing

**Problem:** Entries aren't being recognized.

**Solutions:**
- Check if entries are valid JSON/logfmt
- Try `--allow-prefix` if there's text before JSON
- Verify line endings (use `--delimiter`)
- Inspect raw file: `cat app.log | head`

### Wrong Format Detected

**Problem:** Auto-detection chooses wrong format.

**Solutions:**
- Use explicit `--input-format json` or `--input-format logfmt`
- Check first few lines — they influence detection
- Ensure entries are well-formed

### Mixed Format Issues

**Problem:** Some entries parse, others don't.

**Solutions:**
- Use `--input-format auto` (default)
- Check that mixed entries are both valid formats
- Verify no corruption or encoding issues

## Related Topics

- [Input Options](./input.md) — overview of input handling
- [Timestamp Handling](./timestamps.md) — parsing timestamps in various formats
- [Non-JSON Prefixes](./prefixes.md) — handling prefixed logs
- [Filtering](./filtering.md) — filtering works on all formats
