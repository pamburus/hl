# Time Format Specifications

hl uses strftime-style format codes to display timestamps. This page describes all available format specifiers.

> **Note on Format Usage**
>
> The format you configure with `--time-format` serves two purposes:
>
> 1. **Display formatting:** Controls how timestamps appear in `hl` output
> 2. **Parsing for filters:** The format is also recognized by `--since` and `--until`, allowing you to copy
>    timestamps from output and paste them directly as filter arguments
>
> Example: If your format is `%b %d %T.%3N`, you can copy `Jan 15 10:30:45.123` from output and use:
> ```sh
> hl --since "Jan 15 10:30:45.123" app.log
> ```

## Default Format

The default time format is:

```
%b %d %T.%3N
```

Which displays as: `Jan 15 10:30:45.123`

## Setting Time Format

Use the `-t` or `--time-format` option:

```sh
hl -t '%Y-%m-%d %H:%M:%S' app.log
```

Or set via environment variable:

```sh
export HL_TIME_FORMAT='%Y-%m-%d %H:%M:%S'
```

Or in your configuration file:

```toml
time-format = "%Y-%m-%d %H:%M:%S"
```

## Common Format Examples

| Format | Example Output | Description |
|--------|----------------|-------------|
| `%b %d %T.%3N` | `Jan 15 10:30:45.123` | Default format |
| `%Y-%m-%d %H:%M:%S` | `2024-01-15 10:30:45` | ISO-8601 without milliseconds |
| `%Y-%m-%d %H:%M:%S.%3N` | `2024-01-15 10:30:45.123` | ISO-8601 with milliseconds |
| `%Y-%m-%dT%H:%M:%S%z` | `2024-01-15T10:30:45+0000` | ISO-8601 with timezone |
| `%d/%m/%Y %H:%M:%S` | `15/01/2024 10:30:45` | European format |
| `%m/%d/%Y %I:%M:%S %p` | `01/15/2024 10:30:45 AM` | US format with AM/PM |
| `%c` | `Mon Jan 15 10:30:45 2024` | Locale's date and time |
| `%s` | `1705315845` | Unix timestamp |

## Complete Format Specifiers

hl supports standard strftime format codes. For a complete reference, see:

