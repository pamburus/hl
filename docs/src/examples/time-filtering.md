# Time Filtering

This page demonstrates how to filter log entries by time ranges using `--since` and `--until` options.

> **Important Distinction**
>
> The formats shown on this page are for `--since` and `--until` command-line parameters ONLY.
>
> These formats are **not** recognized in log entries themselves. Log entries must use standard formats
> like RFC 3339 (`2024-01-15T10:30:45Z`) or Unix timestamps. See [Timestamp Handling](../features/timestamps.md)
> for details on log entry timestamp formats.

## Basic Time Filtering

### Show Logs After a Time

Display entries after a specific timestamp:

```sh
hl --since "2024-01-15 10:00:00" app.log
```

### Show Logs Before a Time

Display entries before a specific timestamp:

```sh
hl --until "2024-01-15 18:00" app.log
```

### Show Logs in a Time Range

Combine `--since` and `--until` to specify a time window:

```sh
hl --since "2024-01-15 10:00" --until "2024-01-15 12:00" app.log
```

This shows only entries between 10:00 AM and 12:00 PM on January 15, 2024.

## Absolute Time Formats

### ISO 8601 Format

The most precise and unambiguous format:

```sh
# Full ISO 8601 with timezone
hl --since "2024-01-15T10:00:00Z" app.log

# With timezone offset
hl --since "2024-01-15T10:00:00-08:00" app.log

# With milliseconds
hl --since "2024-01-15T10:00:00.123Z" app.log
```

### Date and Time

Various date-time formats are supported:

```sh
# YYYY-MM-DD HH:MM:SS
hl --since "2024-01-15 10:00:00" app.log

# YYYY-MM-DD HH:MM (seconds default to :00)
hl --since "2024-01-15 10:00" app.log

# YYYY-MM-DD (midnight)
hl --since "2024-01-15" app.log
```

### Date Only

When only a date is specified, it defaults to midnight:

```sh
# Shows entries from 2024-01-15 00:00:00 onwards
hl --since 2024-01-15 app.log

# Shows entries up to 2024-01-16 00:00:00
hl --until 2024-01-16 app.log
```

### Time Only (Today)

Specify time without date to use today's date:

```sh
# Since 10:00 AM today
hl --since "10:00:00" app.log

# Until 6:00 PM today (seconds optional)
hl --until "18:00" app.log
```

## Relative Time Formats

### Hours Ago

```sh
# Last hour
hl --since "1h ago" app.log

# Last 6 hours
hl --since "6h ago" app.log

# Last 24 hours
hl --since "24h ago" app.log
```

### Minutes Ago

```sh
# Last 30 minutes
hl --since "30m ago" app.log

# Last 5 minutes
hl --since "5m ago" app.log
```

### Days Ago

```sh
# Today (since midnight)
hl --since today app.log

# Yesterday onwards
hl --since yesterday app.log

# Last 7 days
hl --since "7 days ago" app.log

# Last 30 days
hl --since "30 days ago" app.log
```

### Weeks Ago

```sh
# Last week
hl --since "1w ago" app.log

# Last 4 weeks
hl --since "4w ago" app.log
```

### Months Ago

```sh
# Duration syntax: approximately 30.44 days
hl --since "-1M" app.log
hl --since "-3M" app.log
```

**Note:** Duration syntax (`-1M`, `-3M`) uses a fixed approximation of 30.44 days per month. For exact calendar month boundaries, use natural language:

```sh
# Natural language: exact calendar month
hl --since "1 month ago" app.log
hl --since "last month" app.log
hl --since "3 months ago" app.log

# Or specify exact date
hl --since 2024-11-01 app.log
```

### Seconds Ago

```sh
# Last 30 seconds
hl --since "30s ago" app.log
hl --since "-30s" app.log
```

### Years Ago

```sh
# Duration syntax: approximately 365.25 days
hl --since "-1y" app.log

# Natural language: likely more precise
hl --since "1 year ago" app.log
hl --since "last year" app.log
```

