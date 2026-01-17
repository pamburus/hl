# Working with Multiple Logs

This page demonstrates how to work with multiple log files and sources simultaneously.

## Viewing Multiple Files

### Basic Multi-File Viewing

View multiple log files merged and sorted chronologically:

```hl/dev/null/shell.sh#L1
hl app.log app.log.1 app.log.2
```

All entries are merged and displayed in timestamp order.

### Glob Patterns

Use shell glob patterns to specify multiple files:

```hl/dev/null/shell.sh#L1
# All log files in a directory
hl /var/log/app/*.log

# All rotated logs
hl app.log*

# Specific pattern
hl service-*.log
```

### Recursive File Discovery

Find and process logs recursively:

```hl/dev/null/shell.sh#L1
# Using find command
find /var/log -name "*.log" -exec hl {} +

# Or with xargs
find /var/log -name "*.log" | xargs hl
```

## Source Identification

### Input Source Display

By default, `hl` shows which file each entry came from when viewing multiple files:

```hl/dev/null/shell.sh#L1
hl service-a.log service-b.log service-c.log
```

Each entry is prefixed with the filename or a shortened source indicator.

### Hiding Source Names

Hide the input source indicator using `--input-info`:

```hl/dev/null/shell.sh#L1
hl --input-info none service-a.log service-b.log
```

Useful when you don't care which file an entry came from. Possible values: `auto`, `none`, `minimal`, `compact`, `full`.

## Chronological Sorting Across Files

### Automatic Sorting

Entries from all files are automatically sorted by timestamp:

```hl/dev/null/shell.sh#L1
# Entries from all files merged in chronological order
hl app-2024-01-15.log app-2024-01-16.log app-2024-01-17.log
```

This provides a unified timeline across all sources.

### Sorting with Rotated Logs

Handle log rotation seamlessly:

```hl/dev/null/shell.sh#L1
# Current and rotated logs, sorted chronologically
hl app.log app.log.1 app.log.2.gz app.log.3.gz
```

`hl` decompresses and merges all files automatically.

## Following Multiple Files

### Real-Time Multi-File Monitoring

Follow multiple log files simultaneously:

```hl/dev/null/shell.sh#L1
hl -F service-a.log service-b.log service-c.log
```

New entries from any file are shown in chronological order, and log rotation is handled for all files.

### Following with Glob Patterns

```hl/dev/null/shell.sh#L1
# Follow all service logs
hl -F /var/log/services/*.log
```

## Filtering Across Multiple Files

### Apply Filters to All Files

Filters apply to all input files:

```hl/dev/null/shell.sh#L1
# Show errors from all services
hl -l error service-*.log

# Show slow requests across all files
hl -q 'duration > 1000' app.log worker.log api.log
```

### Source-Specific Filtering

While `hl` doesn't have built-in source-specific filtering, you can use queries on fields that identify sources:

```hl/dev/null/shell.sh#L1
# If logs have a 'service' field (use hyphens in field names)
hl -f 'service = "api"' service-*.log

# Or filter by filename patterns after the fact
hl service-*.log | grep 'service-a'
```

## Time Range Filtering

### Time Ranges Across Multiple Files

Apply time filters to all files:

```hl/dev/null/shell.sh#L1
# Show entries from all files in the last hour
hl --since "1h ago" app.log app.log.1 app.log.2

# Specific time range across files
hl --since "2024-01-15 10:00" --until "2024-01-15 12:00" service-*.log
```

The index allows efficient time-range queries across all files.

## Compressed Files

### Mixed Compressed and Uncompressed

Process a mix of compressed and uncompressed files:

```hl/dev/null/shell.sh#L1
hl app.log app.log.1.gz app.log.2.bz2 app.log.3.xz app.log.4.zst
```

All files are automatically decompressed and merged.

### All Compressed Files

```hl/dev/null/shell.sh#L1
# All gzipped logs
hl *.log.gz

# Mixed compression formats
hl *.log.{gz,bz2,xz}
```

## Practical Multi-File Examples

### Investigate Across Service Logs

```hl/dev/null/shell.sh#L1
# Find errors across all services in the last hour
hl -l error --since "1h ago" api.log web.log worker.log scheduler.log
```

### Track Request Across Services

```hl/dev/null/shell.sh#L1
# Follow a request by ID through multiple services
hl -f 'request_id = "abc-123"' service-*.log
```

### Multi-Region Analysis

```hl/dev/null/shell.sh#L1
# Merge logs from different regions, sorted chronologically
hl us-east.log us-west.log eu-central.log ap-southeast.log
```

### Deployment Investigation

```hl/dev/null/shell.sh#L1
# Check logs from deployment window across all services
hl --since "2024-01-15 14:30" --until "2024-01-15 15:00" service-*.log
```

### Historical Analysis

```hl/dev/null/shell.sh#L1
# Analyze patterns over multiple days
hl app-2024-01-*.log
```

### Compare Before and After

