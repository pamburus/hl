# Time Handling Overview

`hl` works with timestamps in three different contexts. Understanding the distinction helps you work effectively with time-based features.

## The Three Contexts

| Context | Purpose | Where Used | Documentation |
|---------|---------|------------|---------------|
| **Input Parsing** | Parse timestamps from log entry fields | Log files, stdin | [Timestamp Handling](./timestamps.md) |
| **Filter Arguments** | Specify time ranges for filtering | `--since`, `--until` options | [Time Filtering](../examples/time-filtering.md) |
| **Output Formatting** | Control timestamp display | `--time-format` option | [Time Format Reference](../reference/time-format.md) |

### 1. Input Parsing (Log Entry Timestamps)

When `hl` reads log entries, it looks for timestamp fields (`timestamp`, `time`, `ts`, etc.) and parses them.

**Supported formats:**
- RFC 3339: `2024-01-15T10:30:45.123Z`
- Unix timestamps: `1705315845123` (auto-detected unit)
- ISO-like variants: `2024-01-15 10:30:45.123Z`

**NOT supported in log entries:**
- Human-readable: `"yesterday"`, `"1 hour ago"`
- Relative: `"-1h"`, `"-30m"`

**Example log entries:**
```json
{"timestamp": "2024-01-15T10:30:45Z", "level": "info"}  ✓ Recognized
{"timestamp": 1705315845123, "level": "info"}           ✓ Recognized
{"timestamp": "yesterday", "level": "info"}             ✗ Not recognized
```

### 2. Filter Arguments (`--since` and `--until`)

Command-line time filtering supports many more formats than log entry parsing.

**Supported formats:**
- Everything from input parsing (RFC 3339, Unix timestamps)
- **Plus:** Relative times (`-1h`, `-30m`, `-7d`, `-1M`, `-1y`, `1 hour ago`, `30 minutes ago`)
- **Plus:** Natural language (`yesterday`, `friday`, `today`, `last month`)
- **Plus:** Your configured output format (copy-paste from output!)

**Note:** Duration syntax with `-` prefix uses fixed approximations:
- `-1M` = 30.44 days (approximate month)
- `-1y` = 365.25 days (approximate year)

Natural language ("1 month ago", "last month") is calendar-aware and more precise for month/year boundaries.

**Examples:**
```bash
hl --since "-1h" app.log                    # Duration: 1 hour ago
hl --since "-7d" app.log                    # Duration: 7 days ago
hl --since "-1M" app.log                    # Duration: ~30.44 days ago
hl --since "1 hour ago" app.log             # Natural language
hl --since "yesterday" app.log              # Natural language (named day)
hl --since "1 month ago" app.log            # Natural language (calendar-aware)
hl --since "last month" app.log             # Natural language (calendar month)
hl --since "friday 6pm" app.log             # Natural language (day + time)
hl --since "2024-01-15T10:00:00Z" app.log   # Absolute RFC 3339
hl --since "Jan 15 10:30:45.123" app.log    # Copy from output (if format matches)
```

### 3. Output Formatting (`--time-format`)

Controls how timestamps are **displayed** in `hl` output.

**Format syntax:** strftime-style codes (`%Y`, `%m`, `%d`, `%H`, `%M`, `%S`, etc.)

**Default:** `%b %d %T.%3N` → `Jan 15 10:30:45.123`

**Examples:**
```bash
hl -t "%Y-%m-%d %H:%M:%S.%3N" app.log    # ISO: 2024-01-15 10:30:45.123
hl -t "%H:%M:%S" app.log                 # Time only: 10:30:45
hl -t "%c" app.log                       # Locale: Mon Jan 15 10:30:45 2024
```

## Key Insights

### Copy-Paste Between Output and Filters

The format you configure for output is **also recognized** by `--since` and `--until`:

```bash
# Configure output format
export HL_TIME_FORMAT="%Y-%m-%d %H:%M:%S.%3N"

# View logs - output shows: 2024-01-15 14:30:45.123
hl app.log

# Copy timestamp and use it directly
hl --since "2024-01-15 14:30:45.123" app.log
```

