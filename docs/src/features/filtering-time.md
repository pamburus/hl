# Filtering by Time Range

Time range filtering allows you to show only log entries that fall within a specific time window. This is essential for investigating incidents, analyzing specific periods, or reducing log volume.

## Basic Syntax

Use `--since` and `--until` options to filter by time:

```sh
# Entries after a specific time
hl --since '2024-01-15 10:00:00' application.log

# Entries before a specific time
hl --until '2024-01-15 11:00:00' application.log

# Entries within a time range
hl --since '2024-01-15 10:00:00' --until '2024-01-15 11:00:00' application.log
```

## Time Format Options

hl supports multiple time format inputs for maximum flexibility.

### RFC-3339 Format

Standard ISO-8601/RFC-3339 timestamps:

```sh
# With timezone
hl --since '2024-01-15T10:00:00Z' application.log
hl --since '2024-01-15T10:00:00+00:00' application.log

# Local time (if using -L)
hl -L --since '2024-01-15T10:00:00' application.log
```

### Human-Readable Format

Date and time in various formats:

```sh
# Full date and time
hl --since 'Jan 15 10:00:00' application.log
hl --since 'January 15 10:00:00' application.log

# Date only (midnight assumed)
hl --since '2024-01-15' application.log

# With seconds
hl --since 'Jun 19 11:22:33' application.log
```

### Relative Time Offsets

Use relative offsets from current time:

```sh
# Hours ago
hl --since -1h application.log
hl --since -3h application.log

# Days ago
hl --since -1d application.log
hl --since -7d application.log
hl --since -14d application.log

# Minutes ago
hl --since -30m application.log

# Weeks ago
hl --since -2w application.log
```

### Named Time References

Use convenient named references:

```sh
# Today (since midnight)
hl --since today application.log

# Yesterday (all of yesterday)
hl --since yesterday --until today application.log

# Day names (last occurrence)
hl --since monday application.log
hl --since friday application.log
hl --since sunday application.log
```

## Common Time Range Patterns

### Last N Hours

```sh
# Last hour
hl --since -1h application.log

# Last 3 hours
hl --since -3h application.log

# Last 24 hours
hl --since -24h application.log
```

### Last N Days

```sh
# Last 24 hours (same as -1d)
hl --since -1d application.log

# Last 3 days
hl --since -3d application.log

# Last week
hl --since -7d application.log
```

### Specific Day

```sh
# All of today
hl --since today application.log

# All of yesterday
hl --since yesterday --until today application.log

# Specific date
hl --since '2024-01-15' --until '2024-01-16' application.log
```

### Specific Time Window

```sh
# Between 10 AM and 11 AM
hl --since '2024-01-15 10:00:00' --until '2024-01-15 11:00:00' application.log

# Business hours yesterday
hl --since 'yesterday 09:00:00' --until 'yesterday 17:00:00' application.log
```

### Open-Ended Ranges

```sh
# Everything after a point in time
hl --since '2024-01-15 14:30:00' application.log

# Everything before a point in time
hl --until '2024-01-15 14:30:00' application.log
```

## Timezone Handling

Time filters respect the timezone settings:

### UTC (Default)

```sh
# Times interpreted as UTC
hl --since '2024-01-15 10:00:00' application.log
```

### Local Timezone

```sh
# Times interpreted as local timezone
hl -L --since '2024-01-15 10:00:00' application.log
```

### Specific Timezone

```sh
# Times interpreted in specified timezone
hl -Z 'America/New_York' --since '2024-01-15 10:00:00' application.log
```

### Explicit Timezone in Time String

```sh
# Timezone in the time string takes precedence
hl --since '2024-01-15T10:00:00+05:00' application.log
hl --since '2024-01-15T10:00:00Z' application.log
```

## Combining with Other Filters

### Time + Level

```sh
# Errors in last hour
hl -l e --since -1h application.log

# Warnings yesterday
hl -l w --since yesterday --until today application.log
```

### Time + Field Filter

```sh
# API errors in last 3 hours
hl -f service=api -l e --since -3h application.log

# Database logs today
hl -f component=database --since today application.log
```

### Time + Query

```sh
# Slow requests in last hour
hl -q 'duration > 1.0' --since -1h application.log

# 5xx errors yesterday
hl -q 'status >= 500' --since yesterday --until today application.log
```

### Complete Filtering

```sh
# All filters combined
hl -l e \
   -f service=payment \
   -q 'status >= 500' \
   --since '2024-01-15 10:00:00' \
   --until '2024-01-15 11:00:00' \
   application.log
```

## Performance Benefits

Time filtering is extremely efficient when combined with sorting:

### With Sorting (`-s`)

```sh
# Very fast - uses index to skip irrelevant sections
hl -s --since -1h *.log
```

When using `-s` flag:
- Initial scan builds timestamp index (~2 GiB/s)
- Time filtering uses index to skip blocks
- Dramatically faster for large files
- Index is cached and reused

### Without Sorting

```sh
# Still works but scans entire file
hl --since -1h application.log
```

Without `-s`:
- Scans entire file
- Filters each entry by timestamp
- Still fast but no index optimization

## Examples by Use Case

### Incident Investigation

```sh
# Logs during incident window
hl --since '2024-01-15 14:30:00' --until '2024-01-15 15:00:00' application.log

# Errors during incident
hl -l e --since '2024-01-15 14:30:00' --until '2024-01-15 15:00:00' application.log

# With chronological sorting across multiple files
hl -s -l e --since '2024-01-15 14:30:00' --until '2024-01-15 15:00:00' *.log
```

