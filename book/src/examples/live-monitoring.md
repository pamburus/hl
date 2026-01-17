# Live Monitoring

This page demonstrates live log monitoring techniques using follow mode and piped streaming.

## Follow Mode

### Basic Follow Mode

Monitor a log file in real-time as new entries are written:

```hl/dev/null/shell.sh#L1
hl -F app.log
```

The `-F` (or `--follow`) flag:
- Watches the file for new entries
- Handles log rotation automatically
- Sorts entries chronologically across sources
- Shows only parseable entries (skips unparseable lines)
- Exits immediately on Ctrl+C

### Following Multiple Files

Monitor multiple log files simultaneously:

```hl/dev/null/shell.sh#L1
hl -F service-a.log service-b.log service-c.log
```

Entries from all files are merged and sorted chronologically in real-time.

### Follow with Filtering

Combine follow mode with filters to monitor specific events:

```hl/dev/null/shell.sh#L1
# Watch for errors only
hl -F -l error app.log

# Watch for specific service (use hyphens in field names)
hl -F -f 'service = "api"' app.log

# Watch slow requests
hl -F -q 'duration > 1000' app.log
```

### Follow with Time Range

Start following from a specific point in time:

```hl/dev/null/shell.sh#L1
# Follow from 1 hour ago onwards
hl -F --since "1h ago" app.log

# Follow from a specific timestamp
hl -F --since "2024-01-15 14:00:00" app.log
```

## Piped Streaming

### Basic Piped Streaming

Stream output from applications directly to `hl`:

```hl/dev/null/shell.sh#L1
# From a running application
./myapp 2>&1 | hl -P

# From kubectl
kubectl logs -f my-pod | hl -P

# From docker
docker logs -f my-container | hl -P

# From journalctl
journalctl -f | hl -P
```

Use `-P` to disable the pager for streaming scenarios where you want immediate, continuous output as entries arrive.

### Piped Streaming Shows Everything

Unlike follow mode (`-F`), piped mode (with `-P`) shows:
- All input lines (even unparseable ones)
- Lines without timestamps
- Raw input in original arrival order

```hl/dev/null/shell.sh#L1
# Shows everything, including non-JSON lines
tail -f app.log | hl -P
```

This is useful for:
- Debugging log format issues
- Seeing startup messages that aren't JSON
- Monitoring applications with mixed output formats

### Piped Streaming with Filtering

Apply filters to piped input:

```hl/dev/null/shell.sh#L1
# Filter errors from live stream
kubectl logs -f my-pod | hl -P -l error

# Filter by field
docker logs -f my-container | hl -P -q 'status >= 400'

# Complex filtering
./myapp 2>&1 | hl -P -l warn -q 'duration > 2000'
```

## Follow Mode vs Piped Mode

### When to Use Follow Mode (-F)

Use `-F` when:
- Monitoring one or more log files on disk
- You want automatic log rotation handling
- You want chronological sorting across multiple sources
- You only care about parseable entries
- You want immediate exit on Ctrl+C

```hl/dev/null/shell.sh#L1
hl -F /var/log/app/*.log
```

### When to Use Piped Mode (-P)

Use piped input with `-P` when:
- Streaming from a command (kubectl, docker, journalctl, etc.)
- You want to see ALL output, including unparseable lines
- You want original arrival order
- You're debugging log format or parsing issues

```hl/dev/null/shell.sh#L1
kubectl logs -f my-pod | hl -P
```

### Key Differences

| Feature | Follow Mode (`-F`) | Piped Mode (`pipe | hl -P`) |
|---------|-------------------|------------------------|
| Input source | Files on disk | Standard input |
| Unparseable lines | Hidden | Shown |
| Lines without timestamps | Hidden | Shown |
| Sorting | Chronological | Arrival order |
| Log rotation | Handled automatically | N/A |
| Ctrl+C behavior | Exits immediately | Depends on `--interrupt-ignore-count` |

## Practical Monitoring Examples

### Monitor Production Errors

```hl/dev/null/shell.sh#L1
# File-based monitoring
hl -F -l error /var/log/production/app.log

# Kubernetes monitoring
kubectl logs -f deployment/api | hl -P -l error
```

### Monitor Slow Requests Across Services

```hl/dev/null/shell.sh#L1
# Multiple files
hl -F -q 'duration > 2000' api.log worker.log scheduler.log

# Piped from multiple sources
kubectl logs -f deployment/api | hl -P -q 'duration > 2000'
```

### Monitor Specific User Activity

```hl/dev/null/shell.sh#L1
hl -F -f 'user-id = 12345' app.log
```

### Monitor Authentication Events

```hl/dev/null/shell.sh#L1
hl -F -q 'event in ["login", "logout", "auth_failed"]' auth.log
```

### Monitor Database Operations

```hl/dev/null/shell.sh#L1
hl -F -q 'operation ~= "query" and duration > 1000' db.log
```

### Watch Deployment Progress

```hl/dev/null/shell.sh#L1
# Start from deployment time
hl -F --since "2024-01-15 14:30:00" app.log

# Or use relative time
hl -F --since "5m ago" app.log
```

