# Basic Usage

This page demonstrates common everyday use cases for `hl`.

## Viewing a Single Log File

The simplest usage is to view a log file:

```hl/dev/null/shell.sh#L1
hl app.log
```

This will:
- Parse JSON or logfmt entries
- Apply syntax highlighting and formatting
- Display the output in a pager (if the output is a terminal)

## Viewing Multiple Files

View multiple log files in chronological order:

```hl/dev/null/shell.sh#L1
hl app.log app.log.1 app.log.2
```

`hl` merges and sorts entries from all files by timestamp.

## Reading from Standard Input

Pipe log output directly to `hl`:

```hl/dev/null/shell.sh#L1
# For finite logs, pager helps with navigation
kubectl logs my-pod | hl

# For streaming logs, disable pager for real-time output
kubectl logs -f my-pod | hl -P
```

When `hl`'s output goes to a terminal, the pager runs by default. Use `-P` to disable it for streaming scenarios where you want immediate, continuous output.

## Viewing Compressed Files

`hl` automatically detects and decompresses common formats:

```hl/dev/null/shell.sh#L1
hl app.log.gz
hl app.log.bz2
hl app.log.xz
```

No need to manually decompress the files.

## Following Live Logs

Watch logs in real-time as they are written:

```hl/dev/null/shell.sh#L1
hl -F app.log
```

The `-F` (follow) mode:
- Monitors the file for new entries
- Handles log rotation automatically
- Sorts entries chronologically
- Exits on Ctrl+C

## Viewing Logs with Non-JSON Prefixes

Many applications emit non-JSON prefixes (timestamps, log levels) before JSON:

```hl/dev/null/example-log.txt#L1
2024-01-15 10:30:45 INFO {"message": "Server started", "port": 8080}
```

`hl` automatically detects and parses these:

```hl/dev/null/shell.sh#L1
hl app.log
```

Use `--raw-fields` to see the prefix as a structured field:

```hl/dev/null/shell.sh#L1
hl --raw-fields app.log
```

## Choosing a Theme

Select a different color theme:

```hl/dev/null/shell.sh#L1
hl --theme one-dark-24 app.log
```

List available themes:

```hl/dev/null/shell.sh#L1
hl --list-themes
```

Filter themes by characteristics:

```hl/dev/null/shell.sh#L1
# Show only dark themes
hl --list-themes=dark

# Show 256-color themes
hl --list-themes=256color
```

## Disabling the Pager

View output directly in the terminal without a pager:

```hl/dev/null/shell.sh#L1
hl --paging never app.log
```

Or use the short form `-P`:

```hl/dev/null/shell.sh#L1
hl -P app.log
```

## Default (Unsorted) Mode

By default, `hl` displays entries in the order they appear in the file(s):

```hl/dev/null/shell.sh#L1
hl app.log
```

This is faster than sorting and works well for already-sorted logs or when chronological order doesn't matter. Use `-s` or `--sort` to enable chronological sorting across files.

## Showing Raw Logs

Display raw input without parsing or formatting:

```hl/dev/null/shell.sh#L1
hl --raw app.log
```

Useful for:
- Debugging log format issues
- Viewing logs that aren't JSON/logfmt
- Seeing exactly what the input looks like

You can combine `--raw` with filtering to see raw entries that match criteria.

## Getting Help

Show all available options:

```hl/dev/null/shell.sh#L1
hl --help
```

Show version information:

```hl/dev/null/shell.sh#L1
hl --version
```

## Common Workflows

### Quick Log Check

View the last 50 entries from a log file:

```hl/dev/null/shell.sh#L1
hl app.log | head -n 50
```

### Live Application Logs

Monitor application output in real-time:

```hl/dev/null/shell.sh#L1
./myapp 2>&1 | hl -P
```

### Compressed Archive Inspection

Quickly check an old compressed log:

```hl/dev/null/shell.sh#L1
hl old-logs/app.2024-01-01.log.gz
```

### Multiple Sources Merged

View logs from multiple services together, sorted chronologically:

```hl/dev/null/shell.sh#L1
hl service-a.log service-b.log service-c.log
```

## Next Steps

- [Filtering Examples](filtering.md) — Filter logs by level, field values, and time ranges.
- [Query Examples](queries.md) — Use complex queries to find specific entries.
- [Field Management](field-management.md) — Control which fields are displayed.
- [Live Monitoring](live-monitoring.md) — Advanced follow-mode usage and live streaming.
