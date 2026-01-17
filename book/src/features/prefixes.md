# Non-JSON Prefixes

Many log collection systems and logging frameworks add metadata or timestamps before JSON log entries. The `--allow-prefix` option enables `hl` to handle these prefixed logs.

## Overview

Without `--allow-prefix`, `hl` expects each line to start with a JSON object (`{`) or logfmt key-value pairs. When logs have text before the structured data, `hl` needs to skip this prefix.

Example of prefixed log:
```
2024-01-15 10:30:45 server1: {"level":"info","message":"started"}
```

With `--allow-prefix`, `hl` finds the first `{` and parses from that point.

## Enabling Prefix Handling

Use the `--allow-prefix` flag:

```bash
# Allow non-JSON prefixes
hl --allow-prefix app.log

# Works with all other options
hl --allow-prefix --level error docker.log
```

## How Prefix Handling Works

When `--allow-prefix` is enabled, for each line `hl`:

1. **Scans for the first `{` character** (start of JSON)
2. **Parses from that point** as a JSON object
3. **Ignores all text before the `{`** 
4. **Falls back to normal parsing** if no `{` is found

The prefix is discarded—only the JSON portion is parsed and processed.

## Common Use Cases

### Docker Container Logs

Docker adds timestamps and container metadata before JSON entries:

```
2024-01-15T10:30:45.123456789Z {"level":"info","message":"request processed"}
2024-01-15T10:30:46.789012345Z {"level":"error","message":"connection failed"}
```

```bash
# Read Docker container logs
hl --allow-prefix /var/lib/docker/containers/*/container.log

# Or from docker logs command
docker logs container-name 2>&1 | hl --allow-prefix
```

### Systemd Journal with JSON Payloads

Systemd journal exports can include metadata before JSON:

```
Jan 15 10:30:45 hostname service[1234]: {"level":"info","message":"started"}
```

```bash
# Export and view systemd logs
journalctl -u myservice | hl --allow-prefix

# Or with explicit JSON export
journalctl -u myservice -o json | hl
```

### Syslog with JSON Messages

Syslog often adds standard syslog prefix before JSON payloads:

```
<14>Jan 15 10:30:45 host app[1234]: {"level":"info","message":"event"}
```

```bash
# Process syslog with JSON payloads
hl --allow-prefix /var/log/syslog
```

### Custom Logging Frameworks

Some frameworks add their own metadata:

```
[2024-01-15 10:30:45.123] [THREAD-1] {"level":"info","message":"processing"}
```

```bash
# Handle custom framework logs
hl --allow-prefix custom-app.log
```

### Kubernetes Logs

Kubernetes adds timestamps and stream info before container output:

```
2024-01-15T10:30:45.123456789Z stdout F {"level":"info","message":"started"}
```

```bash
# View Kubernetes pod logs
kubectl logs pod-name | hl --allow-prefix
```

## Prefix Examples

### Docker Format

```
2024-01-15T10:30:45.123456789Z {"timestamp":"2024-01-15T10:30:45.123Z","level":"info","message":"started"}
```

Prefix: `2024-01-15T10:30:45.123456789Z `

### Systemd Format

```
Jan 15 10:30:45 hostname service[1234]: {"level":"info","message":"started"}
```

Prefix: `Jan 15 10:30:45 hostname service[1234]: `

### Syslog RFC3164 Format

```
<14>Jan 15 10:30:45 host app: {"level":"info","message":"event"}
```

Prefix: `<14>Jan 15 10:30:45 host app: `

### Custom Application Format

```
[INFO] 2024-01-15 10:30:45 Thread-1 > {"level":"info","message":"processing"}
```

Prefix: `[INFO] 2024-01-15 10:30:45 Thread-1 > `

### Multiline Prefix

```
=== Log Entry ===
Timestamp: 2024-01-15 10:30:45
{"level":"info","message":"event"}
```

Only the line with `{` is parsed; other lines are ignored.

## Prefix Timestamp vs Entry Timestamp

When prefixes contain timestamps (like Docker logs), you may have two timestamps:

1. **Prefix timestamp** — added by log collector (Docker, systemd, etc.)
2. **Entry timestamp** — in the JSON object itself

`hl` uses the **entry timestamp** from the JSON for:
- Sorting (`--sort`)
- Time filtering (`--since`, `--until`)
- Display in formatted output

The prefix timestamp is discarded.

Example:
```
2024-01-15T10:00:00Z {"timestamp":"2024-01-15T09:55:00Z","message":"event"}
```

`hl` will use `2024-01-15T09:55:00Z` (from JSON), not `2024-01-15T10:00:00Z` (from prefix).

This is usually what you want, as the entry timestamp reflects when the event actually occurred, not when it was collected.

