# Time Display

hl provides flexible options for displaying timestamps in log entries. You can customize the time format, timezone, and precision to match your needs.

## Quick Examples

```sh
# Use local timezone instead of UTC
hl -L application.log

# Use specific timezone
hl -Z 'America/New_York' application.log

# Custom time format
hl -t '%Y-%m-%d %H:%M:%S' application.log

# Combine timezone and format
hl -L -t '%Y-%m-%d %H:%M:%S.%3N' application.log
```

## Timezone Options

### Default: UTC

By default, hl displays all timestamps in UTC:

```sh
hl application.log
```

### Local Timezone

Use the `-L` or `--local` flag to display timestamps in your local timezone:

```sh
hl -L application.log
```

This uses your system's timezone configuration.

### Specific Timezone

Use the `-Z` or `--time-zone` option to specify any timezone:

```sh
hl -Z 'Europe/London' application.log
hl -Z 'Asia/Tokyo' application.log
hl -Z 'America/Los_Angeles' application.log
```

Timezone names use the [IANA Time Zone Database](https://en.wikipedia.org/wiki/List_of_tz_database_time_zones) format (see the "TZ identifier" column).

### Environment Variable

Set a default timezone:

```sh
export HL_TIME_ZONE='Europe/Berlin'
hl application.log
```

Or for local timezone:

```sh
export HL_LOCAL=true
hl application.log
```

## Time Format

### Default Format

The default time format is:

```
%b %d %T.%3N
```

Which displays as: `Jan 15 10:30:45.123`

### Custom Format

Use the `-t` or `--time-format` option:

```sh
hl -t '%Y-%m-%d %H:%M:%S' application.log
```

### Common Formats

```sh
# ISO-8601 with milliseconds
hl -t '%Y-%m-%d %H:%M:%S.%3N' application.log

# US format
hl -t '%m/%d/%Y %I:%M:%S %p' application.log

# European format
hl -t '%d/%m/%Y %H:%M:%S' application.log

# Time only with microsecond precision
hl -t '%H:%M:%S.%6N' application.log

# Unix timestamp
hl -t '%s' application.log
```

### Environment Variable

Set a default format:

```sh
export HL_TIME_FORMAT='%Y-%m-%d %H:%M:%S.%3N'
hl application.log
```

## Complete Format Reference

For all available time format codes, see the [Time Format Reference](../reference/time-format.md).

## Combining Options

### Local Time with Custom Format

```sh
hl -L -t '%Y-%m-%d %H:%M:%S %Z' application.log
```

### Specific Timezone with ISO Format

```sh
hl -Z 'America/New_York' -t '%Y-%m-%dT%H:%M:%S%z' application.log
```

## Subsecond Precision

hl supports millisecond, microsecond, and nanosecond precision:

```sh
# Milliseconds (3 digits)
hl -t '%H:%M:%S.%3N' application.log

# Microseconds (6 digits)
hl -t '%H:%M:%S.%6N' application.log

# Nanoseconds (9 digits)
hl -t '%H:%M:%S.%9N' application.log
```

## Configuration File

Set defaults in your configuration file:

```toml
# ~/.config/hl/config.toml
time-format = "%Y-%m-%d %H:%M:%S.%3N"
time-zone = "America/New_York"
local = false
```

Or use local timezone:

```toml
time-format = "%Y-%m-%d %H:%M:%S.%3N"
local = true
```

## Precedence

Configuration sources are applied in this order (highest priority last):

1. Default values (UTC, default format)
2. Configuration file
3. Environment variables (`HL_TIME_FORMAT`, `HL_TIME_ZONE`, `HL_LOCAL`)
4. Command-line options (`-t`, `-Z`, `-L`)

## Use Cases

### Development

Quick, readable format with milliseconds:

```sh
hl -L -t '%H:%M:%S.%3N' application.log
```

### Production Monitoring

ISO-8601 with timezone for unambiguous timestamps:

```sh
hl -t '%Y-%m-%dT%H:%M:%S.%3N%z' application.log
```

### Incident Investigation

Local timezone for easier correlation with other systems:

```sh
hl -L -t '%Y-%m-%d %H:%M:%S.%3N %Z' application.log
```

### Log Archival

Unix timestamps for machine processing:

```sh
hl -t '%s' application.log
```

### Sharing Logs

Use local timezone with verbose format for clarity:

```sh
hl -L -t '%B %d, %Y at %I:%M:%S %p %Z' application.log
```

## Tips

1. **Use local timezone for investigation** - Easier to correlate with your activities:
   ```sh
   hl -L application.log
   ```

2. **Use UTC for production** - Avoids timezone confusion in distributed systems:
   ```sh
   hl -Z UTC application.log
   ```

3. **Match your logging format** - If your app logs in ISO-8601, display in ISO-8601:
   ```sh
   hl -t '%Y-%m-%dT%H:%M:%S.%3N' application.log
   ```

4. **Use subsecond precision for performance analysis**:
   ```sh
   hl -t '%H:%M:%S.%6N' application.log
   ```

5. **Set defaults** - Put common settings in your config file:
   ```toml
   local = true
   time-format = "%Y-%m-%d %H:%M:%S.%3N"
   ```

## Timezone Names

Common timezone identifiers:

| Timezone | Identifier |
|----------|-----------|
| UTC | `UTC` |
| Eastern Time | `America/New_York` |
| Pacific Time | `America/Los_Angeles` |
| Central European | `Europe/Berlin` |
| British Time | `Europe/London` |
| Japan | `Asia/Tokyo` |
| India | `Asia/Kolkata` |
| Australia Eastern | `Australia/Sydney` |

For a complete list, see the [List of tz database time zones](https://en.wikipedia.org/wiki/List_of_tz_database_time_zones).

## Examples

### Show Timestamps in Different Timezones

```sh
# UTC
hl -Z UTC application.log

# New York
hl -Z 'America/New_York' application.log

# Tokyo
hl -Z 'Asia/Tokyo' application.log

# London
hl -Z 'Europe/London' application.log
```

### Different Format Styles

```sh
# Compact
hl -t '%y%m%d %H%M%S' application.log

# Human-readable
hl -t '%A, %B %d %Y - %H:%M:%S' application.log

# Technical
hl -t '%Y-%m-%dT%H:%M:%S.%6N%z' application.log

# Simple
hl -t '%T' application.log
```

## Related

- [Time Format Reference](../reference/time-format.md) - Complete format specification
- [Filtering by Time Range](./filtering-time.md) - Time-based filtering
- [Configuration Files](../customization/config-files.md) - Set default time display options