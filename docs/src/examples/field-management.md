# Field Management

This page demonstrates how to control which fields are displayed in log output.

## Hiding Fields

### Hide Specific Fields

Hide one or more fields from the output using `-h` or `--hide`:

```sh
# Hide a single field
hl --hide user-id app.log

# Hide multiple fields
hl --hide user-id --hide session-id --hide ip-address app.log
```

Short form using `-h`:

```sh
hl -h user-id -h session-id app.log
```

### Revealing Fields

To reveal a field that might be hidden by configuration, prefix the field name with `!`:

```sh
# Reveal a specific field
hl --hide '!debug-info' app.log
```

### Reveal All Fields

To reveal all fields (override any hiding configuration):

```sh
# Reveal all fields
hl --hide '!*' app.log
```

This ensures all fields are visible, overriding any hiding configuration.

## Field Filtering Patterns

You can hide multiple related fields by repeating the `--hide` option:

```sh
# Hide multiple related fields
hl --hide debug-field1 --hide debug-field2 --hide debug-field3 app.log
```

## Nested Field Handling

### Hide Nested Fields

Control nested JSON field visibility:

```sh
# Hide a nested field using dot notation
hl --hide user.email app.log

# Hide multiple nested fields
hl --hide request.headers.authorization --hide request.headers.cookie app.log
```

### Flatten vs Nested Display

Control how nested objects are displayed with the `--flatten` option:

```sh
# Always flatten nested objects (default)
hl --flatten always app.log

# Never flatten nested objects
hl --flatten never app.log
```

See [Field Expansion](../features/field-expansion.md) for how flattening interacts with expansion modes.

## Practical Examples

### Security-Conscious Output

Hide sensitive fields before sharing logs:

```sh
# Hide PII and credentials
hl --hide password --hide email --hide api-key --hide token app.log
```

### Debugging Specific Components

Show only fields relevant to debugging:

```sh
# Focus on database queries by hiding non-essential fields
hl --hide user-id --hide session-id --hide trace-id db.log
```

### Simplified Dashboard View

Reduce noise for monitoring by hiding less important fields:

```sh
# Hide verbose fields
hl --hide stack-trace --hide headers --hide metadata app.log
```

### API Request Logs

Focus on HTTP-related fields by hiding non-essential data:

```sh
# Hide internal fields
hl --hide trace-id --hide span-id --hide internal-state api.log
```

### Error Investigation

Focus on error details by hiding routine fields:

```sh
# Hide routine fields to focus on errors
hl -l error --hide user-id --hide session-id --hide request-id app.log
```

### Performance Analysis

Focus on timing and resource usage by hiding non-performance fields:

```sh
# Hide non-performance fields
hl --hide user-id --hide session-id --hide ip-address app.log
```

### Clean Message View

Show just the log messages by hiding all custom fields (note: standard log components like time, level, and logger are always shown unless using `--raw`):

```sh
# Hide all custom fields (configure in config file which fields to hide by default)
hl app.log
```

## Combining with Filtering

Field management works alongside filtering:

```sh
# Filter and hide fields
hl -l error --hide user-id --hide session-id app.log

# Hide verbose fields for specific entries
hl -f 'service = "api"' --hide trace-id --hide span-id --hide metadata app.log
```

## Configuration File Settings

Set default field visibility in your config file:

```hl/dev/null/config.toml#L1
[fields]
hide = ["user-id", "session-id", "internal-*"]
```

**Note**: Field names use hyphens. While underscores and hyphens are interchangeable when filtering, use hyphens for consistency with display output.

Command-line options override config file settings.

## Field Visibility Priority

When multiple `--hide` options are specified, they are applied in order. Using `!` to reveal takes precedence:

```sh
# Hide user-id and session-id
hl --hide user-id --hide session-id app.log

# Reveal user-id (overrides any previous hiding)
hl --hide '!user-id' app.log

# Reveal all fields
hl --hide '!*' app.log
```

## Raw Field Display

Show non-JSON prefixes as structured fields:

```sh
# Display prefix timestamp and level as fields
hl --raw-fields app.log
```

This is useful when logs have non-JSON prefixes:

```hl/dev/null/example.txt#L1
2024-01-15 10:30:45 INFO {"message": "Started"}
```

Without `--raw-fields`, the prefix is parsed but not shown as fields. With `--raw-fields`, you'll see `prefix-time` and `prefix-level` fields.

## Common Field Management Patterns

### Development vs Production

```sh
# Development: show everything (reveal all)
hl --hide '!*' app.log

# Production: hide debug fields
hl --hide debug-field1 --hide debug-field2 --hide trace-data app.log
```

### Log Rotation Analysis

```sh
# Compare log patterns across files (timestamps are always shown)
hl app.log app.log.1 app.log.2
```

### Compact Output for Piping

```sh
# Use raw output for grep/awk processing
hl --raw app.log | grep "error"
```

### Audit Trails

```sh
# Hide sensitive fields from audit logs
hl --hide password --hide api-key --hide token audit.log
```

## Tips and Best Practices

- **Use configuration file** — Set frequently-used field visibility in your config file to avoid repeating `--hide` options.
- **Check field names** — Use `--raw` or view a few entries to see available field names.
- **Combine with queries** — Filter entries first, then manage field visibility for cleaner output.
- **Reveal when needed** — Use `--hide '!field'` to reveal specific fields or `--hide '!*'` to reveal all.
- **Nested fields** — Use dot notation for nested fields (e.g., `--hide user.email`).

## Troubleshooting

### Field Still Showing After Hide

If a field still appears after using `--hide`:

- Verify the field name is spelled correctly (case-sensitive).
- Check your config file for conflicting settings.
- Use `--hide '!*'` to reveal all fields and verify the field name.

### Too Much/Too Little Shown

If output is too verbose or sparse:

- Use `--hide '!*'` to reveal all fields and see what's available.
- Then hide specific fields you don't need with `--hide field-name`.
- Configure defaults in your config file to avoid repeating options.

### Nested Fields Not Hiding

For nested fields:

- Use dot notation: `--hide user.email`
- Check the actual field structure with `--raw` or `--hide '!*'`

## Next Steps

- [Field Expansion](../features/field-expansion.md) — Control how complex fields are displayed.
- [Output Formatting](../features/formatting.md) — Customize overall output format.
- [Filtering Examples](filtering.md) — Filter entries before managing field display.
