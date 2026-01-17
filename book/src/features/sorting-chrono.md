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

Sorting uses an efficient two-pass indexing approach:

### How Sorting Works Internally

**First Pass - Indexing:**
- Sequential scan of all input files
- Builds an index with timestamp ranges and level bitmasks per segment
- Creates an optimized map of entry offsets and ordering within segments
- Index is cached in the cache directory for reuse on subsequent runs
- Minimal memory usage—doesn't load all entries into memory

**Second Pass - Output:**
- Uses the index to determine which segments from which files to read and in what order
- Reads entries in optimized order for chronological output
- Nearly sequential I/O when entries are mostly sorted
- Memory usage increases only if entries are heavily shuffled

### Memory Usage

- **Best case** (mostly sorted input): minimal memory, close to streaming
- **Typical case**: moderate memory for buffering out-of-order segments
- **Worst case** (completely shuffled): higher memory usage, but still far less than loading all entries
- Index metadata is compact and cached between runs

### Processing Time

- First run: two sequential passes through the data
- Subsequent runs: index reuse speeds up processing significantly
- I/O time dominates (especially for compressed files)
- Filtering before sorting reduces both passes' work

### Index Caching

The sorting index is cached and reused:

```bash
# First run builds index (slower)
hl -s large.log

# Subsequent runs reuse index (faster)
hl -s --level error large.log
```

Index is invalidated when files change, ensuring correctness.

### Optimization Tips

**Use filters to reduce dataset size:**

```bash
# Filtering uses index metadata to skip irrelevant segments
hl -s --since '1 hour ago' --level warn large.log

# Query filtering reduces processing work
hl -s --query '.user_id=12345' huge.log.gz
```

**Sort only what you need:**

```bash
# Time filters leverage index for efficient segment selection
hl -s --since '2024-01-15 10:00' --until '2024-01-15 11:00' *.log
```

**Leverage index caching:**

- Repeatedly filtering the same large files is fast due to cached index
- Index is stored in cache directory (typically `~/.cache/hl/`)
- Clear cache with `rm -rf ~/.cache/hl/` if needed

**Debug index behavior:**

```bash
# View index metadata (shows timestamp ranges, segments, etc.)
hl -s --dump-index large.log

# This prints the index structure and exits without processing entries
```

This is useful for understanding how files are being indexed and troubleshooting sorting issues.

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