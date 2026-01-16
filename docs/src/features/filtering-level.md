# Filtering by Log Level

Log level filtering is the quickest way to narrow down logs by their severity. hl makes it easy to filter logs to show only messages at or above a specific level.

## Understanding Log Levels

hl recognizes five standard log levels, ordered from most verbose to most severe:

1. **Trace** (`t`) - Extremely detailed debugging information
2. **Debug** (`d`) - Detailed debugging information
3. **Info** (`i`) - Informational messages
4. **Warning** (`w`) - Warning messages
5. **Error** (`e`) - Error messages

## Basic Usage

Use the `-l` or `--level` option to filter by minimum log level:

```sh
hl -l LEVEL application.log
```

## Level Shortcuts

Each level has a single-character shortcut:

```sh
# Show only errors
hl -l e application.log

# Show warnings and errors
hl -l w application.log

# Show info, warnings, and errors (excludes debug and trace)
hl -l i application.log

# Show debug and above (excludes only trace)
hl -l d application.log

# Show all messages including trace
hl -l t application.log
```

## How Level Filtering Works

When you specify a level, hl displays that level **and all more severe levels**.

| Filter | Shows These Levels |
|--------|-------------------|
| `-l t` | trace, debug, info, warning, error (all) |
| `-l d` | debug, info, warning, error |
| `-l i` | info, warning, error |
| `-l w` | warning, error |
| `-l e` | error only |

## Common Use Cases

### Debugging Applications

Show debug messages and above (excludes trace):

```sh
hl -l d application.log
```

### Production Monitoring

In production, you typically don't want debug or trace messages:

```sh
hl -l i application.log
```

### Error Investigation

Focus only on errors:

```sh
hl -l e application.log
```

### Warning Review

Check warnings and errors:

```sh
hl -l w application.log
```

## Combining with Other Filters

Level filtering combines well with other filter types:

### Level + Time Range

Errors in the last hour:

```sh
hl -l e --since -1h application.log
```

### Level + Field Filter

Errors from a specific service:

```sh
hl -l e -f service=api application.log
```

### Level + Query

Warnings with slow response times:

```sh
hl -l w -q 'duration > 0.5' application.log
```

### Level + Multiple Files

Errors across all log files, sorted chronologically:

```sh
hl -l e -s *.log
```

## Setting Default Level

Use an environment variable to set a default level filter:

```sh
export HL_LEVEL=i
hl application.log
```

This will filter to info and above by default, but can be overridden with `-l`.

## Full Level Names

You can also use full level names instead of shortcuts:

```sh
hl -l error application.log
hl -l warning application.log
hl -l info application.log
hl -l debug application.log
hl -l trace application.log
```

## Case Insensitivity

Level filtering is case-insensitive:

```sh
hl -l ERROR application.log
hl -l Error application.log
hl -l error application.log
```

All three commands are equivalent.

## Level Detection

hl automatically detects log levels from common field names:

- `level`
- `severity`
- `loglevel`
- `log_level`

And recognizes various level value formats:

- Lowercase: `error`, `warn`, `info`, `debug`, `trace`
- Uppercase: `ERROR`, `WARN`, `INFO`, `DEBUG`, `TRACE`
- Capitalized: `Error`, `Warn`, `Info`, `Debug`, `Trace`
- Short forms: `ERR`, `WRN`, `INF`, `DBG`, `TRC`
- Numeric: Standard syslog levels

## Performance Considerations

Level filtering is extremely fast because hl:

1. Parses the level early in processing
2. Uses bitmap indexing for level checks
3. Short-circuits processing for filtered-out entries
4. Requires no additional parsing or evaluation

This makes level filtering ideal as a first filter in a chain of filters.

## Examples

### Development Workflow

During development, show everything except trace:

```sh
hl -l d app.log
```

### Production Monitoring

Monitor production logs, hiding debug/trace noise:

```sh
tail -f /var/log/app.log | hl -P -l i
```

### Incident Investigation

Focus on errors during an incident time window:

```sh
hl -l e --since '2024-01-15 10:00' --until '2024-01-15 11:00' app.log
```

### Multi-Service Errors

Find all errors across multiple services:

```sh
hl -l e -s service1/*.log service2/*.log service3/*.log
```

### Warning Review

Daily review of warnings:

```sh
hl -l w --since yesterday --until today app.log
```

## Troubleshooting

### No Output

If you get no output when filtering by level:

1. Verify logs contain entries at that level:
   ```sh
   hl application.log | grep -i error
   ```

2. Check if level field is named differently:
   ```sh
   hl --raw application.log | head
   ```

3. Try without level filter to see all entries:
   ```sh
   hl application.log
   ```

### Unexpected Entries

If you see entries you didn't expect:

- Remember that level filtering shows the specified level **and above**
- For example, `-l w` shows both warnings AND errors
- To show only warnings (excluding errors), use a query:
  ```sh
  hl -q 'level = warning' application.log
  ```

## Best Practices

1. **Start with level filtering** - It's the fastest way to reduce log volume
2. **Use shortcuts** - `-l e` is quicker than `-l error`
3. **Combine with time ranges** - Limit both by time and severity
4. **Set defaults** - Use `HL_LEVEL` for your typical use case
5. **Layer filters** - Apply level filter first, then add field/query filters

## Next Steps

- [Filtering by Field Values](./filtering-fields.md) - Filter by specific field values
- [Filtering by Time Range](./filtering-time.md) - Focus on specific time periods
- [Complex Queries](./filtering-queries.md) - Build sophisticated filters
- [Filtering Examples](../examples/filtering.md) - Real-world filtering scenarios