**Note:** Duration syntax (`-1y`) uses a fixed approximation of 365.25 days. Natural language ("1 year ago", "last year") may provide more calendar-aware results.

## Using Output Format for Filtering

The timestamp format configured via `--time-format` is also recognized by `--since` and `--until`. This means you can **copy a timestamp from `hl` output and paste it directly** as a filter argument.

### Copy-Paste Workflow

```sh
# Your config uses: time-format = "%b %d %T.%3N"
# Output shows: Jan 15 10:30:45.123

# Copy that timestamp and use it:
hl --since "Jan 15 10:30:45.123" app.log
```

### With Custom Formats

```sh
# Config: time-format = "%Y-%m-%d %H:%M:%S"
# Output: 2024-01-15 10:30:45

# Use directly (seconds optional):
hl --since "2024-01-15 10:30:45" --until "2024-01-15 11:00" app.log
```

This works because `hl` tries to parse filter times using your configured format before falling back to other formats.

### Finding the Right Timestamp

```sh
# Step 1: View logs to find the event
hl app.log | grep "deployment started"

# Output shows: Jan 15 14:30:22.456 ... deployment started

# Step 2: Copy that timestamp and use it directly
hl --since "Jan 15 14:30:22.456" app.log
```

**Note:** Your configured output format is automatically recognized by `--since` and `--until`, so you can always copy timestamps directly from output. No format conversion needed!

```sh
# With default format (%b %d %T.%3N)
hl app.log | grep "error"
# Output: Jan 15 14:30:45.123 ERROR ...
# Copy and paste works immediately:
hl --since "Jan 15 14:30:45.123" app.log

# With custom ISO format (if configured in your settings)
hl -t "%Y-%m-%dT%H:%M:%S.%3N" app.log | grep "error"
# Output: 2024-01-15T14:30:45.123 ERROR ...
# Copy and paste also works:
hl --since "2024-01-15T14:30:45.123" app.log
```

## Combining Relative and Absolute Times

Mix relative and absolute time specifications:

```sh
# From a specific date until 2 hours ago
hl --since 2024-01-15 --until "2h ago" app.log

# Last hour up to a specific time
hl --since "1h ago" --until "2024-01-15 18:00" app.log
```

## Practical Time Filtering Examples

### Today's Logs

```sh
# All logs from today (since midnight)
hl --since today app.log

# Or using specific dates
hl --since 2024-01-15 --until 2024-01-16 app.log
```

### Business Hours

```sh
# Today's business hours (9 AM to 5 PM)
hl --since "09:00" --until "17:00" app.log

# Specific date business hours (with seconds)
hl --since "2024-01-15 09:00:00" --until "2024-01-15 17:00:00" app.log
```

### Recent Errors

```sh
# Errors in the last hour
hl -l error --since "1h ago" app.log

# Warnings and errors in the last 30 minutes
hl -l warn --since "30m ago" app.log
```

### Incident Investigation

```sh
# Logs during a known incident window
hl --since "2024-01-15 14:30:00" --until "2024-01-15 15:45:00" app.log

# Include context before and after (without seconds)
hl --since "2024-01-15 14:00" --until "2024-01-15 16:00" app.log
```

### Overnight Logs

```sh
# Last night (6 PM to 6 AM)
hl --since "2024-01-14 18:00:00" --until "2024-01-15 06:00:00" app.log
```

### Weekly Report

```sh
# Last 7 days of errors
hl -l error --since "7 days ago" app.log

# Specific week
hl --since 2024-01-08 --until 2024-01-15 app.log
```

### Rolling Window

```sh
# Last 24 hours
hl --since "24h ago" app.log

# Last 12 hours
hl --since "12h ago" app.log
```

## Time Filtering with Other Filters

Combine time filtering with level and field filters:

