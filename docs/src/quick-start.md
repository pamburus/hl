# Quick Start

This guide will get you up and running with hl in minutes. We'll cover the most common use cases to help you start viewing your logs right away.

## Your First Command

The simplest way to use hl is to point it at a log file:

```sh
hl app.log
```

This will:
- Parse the JSON or logfmt entries in the file
- Format them for human readability
- Display them in a pager (like `less`) for easy navigation

## Basic Examples

### Viewing Multiple Files

Concatenate and view multiple log files:

```sh
hl app.log app.log.1 app.log.2
```

### Streaming Live Logs

Follow a live log file with chronological sorting:

```sh
hl -F app.log
```

Or follow a log file showing everything in original order:

```sh
tail -f app.log | hl -P
```

**Key difference:** `-F` parses and sorts entries chronologically (only shows parsable entries with timestamps), while `tail -f | hl -P` shows everything including unparsable input and entries without timestamps.

### Viewing Compressed Logs

hl automatically handles compressed files:

```sh
hl app.log.gz app.log.zst app.log.bz2
```

No need to decompress first – hl supports gzip, zstd, bzip2, and xz formats.

## Common Filtering Tasks

### Filter by Log Level

Show only errors:

```sh
hl -l e app.log
```

Show warnings and errors:

```sh
hl -l w app.log
```

Show info and above (excludes debug and trace):

```sh
hl -l i app.log
```

### Filter by Field Value

Show logs where `service` equals `api`:

```sh
hl -f service=api app.log
```

Show logs where `status` is not `200`:

```sh
hl -f 'status!=200' app.log
```

### Filter by Time Range

Show logs from the last 3 hours:

```sh
hl --since -3h app.log
```

Show logs from a specific day:

```sh
hl --since '2024-01-15' --until '2024-01-16' app.log
```

## Customizing the Display

### Hide Specific Fields

Hide verbose fields you don't need:

```sh
hl -h headers -h metadata app.log
```

### Show Only Specific Fields

Hide all fields except the ones you want:

```sh
hl -h '*' -h '!method' -h '!url' app.log
```

The `!` prefix reveals a field when others are hidden.

### Change Time Format

Display time in a custom format:

```sh
hl -t '%Y-%m-%d %H:%M:%S' app.log
```

See the [Time Format Reference](./reference/time-format.md) for all available format specifiers.

### Use Local Timezone

Display timestamps in your local timezone instead of UTC:

```sh
hl -L app.log
```

## Working with Multiple Sources

### Sort Logs Chronologically

When viewing multiple log files, sort entries by timestamp:

```sh
hl -s app1.log app2.log app3.log
```

This is especially useful when logs are from different servers or services.

### Follow Multiple Files

Monitor multiple log files in real-time, sorted chronologically:

```sh
hl -F app1.log app2.log app3.log
```

This shows entries sorted by timestamp across all files. Note that `-F` only displays entries with valid, parsable timestamps.

## Choosing a Theme

`hl` comes with several built-in themes. List available themes:

```sh
hl --list-themes
```

Use a specific theme:

```sh
hl --theme frostline app.log
```

Or set it as default via environment variable:

```sh
export HL_THEME=frostline
hl app.log
```

### Interactive Theme Selection

Use [fzf](https://junegunn.github.io/fzf/) to interactively select a theme:

```sh
# Interactively choose a dark-friendly theme with live preview
hl --list-themes=dark | fzf --color='bg+:23,gutter:-1,pointer:210' --highlight-line --preview-window 'right,border-left,88%,<142(up,88%,border-bottom)' --preview="hl -t '%b %d %T' --input-info minimal -c --theme {} sample/*.log"
```

## Getting Help

### View All Options

See the complete list of command-line options:

```sh
hl --help
```

### Quick Reference Card

Common options:

| Option | Description |
|--------|-------------|
| `-l LEVEL` | Filter by log level (e, w, i, d, t) |
| `-f KEY=VALUE` | Filter by field value |
| `-q QUERY` | Complex query filter |
| `--since TIME` | Show logs after this time |
| `--until TIME` | Show logs before this time |
| `-s` | Sort chronologically |
| `-F` | Follow mode (live updates) |
| `-P` | Disable pager |
| `-h KEY` | Hide field |
| `-t FORMAT` | Time format |
| `-L` | Use local timezone |
| `--theme NAME` | Select theme |

## Common Workflows

### Debugging an Error

```sh
# Find all errors in the last hour
hl -l e --since -1h app.log

# Find errors from a specific service
hl -l e -f service=payment app.log

# Find errors with stack traces
hl -l e -q 'exists(stack)' app.log
```

### Monitoring Performance

```sh
# Find slow requests (duration > 1 second)
hl -q 'duration > 1' app.log

# Find failed HTTP requests
hl -q 'status >= 400' app.log

# Combine conditions
hl -q 'duration > 0.5 or status >= 500' app.log
```

### Investigating an Incident

```sh
# View logs from the incident window
hl --since '2024-01-15 14:30:00' --until '2024-01-15 15:00:00' *.log

# Sort across all services
hl -s --since '2024-01-15 14:30:00' service1/*.log service2/*.log

# Focus on errors during that time
hl -s -l e --since '2024-01-15 14:30:00' --until '2024-01-15 15:00:00' *.log
```

## Next Steps

Now that you know the basics, explore the detailed feature documentation:

- [Filtering](./features/filtering.md) - Learn advanced filtering techniques
- [Queries](./features/filtering-queries.md) - Master the query language
- [Themes](./customization/themes.md) - Customize the appearance
- [Configuration](./customization/config-files.md) - Set up your preferences
- [Examples](./examples/basic.md) - See more real-world examples

Happy log viewing!
