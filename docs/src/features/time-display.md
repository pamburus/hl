# Time Display

hl provides flexible options for displaying timestamps in log entries. You can customize the time format, timezone, and precision to match your needs.

## Quick Examples

```sh
# Use local timezone instead of UTC
hl -L app.log

# Use specific timezone
hl -Z 'America/New_York' app.log

# Custom time format
hl -t '%Y-%m-%d %H:%M:%S' app.log

# Combine timezone and format
hl -L -t '%Y-%m-%d %H:%M:%S.%3N' app.log
```

## Configuration

**Time format:**

| Method | Setting |
|--------|---------|
| Config file | [`time-format`](../customization/config-files.md#time-format) |
| CLI option | [`-t, --time-format`](../reference/options.md#time-format) |
| Environment | [`HL_TIME_FORMAT`](../customization/environment.md#hl-time-format) |

**Timezone:**

| Method | Setting |
|--------|---------|
| Config file | [`time-zone`](../customization/config-files.md#time-zone) |
| CLI option | [`-Z, --time-zone`](../reference/options.md#time-zone) or [`-L, --local`](../reference/options.md#local) |
| Environment | [`HL_TIME_ZONE`](../customization/environment.md#hl-time-zone) |

**Defaults:** Format is `%b %d %T.%3N`, timezone is `UTC`.

## Timezone Options

### Default: UTC

By default, hl displays all timestamps in UTC:

```sh
hl app.log
```

### Local Timezone

Use the `-L` or `--local` flag to display timestamps in your local timezone:

```sh
hl -L app.log
```

This uses your system's timezone configuration.

### Specific Timezone

Use the `-Z` or `--time-zone` option to specify any timezone:

```sh
hl -Z 'Europe/London' app.log
hl -Z 'Asia/Tokyo' app.log
hl -Z 'America/Los_Angeles' app.log
```

Timezone names use the [IANA Time Zone Database](https://en.wikipedia.org/wiki/List_of_tz_database_time_zones) format (see the "TZ identifier" column).

### Environment Variable

Set a default timezone:

```sh
export HL_TIME_ZONE='Europe/Berlin'
hl app.log
```

Or use local timezone:

```sh
hl -L app.log
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
hl -t '%Y-%m-%d %H:%M:%S' app.log
```

### Common Formats

```sh
# ISO-8601 with milliseconds
hl -t '%Y-%m-%d %H:%M:%S.%3N' app.log

# US format
hl -t '%m/%d/%Y %I:%M:%S %p' app.log

# European format
hl -t '%d/%m/%Y %H:%M:%S' app.log

# Time only with microsecond precision
hl -t '%H:%M:%S.%6N' app.log

# Unix timestamp
hl -t '%s' app.log
```

### Environment Variable

Set a default format:

```sh
export HL_TIME_FORMAT='%Y-%m-%d %H:%M:%S.%3N'
hl app.log
```

## Complete Format Reference

For all available time format codes, see the [Time Format Reference](../reference/time-format.md).

## Combining Options

### Local Time with Custom Format

```sh
hl -L -t '%Y-%m-%d %H:%M:%S %Z' app.log
```

### Specific Timezone with ISO Format

```sh
hl -Z 'Asia/Tokyo' -t '%Y-%m-%dT%H:%M:%S%z' app.log
```

## Subsecond Precision

hl supports millisecond, microsecond, and nanosecond precision:

```sh
# Milliseconds (3 digits)
hl -t '%H:%M:%S.%3N' app.log

# Microseconds (6 digits)
hl -t '%H:%M:%S.%6N' app.log

# Nanoseconds (9 digits)
hl -t '%H:%M:%S.%9N' app.log
```

## Use Cases

### Development

Quick, readable format with milliseconds:

```sh
hl -L -t '%H:%M:%S.%3N' app.log
```

### Production Monitoring

ISO-8601 with timezone for unambiguous timestamps:

```sh
hl -t '%Y-%m-%dT%H:%M:%S.%3N%z' app.log
```

### Incident Investigation

Local timezone for easier correlation with other systems:

```sh
hl -L -t '%Y-%m-%d %H:%M:%S.%3N %Z' app.log
```

### Log Archival

Unix timestamps for machine processing:

```sh
hl -t '%s' app.log
```

### Sharing Logs

Use local timezone with verbose format for clarity:

```sh
hl -L -t '%B %d, %Y at %I:%M:%S %p %Z' app.log
```

## Tips

1. **Use local timezone for development** - Easier to correlate with your activities:
   ```sh
   hl -L app.log
   ```

2. **Use UTC for production** - Avoids timezone confusion in distributed systems:
   ```sh
   hl -Z UTC app.log
   hl app.log # Default is UTC unless "time-zone" is set in the configuration file
   ```

3. **Match your logging format** - If your app logs in ISO-8601, display in ISO-8601:
   ```sh
   hl -t '%Y-%m-%dT%H:%M:%S.%3N' app.log
   ```

4. **Use subsecond precision for performance analysis**:
   ```sh
   hl -t '%H:%M:%S.%6N' app.log
   ```

5. **Set defaults** - Put common settings in your config file:
   ```toml
   time-format = "%Y-%m-%d %H:%M:%S.%3N"
   # Note: use -L flag for local time, or set time-zone to your timezone
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
hl -Z UTC app.log

# New York
hl -Z 'America/New_York' app.log

# Tokyo
hl -Z 'Asia/Tokyo' app.log

# London
hl -Z 'Europe/London' app.log
```

### Different Format Styles

```sh
# Compact
hl -t '%y%m%d %H%M%S' app.log

# Human-readable
hl -t '%A, %B %d %Y - %H:%M:%S' app.log

# Technical
hl -t '%Y-%m-%dT%H:%M:%S.%6N%z' app.log

# Simple
hl -t '%T' app.log
```

## Related

- [Time Format Reference](../reference/time-format.md) - Complete format specification
- [Filtering by Time Range](./filtering-time.md) - Time-based filtering
- [Configuration Files](../customization/config-files.md#time-format) - Persistent time format configuration
- [Configuration Files](../customization/config-files.md#time-zone) - Persistent timezone configuration
