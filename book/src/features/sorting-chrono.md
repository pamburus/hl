# Chronological Sorting

Chronological sorting is the process of ordering log entries by their timestamps, regardless of which file or source they came from. This is essential for understanding event sequences in distributed systems and analyzing rotated logs.

## Enabling Chronological Sorting

Use the `--sort` (or `-s`) flag to enable chronological sorting:

```bash
# Sort a single file
hl -s application.log

# Sort multiple files together
hl -s app.log.1 app.log.2 app.log.3

# Sort with filtering
hl -s --level warn --query '.service=api' *.log
```

## How Sorting Works

When you enable sorting, `hl`:

1. **Reads all entries** from all input files sequentially
2. **Parses timestamps** from each entry using configured or detected formats
3. **Buffers all entries** in memory with their parsed timestamps
4. **Sorts entries** by timestamp in ascending chronological order
5. **Outputs sorted entries** to the pager or terminal

The entire process completes before any output is displayed—sorting is not a streaming operation.

## Timestamp Extraction

`hl` automatically detects and parses timestamps in various formats:

- **ISO 8601**: `2024-01-15T10:30:45.123Z`, `2024-01-15T10:30:45+00:00`
- **RFC 3339**: `2024-01-15 10:30:45.123`
- **Unix timestamps**: numeric seconds, milliseconds, microseconds, or nanoseconds
- **Common formats**: `Jan 15 10:30:45`, `15/Jan/2024:10:30:45 +0000`

Timestamps can appear in various field names:

- `timestamp`, `time`, `@timestamp`
- `ts`, `t`
- `date`, `datetime`
- `_time`, `syslog_timestamp`

The timestamp can be located anywhere in the JSON structure (top-level or nested).

## Handling Multiple Timestamp Fields

When an entry contains multiple timestamp fields, `hl` uses a priority order:

1. `@timestamp` (common in ELK/Elasticsearch)
2. `timestamp`
3. `time`
4. `ts`
5. Other recognized timestamp field names

You typically don't need to worry about this—`hl` chooses the most semantic timestamp automatically.

## Entries Without Timestamps

Log entries that don't contain a recognized timestamp are handled specially:

- They are placed at the **beginning** of the sorted output
- Among themselves, they maintain their original order
- They are assigned an effective timestamp of zero (epoch start)

Example:

```bash
# Input files:
# file1.log: entry from 2024-01-15 10:00
# file2.log: entry without timestamp
# file3.log: entry from 2024-01-15 09:00

# Output order after sorting:
# 1. entry from file2.log (no timestamp)
# 2. entry from file3.log (2024-01-15 09:00)
# 3. entry from file1.log (2024-01-15 10:00)
```

## Stability of Sorting

When multiple entries have the **same timestamp**, `hl` maintains their relative order based on:

1. **Source order** — entries from earlier files come before entries from later files
2. **Line order** — within the same file, entries maintain their original sequence

This stable sorting behavior ensures deterministic output.

## Multi-File Sorting

Sorting really shines when working with multiple files:

```bash
# Sort rotated logs chronologically
hl -s app.log app.log.1.gz app.log.2.gz

# Sort logs from multiple services
hl -s api.log worker.log scheduler.log
```

This allows you to:

- **Reconstruct event timelines** across file boundaries
- **Follow request flows** through distributed components
- **Analyze log rotation artifacts** where events span multiple files
- **Debug race conditions** by seeing exact chronological order

## Performance Characteristics

Sorting performance depends on several factors:

### Memory Usage

- All log entries must be held in memory during sorting
- Memory usage is proportional to the number and size of entries
- Large log files (multiple gigabytes) may require significant RAM

### Processing Time

- Time complexity is O(n log n) where n is the number of entries
- Dominated by I/O time for reading files (especially compressed files)
- Sorting itself is typically fast—decompression and parsing take longer

### Optimization Tips

**Use filters to reduce dataset size:**

```bash
# Filter before sorting reduces memory and time
hl -s --since '1 hour ago' --level warn large.log

# Query filtering also reduces the dataset
hl -s --query '.user_id=12345' huge.log.gz
```

**Sort only what you need:**

```bash
# Instead of sorting everything, target specific time ranges
hl -s --since '2024-01-15 10:00' --until '2024-01-15 11:00' *.log
```

**Consider alternatives for very large datasets:**

- Use external tools (GNU `sort`, database import) for multi-gigabyte files
- Split large time ranges into smaller chunks
- Use follow mode (`-F`) for live monitoring instead of sorting archives

## Sorting vs Follow Mode

Don't confuse `--sort` with `--follow`:

| Feature | `--sort` (`-s`) | `--follow` (`-F`) |
|---------|-----------------|-------------------|
| **Use case** | Batch processing | Live monitoring |
| **Input** | Static files | Growing files |
| **Timing** | All at once at end | Continuous streaming |
| **Memory** | All entries buffered | Only recent entries |
| **Sorting scope** | Entire dataset | Within sync window |
| **Exit** | After output complete | Runs until interrupted |

Use `--sort` for analyzing historical logs. Use `--follow` for monitoring live logs.

## Combining with Other Features

### Sorting with Time Filtering

```bash
# Sort entries within a specific time range
hl -s --since 'yesterday' --until 'today' *.log
```

Time filtering happens **before** sorting, reducing the dataset size.

### Sorting with Field Filtering

```bash
# Sort only error entries
hl -s --level error app.log

# Sort entries matching a query
hl -s --query 'status >= 500' access.log
```

Filters are applied during reading—only matching entries are kept for sorting.

### Sorting with Field Visibility

```bash
# Sort and show only specific fields
hl -s --hide '*' --hide '!timestamp' --hide '!level' --hide '!message' app.log
```

Field visibility affects output formatting, not sorting behavior.

## Examples

### Distributed System Debugging

```bash
# Sort logs from multiple microservices to see complete request flow
hl -s api-gateway.log auth-service.log payment-service.log \
   --query '.request_id=abc-123-def'
```

### Log Rotation Analysis

```bash
# Sort across rotated files to find when an issue started
hl -s --since '2 hours ago' app.log app.log.1 app.log.2.gz \
   --level error
```

### Compressed Archive Investigation

```bash
# Sort historical compressed logs for a specific day
hl -s logs/app-2024-01-15-*.log.gz --query '.endpoint=/api/users'
```

### Cross-Service Correlation

```bash
# Sort logs from different services to correlate events
hl -s service-a.log service-b.log service-c.log \
   --since '10:00' --until '10:15' \
   --query 'exists(.correlation_id)'
```

## Related Topics

- [Sorting and Following](./sorting.md) — overview of sorting modes
- [Follow Mode](./follow-mode.md) — live streaming with chronological ordering
- [Time Display](./time-display.md) — formatting timestamps in output
- [Filtering by Time](./filtering-time.md) — using `--since` and `--until`
- [Multiple Files](./multiple-files.md) — working with multiple log sources
- [Performance Tips](../reference/performance.md) — optimizing large log processing