# Working with Multiple Files

hl makes it easy to work with multiple log files simultaneously, whether you're concatenating them for viewing, sorting them chronologically, or following them in real-time.

## Basic Usage

### Viewing Multiple Files

Simply list all files as arguments:

```sh
hl app.log app.log.1 app.log.2
```

This concatenates and displays all files in the order specified.

### Using Wildcards

Use shell wildcards to match multiple files:

```sh
# All .log files in current directory
hl *.log

# All log files in subdirectory
hl logs/*.log

# Multiple patterns
hl app*.log error*.log

# Recursive (using shell features)
hl **/*.log  # bash with globstar enabled
```

### Sorting by Time

Sort files using the `-s` flag to get chronological order:

```sh
hl -s app.log app.log.1 app.log.2
```

Without `-s`, entries are shown in file order (app.log, then app.log.1, then app.log.2).

## File Ordering

### Default Order

Files are processed in the order specified:

```sh
# Processes in order: new.log, old.log
hl new.log old.log
```

### Chronological Order with Shell Sorting

Use shell commands to order files by time:

```sh
# Oldest first
hl $(ls -tr *.log)

# Newest first
hl $(ls -t *.log)

# By modification time
hl $(ls -t /var/log/app/*.log)
```

### With Chronological Sorting

Use `-s` flag to sort entries across all files by timestamp:

```sh
# Entries sorted chronologically regardless of which file they're in
hl -s *.log
```

## Multiple File Patterns

### Different Directories

```sh
# Multiple directories
hl service1/*.log service2/*.log service3/*.log

# Absolute and relative paths
hl /var/log/app/*.log ./local/*.log
```

### Mixed File Types

```sh
# Plain and compressed files
hl app.log app.log.1.gz app.log.2.zst

# All automatically detected
hl app.log*
```

### Rotated Log Files

```sh
# Include all rotations
hl app.log app.log.{1..10}

# Or use wildcard
hl app.log*

# Order by time (oldest first)
hl $(ls -tr app.log*)
```

## Input Indicators

When viewing multiple files, hl shows which file each entry came from.

### Default Indicators

```sh
hl app1.log app2.log app3.log
```

Output shows file indicators like:
```
[1] 2024-01-15 10:00:00 info Server started
[2] 2024-01-15 10:00:01 info Processing request
[1] 2024-01-15 10:00:02 warn High memory usage
```

Where `[1]`, `[2]`, etc., indicate which file the entry is from.

### Input Info Layouts

Control how file information is displayed:

```sh
# Minimal (just numbers)
hl --input-info minimal app1.log app2.log

# Compact (numbers with abbreviated names)
hl --input-info compact app1.log app2.log

# Full (complete file paths)
hl --input-info full app1.log app2.log

# None (hide file indicators)
hl --input-info none app1.log app2.log

# Auto (default, adjusts based on number of files)
hl --input-info auto app1.log app2.log
```

## Chronological Sorting Across Files

### Basic Sorting

```sh
# Sort all entries by timestamp
hl -s *.log
```

This:
- Parses all files
- Builds timestamp index
- Displays entries in chronological order
- Shows which file each entry came from

### Sorting with Filters

```sh
# Sorted errors from all files
hl -s -l e *.log

# Sorted entries from last hour
hl -s --since -1h *.log

# Sorted with field filter
hl -s -f service=api *.log
```

### Performance

Sorting is highly optimized:
- Initial scan: ~2 GiB/s
- Builds index for fast access
- Reuses index if files unchanged
- Skips unmodified blocks on rescan

```sh
# Fast even with many large files
hl -s /var/log/app/*.log

# Very fast with time filtering
hl -s --since -1h *.log
```

## Following Multiple Files

### Real-Time Monitoring

Use `-F` flag to follow multiple files:

```sh
hl -F app1.log app2.log app3.log
```

This:
- Watches all files for changes
- Sorts entries chronologically
- Shows new entries in real-time
- Handles file rotations automatically

### Follow with Filters

```sh
# Follow errors across all files
hl -F -l e *.log

# Follow specific service
hl -F -f service=api *.log

# Follow with time window
hl -F --tail 50 *.log
```

### Sync Interval

Control how often entries are sorted in follow mode:

```sh
# Default (100ms)
hl -F app1.log app2.log

# Faster updates (50ms)
hl -F --sync-interval-ms 50 app1.log app2.log

# Slower, more efficient (500ms)
hl -F --sync-interval-ms 500 app1.log app2.log
```

### Preload with --tail

Show recent history before following:

```sh
# Last 100 entries from each file
hl -F --tail 100 app1.log app2.log

# Last 20 entries
hl -F --tail 20 *.log
```

## Combining Files from Different Sources

### Different Services

```sh
# Multiple microservices
hl -s \
  service1/app.log \
  service2/app.log \
  service3/app.log
```

### Different Servers

```sh
# Logs from different servers (if mounted)
hl -s \
  /mnt/server1/app.log \
  /mnt/server2/app.log \
  /mnt/server3/app.log
```