### Multi-Service Monitoring

```hl/dev/null/shell.sh#L1
# Monitor all services for errors
hl -F -l error /var/log/services/*.log

# With service identification
hl -F -l error api.log web.log worker.log
```

## Advanced Monitoring Patterns

### Conditional Alerting Pattern

```hl/dev/null/shell.sh#L1
# Monitor and alert on specific conditions
hl -F -l error -f 'service = "payment"' app.log | while read line; do
    echo "$line"
    # Send alert (e.g., to Slack, PagerDuty, etc.)
done
```

### Performance Monitoring

```hl/dev/null/shell.sh#L1
# Track requests over threshold
hl -F -q 'duration > 3000' app.log
```

### Security Monitoring

```hl/dev/null/shell.sh#L1
# Monitor failed authentication attempts
hl -F -f 'event = "auth_failed"' auth.log
```

### Health Check Filtering

```hl/dev/null/shell.sh#L1
# Exclude health checks from monitoring
hl -F -q 'not (path = "/health" or path = "/ping")' app.log
```

### Multi-Region Monitoring

```hl/dev/null/shell.sh#L1
# Monitor logs from different regions (sorted chronologically)
hl -F us-east.log us-west.log eu-central.log ap-southeast.log
```

## Interruption Handling

### Follow Mode Interruption

Follow mode exits immediately on Ctrl+C:

```hl/dev/null/shell.sh#L1
hl -F app.log
# Press Ctrl+C -> exits immediately
```

The `--interrupt-ignore-count` option is **ignored** in follow mode.

### Piped Mode Interruption

For piped input, you can configure interrupt tolerance:

```hl/dev/null/shell.sh#L1
# Ignore first Ctrl+C (useful when using pager)
./myapp | hl -P --interrupt-ignore-count 1

# Ignore first 2 interrupts
kubectl logs -f my-pod | hl -P --interrupt-ignore-count 2
```

This is useful when:
- The pager (like `less`) needs to handle Ctrl+C first
- You want to stop the pager without killing `hl`
- You want to see shutdown logs after interrupting the application

## Monitoring with Themes

Use appropriate themes for different monitoring contexts:

```hl/dev/null/shell.sh#L1
# High-contrast for critical monitoring
hl -F --theme classic -l error app.log

# Comfortable for long-term monitoring
hl -F --theme one-dark-24 app.log

# Minimal for focused monitoring
hl -F --theme neutral app.log
```

## Combining Monitoring with Output Control

### Hide Noise

```hl/dev/null/shell.sh#L1
# Focus on errors and hide verbose fields
hl -F -l error --hide stack-trace --hide metadata app.log
```

### Show Compact Output

```hl/dev/null/shell.sh#L1
# Hide verbose fields for quick scanning
hl -F --hide trace-id --hide span-id app.log
```

### Timestamp Format

```hl/dev/null/shell.sh#L1
# Use local timezone for monitoring
hl -F -L app.log

# Use UTC (default)
hl -F app.log
```

## Performance Considerations

### Follow Mode Performance

Follow mode is efficient:
- Uses file system notifications (inotify/kqueue) for changes
- Handles log rotation gracefully
- Minimal CPU usage when idle
- Sorts efficiently across multiple files

### Piped Mode Performance

Piped mode processes input as it arrives:
- No buffering delays
- Minimal memory usage
- Suitable for high-volume streams
- Can handle mixed JSON/plain text

## Troubleshooting

### No Output in Follow Mode

If follow mode produces no output:

- Check if the file exists and is being written to
- Verify entries have recognized timestamps (follow mode skips entries without timestamps)
- Use `--raw` to see unparseable content
- Try piped mode instead: `tail -f app.log | hl -P`

### Missing Lines in Follow Mode

Follow mode only shows parseable entries with timestamps. If you're missing lines:

- Use piped mode to see everything: `tail -f app.log | hl -P`
- Check log format with `--raw`
- Verify timestamp format is recognized

### Log Rotation Not Detected

If log rotation isn't working:

- Verify you're using `-F` (follow mode)
- Check file permissions
- Ensure the application is writing to the new file

### High CPU Usage

If monitoring uses high CPU:

- Reduce filter complexity
- Limit the number of files being monitored
- Check if log volume is unusually high

## Tips and Best Practices

- **Use follow mode for files** — Better rotation handling and chronological sorting
- **Use piped mode for commands** — See all output, including non-JSON lines
- **Filter early** — Apply level and field filters to reduce output volume
- **Choose the right theme** — Pick a theme that's comfortable for extended monitoring
- **Combine with field management** — Hide noisy fields to focus on what matters
- **Test filters first** — Try filters on static logs before using in live mode
- **Monitor multiple sources** — Take advantage of automatic chronological merging

## Next Steps

- [Follow Mode](../features/follow-mode.md) — Detailed follow mode documentation
- [Filtering Examples](filtering.md) — More filtering patterns for monitoring
- [Field Management](field-management.md) — Control field visibility during monitoring
