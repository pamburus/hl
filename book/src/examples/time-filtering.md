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

```hl/dev/null/shell.sh#L1
hl --since "2024-01-15 10:00:00" app.log
```

### Show Logs Before a Time

Display entries before a specific timestamp:

```hl/dev/null/shell.sh#L1
hl --until "2024-01-15 18:00:00" app.log
```

### Show Logs in a Time Range

Combine `--since` and `--until` to specify a time window:

```hl/dev/null/shell.sh#L1
hl --since "2024-01-15 10:00:00" --until "2024-01-15 12:00:00" app.log
```

This shows only entries between 10:00 AM and 12:00 PM on January 15, 2024.

## Absolute Time Formats

### ISO 8601 Format

The most precise and unambiguous format:

```hl/dev/null/shell.sh#L1
# Full ISO 8601 with timezone
hl --since "2024-01-15T10:00:00Z" app.log

# With timezone offset
hl --since "2024-01-15T10:00:00-08:00" app.log

# With milliseconds
hl --since "2024-01-15T10:00:00.123Z" app.log
```

### Date and Time

Various date-time formats are supported:

```hl/dev/null/shell.sh#L1
# YYYY-MM-DD HH:MM:SS
hl --since "2024-01-15 10:00:00" app.log

# YYYY-MM-DD HH:MM
hl --since "2024-01-15 10:00" app.log

# YYYY-MM-DD (midnight)
hl --since "2024-01-15" app.log
```

### Date Only

When only a date is specified, it defaults to midnight:

```hl/dev/null/shell.sh#L1
# Shows entries from 2024-01-15 00:00:00 onwards
hl --since "2024-01-15" app.log

# Shows entries up to 2024-01-16 00:00:00
hl --until "2024-01-16" app.log
```

### Time Only (Today)

Specify time without date to use today's date:

```hl/dev/null/shell.sh#L1
# Since 10:00 AM today
hl --since "10:00:00" app.log

# Until 6:00 PM today
hl --until "18:00:00" app.log
```

## Relative Time Formats

### Hours Ago

```hl/dev/null/shell.sh#L1
# Last hour
hl --since "1h ago" app.log

# Last 6 hours
hl --since "6h ago" app.log

# Last 24 hours
hl --since "24h ago" app.log
```

### Minutes Ago

```hl/dev/null/shell.sh#L1
# Last 30 minutes
hl --since "30m ago" app.log

# Last 5 minutes
hl --since "5m ago" app.log
```

### Days Ago

```hl/dev/null/shell.sh#L1
# Today (since midnight)
hl --since "0d ago" app.log

# Yesterday onwards
hl --since "1d ago" app.log

# Last 7 days
hl --since "7d ago" app.log

# Last 30 days
hl --since "30d ago" app.log
```

### Weeks Ago

```hl/dev/null/shell.sh#L1
# Last week
hl --since "1w ago" app.log

# Last 4 weeks
hl --since "4w ago" app.log
```

### Months Ago

```hl/dev/null/shell.sh#L1
# Last month
hl --since "1M ago" app.log

# Last 3 months
hl --since "3M ago" app.log
```

### Seconds Ago

```hl/dev/null/shell.sh#L1
# Last 30 seconds
hl --since "30s ago" app.log
```

## Using Output Format for Filtering

The timestamp format configured via `--time-format` is also recognized by `--since` and `--until`. This means you can **copy a timestamp from `hl` output and paste it directly** as a filter argument.

### Copy-Paste Workflow

```hl/dev/null/shell.sh#L1
# Your config uses: time-format = "%b %d %T.%3N"
# Output shows: Jan 15 10:30:45.123

# Copy that timestamp and use it:
hl --since "Jan 15 10:30:45.123" app.log
```

### With Custom Formats

```hl/dev/null/shell.sh#L1
# Config: time-format = "%Y-%m-%d %H:%M:%S"
# Output: 2024-01-15 10:30:45

# Use directly:
hl --since "2024-01-15 10:30:45" --until "2024-01-15 11:00:00" app.log
```

This works because `hl` tries to parse filter times using your configured format before falling back to other formats.

### Finding the Right Timestamp

```hl/dev/null/shell.sh#L1
# Step 1: View logs to find the event
hl app.log | grep "deployment started"

# Output shows: Jan 15 14:30:22.456 ... deployment started

# Step 2: Copy that timestamp and use it
hl --since "Jan 15 14:30:22.456" app.log
```

**Tip:** To see timestamps in a specific format for copying, use `--time-format`:

```hl/dev/null/shell.sh#L1
# Show timestamps in ISO format for precise copy-paste
hl -t "%Y-%m-%dT%H:%M:%S.%3N" app.log | grep "error"

# Then use the copied timestamp
hl --since "2024-01-15T14:30:45.123" app.log
```

