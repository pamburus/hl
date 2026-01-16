# Compressed File Support

hl natively supports reading compressed log files without requiring manual decompression. This makes working with archived and rotated logs seamless and efficient.

## Supported Formats

hl automatically detects and decompresses these formats:

- **gzip** (`.gz`) - Most common compression format
- **bzip2** (`.bz2`) - Higher compression ratio
- **xz** (`.xz`) - Very high compression ratio
- **zstd** (`.zst`) - Fast compression/decompression

## Basic Usage

### Single Compressed File

Simply pass the compressed file as you would a plain file:

```sh
# gzip
hl application.log.gz

# bzip2
hl application.log.bz2

# xz
hl application.log.xz

# zstd
hl application.log.zst
```

hl automatically detects the compression format and decompresses on the fly.

### Multiple Compressed Files

```sh
# All compressed files
hl app.log.1.gz app.log.2.gz app.log.3.gz

# Mix compressed and uncompressed
hl app.log app.log.1.gz app.log.2.zst

# Using wildcards
hl *.log.gz
```

## Automatic Detection

hl detects compression in two ways:

### By File Extension

```sh
# Recognized by extension
hl app.log.gz      # gzip
hl app.log.bz2     # bzip2
hl app.log.xz      # xz
hl app.log.zst     # zstd
```

### By File Content

Even without standard extensions, hl detects compression from file headers:

```sh
# Works even without .gz extension
hl app.log.1       # Detects gzip if compressed
hl archived.s      # Common in some systems
```

## Working with Rotated Logs

### Standard Rotation

Many log rotation systems compress old logs:

```sh
# Current and rotated logs
hl app.log app.log.1.gz app.log.2.gz app.log.3.gz

# Using wildcards
hl app.log*

# Sorted chronologically
hl -s app.log*
```

### Time-Ordered Processing

```sh
# Process oldest to newest
hl $(ls -tr app.log*)

# Newest to oldest
hl $(ls -t app.log*)
```

## Performance Characteristics

### Decompression Speed

Different formats have different performance characteristics:

| Format | Compression | Decompression | Use Case |
|--------|-------------|---------------|----------|
| gzip | Fast | Fast | General purpose |
| zstd | Very fast | Very fast | Modern systems |
| bzip2 | Slow | Moderate | High compression |
| xz | Very slow | Moderate | Maximum compression |

### Processing Speed

```sh
# Fast formats (zstd, gzip)
hl app.log.zst    # Fastest
hl app.log.gz     # Fast

# Slower formats (bzip2, xz)
hl app.log.bz2    # Moderate
hl app.log.xz     # Slower
```

hl processes compressed files efficiently, but decompression adds overhead compared to plain files.

## Combining with Other Features

### With Filtering

All filtering works normally with compressed files:

```sh
# Level filtering
hl -l e app.log.gz

# Field filtering
hl -f service=api app.log.gz

# Time filtering
hl --since -1h app.log.gz

# Query filtering
hl -q 'status >= 500' app.log.gz
```

### With Sorting

Chronological sorting across compressed files:

```sh
# Sort compressed files
hl -s *.log.gz

# Sort mix of compressed and plain
hl -s app.log app.log.1.gz app.log.2.zst

# Sort with filtering
hl -s -l e --since yesterday *.log.gz
```

### With Streaming

You can pipe compressed streams into hl:

```sh
# Decompress and view
zcat app.log.gz | hl -P

# From remote server
ssh server 'zcat /var/log/app.log.gz' | hl -P

# Decompress on the fly
curl https://example.com/logs/app.log.gz | gunzip | hl -P
```

## Common Scenarios

### Viewing Archived Logs

```sh
# Today's and archived logs
hl app.log app.log.1.gz app.log.2.gz

# All available logs
hl app.log*

# Sorted chronologically
hl -s app.log*
```

### Searching Across Archives

```sh
# Find errors in all archives
hl -l e app.log*.gz

# Search for specific request
hl -f request.id=abc-123 app.log*.gz

# Time-based search in archives
hl --since '2024-01-10' --until '2024-01-12' *.log.gz
```

### Incident Investigation

```sh
# Search across days of compressed logs
hl -s -l e --since '2024-01-15' *.log.gz

# Specific time window in archives
hl -s --since '2024-01-15 14:00:00' --until '2024-01-15 16:00:00' *.log.gz
```

### Log Analysis

