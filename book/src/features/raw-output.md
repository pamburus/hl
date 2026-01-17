# Raw Output

Raw output mode allows you to output the original JSON source entries instead of `hl`'s formatted representation. This is useful for piping to other tools, preserving exact formatting, or re-processing filtered results.

## Enabling Raw Output

Use the `--raw` (or `-r`) flag:

```bash
# Output raw JSON instead of formatted output
hl --raw app.log

# Disable raw mode (if enabled in config)
hl --no-raw app.log
```

## How Raw Output Works

When raw output is enabled, `hl`:

1. **Reads and parses** log entries normally
2. **Applies all filters** (level, query, time range, field filters)
3. **Outputs the original source JSON** for entries that pass filters
4. **Preserves exact formatting** from the source (whitespace, field order, etc.)

The key point: **filtering still applies in raw mode**. Raw output doesn't mean "bypass all processing"—it means "output the original format instead of the formatted representation."

## Use Cases

### Piping to JSON Tools

Raw mode is ideal for piping to tools that expect JSON input:

```bash
# Filter with hl, process with jq
hl --raw --level error app.log | jq '.message'

# Extract specific field values
hl --raw --query '.user_id=123' app.log | jq -r '.request_id'

# Pipe to another JSON processor
hl --raw --since '1 hour ago' app.log | json_pp
```

### Re-processing Filtered Results

Filter with `hl` and save the matching entries in their original format:

```bash
# Extract errors to a new file
hl --raw --level error app.log > errors.json

# Save entries for a specific time range
hl --raw --since '2024-01-15 10:00' --until '2024-01-15 11:00' \
   app.log > time-range.json
```

### Preserving Exact Format

When you need the exact original JSON format:

```bash
# Keep original formatting and field order
hl --raw --query '.important=true' app.log > important.json
```

### Building JSON Pipelines

Combine `hl`'s filtering with other JSON tools:

```bash
# Multi-stage filtering pipeline
hl --raw --level warn app.log \
  | jq 'select(.duration > 1000)' \
  | jq -r '.request_id'

# Extract and transform
hl --raw --query '.event=purchase' app.log \
  | jq '{user: .user_id, amount: .amount, time: .timestamp}'
```

### Data Export

Export filtered logs for analysis in other tools:

```bash
# Export to file for analysis
hl --raw --since 'yesterday' --level error app.log > analysis-errors.json

# Import into database or analytics tool
hl --raw --query '.service=api' app.log | mongoimport --collection logs
```

## Raw Output vs Formatted Output

### Formatted Output (Default)

```bash
hl app.log
```

Example output:
```
2024-01-15 10:30:45.123 INFO [api] user.id: 123, user.name: "Alice", action: "login"
```

Features:
- Human-readable formatting
- Field visibility controls apply
- Time display customization
- Color and theme styling
- Field flattening and expansion

### Raw Output

```bash
hl --raw app.log
```

Example output:
```json
{"timestamp":"2024-01-15T10:30:45.123Z","level":"info","service":"api","user":{"id":123,"name":"Alice"},"action":"login"}
```

Characteristics:
- Original JSON format preserved
- Exact whitespace and field order from source
- No color or styling (plain JSON)
- Field visibility controls **do not apply**
- Suitable for machine processing

## Filtering with Raw Output

All filtering options work with raw output:

### Level Filtering

```bash
# Only output raw entries at error level or above
hl --raw --level error app.log
```

### Query Filtering

```bash
# Output raw entries matching query
hl --raw --query '.status >= 500' access.log

# Complex queries work too
hl --raw --query 'level >= warn and exists(.error_details)' app.log
```

### Time Filtering

```bash
# Output raw entries within time range
hl --raw --since '2024-01-15 10:00' --until '2024-01-15 11:00' app.log
```

### Field Filtering

```bash
# Output raw entries matching field values
hl --raw --filter 'user_id=123' app.log
```

## Raw Output with Multiple Files

Raw output works with multiple input files:

```bash
# Output raw entries from multiple files
hl --raw app.log.1 app.log.2 app.log.3

# Sorted raw output
hl --raw --sort *.log

# Follow mode with raw output
hl --raw -F app.log
```

