# Performance Tips

This page provides guidance on optimizing `hl`'s performance for different use cases.

## Understanding `hl`'s Architecture

`hl` uses several performance optimization strategies:

1. **Parallel processing**: Multiple threads process log entries concurrently
2. **Timestamp indexing**: Pre-built indexes enable fast chronological sorting and filtering
3. **Level bitmasks**: Index-level filtering skips entire file segments without reading them
4. **Streaming architecture**: Memory-efficient processing of large files
5. **Smart buffering**: Configurable buffer sizes for different workloads

## Chronological Sorting Performance

### Use `--sort` for Multiple Files

When processing multiple log files that need chronological ordering, use `--sort`:

```bash
hl --sort app1.log app2.log app3.log
```

**How it works**:
- Builds a timestamp index for each file
- Index includes timestamp ranges and level bitmasks per segment
- Enables very fast filtering without reading entire files

### Index Optimization

The timestamp index includes:
- **Timestamp ranges**: Min/max timestamps per segment
- **Level bitmasks**: Which log levels appear in each segment
- **Segment metadata**: Size, offset, and statistics

**Benefits**:
- Segments without matching timestamps are skipped entirely
- Level filtering skips segments without the requested level
- No parsing needed for filtered-out segments

### Entries Without Timestamps

**Important**: Entries without recognized timestamps are discarded in `--sort` mode.

If you have entries without timestamps:
- Use streaming mode instead (no `--sort`)
- Or ensure all entries have parseable timestamps
- Check supported timestamp formats in the timestamp documentation

## Level Filtering Performance

### Combine `--level` with `--sort`

Level filtering is extremely fast when combined with `--sort`:

```bash
hl --sort --level error app.log
```

**Why it's fast**:
- The index contains level bitmasks for each segment
- Segments without error-level entries are skipped entirely
- No file I/O or parsing for irrelevant segments

### Level Filtering Without Sort

Without `--sort`, `hl` must parse every entry to check its level:

```bash
# Slower: must parse all entries
hl --level error app.log

# Faster: uses index to skip segments
hl --sort --level error app.log
```

## Filtering Performance

### Index-Optimized Filters (Use with `--sort`)

When using `--sort`, three filters have **special index-based optimizations** that can dramatically improve performance:

1. **`-l, --level`** - Level filtering
2. **`--since`** - Time range start
3. **`--until`** - Time range end

These filters leverage the timestamp index to skip entire file segments without reading or parsing them.

```bash
# ✅ FAST: Uses index to skip segments without error-level entries
hl --sort --level error app.log

# ✅ FAST: Uses index to skip segments outside time range
hl --sort --since "2024-01-15 10:00:00" --until "2024-01-15 11:00:00" app.log

# ✅ FAST: Combines both optimizations
hl --sort --level warn --since "2024-01-15 10:00:00" large-file.log
```

### Query Filters (No Index Optimization)

**Important**: Using `-q` for level or time filtering does **not** get the same performance benefits:

```bash
# ❌ SLOWER: Must process all entries, no index optimization
hl --sort -q 'level >= warn' app.log

# ✅ FASTER: Uses index optimization
hl --sort --level warn app.log
```

### All Other Filters

All filtering except `--level`, `--since`, and `--until` is performed per entry and requires processing all input:

- `--filter` / `-f` - Per-entry field matching
- `--query` / `-q` - Per-entry query evaluation

```bash
# These require processing every entry (no index optimization):
hl --sort -f 'status=500' app.log
hl --sort -q 'method=POST and status>=400' app.log
```

**Best practice**: Combine index-optimized filters with per-entry filters to reduce the data set first:

```bash
# Filter by time range and level using index, then apply field filter
hl --sort --level error --since "2024-01-15 10:00:00" -f 'status=500' app.log
```

## Concurrency Settings

### Thread Count

Control the number of processing threads with `--concurrency`:

```bash
# Use 8 threads
hl --concurrency 8 --sort large-file.log
```

**Default**: Number of CPU cores

### When to Adjust Concurrency

There are only two legitimate reasons to adjust concurrency:

1. **Reduce CPU/RAM pressure** when resources are limited (e.g., running in production containers with constrained resources)
   ```bash
   # Reduce to 2 threads to limit resource usage
   hl --concurrency 2 --sort app.log
   ```

2. **Performance fine-tuning** for specific huge workloads through experimentation
   - Benchmark with different values to find what works best for your specific scenario
   - Results are highly workload-dependent

**For most use cases, the default concurrency setting is optimal.**

## Buffer Size Tuning

### Default Buffer Size

Default: `256 KiB` per segment

**The default is highly optimized for both performance and memory usage. Generally, you don't need to tune it.**

### When to Adjust Buffer Size

Only consider adjusting buffer size in specific scenarios:

#### Performance Impact of Small Buffers

**Warning**: Values **below 64 KiB can significantly degrade performance**.

The buffer size affects how data is read and processed. Very small buffers increase overhead from frequent I/O operations and segment processing.

```bash
# ❌ Avoid: can cause performance issues
hl --buffer-size "4 KiB" app.log

# ✅ Minimum recommended
hl --buffer-size "64 KiB" app.log
```