```sh
# Performance analysis across archived logs
hl -q 'duration > 1.0' *.log.gz

# Error patterns over time
hl -s -l e --since -7d *.log*
```

## Multiple Compression Formats

hl handles mixed formats seamlessly:

```sh
# Different formats in one command
hl app.log \
   app.log.1.gz \
   app.log.2.zst \
   app.log.3.bz2 \
   app.log.4.xz

# Wildcard matching all
hl app.log*
```

## Remote Compressed Logs

### Via SSH

```sh
# View remote compressed log
ssh server 'cat /var/log/app.log.gz' | hl -P

# Or decompress remotely
ssh server 'zcat /var/log/app.log.gz' | hl -P
```

### Via HTTP

```sh
# Download and view
curl -s https://example.com/logs/app.log.gz | hl -P

# With authentication
curl -s -H "Authorization: Bearer $TOKEN" \
  https://api.example.com/logs/app.log.gz | hl -P
```

## Tips and Best Practices

1. **Use wildcards for rotated logs**:
   ```sh
   hl -s app.log*
   ```

2. **Apply filters to reduce processing**:
   ```sh
   hl -l e *.log.gz
   ```

3. **Use sorting for chronological view**:
   ```sh
   hl -s *.log.gz
   ```

4. **Prefer faster compression formats**:
   - zstd for best performance
   - gzip for compatibility

5. **Increase buffer size for compressed files**:
   ```sh
   hl --buffer-size "512 KiB" *.log.gz
   ```

6. **Use time filters to skip files**:
   ```sh
   hl -s --since -1d *.log.gz
   ```

## Limitations

1. **No writing to compressed files** - hl is read-only

2. **Cannot seek in compressed streams** - Sequential reading only

3. **Memory usage** - Compressed files require decompression buffer

4. **Performance** - Decompression adds CPU overhead

## Troubleshooting

### File Not Recognized

If hl doesn't recognize compression:

```sh
# Check file type
file app.log.1

# Try explicit decompression
zcat app.log.1 | hl -P
```

### Slow Performance

If processing is slow:

```sh
# Use faster compression (zstd)
# Or apply filters
hl -l e *.log.gz

# Use time range
hl --since -1d *.log.gz

# Increase concurrency
hl -C 4 *.log.gz
```

### Corrupted Files

If you encounter corrupted compressed files:

```sh
# Test decompression
gunzip -t app.log.gz

# Skip bad file
hl app.log app.log.2.gz  # Skip corrupted app.log.1.gz
```

### Out of Memory

If processing large compressed files causes memory issues:

```sh
# Process one at a time
for f in *.log.gz; do
  hl "$f"
done

# Or use smaller buffer
hl --buffer-size "128 KiB" *.log.gz
```

## Examples by Use Case

### Daily Log Review

```sh
# Review compressed logs from last week
hl -s --since -7d *.log*

# Yesterday's errors in archives
hl -l e --since yesterday --until today *.log.gz
```

### Historical Analysis

```sh
# Pattern over last 30 days
hl -s -q 'status >= 500' --since -30d *.log*

# Service behavior in archives
hl -f service=api --since -14d *.log*
```

### Space-Efficient Storage

```sh
# Process without decompressing to disk
hl *.log.gz

# Pipe to another tool without temp files
hl -l e *.log.gz | grep "database"
```

### Archive Migration

```sh
# Read old format archives
hl old-logs/*.log.bz2

# Compare with new format
hl new-logs/*.log.zst
```

## Compression Format Details

### gzip (.gz)

```sh
# Most compatible
hl app.log.gz

# Fast decompression
# Good compression ratio
# Universal support
```

### zstd (.zst)

```sh
# Modern compression
hl app.log.zst

# Fastest decompression
# Excellent compression ratio
# Best choice for new systems
```

### bzip2 (.bz2)

```sh
# High compression
hl app.log.bz2

# Better compression than gzip
# Slower decompression
# Good for long-term storage
```

### xz (.xz)

```sh
# Maximum compression
hl app.log.xz

# Best compression ratio
# Slowest decompression
# Use for maximum space savings
```

## Related

- [Multiple Files](./multiple-files.md) - Working with multiple log files
- [Viewing Logs](./viewing-logs.md) - Basic log viewing
- [Chronological Sorting](./sorting-chrono.md) - Sorting across files
- [Performance Tips](../reference/performance.md) - Optimization strategies