```sh
# Recent errors from API service (use hyphens in field names)
hl -l error --since "1h ago" -f 'service = "api"' app.log

# Slow requests in the last 30 minutes
hl --since "30m ago" -f 'duration > 1000' app.log

# Production errors during deployment window
hl -l error --since "2024-01-15 10:00" --until "2024-01-15 10:30" -f 'env = "production"' app.log
```

## Time Zones

### UTC Times (Default)

By default, times without timezone info are interpreted as UTC:

```sh
# Interpreted as UTC (default)
hl --since "2024-01-15 10:00:00" app.log

# Or explicitly specify UTC with Z suffix
hl --since 2024-01-15T10:00:00Z app.log
```

### Using Local Time Zone

Use `-L` or `--local` to interpret times as local timezone:

```sh
# Interpret times as local timezone
hl -L --since "2024-01-15 10:00:00" app.log
```

### Using Specific Time Zone

Use `-Z` or `--time-zone` to interpret times in a specific timezone:

```sh
# Interpret times as America/New_York
hl -Z America/New_York --since "2024-01-15 10:00:00" app.log

# Interpret times as Europe/London
hl -Z Europe/London --since "2024-01-15 10:00:00" app.log
```

### Explicit Time Zone Offset in Timestamp

You can also include timezone offset directly in the timestamp:

```sh
# Pacific Time (UTC-8)
hl --since "2024-01-15T10:00:00-08:00" app.log

# Eastern Time (UTC-5)
hl --since "2024-01-15T10:00:00-05:00" app.log

# Central European Time (UTC+1)
hl --since "2024-01-15T10:00:00+01:00" app.log
```

**Note:** The `-Z/--time-zone` and `-L/--local` options affect how timestamps **without** timezone info are interpreted. Timestamps with explicit timezone offsets are always interpreted according to their offset.

## Performance Benefits

Time filtering with sorted logs is efficient:

- `hl` uses an index to quickly locate entries in the time range.
- Reading is minimized to only the relevant portions of the file.
- Multiple files are efficiently filtered before merging.

```sh
# Fast even on large files
hl --since "1h ago" large-app.log

# Efficient across multiple files (seconds optional)
hl --since "2024-01-15 10:00" app.log app.log.1 app.log.2
```

## Common Patterns

### Debug Recent Issues

```sh
# What happened in the last 5 minutes?
hl --since "5m ago" app.log
```

### Morning Review

```sh
# Check overnight logs
hl --since "18:00 yesterday" --until "08:00 today" app.log
```

### Deployment Verification

```sh
# Check logs since deployment
hl --since "2024-01-15 14:30" app.log
```

### Historical Analysis

```sh
# Compare yesterday to today
hl --since yesterday --until today app.log  # Yesterday
hl --since today app.log                     # Today
```

### Peak Hours Analysis

```sh
# Analyze lunchtime traffic (12-2 PM)
hl --since "12:00" --until "14:00" app.log
```

## Tips and Best Practices

- **Use relative times** for recent logs (`--since "1h ago"` is easier than calculating exact timestamps).
- **Use absolute times** for historical analysis or specific incidents.
- **Combine with field filters** to narrow results further.
- **Include context** by expanding the time window slightly before and after an incident.
- **Check timezone** if logs and filters are in different timezones.

## Troubleshooting

### No Results Returned

If time filtering returns no results:

- Verify the time range is correct (check log file timestamps).
- Ensure timezone matches (use `--raw` to see original timestamps).
- Check if logs are actually in the specified time range.

### Unexpected Results

If you get unexpected results:

- Check if timestamps are in a different timezone.
- Verify the time format is being parsed correctly.
- Use absolute times for precision instead of relative times.

## Next Steps

- [Filtering Examples](filtering.md) — Level and field-based filtering.
- [Query Examples](queries.md) — Complex query syntax.
- [Reference: Time Format Specifications](../reference/time-format.md) — Complete time format reference.