#### Memory Impact in `--sort` Mode

**Large buffers can significantly increase memory usage in `--sort` mode** under specific conditions:

1. **Highly unordered input data**
2. **Multiple input files with overlapping timestamps**
3. **High-volume logs from the same time period**

**Why**: In `--sort` mode, segments remain in memory until all their entries earlier than the current processing time point are consumed. With highly unordered data or many overlapping inputs, multiple segments per file (and across files) may be held in memory simultaneously.

**Example scenario with high memory usage**:
```bash
# 50 Kubernetes pod logs from the same time period, highly unordered
hl --sort --buffer-size "2 MiB" pod-*.log
# Memory can be >> concurrency × buffer_size
# because many segments from all 50 files may be in the queue simultaneously
```

### Memory Usage Formula

**Simple case** (streaming or well-ordered data):
```
Memory ≈ buffer_size × concurrency × 2
```

**Complex case** (`--sort` with unordered/overlapping data):
```
Memory ≈ buffer_size × (segments_in_queue)
where segments_in_queue can be >> concurrency
```

With many overlapping input files, you may have at least one segment per file in the queue, plus additional segments from highly unordered files.

### Large Entries

Individual log entries **larger than buffer size** consume additional memory in all modes:
- **Default max entry size**: `64 MiB`
- Controlled by `--max-message-size`

```bash
# Allow very large log entries
hl --max-message-size "128 MiB" app.log
```

### Recommendations

1. **Start with defaults** - they work well for most cases
2. **Never go below 64 KiB** - performance penalty is significant
3. **Consider larger buffers** (1-4 MiB) only if:
   - Benchmarking shows measurable performance improvement for your specific workload
   - You're willing to accept higher memory consumption
   - **Note**: Significant performance gains from larger buffers are unlikely in most real-world scenarios
4. **Use smaller buffers** (64 KiB) if:
   - Memory is constrained
   - **Using `--sort` with many overlapping files** - this is the primary reason to reduce buffer size, as it can cause very high memory consumption
5. **Monitor memory usage** when sorting many overlapping log files, and reduce buffer size if needed

**Bottom line**: The default buffer size is well-tuned. Only adjust if you've profiled your workload and confirmed it helps.

## Large File Strategies

When processing large amounts of large files, **`--sort` combined with index-optimized filters** can dramatically improve performance:

```bash
# Fast: index allows skipping irrelevant segments
hl --sort \
   --level error \
   --since "2024-01-15 10:00:00" \
   --until "2024-01-15 11:00:00" \
   large-file1.log large-file2.log large-file3.log
```

**Why this is fast**:
- The timestamp index contains level bitmasks and time ranges per segment
- Entire segments outside the time range are skipped
- Segments without error-level entries are skipped
- No reading, parsing, or filtering needed for skipped segments

**Alternative strategies** if index-optimized filters don't apply:
- **Pre-filter with `ripgrep` (`rg`)** to reduce input before piping to `hl`
  - `rg` processes data in parallel and is much faster than `grep`
  - **Note**: `hl` itself is orders of magnitude faster than `grep`, so avoid `grep` for pre-filtering
- **Split large files** and process separately if needed

```bash
# Pre-filter with ripgrep (fast parallel search)
rg "ERROR" huge-file.log | hl
```



## Benchmarking Your Workload

### Measure Performance

Use shell timing to measure performance:

```bash
# Time a command (pager auto-disabled when redirecting)
time hl --sort --level error large-file.log > /dev/null

# Compare different settings
time hl --sort --concurrency 4 large-file.log > /dev/null
time hl --sort --concurrency 16 large-file.log > /dev/null
```

### Profiling Tips

1. **Disable output**: Redirect to `/dev/null` to measure processing time (pager is automatically disabled)
2. **Use consistent data**: Test with the same files for comparison
3. **Warm cache**: Run twice, measure the second run to eliminate cold cache effects

## Common Performance Patterns

### Fast Error Analysis

```bash
# Extremely fast: uses index with level bitmask
hl --sort --level error large-file.log
```

### Fast Multi-File Search

```bash
# Efficient: sorts once, filters via index
hl --sort --level warn app1.log app2.log app3.log
```

### Fast Time Range Queries

```bash
# Index enables fast range filtering
hl --sort \
   --since "2024-01-15 10:00:00" \
   --until "2024-01-15 11:00:00" \
   app.log
```

### Efficient Pipeline Processing

```bash
# Stream mode + raw output for fast pipeline
hl --raw -f 'status>=500' app.log | jq '.status' | sort | uniq -c
```

## Summary

**For best performance**:

1. **Use `--sort`** when processing multiple files or filtering by level
2. **Use index-optimized filters** (`--level`, `--since`, `--until`) with `--sort` for dramatic speedups
3. **Combine index filters** to reduce data before per-entry filtering
4. **Adjust concurrency** only if resources are constrained or you're fine-tuning huge workloads
5. **Keep default buffer size** unless you've profiled and confirmed a change helps

**Remember**: Profile your specific workload to find optimal settings. Performance characteristics vary based on log format, entry size, and query complexity.
