# Field Management

This page demonstrates how to control which fields are displayed in log output.

## Hiding Fields

### Hide Specific Fields

Hide one or more fields from the output:

```hl/dev/null/shell.sh#L1
# Hide a single field
hl --hide user_id app.log

# Hide multiple fields
hl --hide user_id --hide session_id --hide ip_address app.log
```

Short form using `-h`:

```hl/dev/null/shell.sh#L1
hl -h user_id -h session_id app.log
```

### Hide All Fields

Display only the core log elements (timestamp, level, message) without any additional fields:

```hl/dev/null/shell.sh#L1
hl --hide-all app.log
```

This is useful for getting a quick overview of log messages without field clutter.

### Hide All Except Specific Fields

Combine `--hide-all` with `--show` to display only selected fields:

```hl/dev/null/shell.sh#L1
# Show only user_id and request_id fields
hl --hide-all --show user_id --show request_id app.log
```

## Showing Fields

### Show Hidden Fields

By default, some fields might be hidden by your configuration. Use `--show` to display them:

```hl/dev/null/shell.sh#L1
# Show a specific field
hl --show debug_info app.log
```

Short form using `-H`:

```hl/dev/null/shell.sh#L1
hl -H debug_info app.log
```

### Show All Fields

Override any hiding configuration and show all fields:

```hl/dev/null/shell.sh#L1
hl --hide-none app.log
```

This is the opposite of `--hide-all` and ensures all fields are visible.

## Predefined Fields

### Hiding Standard Log Components

You can hide standard log elements:

```hl/dev/null/shell.sh#L1
# Hide timestamp
hl --hide-time app.log

# Hide log level
hl --hide-level app.log

# Hide logger name
hl --hide-logger app.log

# Hide caller information
hl --hide-caller app.log
```

Combine multiple options:

```hl/dev/null/shell.sh#L1
# Minimal output: just messages and custom fields
hl --hide-time --hide-level --hide-logger app.log
```

## Field Filtering Patterns

### Hide by Pattern

Hide fields matching a pattern (if supported):

```hl/dev/null/shell.sh#L1
# Hide all internal fields (starting with underscore)
hl --hide '_*' app.log

# Hide all debug-related fields
hl --hide 'debug_*' app.log
```

## Nested Field Handling

### Hide Nested Fields

Control nested JSON field visibility:

```hl/dev/null/shell.sh#L1
# Hide a nested field
hl --hide user.email app.log

# Hide all fields under a parent
hl --hide request.headers app.log
```

### Flatten vs Nested Display

Some fields are displayed nested by default. Control this with field expansion options:

```hl/dev/null/shell.sh#L1
# Show nested fields flattened
hl --flatten app.log
```

See [Field Expansion](../features/field-expansion.md) for more details.

## Practical Examples

### Security-Conscious Output

Hide sensitive fields before sharing logs:

```hl/dev/null/shell.sh#L1
# Hide PII and credentials
hl --hide password --hide email --hide api_key --hide token app.log
```

### Debugging Specific Components

Show only fields relevant to debugging:

```hl/dev/null/shell.sh#L1
# Focus on database queries
hl --hide-all --show query --show duration --show rows_affected db.log
```

### Simplified Dashboard View

Reduce noise for monitoring:

```hl/dev/null/shell.sh#L1
# Show only critical information
hl --hide-all --show status --show duration --show error app.log
```

### API Request Logs

Focus on HTTP-related fields:

```hl/dev/null/shell.sh#L1
# Show only HTTP request details
hl --hide-all --show method --show path --show status --show duration api.log
```

### Error Investigation

Show error details while hiding routine fields:

```hl/dev/null/shell.sh#L1
# Show error-related fields only
hl -l error --hide-all --show error --show stack_trace --show exception_type app.log
```

### Performance Analysis

Focus on timing and resource usage:

```hl/dev/null/shell.sh#L1
# Show performance metrics
hl --hide-all --show duration --show cpu_time --show memory_mb --show query_count app.log
```

### Clean Message View

Show just the log messages without any fields:

```hl/dev/null/shell.sh#L1
# Messages only
hl --hide-all --hide-time --hide-level --hide-logger app.log
```

## Combining with Filtering

Field management works alongside filtering:

```hl/dev/null/shell.sh#L1
# Filter and hide fields
hl -l error --hide user_id --hide session_id app.log

# Show only specific fields for specific entries
hl -f 'service = "api"' --hide-all --show path --show status --show duration app.log
```

## Configuration File Settings

Set default field visibility in your config file:

```hl/dev/null/config.toml#L1
[fields]
hide = ["user_id", "session_id", "internal_*"]
show = ["request_id", "trace_id"]
```

Command-line options override config file settings.

## Field Visibility Priority

When both `--hide` and `--show` are specified for the same field, the last option wins:

```hl/dev/null/shell.sh#L1
# user_id will be shown (last option wins)
hl --hide user_id --show user_id app.log

# user_id will be hidden (last option wins)
hl --show user_id --hide user_id app.log
```

When using `--hide-all` and `--show`:

```hl/dev/null/shell.sh#L1
# Only request_id is shown (--show overrides --hide-all for specific fields)
hl --hide-all --show request_id app.log
```

## Raw Field Display

Show non-JSON prefixes as structured fields:

```hl/dev/null/shell.sh#L1
# Display prefix timestamp and level as fields
hl --raw-fields app.log
```

This is useful when logs have non-JSON prefixes:

```hl/dev/null/example.txt#L1
2024-01-15 10:30:45 INFO {"message": "Started"}
```

Without `--raw-fields`, the prefix is parsed but not shown as fields. With `--raw-fields`, you'll see `prefix_time` and `prefix_level` fields.

## Common Field Management Patterns

### Development vs Production

```hl/dev/null/shell.sh#L1
# Development: show everything
hl --hide-none app.log

# Production: hide debug fields
hl --hide debug_* --hide trace_* app.log
```

### Log Rotation Analysis

```hl/dev/null/shell.sh#L1
# Hide timestamps when comparing log patterns across files
hl --hide-time app.log app.log.1 app.log.2
```

### Compact Output for Piping

```hl/dev/null/shell.sh#L1
# Minimal output for grep/awk processing
hl --hide-all --hide-time --hide-level app.log | grep "error"
```

### Audit Trails

```hl/dev/null/shell.sh#L1
# Show only audit-relevant fields
hl --hide-all --show user --show action --show resource --show timestamp audit.log
```

## Tips and Best Practices

- **Start broad, then narrow** — Use `--hide-all` then selectively `--show` fields you need.
- **Use field patterns** — Hide groups of related fields with wildcards (e.g., `debug_*`).
- **Save common patterns** — Set frequently-used field visibility in your config file.
- **Check field names** — Use `--raw` or view a few entries to see available field names.
- **Combine with queries** — Filter entries first, then manage field visibility for cleaner output.

## Troubleshooting

### Field Still Showing After Hide

If a field still appears after using `--hide`:

- Check if a later `--show` option is overriding it.
- Verify the field name is spelled correctly.
- Check your config file for conflicting settings.

### Too Much/Too Little Shown

If output is too verbose or sparse:

- Use `--hide-all` as a baseline and add back needed fields.
- Or use `--hide-none` to see everything, then hide specific fields.

### Nested Fields Not Hiding

For nested fields:

- Use dot notation: `--hide user.email`
- Or hide the entire parent: `--hide user`

## Next Steps

- [Field Expansion](../features/field-expansion.md) — Control how complex fields are displayed.
- [Output Formatting](../features/formatting.md) — Customize overall output format.
- [Filtering Examples](filtering.md) — Filter entries before managing field display.
