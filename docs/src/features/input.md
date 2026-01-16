# Input Options

`hl` provides flexible options for handling various input formats, timestamp conventions, and non-standard log formats. These options allow you to process logs from diverse sources and systems.

## Overview

Input options control how `hl` reads and interprets log data:

- **Input formats** — JSON, logfmt, or automatic detection
- **Timestamp handling** — Unix timestamp units and time parsing
- **Non-JSON prefixes** — handling logs with text before JSON entries
- **Entry delimiters** — controlling how log entries are separated

## Quick Examples

```sh
# Force JSON input format
hl --input-format json app.log

# Handle logfmt format
hl --input-format logfmt app.log

# Allow non-JSON prefixes (Docker, systemd logs)
hl --allow-prefix docker.log

# Specify Unix timestamp unit
hl --unix-timestamp-unit ms app.log

# Use custom delimiter
hl --delimiter nul binary.log
```

## Input Format Detection

By default, `hl` automatically detects the input format:

```sh
# Auto-detect format (default)
hl app.log
```

The auto-detection works by examining the first few lines of input and identifying:
- JSON entries (objects starting with `{`)
- Logfmt entries (key=value pairs)
- Mixed formats (handled line by line)

### Explicit Format Specification

You can force a specific format:

```sh
# Force JSON parsing
hl --input-format json app.log

# Force logfmt parsing
hl --input-format logfmt app.log

# Auto-detect (explicit)
hl --input-format auto app.log
```

**When to use explicit format:**
- Input contains ambiguous patterns
- You want to fail fast on format mismatches
- Performance optimization (skip detection phase)
- Strict validation requirements

See [Input Formats](./input-formats.md) for detailed format specifications.

## Timestamp Handling

`hl` automatically detects and parses various timestamp formats, but you can provide hints for better accuracy.

### Unix Timestamp Units

When log entries use Unix timestamps (numeric values), specify the unit:

```sh
# Timestamps are in milliseconds
hl --unix-timestamp-unit ms app.log

# Timestamps are in seconds
hl --unix-timestamp-unit s app.log

# Timestamps are in microseconds
hl --unix-timestamp-unit us app.log

# Timestamps are in nanoseconds
hl --unix-timestamp-unit ns app.log

# Auto-detect (default)
hl --unix-timestamp-unit auto app.log
```

**Default:** `auto`

Auto-detection examines timestamp values and infers the unit based on magnitude and patterns. Explicit specification ensures correct parsing when auto-detection might be ambiguous.

Example:
```json
{"timestamp": 1705315845123, "message": "event"}
```

- With `--unix-timestamp-unit ms`: interpreted as `2024-01-15 10:30:45.123`
- With `--unix-timestamp-unit s`: interpreted as `56087-09-27 ...` (incorrect)
- With `auto`: correctly detects milliseconds

See [Timestamp Handling](./timestamps.md) for detailed timestamp parsing behavior.

## Non-JSON Prefixes

Many log sources add prefixes before JSON entries:

```
2024-01-15 10:30:45 server1: {"level":"info","message":"started"}
Jan 15 10:30:45 myapp[1234]: {"level":"info","message":"started"}
```

Use `--allow-prefix` to handle these:

```sh
# Allow and skip non-JSON prefixes
hl --allow-prefix docker.log
hl --allow-prefix systemd.log
hl --allow-prefix syslog.json
```

When enabled, `hl`:
1. Scans each line for the first `{` character
2. Attempts to parse from that point as JSON
3. Ignores any text before the `{`

This is useful for:
- Docker container logs
- Systemd journal exports
- Syslog with JSON payloads
- Custom logging frameworks that add metadata prefixes

See [Non-JSON Prefixes](./prefixes.md) for more details and examples.

## Entry Delimiters

By default, `hl` automatically detects log entry boundaries. You can explicitly set the delimiter:

```sh
# Auto delimiter (default) - smart newline + skip continuation lines
hl --delimiter auto app.log

# Line feed only (strict Unix style)
hl --delimiter lf app.log

# Smart newline (accepts LF or CRLF)
hl --delimiter crlf app.log

# Carriage return only (old Mac style)
hl --delimiter cr app.log

# Null byte (for null-delimited logs)
hl --delimiter nul binary.log
```

**Default:** `auto`

### How Delimiters Work

- **`lf`**: Treats `\n` (line feed) as delimiter. Strict Unix newlines only.
- **`cr`**: Treats `\r` (carriage return) as delimiter. Old Mac style.
- **`crlf`**: Accepts **either** `\n` or `\r\n` as delimiter (not strict CRLF only). This is a "smart newline" that works with both Unix and Windows line endings.
- **`nul`**: Treats null byte (`\0`) as delimiter. For null-delimited streams.
- **`auto`**: Smart newline (accepts `\n` or `\r\n`) **plus** continuation line detection. Skips newlines followed by lines starting with `}`, space, or tab. This works well for pretty-printed JSON and most structured logs.

**Note:** The `crlf` delimiter name is somewhat misleading - it accepts both LF-only and CRLF line endings, making it compatible with both Unix and Windows files. It does not require strict `\r\n` sequences.

