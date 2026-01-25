# Viewing Logs

hl is designed to make viewing structured logs effortless. This section covers the fundamental ways to view and navigate your log files.

## Basic Viewing

The most straightforward way to view a log file is to simply pass it as an argument:

```sh
hl application.log
```

This command:
1. Detects the log format (JSON or logfmt)
2. Parses each log entry
3. Formats it for human readability
4. Opens it in a pager for easy navigation

## What hl Does for You

When you view logs with hl, it automatically:

- **Colorizes output** using syntax highlighting to make entries easier to scan
- **Formats timestamps** in a readable format
- **Organizes fields** in a consistent, logical order
- **Handles multi-line content** intelligently
- **Opens a pager** so you can navigate large files easily

## Supported Log Formats

hl supports two primary structured log formats:

### JSON Logs

```json
{"time":"2024-01-15T10:30:45Z","level":"info","message":"Server started","port":8080}
```

hl parses standard JSON logs and extracts common fields like timestamp, level, message, and custom fields.

### Logfmt Logs

```
time=2024-01-15T10:30:45Z level=info message="Server started" port=8080
```

Logfmt is a key=value format popular in many logging frameworks. hl handles quoted values, escaping, and nested structures.

### Auto-Detection

You don't need to specify the format – hl automatically detects which format your logs use and processes them accordingly.

## Output Structure

By default, hl displays log entries with:

1. **Input indicator** - Shows which file the entry came from (when viewing multiple files)
2. **Timestamp** - Formatted for readability
3. **Log level** - Color-coded (error, warn, info, debug, trace)
4. **Logger name** - If present in the log
5. **Message** - The main log message
6. **Additional fields** - All other fields in the log entry
7. **Caller information** - File and line number, if available

## Navigation

When viewing logs in a pager (the default behavior), you can use standard pager commands:

### Common less Commands

- `Space` or `f` - Page forward
- `b` - Page backward
- `/pattern` - Search forward for a pattern
- `?pattern` - Search backward for a pattern
- `n` - Next search result
- `N` - Previous search result
- `g` - Go to beginning
- `G` - Go to end
- `q` - Quit

### Disabling the Pager

For some use cases, you might want to disable the pager:

```sh
hl -P application.log
```

The `-P` flag is useful when:
- Piping output to another command
- Following live logs
- Capturing output to a file
- Working in a script

## Next Topics

Learn more about specific viewing features:

- [Automatic Pager Integration](./pager.md) - Customizing pager behavior
- [Live Streaming](./streaming.md) - Viewing logs in real-time
- [Multiple Files](./multiple-files.md) - Working with multiple log sources
- [Compressed Files](./compressed.md) - Viewing compressed logs directly
