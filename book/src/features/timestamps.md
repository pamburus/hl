# Timestamp Handling

`hl` automatically detects and parses timestamps in various formats from log entries. Understanding how timestamp handling works helps you work with logs from different sources and ensure correct chronological ordering.

## Overview

Timestamp handling in `hl` involves:

- **Automatic detection** of timestamp fields in log entries
- **Format recognition** supporting multiple timestamp formats
- **Unix timestamp parsing** with automatic unit detection
- **Timezone handling** for display and filtering
- **Fallback behavior** for entries without timestamps

## Timestamp Field Detection

`hl` looks for timestamps in several common field names:

### Standard Field Names

The following field names are recognized as timestamps (in priority order):

1. `@timestamp` — ELK/Elasticsearch standard
2. `timestamp` — Most common field name
3. `time` — Common shorthand
4. `ts` — Abbreviated form
5. `t` — Very short form
6. `date` — Explicit date field
7. `datetime` — Combined date and time
8. `_time` — Splunk-style
9. `syslog_timestamp` — Syslog convention

### Field Name Matching

- Field names are matched **case-insensitively**: `Timestamp`, `TIMESTAMP`, and `timestamp` all match
- Both top-level and nested fields are searched: `event.timestamp` works
- First matching field by priority is used

### Multiple Timestamp Fields

When an entry contains multiple timestamp fields, `hl` uses the highest priority one:

```json
{
  "@timestamp": "2024-01-15T10:30:45Z",
  "time": "2024-01-15T10:00:00Z",
  "created_at": "2024-01-15T09:00:00Z"
}
```

`hl` uses `@timestamp` (highest priority), ignoring the other time fields.

## Supported Timestamp Formats

`hl` recognizes many timestamp formats automatically:

### ISO 8601 / RFC 3339

The most common and recommended format:

```json
{"timestamp": "2024-01-15T10:30:45.123Z"}
{"timestamp": "2024-01-15T10:30:45+00:00"}
{"timestamp": "2024-01-15T10:30:45-07:00"}
{"timestamp": "2024-01-15T10:30:45.123456Z"}
```

Variations supported:
- With or without timezone: `Z`, `+00:00`, `-07:00`
- With or without fractional seconds: `.123`, `.123456`, `.123456789`
- With `T` separator or space: `2024-01-15T10:30:45` or `2024-01-15 10:30:45`

### Unix Timestamps

Numeric timestamps representing seconds, milliseconds, microseconds, or nanoseconds since epoch:

```json
{"timestamp": 1705315845}
{"timestamp": 1705315845123}
{"timestamp": 1705315845123456}
{"timestamp": 1705315845123456789}
{"ts": 1705315845.123}
```

Supported as:
- Integer values
- Floating-point values
- String values containing numbers

