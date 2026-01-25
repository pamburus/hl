# Chronological Sorting

Chronological sorting organizes log entries by their timestamps, regardless of which file or source they came from. This is essential for understanding event sequences in distributed systems, analyzing rotated logs, and debugging issues across multiple services.

## Enabling Chronological Sorting

Use the `--sort` (or `-s`) flag:

```sh
# Sort a single file
hl -s app.log

# Sort multiple files together
hl -s app.log app.log.1 app.log.2

# Sort with filtering
hl -s --level warn *.log
```

## How Sorting Works

When you enable sorting, `hl`:

1. **Reads all entries** from all input files
2. **Parses timestamps and levels** from each entry
3. **Builds an index** with timestamp ranges, level bitmasks, and offsets (cached for reuse)
4. **Sorts entries** chronologically using the index
5. **Outputs sorted entries** in chronological order

The key advantage: **filtering still applies**, and the index-based approach makes sorted filtering very efficient even on large files. The index contains level information, so level filtering is extremely fast – if a file segment doesn't contain the requested time range or log levels at all, it's skipped entirely during processing (never read, parsed, or filtered).

## When to Use Sort Mode

Sort mode is ideal when you need to:

- **Reconstruct event sequences** across multiple files or sources
- **Analyze rotated logs** where newer entries might be split across files
- **Correlate events** from different services with interleaved timestamps
- **Debug distributed systems** where logs from different nodes need chronological ordering
- **Investigate historical data** with precise time-based ordering

## Timestamp Extraction

`hl` automatically detects and parses timestamps in various formats:

- **RFC 3339**: `2024-01-15T10:30:45.123Z`, `2024-01-15T10:30:45+00:00`
- **ISO 8601 variants**: `2024-01-15 10:30:45.123`, `2024-01-15 10:30:45` (space separator, optional timezone)
- **Unix timestamps**: numeric seconds, milliseconds, microseconds, or nanoseconds (integer or float)

See [Timestamp Handling](./timestamps.md) for the complete list of supported formats and parsing details.

Timestamps can appear in various field names and can be located anywhere in the JSON structure (top-level or nested).

**Commonly used timestamp field names:** `ts`, `time`, `timestamp`, `@timestamp`

These field names are configurable via the configuration file (see [Configuration Files](../customization/config-files.md)):

```toml
[fields.predefined.time]
names = ["ts", "time", "timestamp", "@timestamp"]
```

### Handling Multiple Timestamp Fields

When an entry contains multiple timestamp fields, `hl` uses the priority order according to the configuration.

You typically don't need to configure this – `hl` chooses the most semantic timestamp automatically.

## Entries Without Timestamps

Log entries without a recognized timestamp are **discarded** in sort mode.

Sort mode requires valid timestamps to determine chronological ordering. Entries that cannot be parsed or don't contain recognizable timestamp fields are filtered out during the indexing phase.

**Note:** If you need to see all entries including those without timestamps, use unsorted mode (the default) or piped input with `tail -f | hl -P`.

## Sorting Stability

When multiple entries have the **same timestamp**, `hl` maintains their relative order based on:

1. **Source order** — entries from earlier files come before entries from later files
2. **Line order** — within the same file, entries maintain their original sequence

This stable sorting ensures deterministic, reproducible output.

## Performance Characteristics

Sorting uses an efficient two-pass indexing approach:

### First Pass - Indexing

- Sequential scan of all input files
- Parses each entry to extract timestamp and log level
- Builds an index per segment containing:
  - Timestamp ranges (min/max)
  - Level bitmasks (which log levels are present)
  - Entry offsets and ordering information
- Index is cached in `~/.cache/hl/` for reuse on subsequent runs
- **Minimal memory usage** — doesn't load all entries into memory

### Second Pass - Output

- Uses the index to determine which segments to process
- **Skips entire segments** that don't match level filters (using level bitmasks — never reads or parses them)
- **Skips segments** outside time range filters (using timestamp ranges — never reads or parses them)
- Reads and processes only matching segments in chronological order
- Nearly sequential I/O when entries are mostly sorted
- Memory usage increases only if entries are heavily shuffled

### Index Caching

The sorting index is cached and reused:

```sh
# First run builds index (slower)
hl -s large.log

# Subsequent runs reuse index (faster)
hl -s --level error large.log
```

The index is automatically invalidated when files change, ensuring correctness.

### Optimization Tips

**Use filters to reduce dataset size:**

```sh
# Level filtering uses index bitmasks to skip entire segments
hl -s --level error large.log

# Time filtering leverages index timestamp ranges to skip irrelevant segments
hl -s --since '1 hour ago' --level warn large.log

# Combined filters maximize segment skipping
hl -s --level error --since '1 hour ago' huge.log.gz
```

**Sort only what you need:**

```sh
# Efficient segment selection with time filters
hl -s --since '2024-01-15 10:00' --until '2024-01-15 11:00' *.log
```

**Leverage cached indexes:**

- Repeatedly filtering the same files is fast due to cached index
- Clear cache with `rm -rf ~/.cache/hl/` if needed

## Multi-File Sorting

Sorting excels when working with multiple files:

```sh
# Sort rotated logs chronologically
hl -s app.log app.log.1.gz app.log.2.gz

# Sort logs from multiple services
hl -s api.log worker.log scheduler.log

# Sort with wildcards
hl -s logs-2024-01-*.log.gz
```

This allows you to:

- **Reconstruct event timelines** across file boundaries
- **Follow request flows** through distributed components
- **Analyze log rotation artifacts** where events span multiple files
- **Debug race conditions** by seeing exact chronological order

## Combining with Filters

### With Filters

All standard filters work with sorting. Filters are applied efficiently during the index scan, reducing the dataset size before sorting.

```sh
# Sort with time range
hl -s --since 'yesterday' --until 'today' *.log

# Sort with level filter
hl -s --level error app.log

# Sort with query
hl -s --query 'status >= 500' access.log
```

See [Filtering](./filtering.md) for complete filter documentation.

## Examples

### Multi-Service Debugging

```sh
# Sort logs from multiple microservices to trace a request
hl -s api-gateway.log auth-service.log payment-service.log \
   --query 'request-id = "abc-123-def"'
```

### Rotated Log Analysis

```sh
# Sort across rotated files (including compressed)
hl -s app.log app.log.1 app.log.2.gz
```

## Sorting vs Follow Mode

Don't confuse `--sort` with `--follow`:

| Feature | `--sort` (`-s`) | `--follow` (`-F`) |
|---------|-----------------|-------------------|
| **Use case** | Batch processing | Live monitoring |
| **Input** | Static files | Growing files |
| **Timing** | All at once at end | Continuous streaming |
| **Memory** | Indexed buffering | Only recent entries |
| **Sorting scope** | Entire dataset | Within sync window |
| **Exit** | After output complete | Runs until interrupted |

Use `--sort` for analyzing historical logs. Use `--follow` for monitoring live logs.

See [Follow Mode](./follow-mode.md) for details on live log monitoring.

## Related Topics

- [Follow Mode](./follow-mode.md) — live monitoring with chronological sorting
- [Live Streaming](./streaming.md) — streaming behavior and `-F` vs piping
- [Time Display](./time-display.md) — formatting timestamps in output
- [Filtering by Time](./filtering-time.md) — using `--since` and `--until`
- [Multiple Files](./multiple-files.md) — working with multiple log sources