**[strftime format specification](https://man7.org/linux/man-pages/man3/strftime.3.html)**

### Date Specifiers

| Code | Description | Example |
|------|-------------|---------|
| `%Y` | Year with century | `2024` |
| `%y` | Year without century (00-99) | `24` |
| `%m` | Month as decimal (01-12) | `01` |
| `%b` | Abbreviated month name | `Jan` |
| `%B` | Full month name | `January` |
| `%d` | Day of month (01-31) | `15` |
| `%e` | Day of month, space-padded (1-31) | `15` |
| `%j` | Day of year (001-366) | `015` |
| `%a` | Abbreviated weekday name | `Mon` |
| `%A` | Full weekday name | `Monday` |
| `%w` | Weekday as decimal (0-6, Sunday=0) | `1` |
| `%U` | Week number (Sunday first) | `02` |
| `%W` | Week number (Monday first) | `02` |

### Time Specifiers

| Code | Description | Example |
|------|-------------|---------|
| `%H` | Hour (00-23) | `10` |
| `%I` | Hour (01-12) | `10` |
| `%M` | Minute (00-59) | `30` |
| `%S` | Second (00-60) | `45` |
| `%p` | AM or PM | `AM` |
| `%P` | am or pm (lowercase) | `am` |
| `%T` | Time in HH:MM:SS format | `10:30:45` |
| `%R` | Time in HH:MM format | `10:30` |

### Subsecond Precision (hl Extensions)

hl extends strftime with subsecond precision:

| Code | Description | Example |
|------|-------------|---------|
| `%3N` | Milliseconds (3 digits) | `123` |
| `%6N` | Microseconds (6 digits) | `123456` |
| `%9N` | Nanoseconds (9 digits) | `123456789` |
| `%N` | Full nanoseconds available | varies |

### Timezone Specifiers

| Code | Description | Example |
|------|-------------|---------|
| `%z` | Timezone offset from UTC | `+0000` |
| `%Z` | Timezone name or abbreviation | `UTC` |

### Combined Formats

| Code | Description | Example |
|------|-------------|---------|
| `%c` | Locale's date and time | `Mon Jan 15 10:30:45 2024` |
| `%x` | Locale's date | `01/15/24` |
| `%X` | Locale's time | `10:30:45` |
| `%F` | Date in YYYY-MM-DD format | `2024-01-15` |

### Other Specifiers

| Code | Description | Example |
|------|-------------|---------|
| `%s` | Unix timestamp (seconds since epoch) | `1705315845` |
| `%%` | Literal `%` character | `%` |
| `%n` | Newline | |
| `%t` | Tab | |

## Examples

### ISO-8601 Formats

```sh
# Basic ISO-8601
hl -t '%Y-%m-%dT%H:%M:%S' app.log

# ISO-8601 with milliseconds
hl -t '%Y-%m-%dT%H:%M:%S.%3N' app.log

# ISO-8601 with timezone
hl -t '%Y-%m-%dT%H:%M:%S%z' app.log

# Full ISO-8601 with timezone
hl -t '%Y-%m-%dT%H:%M:%S.%3N%z' app.log
```

### Human-Readable Formats

```sh
# Compact format
hl -t '%y-%m-%d %T' app.log

# Verbose format
hl -t '%A, %B %d, %Y at %I:%M:%S %p' app.log

# Unix-style
hl -t '%b %d %H:%M:%S' app.log
```

### High Precision

```sh
# With milliseconds
hl -t '%H:%M:%S.%3N' app.log

# With microseconds
hl -t '%H:%M:%S.%6N' app.log

# With nanoseconds
hl -t '%H:%M:%S.%9N' app.log
```

### Copy-Paste Workflow

Your configured format is automatically recognized by `--since` and `--until`, so you can copy timestamps directly from output without changing anything:

```sh
# With default format (%b %d %T.%3N)
hl app.log | grep "error"
# Output: Jan 15 10:30:45.123 ERROR ...

# Copy timestamp and paste directly:
hl --since "Jan 15 10:30:45.123" app.log
```

If you prefer a different format for readability or consistency, configure it once and copy-paste will still work:

```sh
# Configure ISO format (personal preference)
hl -t '%Y-%m-%d %H:%M:%S.%3N' app.log
# Output: 2024-01-15 10:30:45.123 ERROR ...

# Copy and paste works with any configured format:
hl --since "2024-01-15 10:30:45.123" app.log

# Or compact format
hl -t '%m-%d %T' app.log
# Output: 01-15 10:30:45 ERROR ...

# Copy and paste still works:
hl --since "01-15 10:30:45" app.log
```

**Key point:** You don't need to change the format for copy-paste to work. Choose a format you like, and `hl` will recognize it in `--since`/`--until`.

## Timezone Handling

Time format works together with timezone settings:

```sh
# Display in UTC (default)
hl -t '%Y-%m-%d %H:%M:%S %Z' app.log

# Display in local timezone
hl -L -t '%Y-%m-%d %H:%M:%S %Z' app.log

# Display in specific timezone
hl -Z 'America/New_York' -t '%Y-%m-%d %H:%M:%S %Z' app.log
```

See [Time Display](../features/time-display.md) for more about timezone options.

## Configuration File

Set a default time format in your configuration file:

```toml
# config.toml
time-format = "%Y-%m-%d %H:%M:%S.%3N"
time-zone = "UTC"
```

## Tips

1. **Use subsecond precision for debugging**
   ```sh
   hl -t '%T.%6N' app.log
   ```

2. **Use ISO-8601 for logs that will be shared**
   ```sh
   hl -t '%Y-%m-%dT%H:%M:%S.%3N%z' app.log
   ```

3. **Use compact formats for quick viewing**
   ```sh
   hl -t '%m-%d %T' app.log
   ```

4. **Use verbose formats for presentations**
   ```sh
   hl -t '%B %d, %Y - %I:%M:%S %p' app.log
   ```

## Platform Differences

Most format codes are standardized, but some may behave differently across platforms:

- `%Z` timezone name may vary
- Locale-dependent formats (`%c`, `%x`, `%X`) depend on system locale
- Some codes may not be available on all platforms

For maximum portability, stick to basic codes like `%Y`, `%m`, `%d`, `%H`, `%M`, `%S`.

## Related

- [Time Display Options](../features/time-display.md) - Timezone and formatting options
- [Filtering by Time Range](../features/filtering-time.md) - Time-based filtering
- [strftime(3) man page](https://man7.org/linux/man-pages/man3/strftime.3.html) - Complete specification

## Quick Reference

Most commonly used:

```sh
# Default
hl -t '%b %d %T.%3N'

# ISO date with time
hl -t '%Y-%m-%d %H:%M:%S'

# US format
hl -t '%m/%d/%Y %I:%M:%S %p'

# European format
hl -t '%d/%m/%Y %H:%M:%S'

# Time only
hl -t '%H:%M:%S.%3N'

# Date only
hl -t '%Y-%m-%d'
```