See [Unix Timestamp Units](#unix-timestamp-units) for unit detection.

### Common Log Formats

```json
{"time": "Jan 15 10:30:45"}
{"time": "Jan 15, 2024 10:30:45"}
{"time": "15/Jan/2024:10:30:45 +0000"}
{"timestamp": "2024-01-15 10:30:45"}
{"timestamp": "01/15/2024 10:30:45"}
```

### Custom Formats

Many custom timestamp formats are automatically recognized through heuristics. If your format isn't recognized, the entry will still be processed but may not sort correctly.

## Unix Timestamp Units

When timestamps are numeric Unix times, `hl` detects the unit:

### Automatic Detection

By default, `hl` automatically infers the unit based on magnitude:

```bash
hl app.log
```

Detection rules:
- **Seconds**: Values around 10 digits (e.g., `1705315845`)
- **Milliseconds**: Values around 13 digits (e.g., `1705315845123`)
- **Microseconds**: Values around 16 digits (e.g., `1705315845123456`)
- **Nanoseconds**: Values around 19 digits (e.g., `1705315845123456789`)

Example:
```json
{"timestamp": 1705315845}     → detected as seconds
{"timestamp": 1705315845123}  → detected as milliseconds
```

### Explicit Unit Specification

Override auto-detection when needed:

```bash
# Timestamps are in milliseconds
hl --unix-timestamp-unit ms app.log

# Timestamps are in seconds
hl --unix-timestamp-unit s app.log

# Timestamps are in microseconds
hl --unix-timestamp-unit us app.log

# Timestamps are in nanoseconds
hl --unix-timestamp-unit ns app.log
```

**When to specify explicitly:**
- Auto-detection is ambiguous (e.g., timestamps near magnitude boundaries)
- You know the exact unit and want to skip detection
- Validation: ensure all timestamps use expected unit

### Floating-Point Unix Timestamps

Floating-point values are also supported:

```json
{"timestamp": 1705315845.123}
{"ts": 1705315845.123456}
```

The fractional part represents sub-second precision regardless of the unit setting.

## Timezone Handling

### Source Timestamp Timezones

Timestamps in log entries can include timezone information:

```json
{"timestamp": "2024-01-15T10:30:45Z"}           // UTC
{"timestamp": "2024-01-15T10:30:45+00:00"}      // UTC explicit
{"timestamp": "2024-01-15T10:30:45-08:00"}      // PST
{"timestamp": "2024-01-15T10:30:45+05:30"}      // IST
```

`hl` parses the timezone and converts to a consistent internal representation.

### Display Timezone

Control how timestamps are **displayed** (doesn't affect filtering or sorting):

```bash
# Display in UTC (default)
hl app.log

# Display in local timezone
hl --local app.log

# Display in specific timezone
hl --time-zone 'America/New_York' app.log
hl --time-zone 'Europe/London' app.log
hl --time-zone 'Asia/Tokyo' app.log
```

The source timestamps remain unchanged; only the display format is affected.

See [Time Display](./time-display.md) for more on timezone display options.

### Timestamps Without Timezone

When timestamps lack timezone information:

```json
{"timestamp": "2024-01-15 10:30:45"}
```

`hl` typically assumes UTC. For local time interpretation, you may need to process timestamps differently or ensure your logs include timezone information.

## Entries Without Timestamps

Log entries without recognized timestamp fields are handled specially:

### In Normal Mode

Entries without timestamps are processed normally but treated as having no time information:

```bash
hl app.log
```

They appear in the output in their original order.

### In Sort Mode

Entries without timestamps are placed at the beginning of sorted output:

```bash
hl --sort app.log
```

They're treated as having a timestamp of zero (epoch start: 1970-01-01 00:00:00 UTC).

### In Follow Mode

Entries without recognized timestamps are **skipped** and not displayed:

```bash
hl -F app.log
```

This is a key behavioral difference—follow mode requires valid timestamps for chronological sorting.

### Time Filtering

Entries without timestamps are excluded by time filters:

```bash
# Entries without timestamps won't match
hl --since '1 hour ago' app.log
```

## Timestamp Parsing Performance

### Caching

`hl` caches timestamp format detection results to optimize parsing:

- First few entries determine the format
- Same format is applied to subsequent entries
- Format cache adapts if entries use different formats

### Performance Tips

For maximum performance:

```bash
# Use consistent timestamp formats in your logs
# ISO 8601 is recommended for fastest parsing
{"timestamp": "2024-01-15T10:30:45.123Z"}

# Avoid mixed formats in the same file
# Mixed formats force per-entry format detection
```

## Examples

### Standard ISO 8601 Logs

```json
{"timestamp":"2024-01-15T10:30:45.123Z","level":"info","message":"request processed"}
```

```bash
hl app.log
```

Timestamp is automatically detected and parsed.

### Unix Millisecond Timestamps

```json
{"ts":1705315845123,"level":"info","message":"event occurred"}
```

```bash
# Auto-detected as milliseconds
hl app.log

# Or explicit
hl --unix-timestamp-unit ms app.log
```

### Nested Timestamp Fields

```json
{"event":{"timestamp":"2024-01-15T10:30:45Z"},"data":"..."}
```

```bash
hl app.log
```

Nested timestamps are found and parsed automatically.

### Multiple Timestamp Formats

```json
{"timestamp":"2024-01-15T10:30:45Z","level":"info"}
{"ts":1705315845,"level":"info"}
{"time":"Jan 15 10:30:45","level":"info"}
```

```bash
hl app.log
```

Each entry's format is detected individually.

### Timezone-Aware Logs

```json
{"timestamp":"2024-01-15T10:30:45-08:00","level":"info","message":"PST event"}
{"timestamp":"2024-01-15T18:30:45Z","level":"info","message":"UTC event"}
```

```bash
# Display in local timezone
hl --local app.log

# Display in specific timezone
hl --time-zone 'America/Los_Angeles' app.log
```

Both timestamps are parsed correctly despite different source timezones.

## Configuration

Set timestamp handling defaults:

```toml
# ~/.config/hl/config.toml
unix-timestamp-unit = "ms"
time-zone = "local"
```

Or via environment variables:

```bash
export HL_UNIX_TIMESTAMP_UNIT=ms

# For local time, use -L flag
hl -L app.log
```

## Troubleshooting

### Timestamps Not Recognized

**Problem:** Entries appear without timestamps or at wrong times.

**Solutions:**
- Check field names—use standard names like `timestamp`, `time`, `ts`
- Verify timestamp format is one of the supported formats
- Inspect raw entry: `hl --raw app.log | head -1`
- Try explicit unit: `--unix-timestamp-unit ms`

### Wrong Timestamp Values

**Problem:** Timestamps are far in future/past.

**Solutions:**
- Specify correct `--unix-timestamp-unit`
- Check that numeric timestamps are actually Unix times
- Verify source log timestamp field contains valid values

### Entries Out of Order in Sort Mode

**Problem:** Sorted entries appear in wrong order.

**Solutions:**
- Ensure all entries have recognized timestamps
- Check that timestamps are in consistent format
- Verify timezone information is correct
- Test with `--sort` to see chronological order

### Entries Missing in Follow Mode

**Problem:** Some entries don't appear in follow mode.

**Cause:** Entries without recognized timestamps are skipped in follow mode.

**Solutions:**
- Ensure all entries have valid timestamp fields
- Use standard field names (`timestamp`, `time`, etc.)
- Check timestamp format is recognized
- For debugging, use piped input: `tail -f app.log | hl -P`

### Mixed Timestamp Formats

**Problem:** File has multiple timestamp formats, parsing is slow.

**Solutions:**
- Standardize timestamp format in logs if possible
- ISO 8601 is recommended: `2024-01-15T10:30:45.123Z`
- Performance impact is usually minimal for typical volumes

## Best Practices

### For Log Producers

When generating logs, use:

```json
{
  "timestamp": "2024-01-15T10:30:45.123Z",
  "level": "info",
  "message": "event"
}
```

Recommendations:
- Use `timestamp` field name (most common)
- Use ISO 8601 format with timezone (`Z` for UTC)
- Include fractional seconds for precision (`.123` for milliseconds)
- Always include timezone information

### For Log Consumers

When processing logs with `hl`:

- Let auto-detection work—it's accurate for standard formats
- Use `--unix-timestamp-unit` only when auto-detection fails
- Check entries without timestamps if sorting seems wrong
- Use `--local` for easier reading of recent logs

### For Debugging

To understand timestamp parsing:

```bash
# View raw entries to see timestamp fields
hl --raw app.log | jq '.timestamp'

# Sort to verify chronological order
hl --sort app.log | head

# Check specific time range
hl --since '10 minutes ago' --until 'now' app.log
```

## Related Topics

- [Input Options](./input.md) — overview of input handling
- [Input Formats](./input-formats.md) — JSON and logfmt formats
- [Time Display](./time-display.md) — formatting and timezone display
- [Filtering by Time](./filtering-time.md) — using `--since` and `--until`
- [Sorting and Following](./sorting.md) — chronological ordering