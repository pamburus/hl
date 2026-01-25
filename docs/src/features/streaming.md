# Live Streaming

hl supports viewing live log streams in real-time, making it perfect for monitoring applications as they run. This page covers how to use hl with live log data.

> **💡 Two Different Approaches**
> 
> - **`hl -F app.log`** - Parses, filters, and sorts messages chronologically. Only shows entries with valid timestamps that can be parsed.
> - **`tail -f app.log | hl -P`** - Shows everything in original order, including unparsable input and messages without recognized timestamps.
> 
> Choose based on your needs: structured sorting vs. raw completeness.

## Basic Live Streaming

### Method 1: hl's Follow Mode (`-F`)

Use hl's native `-F` flag for parsed, sorted log streaming:

```sh
hl -F app.log
```

The `-F` flag:
- Watches the file for changes
- Parses and sorts entries chronologically
- Disables the pager automatically
- Handles file rotations gracefully
- **Only shows entries with valid, recognized timestamps**
- **Filters out unparsable input**

See [Follow Mode](./follow-mode.md) for comprehensive documentation on follow mode features and configuration.

### Method 2: Piping from `tail -f`

Pipe from `tail -f` to see everything in original order:

```sh
tail -f app.log | hl -P
```

Use `-P` to disable the pager for streaming scenarios where you want immediate, continuous output as entries arrive.

**Key differences from `-F`:**
- Shows **all input** including unparsable lines
- Shows entries **without recognized timestamps**
- Preserves **original order** (no chronological sorting)
- Shows entries **as they arrive** in the file

This is ideal when you need to see everything, including malformed entries or during debugging.

## When to Use Each Method

### Use `hl -F` when:
- You want chronologically sorted output
- You're monitoring multiple log files simultaneously
- You only care about valid, parsable log entries
- You want to filter by time range or use `--tail` to see recent history

### Use `tail -f | hl -P` when:
- You need to see **everything**, including unparsable input
- You want logs in their **original order**
- You're debugging and need to see malformed entries
- You're following non-file sources (Docker, kubectl, etc.)
- Your logs might have lines without timestamps

## Practical Example: The Difference

Consider a log file with mixed content:

```
2024-01-15 10:00:00 {"level":"info","message":"Server started"}
Starting up...
2024-01-15 10:00:01 {"level":"info","message":"Port opened"}
Some debug output without timestamp
2024-01-15 10:00:02 {"level":"error","message":"Connection failed"}
```

**With `hl -F`:**
- Shows only the 3 JSON lines with timestamps
- Displays them in chronological order
- Skips "Starting up..." and "Some debug output without timestamp"

**With `tail -f | hl -P`:**
- Shows all 5 lines in original order
- Formats the parsable JSON lines
- Shows unparsable lines as-is

## Common Streaming Scenarios

### Following a Single Log File

For parsed, sorted output:

```sh
hl -F /var/log/app.log
```

For raw, complete output in original order:

```sh
tail -f /var/log/app.log | hl -P
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

See [Follow Mode](./follow-mode.md) for detailed configuration options including `--sync-interval-ms`, `--tail`, and multi-file monitoring strategies.

## Advanced Live Streaming

### Following Process Substitution

Monitor multiple Kubernetes pods:

```sh
hl -F \
  <(kubectl logs -f deployment/web-1) \
  <(kubectl logs -f deployment/web-2) \
  <(kubectl logs -f deployment/api-1)
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
| Chronological sorting | ❌ (original order) | ✓ (sorted) |
| File rotation handling | Limited | Automatic |
| Preload history | Manual | `--tail` option |
| Shows unparsable input | ✓ (everything) | ❌ (parsed only) |
| Shows entries without timestamp | ✓ | ❌ |
| Order | Original | Chronological |
| **Best for** | Complete output, debugging | Sorted multi-file monitoring |

**Key Difference:** `hl -F` parses, filters, and sorts. `tail -f \| hl -P` shows everything as-is.

## Use Cases

### Development

Monitor your application during development (shows all output including non-JSON):

```sh
npm start 2>&1 | hl -P -l d
```

### Production Monitoring

Follow production logs with error filtering and chronological sorting (see [Follow Mode](./follow-mode.md) for more options):

```sh
hl -F --level error /var/log/app/*.log
```

### Debugging Application Issues

When debugging, use `tail -f | hl -P` to see everything including startup messages, errors, and unparsable output:

```sh
tail -f app.log | hl -P
```

This ensures you don't miss important diagnostic information that might not be in valid JSON format.

### Tracing Specific Requests

Stream logs with specific query (use piping to preserve all output):

```sh
kubectl logs -f pod | hl -P -q 'request-id = "abc123"'
```

## Tips and Tricks

1. **Choose the right method** - Use `-F` for clean, sorted output across multiple files. Use `tail -f | hl -P` when you need to see everything including non-JSON output:
   ```sh
   # Sorted, parsed output only
   hl -F app.log
   
   # Everything, including unparsable lines
   tail -f app.log | hl -P
   ```

2. **Use color themes** - Even with `-P`, colors are enabled if outputting to a terminal:
   ```sh
   tail -f app.log | hl -P --theme hl-dark
   ```

3. **Hide noisy fields** - Reduce clutter in live streams:
   ```sh
   tail -f app.log | hl -P -h headers -h metadata
   ```

4. **Combine with grep** - Post-filter the output:
   ```sh
   tail -f app.log | hl -P -l e | grep "database"
   ```

5. **Save to file while viewing** - Tee the output:
   ```sh
   tail -f app.log | hl -P | tee formatted.log
   ```

6. **Local timezone** - View timestamps in your local time:
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

## Related Topics

- [Follow Mode](./follow-mode.md) — comprehensive guide to `-F` flag features
- [Chronological Sorting](./sorting.md) — batch sorting with `--sort`
- [Multiple Files](./multiple-files.md) — handling multiple log sources
- [Filtering](./filtering.md) — apply filters to reduce noise in live streams
