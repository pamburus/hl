# Follow Mode

Follow mode provides live log monitoring with automatic chronological sorting and file rotation handling. It's designed for real-time log observation across multiple sources.

## Enabling Follow Mode

Use the `--follow` (or `-F`) flag:

```bash
# Follow a single log file
hl -F /var/log/app.log

# Follow multiple files
hl -F service-*.log

# Follow with filtering
hl -F --level error --query '.request_id=abc' *.log
```

## How Follow Mode Works

When you start `hl` in follow mode:

1. **Opens all specified files** and determines their current size
2. **Preloads recent entries** based on the `--tail` setting (default: 10 entries per file)
3. **Monitors files continuously** for new data appended
4. **Parses new entries** as they arrive
5. **Buffers entries** within a time window (`--sync-interval-ms`)
6. **Sorts buffered entries chronologically** across all sources
7. **Displays sorted entries** to the terminal
8. **Detects file rotation** and handles it transparently

Follow mode runs indefinitely until you interrupt it (Ctrl-C).

## Key Behavioral Characteristics

### Only Displays Parseable Entries with Timestamps

Follow mode is designed for structured log monitoring. It:

- **Requires valid timestamps** — entries without recognized timestamps are skipped
- **Requires parseable format** — unparseable lines are silently ignored
- **Shows only structured data** — raw text, startup messages, or mixed formats won't appear

This is fundamentally different from piping `tail -f` into `hl`:

```bash
# Follow mode: only parsed, timestamped entries
hl -F app.log

# Piped mode: shows everything, including unparseable lines
tail -f app.log | hl -P
```

If you need to see **all** output including unparseable content, use the piped approach with `-P` (paging disabled).

### Chronological Sorting

Unlike piped input which shows entries in arrival order, follow mode sorts entries chronologically across all monitored files within the sync interval window.

```bash
# These might show different ordering for the same inputs:
hl -F service-a.log service-b.log        # Chronologically sorted
tail -f service-*.log | hl -P            # Arrival order
```

The sorting window is controlled by `--sync-interval-ms` (default: 100ms).

### Automatic Pager Disabling

Follow mode automatically disables the pager and streams output directly to your terminal, since paging doesn't make sense for infinite streams.

You don't need to specify `-P` when using `-F`—it's implied.

## Configuration Options

### Tail Preload Window

Control how many recent entries to display when follow mode starts:

```bash
# Show last 20 entries from each file
hl -F --tail 20 *.log

# Show last 50 entries (useful for context)
hl -F --tail 50 app.log

# Don't show any history, only new entries
hl -F --tail 0 app.log
```

**Default:** `--tail 10`

The tail window is applied **per file**. If you follow 3 files with `--tail 10`, you'll see up to 30 recent entries when starting (10 from each file, sorted chronologically).

### Sync Interval

The sync interval controls the time window for chronological sorting:

```bash
# Use 500ms window (better ordering, more latency)
hl -F --sync-interval-ms 500 *.log

# Use 50ms window (faster, less accurate ordering)
hl -F --sync-interval-ms 50 *.log
```

**Default:** `--sync-interval-ms 100`

#### How Sync Interval Affects Behavior

The sync interval creates a sliding time window within which entries are buffered and sorted before display.

- **Larger intervals** (e.g., 500ms):
  - More accurate chronological ordering when entries arrive out of order
  - Higher latency between event occurrence and display
  - Better for distributed systems with clock skew or network delays

- **Smaller intervals** (e.g., 50ms):
  - Lower latency, near-instant display
  - Entries might appear slightly out of order if sources have timing differences
  - Better for single-host or well-synchronized systems

#### Example Scenario

Imagine monitoring two services with a 100ms sync interval:

```
Time    Service A           Service B
----    ---------           ---------
10:00.0 [entry 1]
10:00.1                     [entry 2]
10:00.2 [entry 3]
```

With 100ms sync interval, `hl` will:
- Buffer entries arriving within each 100ms window
- Sort them by timestamp
- Display them in chronological order: entry 1, entry 2, entry 3

Without buffering (sync interval = 0), you might see: entry 1, entry 3, entry 2 (arrival order).

## File Rotation Handling

Follow mode automatically detects and handles log file rotation:

### Detection Methods

`hl` monitors for:

- **File truncation** — when a file shrinks or is replaced
- **File recreation** — when a file is deleted and recreated with the same name
- **Inode changes** — when the underlying file changes (on systems that support this)

### Rotation Behavior

When rotation is detected, `hl`:

1. **Finishes reading** any remaining data from the old file
2. **Closes the old file** handle
3. **Opens the new file** with the same name
4. **Continues monitoring** seamlessly

No entries are lost during rotation.

### Compatible with Standard Tools

This works transparently with:

- **logrotate** (copytruncate or create mode)
- **Application-managed rotation** (rename-and-recreate pattern)
- **Container log rotation** (Docker, Kubernetes)

Example with logrotate:

```bash
# Start following
hl -F /var/log/app.log

# Meanwhile, logrotate runs and rotates the file
# hl automatically detects and switches to the new file
```

## Combining with Filters

Follow mode works seamlessly with all filtering options:

### Level Filtering

```bash
# Only show errors and above
hl -F --level error *.log

# Show warnings and above from multiple services
hl -F --level warn service-a.log service-b.log
```

### Query Filtering

```bash
# Follow specific user's activity
hl -F --query '.user_id=12345' app.log

# Follow failed requests
hl -F --query 'status >= 500' access.log

# Complex query
hl -F --query 'level >= warn and (.service=api or .service=auth)' *.log
```

### Time Filtering