### Daily Review

```sh
# All of yesterday
hl --since yesterday --until today application.log

# Yesterday's errors
hl -l e --since yesterday --until today application.log

# Yesterday's warnings and errors
hl -l w --since yesterday --until today application.log
```

### Recent Activity

```sh
# Last hour
hl --since -1h application.log

# Last 30 minutes
hl --since -30m application.log

# Last 3 hours of API errors
hl -f service=api -l e --since -3h application.log
```

### Specific Time Windows

```sh
# Morning logs (9 AM to noon)
hl --since 'today 09:00:00' --until 'today 12:00:00' application.log

# Overnight logs
hl --since 'yesterday 18:00:00' --until 'today 06:00:00' application.log

# Weekend logs
hl --since 'saturday 00:00:00' --until 'monday 00:00:00' application.log
```

### Performance Analysis

```sh
# Slow requests in last hour
hl -q 'duration > 0.5' --since -1h application.log

# Peak hour analysis
hl --since 'today 12:00:00' --until 'today 13:00:00' application.log
```

## Advanced Patterns

### Rolling Time Windows

```sh
# Last 24 hours (rolling window)
hl --since -24h application.log

# Last 7 days (rolling week)
hl --since -7d application.log

# Last 30 days (rolling month)
hl --since -30d application.log
```

### Business Hours

```sh
# Today's business hours
hl --since 'today 09:00:00' --until 'today 17:00:00' application.log

# Yesterday's business hours
hl --since 'yesterday 09:00:00' --until 'yesterday 17:00:00' application.log
```

### Week-Based Analysis

```sh
# This week (since Monday)
hl --since monday application.log

# Last week (Monday to Sunday)
hl --since 'monday -7d' --until monday application.log
```

### Before/After Event

```sh
# 1 hour before incident
hl --until '2024-01-15 14:30:00' --since '2024-01-15 13:30:00' application.log

# 2 hours after deployment
hl --since '2024-01-15 10:00:00' --until '2024-01-15 12:00:00' application.log
```

## Troubleshooting

### No Results

If you get no output:

1. **Check timezone interpretation**:
   ```sh
   # Try with local timezone
   hl -L --since '2024-01-15 10:00:00' application.log
   
   # Or explicit UTC
   hl --since '2024-01-15T10:00:00Z' application.log
   ```

2. **Verify time range is correct**:
   ```sh
   # Check what you're asking for
   hl --since -1h application.log  # Last hour
   hl --since -1d application.log  # Last 24 hours
   ```

3. **Check if logs have timestamps**:
   ```sh
   hl --raw application.log | head
   ```

4. **Verify logs are in range**:
   ```sh
   # See all entries first
   hl application.log | head
   hl application.log | tail
   ```

### Unexpected Results

1. **Time parsed in wrong timezone**:
   ```sh
   # Use explicit timezone
   hl --since '2024-01-15T10:00:00+00:00' application.log
   
   # Or set timezone
   hl -Z UTC --since '2024-01-15 10:00:00' application.log
   ```

2. **Relative time confusion**:
   ```sh
   # -1d means last 24 hours, not "yesterday"
   hl --since -1d application.log
   
   # For yesterday specifically
   hl --since yesterday --until today application.log
   ```

3. **Date format not recognized**:
   ```sh
   # Use standard formats
   hl --since '2024-01-15 10:00:00' application.log
   hl --since 'Jan 15 10:00:00' application.log
   ```

### Performance Issues

If time filtering is slow:

1. **Use sorted mode**:
   ```sh
   hl -s --since -1h *.log
   ```

2. **Narrow the time range**:
   ```sh
   # Instead of
   hl --since -7d application.log
   
   # Use
   hl --since -1d application.log
   ```

3. **Combine with other filters**:
   ```sh
   hl -l e --since -1h application.log
   ```

## Time Format Reference

For valid time formats in filtering, hl accepts:

- **ISO-8601/RFC-3339**: `2024-01-15T10:00:00Z`
- **Date and time**: `2024-01-15 10:00:00`, `Jan 15 10:00:00`
- **Date only**: `2024-01-15`, `Jan 15`
- **Relative**: `-1h`, `-3h`, `-1d`, `-7d`, `-30m`, `-2w`
- **Named**: `today`, `yesterday`, `monday`, `friday`
- **Current time format**: If you've set a custom format with `-t`, you can use that format in `--since`/`--until`

See [Time Format Reference](../reference/time-format.md) for time display formatting (different from input parsing).

## Best Practices

1. **Use relative times for recent logs**:
   ```sh
   hl --since -1h application.log
   ```

2. **Use absolute times for incident investigation**:
   ```sh
   hl --since '2024-01-15 14:30:00' --until '2024-01-15 15:00:00' application.log
   ```

3. **Combine with sorted mode for performance**:
   ```sh
   hl -s --since -1d *.log
   ```

4. **Use explicit timezones to avoid confusion**:
   ```sh
   hl --since '2024-01-15T10:00:00Z' application.log
   ```

5. **Start broad, then narrow**:
   ```sh
   # Start with
   hl --since -1d application.log
   
   # Then narrow
   hl --since -1h application.log
   ```

6. **Layer with other filters**:
   ```sh
   hl -l e --since -1h application.log
   ```

## Related

- [Time Display Options](./time-display.md) - Formatting timestamps in output
- [Chronological Sorting](./sorting-chrono.md) - Fast time-based sorting
- [Filtering Overview](./filtering.md) - All filtering methods
- [Time Filtering Examples](../examples/time-filtering.md) - Real-world scenarios