### Different Capabilities for Different Needs

**Log producers** (applications writing logs) should use:
- RFC 3339 with timezone: `2024-01-15T10:30:45.123Z`
- Unix timestamps: `1705315845123`

**Log consumers** (you, using `hl`) can use:
- Human-readable filters: `--since "1 hour ago"`
- Natural language: `--since "yesterday"`
- Custom formats: whatever you configure

This separation allows flexibility in filtering while maintaining strict, machine-parseable formats in logs.

## Common Questions

### "Why doesn't `hl` recognize 'yesterday' in my logs?"

Because "yesterday" is a relative term that changes meaning over time. Logs should contain absolute timestamps. The string "yesterday" is only useful for filtering (where it's interpreted relative to *now*).

### "Can I use `--since '1h'` or must I use `--since '1h ago'`?"

You can use either:
- `--since "-1h"` (duration syntax with minus sign)
- `--since "1 hour ago"` (natural language)

Both work. The bare `1h` is NOT accepted (it's ambiguous).

### "What's the difference between `-1M` and `last month`?"

- `-1M` = approximately 30.44 days ago (fixed duration from humantime crate)
- `"1 month ago"` or `"last month"` = calendar-aware parsing (via chrono-english crate)

Use duration syntax (`-1M`) for "roughly 30 days" and natural language (`"1 month ago"`, `"last month"`) for calendar month boundaries.

### "How do I change the timestamp format in my logs?"

`hl` doesn't change log files. The `--time-format` option only affects how `hl` **displays** timestamps. Your log files remain unchanged. To change timestamps in log files, reconfigure your application's logging.

### "Why do some examples show 'Jan 15' and others '2024-01-15'?"

These are different OUTPUT formats controlled by `--time-format`. Both can be used for filtering (via `--since`/`--until`) if you configure that format. The INPUT logs can use any supported format regardless of output format.

## Workflow Example

Here's how the three contexts work together:

```bash
# 1. Application writes logs with RFC 3339 timestamps (INPUT)
#    {"timestamp": "2024-01-15T14:30:45.123Z", "level": "error", "message": "..."}

# 2. View logs with custom display format (OUTPUT)
hl -t "%b %d %T.%3N" app.log
#    Output: Jan 15 14:30:45.123 ERROR ...

# 3. Filter using human-readable time (FILTER)
hl --since "1 hour ago" app.log

# 4. Or copy timestamp from output and use it (FILTER using OUTPUT format)
hl --since "Jan 15 14:30:45.123" app.log

# 5. Or use natural language (FILTER)
hl --since "yesterday" --until "today" app.log
```

## Quick Reference

| **What**              | **Format Example**                | **Where**              |
|-----------------------|-----------------------------------|------------------------|
| **Log entry field**   | `"2024-01-15T10:30:45.123Z"`     | Inside JSON/logfmt     |
| **Filter (recent)**   | `--since "-1h"` or `"1 hour ago"` or `"-1M"` | Command line           |
| **Filter (natural)**  | `--since "yesterday"`            | Command line           |
| **Filter (absolute)** | `--since "2024-01-15 10:00"`     | Command line           |
| **Filter (copy)**     | `--since "Jan 15 10:30:45.123"`  | Copy from output       |
| **Output format**     | `-t "%Y-%m-%d %H:%M:%S.%3N"`     | Command line or config |

## Summary

- **Log entries** must use RFC 3339, ISO variants, or Unix timestamps
- **`--since` and `--until`** accept those PLUS human-readable formats
- **Output format** controls display AND is recognized by filters
- **Copy-paste** from output to filters works because of this integration

## See Also

- [Timestamp Handling](./timestamps.md) - Input parsing details
- [Time Filtering Examples](../examples/time-filtering.md) - Filter format examples  
- [Time Display](./time-display.md) - Timezone and display options
- [Time Format Reference](../reference/time-format.md) - strftime codes