```hl/dev/null/shell.sh#L1
# Before deployment
hl --until "2024-01-15 14:30" app.log

# After deployment
hl --since "2024-01-15 14:30" app.log
```

## Performance with Multiple Files

### Efficient Multi-File Processing

`hl` processes multiple files efficiently:

- **Index-based**: Uses per-file indexes for fast sorting
- **Streaming**: Doesn't load all files into memory
- **Parallel processing**: Can process files in parallel internally
- **Smart I/O**: Reads only necessary portions when time filtering

```hl/dev/null/shell.sh#L1
# Fast even with many large files
hl --since "1h ago" large-*.log
```

### Debugging Performance

Check index usage with `--dump-index`:

```hl/dev/null/shell.sh#L1
# See index metadata for a file
hl --dump-index app.log

# Check multiple files
for f in *.log; do echo "=== $f ==="; hl --dump-index "$f"; done
```

## Complex Multi-File Scenarios

### Microservices Architecture

```hl/dev/null/shell.sh#L1
# Monitor all microservices for errors
hl -F -l error \
  gateway.log \
  auth-service.log \
  user-service.log \
  payment-service.log \
  notification-service.log
```

### Distributed System Debugging

```hl/dev/null/shell.sh#L1
# Track distributed transaction
hl -f 'transaction_id = "txn-456"' \
  api-node-1.log \
  api-node-2.log \
  api-node-3.log \
  database.log \
  cache.log
```

### Multi-Environment Comparison

```hl/dev/null/shell.sh#L1
# Compare staging vs production
hl -f 'env = "staging"' staging-*.log > staging-errors.txt
hl -f 'env = "production"' production-*.log > production-errors.txt
```

### Log Aggregation

```hl/dev/null/shell.sh#L1
# Aggregate logs from all sources for a time period
hl --since 2024-01-15 --until 2024-01-16 \
  /var/log/app/*.log \
  /var/log/services/*.log \
  > daily-aggregate-2024-01-15.txt
```

## Combining Multiple Sources with Filtering

### Multi-File with Field Filters

```hl/dev/null/shell.sh#L1
# Find slow database queries across all application servers
hl -q 'operation = "query" and duration > 1000' app-server-*.log
```

### Multi-File with Level and Time Filters

```hl/dev/null/shell.sh#L1
# Errors in the last hour from all services
hl -l error --since "1h ago" service-*.log
```

### Multi-File with Field Management

```hl/dev/null/shell.sh#L1
# Hide verbose fields across all files
hl --hide trace-id --hide span-id --hide metadata api-*.log
```

## Input Source in Queries

If logs include a source field, you can filter by it:

```hl/dev/null/shell.sh#L1
# If logs have 'hostname' field
hl -f 'hostname = "server-1"' distributed-*.log

# If logs have 'service-name' field
hl -q 'service-name in ["api", "worker"]' all-services.log
```

## Tips and Best Practices

### Order Files Logically

```hl/dev/null/shell.sh#L1
# Chronological order helps (though hl sorts anyway)
hl app.log app.log.1 app.log.2  # newest to oldest
```

### Use Descriptive Globs

```hl/dev/null/shell.sh#L1
# Clear intent
hl service-api-*.log

# Better than
hl *.log
```

### Limit File Count for Interactive Use

For interactive viewing, limit to relevant files:

```hl/dev/null/shell.sh#L1
# Instead of all files
hl *.log

# Be selective
hl app.log app.log.1 app.log.2
```

### Combine with Shell Tools

```hl/dev/null/shell.sh#L1
# Find relevant files first
find /var/log -name "api-*.log" -mtime -7 | xargs hl -l error
```

### Use Time Filters to Reduce Dataset

```hl/dev/null/shell.sh#L1
# More efficient than processing all data
hl --since "1h ago" large-file-*.log
```

## Troubleshooting

### Files Not Merging Correctly

If entries aren't merging in the expected order:

- Verify all files have recognized timestamps
- Check timezone consistency across files
- Use `--dump-index` to inspect file metadata
- Verify chronological sorting is enabled with `-s` or `--sort` flag if needed

### Missing Entries

If some entries are missing:

- Check if they have valid timestamps (entries without timestamps may be skipped in some modes)
- Verify file permissions
- Use `--raw` to see unparseable content

### Performance Issues

If processing many files is slow:

- Use time filtering to limit the data range
- Check if indexes are being used (`~/.cache/hl/`)
- Reduce the number of files processed
- Consider pre-filtering with `ripgrep` (`rg`) before piping to `hl` (much faster than `grep`)

### Source Names Not Showing

If source names aren't displayed:

- Verify you're processing multiple files (single-file mode may not show source)
- Check if `--input-info none` is set
- Ensure output is formatted (not using `--raw`)
- Try `--input-info full` to force display

## Next Steps

- [Live Monitoring](live-monitoring.md) — Follow multiple files in real-time
- [Filtering Examples](filtering.md) — Apply filters across multiple sources
- [Sorting](../features/sorting.md) — Understand chronological sorting behavior