```bash
# Only show entries from the last hour
hl -F --since '1 hour ago' app.log

# Show entries within a time range (useful for historical follow)
hl -F --since '10:00' --until '11:00' app.log
```

Note: In live follow mode, `--until` will cause `hl` to exit once that time is reached.

## Exit Behavior

### Normal Operation

Follow mode runs indefinitely until interrupted. Press Ctrl-C to exit.

**Follow mode exits immediately on Ctrl-C** — unlike pager mode, there is no interrupt ignore count in follow mode. A single Ctrl-C will terminate the process.

### Automatic Exit Conditions

Follow mode exits automatically when:

- **Ctrl-C is pressed** (single interrupt, immediate exit)
- **All files are deleted** and not recreated
- **--until** time is reached (if specified)
- **Unrecoverable error** occurs (e.g., permission denied)

### Note on --interrupt-ignore-count

The `--interrupt-ignore-count` option is **ignored in follow mode**. This option is only useful in other scenarios:

**When piping from an application:**
```bash
# myapp receives Ctrl-C and shuts down gracefully
# hl continues running to display shutdown logs
myapp | hl -P
```

Without interrupt ignore count, pressing Ctrl-C would terminate `hl` immediately, preventing you from seeing the application's graceful shutdown messages.

**When using a pager:**
```bash
# In pager (less), press Ctrl-C to stop loading and navigate buffer
hl large.log
```

If you're in `less` with Shift+F (follow mode in less) and data is still loading, Ctrl-C tells `less` to stop loading so you can navigate the already-loaded buffer. The interrupt ignore count prevents `hl` from terminating prematurely in this scenario.

**In follow mode, immediate exit is desired** — you're monitoring files directly and want quick termination when you're done.

## Multiple File Monitoring

Follow mode excels at monitoring multiple files simultaneously:

```bash
# Monitor all service logs
hl -F /var/log/service-*.log

# Monitor logs from multiple directories
hl -F /var/log/app/service-a.log /var/log/app/service-b.log

# Monitor with pattern expansion
hl -F logs/**/*.log
```

Each file is:
- Monitored independently
- Has its own tail preload window
- Contributes entries to the chronologically sorted output stream

### Input Info Display

When following multiple files, it's helpful to see which file each entry came from:

```bash
# Show minimal file info (just filename)
hl -F --input-info minimal *.log

# Show compact info (file number and name)
hl -F --input-info compact *.log

# Show full info (full path)
hl -F --input-info full *.log
```

See [Multiple Files](./multiple-files.md) for more on input info display.

## Performance Considerations

### Resource Usage

Follow mode is designed to be lightweight:

- **Low CPU usage** when files are idle
- **Minimal memory** — only buffers entries within the sync window
- **Efficient I/O** — uses OS-level file monitoring where available

### High-Volume Logs

For high-volume logs (thousands of entries per second):

```bash
# Reduce sync interval to minimize buffering
hl -F --sync-interval-ms 50 high-volume.log

# Use filters to reduce output
hl -F --level warn --query '.critical=true' high-volume.log

# Disable tail preload for faster startup
hl -F --tail 0 high-volume.log
```

### Many Files

When following many files (dozens or more):

- OS file descriptor limits may apply
- Consider monitoring only active files
- Use wildcards that match current rotated files only

## Common Patterns

### Development Monitoring

```bash
# Follow application log during development
hl -F --tail 20 --level debug app.log
```

### Production Monitoring

```bash
# Follow multiple production services, errors only
hl -F --tail 50 --level error \
   /var/log/service-*.log \
   --input-info minimal
```

### Debugging Specific Issues

```bash
# Follow logs for a specific request/transaction
hl -F --tail 100 --query '.trace_id=abc-123-def' *.log
```

### Multi-Service Correlation

```bash
# Follow logs from different services, sorted chronologically
hl -F --sync-interval-ms 200 \
   api.log worker.log database.log \
   --query 'exists(.request_id)' \
   --input-info compact
```

### Continuous Integration Monitoring

```bash
# Follow build logs with context
hl -F --tail 0 --level info /var/log/ci/build.log
```

## Troubleshooting

### Not Seeing New Entries

Check that:
- Files are actually being appended to (use `tail -f` to verify)
- Entries have valid, recognized timestamps
- Entries are parseable JSON or logfmt
- Filters aren't excluding the entries

### Entries Appearing Out of Order

- Increase `--sync-interval-ms` to allow more time for buffering
- Check if source systems have clock skew
- Verify that timestamps are correct in the source logs

### Missing Entries After Rotation

- Ensure file rotation preserves the filename (hl follows by name)
- Check that rotation doesn't happen too frequently for the sync interval
- Verify file permissions after rotation

### High CPU Usage

- Reduce `--sync-interval-ms` if it's very large
- Use filters to reduce the volume of processed entries
- Check for runaway log generation in source applications

## When to Use Follow Mode

**Use follow mode (`-F`) when you want:**
- Live monitoring with chronological sorting
- Automatic file rotation handling
- Multi-file log stream merging
- Clean, structured log output only
- Production system monitoring

**Use piped input (`tail -f | hl -P`) when you want:**
- Complete raw output (all lines, even unparseable)
- Original arrival order preserved
- Debugging (seeing startup messages, mixed formats)
- Simple single-file monitoring
- Full control over the input stream

## Related Topics

- [Live Streaming](./streaming.md) — streaming behavior and `-F` vs piping
- [Sorting and Following](./sorting.md) — overview of sorting modes
- [Chronological Sorting](./sorting-chrono.md) — batch sorting with `--sort`
- [Multiple Files](./multiple-files.md) — working with multiple log sources
- [Filtering](./filtering.md) — all filtering options