# Live Streaming

hl supports viewing live log streams in real-time, making it perfect for monitoring applications as they run. This page covers how to use hl with live log data.

## Basic Live Streaming

The simplest way to stream live logs is to pipe them into hl:

```sh
tail -f application.log | hl -P
```

The `-P` flag disables the pager, allowing you to see new entries as they arrive.

## Why Use `-P` for Streaming?

When streaming live logs, the pager would interfere with seeing new entries in real-time. The `-P` flag ensures:

- New log entries appear immediately
- No buffering delays
- Output flows continuously
- Standard terminal scrolling works

## Common Streaming Scenarios

### Following a Single Log File

```sh
tail -f /var/log/application.log | hl -P
```

### Following Application Output

```sh
./my-application 2>&1 | hl -P
```

### Following Docker Logs

```sh
docker logs -f container-name | hl -P
```

### Following Kubernetes Logs

```sh
kubectl logs -f pod-name | hl -P
```

### Following journalctl

```sh
journalctl -f | hl -P
```

## Filtering Live Streams

You can apply filters to live streams just like regular files:

### Filter by Level

Show only errors and warnings:

```sh
tail -f app.log | hl -P -l w
```

### Filter by Field

Show only logs from a specific service:

```sh
kubectl logs -f pod-name | hl -P -f service=api
```

### Complex Queries

Filter slow requests in real-time:

```sh
tail -f app.log | hl -P -q 'duration > 1'
```

## Live Streaming with Multiple Sources

hl can follow multiple log sources simultaneously with the `-F` (follow) flag:

```sh
hl -F app1.log app2.log app3.log
```

This is different from using `tail -f` because hl:
- Automatically sorts entries by timestamp
- Monitors all files for changes
- Handles file rotations gracefully
- Displays which file each entry came from

### Follow Mode Features

When using `-F`:
- Pager is automatically disabled
- Entries are sorted chronologically within a time window
- New entries appear in real-time
- Works with multiple files and compressed files

### Controlling Sync Interval

The `--sync-interval-ms` option controls how often entries are sorted:

```sh
hl -F --sync-interval-ms 500 app1.log app2.log
```

- Lower values (e.g., 100ms) give faster updates but use more CPU
- Higher values (e.g., 1000ms) are more efficient for high-volume logs
- Default is 100ms

### Preloading Historical Entries

Use `--tail` to show the last N entries before following:

```sh
hl -F --tail 100 app1.log app2.log
```

This displays the last 100 entries from each file before switching to live mode.

## Advanced Live Streaming

### Following Process Substitution

Monitor multiple Kubernetes pods:

```sh
hl -F \
  <(kubectl logs -f deployment/web-1) \
  <(kubectl logs -f deployment/web-2) \
  <(kubectl logs -f deployment/api-1)
```

### Combining Static and Live Sources

```sh
hl -F archived.log current.log
```

hl will read `archived.log` completely, then follow changes in `current.log`.

### With Time Range Filtering

Show only recent entries when starting:

```sh
hl -F --since -1h --tail 50 application.log
```

## Performance Considerations

### Buffer Size

For high-throughput streams, increase the buffer size:

```sh
tail -f app.log | hl -P --buffer-size "1 MiB"
```

### Maximum Message Size

Handle very large log entries:

```sh
tail -f app.log | hl -P --max-message-size "128 MiB"
```

## Stopping Live Streams

To exit a live stream:

1. Press `Ctrl+C` once for a clean shutdown
2. Press `Ctrl+C` multiple times to force exit

The `--interrupt-ignore-count` option controls how many interrupts are ignored:

```sh
tail -f app.log | hl -P --interrupt-ignore-count 5
```

This allows up to 5 accidental `Ctrl+C` presses before actually exiting.

## Comparison: Piping vs Follow Mode

| Feature | `tail -f \| hl -P` | `hl -F` |
|---------|-------------------|---------|
| Single file | ✓ | ✓ |
| Multiple files | Manual setup | Native support |
| Chronological sorting | No | Yes |
| File rotation handling | Limited | Automatic |
| Preload history | Manual | `--tail` option |
| Performance | Good | Optimized |

## Use Cases

### Development

Monitor your application during development:

```sh
npm start 2>&1 | hl -P -l d
```

### Production Monitoring

Follow production logs with error filtering:

```sh
hl -F -l e --tail 20 /var/log/app/*.log
```

### Debugging

Stream logs with specific query:

```sh
kubectl logs -f pod | hl -P -q 'request.id = "abc123"'
```

### Multi-Service Monitoring

Watch multiple services in sync:

```sh
hl -F --sync-interval-ms 200 \
  <(kubectl logs -f svc/web) \
  <(kubectl logs -f svc/api) \
  <(kubectl logs -f svc/worker)
```

## Tips and Tricks

1. **Use color themes** - Even with `-P`, colors are enabled if outputting to a terminal:
   ```sh
   tail -f app.log | hl -P --theme hl-dark
   ```

2. **Hide noisy fields** - Reduce clutter in live streams:
   ```sh
   tail -f app.log | hl -P -h headers -h metadata
   ```

3. **Combine with grep** - Post-filter the output:
   ```sh
   tail -f app.log | hl -P -l e | grep "database"
   ```

4. **Save to file while viewing** - Tee the output:
   ```sh
   tail -f app.log | hl -P | tee formatted.log
   ```

5. **Local timezone** - View timestamps in your local time:
   ```sh
   tail -f app.log | hl -P -L
   ```

## Troubleshooting

### No Output Appearing

If you don't see output:

1. Check if the source is actually producing logs
2. Verify filters aren't too restrictive
3. Try without filters: `tail -f app.log | hl -P`
4. Check buffer settings

### Delayed Output

If output is delayed:

1. Reduce sync interval: `--sync-interval-ms 50`
2. Check the source isn't buffering
3. For `tail -f`, use `tail -f -n 0` to start from the end

### High CPU Usage

If CPU usage is high:

1. Increase sync interval: `--sync-interval-ms 500`
2. Reduce the number of files being followed
3. Apply filters to reduce processing load

## Next Steps

- [Multiple Files](./multiple-files.md) - Learn more about handling multiple log sources
- [Follow Mode](./follow-mode.md) - Deep dive into `-F` flag features
- [Filtering](./filtering.md) - Apply filters to reduce noise in live streams