### When to Use Each Delimiter

**Use `auto` (default)** for:
- Pretty-printed JSON logs
- Mixed or unknown line endings
- Logs with multi-line entries

**Use `lf`** for:
- Strict Unix/Linux logs
- Single-line JSON/logfmt entries
- When you know entries never span multiple lines

**Use `crlf`** for:
- Logs that may have Unix or Windows line endings
- When you want flexibility without continuation line detection
- Single-line entries from mixed sources

**Use `nul`** for:
- Null-delimited output from tools
- Binary log formats

### Null-Delimited Logs

Some tools output null-delimited JSON for safe handling of multi-line values:

```sh
# Process null-delimited log stream
producer-tool | hl --delimiter nul
```

This is useful when log entries themselves contain newlines.

## Combining Input Options

All input options can be combined:

```sh
# Handle Docker logs with millisecond timestamps
hl --allow-prefix \
   --unix-timestamp-unit ms \
   --input-format json \
   docker-container.log

# Process custom format with explicit settings
hl --input-format logfmt \
   --delimiter crlf \
   windows-app.log
```

## Environment Variables

Input options can be set via environment variables:

```sh
export HL_INPUT_FORMAT=json
export HL_ALLOW_PREFIX=true
export HL_UNIX_TIMESTAMP_UNIT=ms
export HL_DELIMITER=lf

hl app.log
```

See [Environment Variables](../customization/environment.md) for the complete list.

## Configuration Files

Save frequently-used input options in configuration:

```toml
# ~/.config/hl/config.toml
input-format = "json"
allow-prefix = true
unix-timestamp-unit = "ms"
```

See [Configuration Files](../customization/config-files.md) for details.

## Performance Considerations

### Format Detection Overhead

Auto-detection adds minimal overhead (examines first few lines). For very high-performance scenarios with known formats, use explicit `--input-format`.

### Prefix Scanning

`--allow-prefix` adds a small overhead to scan for `{` on each line. If your logs don't have prefixes, omit this option for slightly better performance.

### Delimiter Detection

Auto delimiter detection is fast. Explicit delimiter specification provides no meaningful performance benefit.

## Examples

### Docker Container Logs

```sh
# Docker adds timestamps and container info before JSON
hl --allow-prefix /var/lib/docker/containers/*/container.log
```

### Systemd Journal Exports

```sh
# Systemd journal with JSON payloads
journalctl -u myservice -o json | hl --input-format json
```

### Application with Millisecond Timestamps

```sh
# Java application using millisecond timestamps
hl --unix-timestamp-unit ms java-app.log
```

### Mixed Format Logs

```sh
# Logs with both JSON and logfmt entries
hl --input-format auto mixed.log
```

### Windows Application Logs

```sh
# Windows line endings and logfmt format
hl --input-format logfmt --delimiter crlf app.log
```

### High-Precision Timestamps

```sh
# Logs with nanosecond precision
hl --unix-timestamp-unit ns high-precision.log
```

### Custom Logging Framework

```sh
# Framework adds prefix, uses JSON, millisecond timestamps
hl --allow-prefix \
   --input-format json \
   --unix-timestamp-unit ms \
   custom-framework.log
```

## Troubleshooting

### Entries Not Appearing

**Problem:** Log entries aren't showing up.

**Solutions:**
- Try `--allow-prefix` if entries have text before JSON
- Check `--input-format` matches actual format
- Verify delimiter with `--delimiter`

### Incorrect Timestamps

**Problem:** Timestamps are wrong (far in future/past).

**Solutions:**
- Specify correct `--unix-timestamp-unit`
- Check source timestamp format in raw logs
- Verify timezone settings

### Parse Errors

**Problem:** Getting parse errors or malformed entry warnings.

**Solutions:**
- Verify input format matches `--input-format`
- Check if `--allow-prefix` is needed
- Inspect raw log file for format issues
- Try `--input-format auto` to let `hl` detect

### Mixed Content

**Problem:** Some entries parse, others don't.

**Solutions:**
- Use `--input-format auto` for mixed formats
- Enable `--allow-prefix` if needed
- Check that all entries are actually JSON/logfmt

## When to Use Input Options

**Use `--input-format`:**
- Known format, want strict validation
- Performance-critical scenarios
- Avoiding ambiguous auto-detection

**Use `--allow-prefix`:**
- Docker/systemd/syslog logs
- Custom frameworks that add metadata
- Any logs with text before JSON

**Use `--unix-timestamp-unit`:**
- Timestamps are numeric Unix times
- Auto-detection gives wrong results
- Need explicit precision specification

**Use `--delimiter`:**
- Non-standard line endings (Windows)
- Null-delimited streams
- Custom entry separators

## Related Topics

- [Input Formats](./input-formats.md) — JSON and logfmt format details
- [Timestamp Handling](./timestamps.md) — timestamp parsing and formats
- [Non-JSON Prefixes](./prefixes.md) — handling prefixed logs
- [Configuration Files](../customization/config-files.md) — saving input preferences
- [Environment Variables](../customization/environment.md) — environment configuration