## Performance Considerations

### Overhead

`--allow-prefix` adds minimal overhead:
- Scans each line for the first `{` character
- No performance impact if prefix is short
- Slight overhead for long prefixes (rare)

For typical use cases (Docker, systemd logs), the overhead is negligible.

### When to Avoid

If your logs **never** have prefixes, omit `--allow-prefix`:
- Slightly faster processing
- Stricter validation (entries must start with `{`)

## Combining with Other Options

### With Input Format

```bash
# Explicitly specify JSON format with prefixes
hl --allow-prefix --input-format json app.log
```

### With Filtering

```bash
# Filter prefixed logs
hl --allow-prefix --level error --query '.service=api' docker.log
```

### With Sorting

```bash
# Sort prefixed logs chronologically
hl --allow-prefix --sort container-*.log
```

### With Follow Mode

```bash
# Follow prefixed logs
hl --allow-prefix -F /var/log/containers/*.log
```

## Limitations

### JSON Detection Only

`--allow-prefix` only detects JSON (looks for `{`). It doesn't handle logfmt with prefixes.

For prefixed logfmt, you'll need to preprocess:

```bash
# Strip prefixes before passing to hl
sed 's/^[^a-z]*//' logfmt-with-prefix.log | hl --input-format logfmt
```

### Single-Line Prefixes

Prefixes must be on the same line as the JSON object. Multi-line prefixes aren't supported.

If your prefix is on a separate line, the JSON line will be parsed normally (since it starts with `{`).

### Prefix Information is Lost

The prefix text is completely discarded. If you need information from the prefix, you'll need to preprocess the logs.

## Examples

### Docker Containers

```bash
# View all container logs
hl --allow-prefix /var/lib/docker/containers/*/*-json.log

# Filter Docker logs by level
hl --allow-prefix --level error container.log

# Follow live Docker logs
docker logs -f mycontainer 2>&1 | hl --allow-prefix -P
```

### Systemd Services

```bash
# View service logs
journalctl -u nginx | hl --allow-prefix

# Follow service logs with filtering
journalctl -u app -f | hl --allow-prefix --level warn -P
```

### Kubernetes

```bash
# View pod logs
kubectl logs pod-name | hl --allow-prefix

# Follow with filtering
kubectl logs -f deployment/api | hl --allow-prefix --query '.user_id=123' -P
```

### Syslog

```bash
# Process syslog with JSON messages
hl --allow-prefix --input-format json /var/log/syslog

# Filter by application
grep 'myapp' /var/log/syslog | hl --allow-prefix
```

### Mixed Sources

```bash
# Process logs from multiple sources with prefixes
hl --allow-prefix --sort \
   docker-app.log \
   systemd-service.log \
   custom-framework.log
```

## Configuration

Enable prefix handling by default:

```toml
# ~/.config/hl/config.toml
allow-prefix = true
```

Or via environment variable:

```bash
export HL_ALLOW_PREFIX=true
hl app.log
```

## Troubleshooting

### Entries Not Appearing

**Problem:** Lines with prefixes aren't being parsed.

**Solutions:**
- Ensure `--allow-prefix` is enabled
- Verify the line actually contains `{` somewhere
- Check that the JSON portion is valid

### Getting Unparsed Prefix Text

**Problem:** Seeing prefix text in output.

**Solutions:**
- Enable `--allow-prefix`
- Verify JSON starts with `{` on the same line

### Wrong Timestamps

**Problem:** Timestamps seem incorrect.

**Cause:** Prefix timestamp is different from entry timestamp.

**Understanding:** `hl` uses the timestamp from the JSON, not the prefix. This is usually correct—the prefix timestamp is when the log was collected, the entry timestamp is when the event occurred.

### Logfmt with Prefixes

**Problem:** Logfmt entries with prefixes aren't working.

**Solution:** `--allow-prefix` only works with JSON. Preprocess logfmt to remove prefixes:

```bash
sed 's/^.*: //' prefixed-logfmt.log | hl --input-format logfmt
```

## When to Use --allow-prefix

**Use `--allow-prefix` when:**
- Processing Docker container logs
- Reading systemd journal output
- Viewing syslog with JSON payloads
- Working with custom frameworks that add metadata
- Any logs with text before `{`

**Don't use `--allow-prefix` when:**
- Logs are pure JSON (no prefix)
- You want strict validation
- Every line should start with `{`

## Related Topics

- [Input Options](./input.md) — overview of input handling
- [Input Formats](./input-formats.md) — JSON and logfmt formats
- [Timestamp Handling](./timestamps.md) — how timestamps are parsed
- [Multiple Files](./multiple-files.md) — working with multiple sources