### Mixed Local and Remote

```sh
# Combine with process substitution
hl -s \
  local.log \
  <(ssh server1 'cat /var/log/app.log') \
  <(ssh server2 'cat /var/log/app.log')
```

## Common Patterns

### All Logs from Today

```sh
# View today's logs across all files
hl -s --since today *.log
```

### Incident Investigation

```sh
# Chronological view during incident
hl -s \
  --since '2024-01-15 14:30:00' \
  --until '2024-01-15 15:00:00' \
  service*/*.log
```

### Error Analysis Across Services

```sh
# All errors from all services
hl -s -l e service*/*.log

# Errors in last hour
hl -s -l e --since -1h *.log
```

### Request Tracing

```sh
# Trace request across all services
hl -s -f request.id=abc-123 service*/*.log

# With chronological order
hl -s -q 'trace.id = "xyz789"' *.log
```

### Performance Monitoring

```sh
# Slow requests across all services
hl -s -q 'duration > 1.0' service*/*.log

# Recent slow requests
hl -s -q 'duration > 0.5' --since -1h *.log
```

## Large Number of Files

### Handling Many Files

hl efficiently handles hundreds of files:

```sh
# Hundreds of files
hl -s /var/log/app/**/*.log

# With filtering for performance
hl -s -l e --since -1h /var/log/app/**/*.log
```

### File Limits

Be aware of shell limits:

```sh
# If you hit "Argument list too long"
find /var/log/app -name "*.log" -exec hl -s {} +

# Or use xargs
find /var/log/app -name "*.log" | xargs hl -s
```

### Selective Loading

```sh
# Only recent files
hl -s $(find . -name "*.log" -mtime -1)

# Files modified in last week
hl -s $(find . -name "*.log" -mtime -7)
```

## Compressed Files

hl automatically handles mixed compressed and uncompressed files:

```sh
# Mix of formats
hl app.log app.log.1.gz app.log.2.zst app.log.3.bz2

# All rotations including compressed
hl app.log*

# Sorted across all
hl -s app.log*
```

See [Compressed Files](./compressed.md) for more details.

## Performance Tips

1. **Use sorting for chronological order**:
   ```sh
   hl -s *.log  # Much faster than manual sorting
   ```

2. **Apply filters to reduce data**:
   ```sh
   hl -s -l e --since -1h *.log
   ```

3. **Increase buffer size for many files**:
   ```sh
   hl --buffer-size "512 KiB" -s *.log
   ```

4. **Use concurrency for parallel processing**:
   ```sh
   hl -C 8 -s *.log  # Use 8 threads
   ```

5. **Limit time range for faster indexing**:
   ```sh
   hl -s --since -1d *.log  # Only index relevant entries
   ```

## Examples by Use Case

### Daily Review

```sh
# Yesterday's logs from all services
hl -s --since yesterday --until today service*/*.log

# Yesterday's errors
hl -s -l e --since yesterday --until today *.log
```

### Debugging Across Services

```sh
# Follow all services for errors
hl -F -l e service1.log service2.log service3.log

# Trace specific request
hl -s -f trace.id=xyz789 service*/*.log
```

### Performance Analysis

```sh
# Slow requests across all services
hl -s -q 'duration > 1.0' --since -1h service*/*.log

# 95th percentile investigation
hl -s -q 'duration > 0.5' service*/*.log
```

### Deployment Verification

```sh
# Follow logs after deployment
hl -F --tail 50 --since '2024-01-15 10:00:00' *.log

# Check for errors after deployment
hl -s -l e --since '2024-01-15 10:00:00' *.log
```

## Troubleshooting

### Too Many Files

If shell complains about too many arguments:

```sh
# Use find with -exec
find . -name "*.log" -exec hl -s {} +

# Or xargs
find . -name "*.log" | xargs hl -s

# Or process in batches
hl -s dir1/*.log
hl -s dir2/*.log
```

### File Order Confusion

If entries seem out of order:

```sh
# Use -s for chronological sorting
hl -s *.log

# Check file order
ls -la *.log
```

### Performance Issues

If processing is slow:

```sh
# Add time filter
hl -s --since -1d *.log

# Increase concurrency
hl -C 8 -s *.log

# Use level filter
hl -s -l e *.log
```

### Missing Entries

If some entries don't appear:

```sh
# Check all files are listed
ls *.log

# Verify sorting isn't filtering
hl *.log  # Without -s to see all entries

# Check time range
hl -s *.log  # No time filter
```

## Related

- [Chronological Sorting](./sorting-chrono.md) - Details on -s flag
- [Follow Mode](./follow-mode.md) - Real-time monitoring with -F
- [Compressed Files](./compressed.md) - Working with compressed logs
- [Viewing Logs](./viewing-logs.md) - Basic log viewing
- [Multiple Logs Examples](../examples/multiple-logs.md) - Real-world scenarios