Each matching entry's original JSON is output, regardless of which file it came from.

## Raw Field Values

There's a related but different option: `--raw-fields`

```bash
# Output field values without unescaping or prettifying
hl --raw-fields app.log
```

`--raw-fields` affects **formatted output** (not raw mode):
- Shows field values exactly as they appear in source
- Doesn't unescape strings
- Doesn't prettify numbers or values

This is different from `--raw`, which outputs entire entries in original JSON format.

You can combine them:

```bash
# Raw output with raw field values (redundant but allowed)
hl --raw --raw-fields app.log
```

In raw mode, `--raw-fields` has no effect since the entire entry is already raw.

## Output Format

Raw output preserves the source format exactly:

### Single-line JSON (most common)

```json
{"timestamp":"2024-01-15T10:30:45Z","level":"info","message":"request processed"}
```

### Pretty-printed JSON (if source was formatted)

```json
{
  "timestamp": "2024-01-15T10:30:45Z",
  "level": "info",
  "message": "request processed"
}
```

### Logfmt entries

If the source format is logfmt, raw output will output logfmt:

```
timestamp=2024-01-15T10:30:45Z level=info message="request processed"
```

Raw mode preserves whatever format was in the source files.

## Examples

### Extract Errors for External Analysis

```bash
# Get all errors in JSON format
hl --raw --level error /var/log/app.log > errors.json

# Import into analysis tool
cat errors.json | your-analysis-tool
```

### Complex Query + JSON Processing

```bash
# Filter with hl, extract fields with jq
hl --raw --query 'status >= 500 and duration > 1000' access.log \
  | jq -r '[.timestamp, .status, .url, .duration] | @csv' \
  > slow-errors.csv
```

### Multi-File Filtered Export

```bash
# Extract and combine entries from multiple files
hl --raw --sort --since 'yesterday' \
   service-a.log service-b.log service-c.log \
   --query '.trace_id=abc-123' \
   > trace-abc-123.json
```

### Live Stream to Processing Tool

```bash
# Follow and pipe raw JSON to processing pipeline
hl --raw -F app.log | your-log-processor
```

### Time Range Extraction

```bash
# Extract specific hour for detailed analysis
hl --raw \
   --since '2024-01-15 10:00' \
   --until '2024-01-15 11:00' \
   --sort \
   app.log app.log.1.gz \
   > 2024-01-15-10h.json
```

### Conditional Export

```bash
# Export only entries with specific attributes
hl --raw --query 'exists(.error) and .severity=critical' *.log \
  | jq -s '.' \
  > critical-errors-collection.json
```

### Data Pipeline

```bash
# Multi-stage data extraction and transformation
hl --raw --level info --query '.event=user_login' app.log \
  | jq -c '{user: .user_id, time: .timestamp, ip: .client_ip}' \
  | awk '{print}' \
  | sort -u \
  > unique-logins.jsonl
```

## When to Use Raw Output

**Use raw output when you need:**
- Original JSON format preserved
- Piping to JSON processing tools (jq, json_pp, etc.)
- Exporting filtered results for external analysis
- Building data pipelines
- Exact source format (not `hl`'s formatting)

**Use formatted output when you need:**
- Human-readable log viewing
- Terminal-friendly display
- Color and theme styling
- Field visibility control
- Interactive log exploration

## Performance Considerations

Raw output is typically **faster** than formatted output because:

- No formatting or prettifying required
- No color/theme processing
- No field flattening or expansion
- Direct pass-through of source JSON

For large-scale filtering and export operations, raw mode can provide a performance benefit.

## Configuration

Set raw mode as default in your config file:

```toml
# ~/.config/hl/config.toml
raw = true
```

Or via environment variable:

```bash
export HL_RAW=true
hl app.log
```

You can always override with `--no-raw` on the command line.

## Related Topics

- [Output Formatting](./formatting.md) — overview of formatting options
- [Field Visibility](./field-visibility.md) — controlling formatted output
- [Filtering by Queries](./filtering-queries.md) — query expressions for filtering
- [Multiple Files](./multiple-files.md) — working with multiple log sources