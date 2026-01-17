# Sorting and Following

`hl` provides powerful capabilities for organizing and monitoring log entries across time and multiple sources.

## Overview

By default, `hl` displays log entries in the order they appear in the input—line by line, file by file. However, when working with logs from distributed systems, rotated files, or live streams, you often need entries sorted chronologically to understand the sequence of events.

`hl` offers two main modes for time-based ordering:

- **Sort mode** (`-s, --sort`) — chronologically sorts all entries before displaying
- **Follow mode** (`-F, --follow`) — continuously monitors files and displays entries in near-real-time chronological order

## Sort Mode

The `--sort` (or `-s`) option tells `hl` to:

1. Read all log entries from all input files
2. Parse timestamps from each entry
3. Sort entries chronologically across all sources
4. Display the sorted output

```bash
# Sort entries from multiple log files chronologically
hl -s app.log app.log.1 app.log.2

# Sort and filter
hl -s --level warn *.log

# Sort entries from compressed archives
hl -s logs-2024-01-*.log.gz
```

### When to Use Sort Mode

Sort mode is ideal when you need to:

- **Reconstruct event sequences** across multiple files or sources
- **Analyze rotated logs** where newer entries might be split across files
- **Correlate events** from different services with interleaved timestamps
- **Debug distributed systems** where logs from different nodes need chronological ordering

### Performance Considerations

Sort mode uses an efficient two-pass indexing approach:

- **First pass** — builds an index with timestamp ranges and offsets (cached for reuse)
- **Second pass** — reads entries in optimized order using the index
- **Memory usage** — minimal for sorted data, moderate for shuffled data (never loads all entries)
- **Index caching** — subsequent runs on the same files are significantly faster

Filters (`-l`, `-q`, `--since`, `--until`) leverage the index to skip irrelevant segments, making filtered sorting very efficient even on large files.

## Follow Mode

The `--follow` (or `-F`) option enables live log monitoring with automatic chronological sorting across multiple streams:

```bash
# Follow a single log file
hl -F /var/log/app.log

# Follow multiple files with chronological merging
hl -F service-*.log

# Follow with filtering
hl -F --level error --query '.service=api' *.log
```

### How Follow Mode Works

When you use `-F`, `hl`:

1. **Opens all specified files** and seeks to recent entries
2. **Preloads recent history** (controlled by `--tail`)
3. **Monitors files continuously** for new entries
4. **Sorts entries** within a time window (controlled by `--sync-interval-ms`)
5. **Displays sorted entries** as they arrive
6. **Handles file rotation** automatically (detects when files are truncated or recreated)

### Key Characteristics

Follow mode has important behavioral differences from piped input:

- **Only shows entries with valid timestamps** — unparsable lines or entries without recognized timestamps are skipped
- **Sorts entries chronologically** across all monitored files within the sync interval
- **Handles rotation** — detects and follows rotated files automatically
- **Disables paging automatically** — output streams directly to your terminal
- **Preloads recent context** — shows recent entries when starting (configurable with `--tail`)

### Configuration Options

#### Tail Window

Control how many recent entries to display when follow mode starts:

```bash
# Show last 20 entries from each file when starting
hl -F --tail 20 *.log

# Start from the end without showing history
hl -F --tail 0 *.log
```

Default: `--tail 10`

#### Sync Interval

Control the time window for chronological sorting:

```bash
# Use 500ms window for sorting (better ordering, slightly more delay)
hl -F --sync-interval-ms 500 *.log

# Use 50ms window (faster display, less accurate ordering)
hl -F --sync-interval-ms 50 *.log
```

Default: `--sync-interval-ms 100`

The sync interval represents the time buffer `hl` uses to collect and sort entries before displaying them. A larger interval provides better chronological accuracy when entries arrive out of order, but introduces more display latency.

### Follow Mode vs Piped Input

There's a fundamental difference between `hl -F` and piping with `tail -f`:

```bash
# Built-in follow: parsed, sorted, handles rotation, skips unparsable
hl -F /var/log/app.log

# Piped follow: raw, unsorted, shows everything including unparsable
tail -f /var/log/app.log | hl -P
```

**Use `hl -F` when you want:**
- Chronologically sorted output across multiple files
- Automatic file rotation handling
- Clean, parsed log entries only
- Multi-file monitoring with merged streams

**Use `tail -f | hl -P` when you want:**
- Complete raw output including unparsable lines
- Original ordering preserved
- Debugging (seeing startup messages, mixed formats, etc.)
- Simple single-file monitoring

See [Live Streaming](./streaming.md) for more details on streaming behavior and the differences between these approaches.

## File Rotation Handling

In follow mode, `hl` automatically detects when log files are rotated:

- **Truncation detection** — notices when a file shrinks or is recreated
- **Continues monitoring** — switches to the new file seamlessly
- **No data loss** — catches entries written during rotation

This works transparently with common rotation schemes (logrotate, application-managed rotation, etc.).

## Sorting Without Timestamps

Entries without recognizable timestamps:

- In **sort mode**: appear at the beginning of output (treated as having a zero/minimum timestamp)
- In **follow mode**: are **skipped** (not displayed)

This is an important distinction—if you need to see all output including unparsable entries, use piped input with `-P` instead of `-F`.

## Exit Behavior

- **Sort mode** exits after displaying all entries
- **Follow mode** runs indefinitely until interrupted (Ctrl-C exits immediately)

Follow mode exits immediately on a single Ctrl-C. The `--interrupt-ignore-count` option is ignored in follow mode—it only applies when using a pager or piping from another application.

## Examples

### Reconstructing Distributed Traces

```bash
# Sort logs from multiple services to see the complete request flow
hl -s api.log auth.log database.log --query '.trace_id=abc123'
```

### Live Monitoring with Context

```bash
# Follow with 50 lines of history and error-level filtering
hl -F --tail 50 --level error *.log
```

### Analyzing Rotated Archives

```bash
# Sort all archived logs for a specific time range
hl -s --since '2024-01-15 00:00' --until '2024-01-15 23:59' \
   app.log.2024-01-*.gz
```

### High-Frequency Log Monitoring

```bash
# Reduce sync interval for high-volume logs
hl -F --sync-interval-ms 50 --tail 5 high-volume.log
```

## Related Topics

- [Live Streaming](./streaming.md) — detailed streaming behavior and `-F` vs piping
- [Multiple Files](./multiple-files.md) — working with multiple log sources
- [Time Display](./time-display.md) — controlling timestamp formatting
- [Filtering by Time](./filtering-time.md) — using `--since` and `--until`