## Combining Relative and Absolute Times

Mix relative and absolute time specifications:

```hl/dev/null/shell.sh#L1
# From a specific date until 2 hours ago
hl --since "2024-01-15" --until "2h ago" app.log

# Last hour up to a specific time
hl --since "1h ago" --until "2024-01-15 18:00:00" app.log
```

## Practical Time Filtering Examples

### Today's Logs

```hl/dev/null/shell.sh#L1
# All logs from today (since midnight)
hl --since "0d ago" app.log

# Or using a date
hl --since "2024-01-15" --until "2024-01-16" app.log
```

### Business Hours

```hl/dev/null/shell.sh#L1
# Today's business hours (9 AM to 5 PM)
hl --since "09:00" --until "17:00" app.log

# Specific date business hours
hl --since "2024-01-15 09:00" --until "2024-01-15 17:00" app.log
```

### Recent Errors

```hl/dev/null/shell.sh#L1
# Errors in the last hour
hl -l error --since "1h ago" app.log

# Warnings and errors in the last 30 minutes
hl -l warn --since "30m ago" app.log
```

### Incident Investigation

```hl/dev/null/shell.sh#L1
# Logs during a known incident window
hl --since "2024-01-15 14:30:00" --until "2024-01-15 15:45:00" app.log

# Include context before and after
hl --since "2024-01-15 14:00:00" --until "2024-01-15 16:00:00" app.log
```

### Overnight Logs

```hl/dev/null/shell.sh#L1
# Last night (6 PM to 6 AM)
hl --since "2024-01-14 18:00" --until "2024-01-15 06:00" app.log
```

### Weekly Report

```hl/dev/null/shell.sh#L1
# Last 7 days of errors
hl -l error --since "7d ago" app.log

# Specific week
hl --since "2024-01-08" --until "2024-01-15" app.log
```

### Rolling Window

```hl/dev/null/shell.sh#L1
# Last 24 hours
hl --since "24h ago" app.log

# Last 12 hours
hl --since "12h ago" app.log
```

## Time Filtering with Other Filters

Combine time filtering with level and field filters:

```hl/dev/null/shell.sh#L1
# Recent errors from API service (use hyphens in field names)
hl -l error --since "1h ago" -f 'service = "api"' app.log

# Slow requests in the last 30 minutes
hl --since "30m ago" -f 'duration > 1000' app.log

# Production errors during deployment window
hl -l error --since "2024-01-15 10:00" --until "2024-01-15 10:30" -f 'env = "production"' app.log
```

## Time Zones

### UTC Times

Specify UTC explicitly:

```hl/dev/null/shell.sh#L1
# ISO 8601 with Z suffix
hl --since "2024-01-15T10:00:00Z" app.log
```

### Local Time Zone

Times without timezone info are interpreted as local time:

```hl/dev/null/shell.sh#L1
# Uses local timezone
hl --since "2024-01-15 10:00:00" app.log
```

### Explicit Time Zone Offset

```hl/dev/null/shell.sh#L1
# Pacific Time (UTC-8)
hl --since "2024-01-15T10:00:00-08:00" app.log

# Eastern Time (UTC-5)
hl --since "2024-01-15T10:00:00-05:00" app.log

# Central European Time (UTC+1)
hl --since "2024-01-15T10:00:00+01:00" app.log
```

## Performance Benefits

Time filtering with sorted logs is efficient:

- `hl` uses an index to quickly locate entries in the time range.
- Reading is minimized to only the relevant portions of the file.
- Multiple files are efficiently filtered before merging.

```hl/dev/null/shell.sh#L1
# Fast even on large files
hl --since "1h ago" large-app.log

# Efficient across multiple files
hl --since "2024-01-15 10:00" app.log app.log.1 app.log.2
```

## Common Patterns

### Debug Recent Issues

```hl/dev/null/shell.sh#L1
# What happened in the last 5 minutes?
hl --since "5m ago" app.log
```

### Morning Review

```hl/dev/null/shell.sh#L1
# Check overnight logs
hl --since "18:00 yesterday" --until "08:00 today" app.log
```

### Deployment Verification

```hl/dev/null/shell.sh#L1
# Check logs since deployment
hl --since "2024-01-15 14:30:00" app.log
```

### Historical Analysis

```hl/dev/null/shell.sh#L1
# Compare yesterday to today
hl --since "1d ago" --until "0d ago" app.log  # Yesterday
hl --since "0d ago" app.log                    # Today
```

### Peak Hours Analysis

```hl/dev/null/shell.sh#L1
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
