# Field Visibility

Field visibility controls allow you to hide or reveal specific fields in log entries, helping you focus on what matters and reduce visual clutter.

## Basic Usage

Use the `-h` or `--hide` option to control field visibility:

```sh
# Hide a specific field
hl -h metadata application.log

# Hide multiple fields
hl -h headers -h metadata application.log
```

## Hiding Fields

### Single Field

```sh
# Hide the 'headers' field
hl -h headers application.log
```

### Multiple Fields

```sh
# Hide several fields
hl -h headers -h metadata -h debug_info application.log
```

### Nested Fields

```sh
# Hide nested field
hl -h request.headers application.log

# Hide deeply nested field
hl -h user.profile.preferences application.log
```

## Revealing Fields

Use the `!` prefix to reveal fields when others are hidden:

### Hide All Except Specific Fields

```sh
# Hide all fields, show only message and level
hl -h '*' -h '!message' -h '!level' application.log

# Show only specific fields
hl -h '*' -h '!message' -h '!level' -h '!time' application.log
```

### Reveal Nested Field

```sh
# Hide headers but show content-type
hl -h headers -h '!headers.content-type' application.log

# Hide request but show method and path
hl -h request -h '!request.method' -h '!request.path' application.log
```

## Wildcard Patterns

### Hide All Fields

```sh
# Hide all custom fields (keeps standard fields like time, level, message)
hl -h '*' application.log
```

### Pattern Matching

```sh
# Hide all fields starting with 'debug'
hl -h 'debug*' application.log

# Hide all fields containing 'internal'
hl -h '*internal*' application.log
```

## Common Patterns

### Minimal Output

Show only essential fields:

```sh
# Only time, level, and message
hl -h '*' -h '!time' -h '!level' -h '!message' application.log
```

### Hide Verbose Fields

```sh
# Hide noisy fields
hl -h headers -h body -h metadata application.log

# Hide debug fields
hl -h 'debug*' -h 'trace*' application.log
```

### Focus on Errors

```sh
# Show errors with minimal fields
hl -l e -h '*' -h '!message' -h '!error' -h '!stack' application.log
```

### API Monitoring

```sh
# Show only request/response info
hl -h '*' \
   -h '!method' \
   -h '!path' \
   -h '!status' \
   -h '!duration' \
   application.log
```

## Hiding Empty Fields

Use the `-e` or `--hide-empty-fields` flag to automatically hide fields with empty values:

```sh
# Hide null, empty strings, empty objects, and empty arrays
hl -e application.log

# Combine with field hiding
hl -e -h metadata application.log
```

### Show Empty Fields

Use `-E` or `--show-empty-fields` to override config defaults:

```sh
# Explicitly show empty fields
hl -E application.log
```

## Nested Field Visibility

### Hide Parent, Show Child

```sh
# Hide entire request object except specific fields
hl -h request -h '!request.method' -h '!request.path' application.log
```

### Partial Object Display

```sh
# Hide some nested fields, keep others
hl -h user.profile -h user.settings application.log
```

For JSON like:
```json
{
  "user": {
    "id": 123,
    "name": "Alice",
    "profile": {...},
    "settings": {...}
  }
}
```

This shows `user.id` and `user.name` but hides `user.profile` and `user.settings`.

## Array Field Visibility

```sh
# Hide array field
hl -h tags application.log

# Hide nested array
hl -h user.roles application.log
```

## Use Cases

### Debugging Specific Issues

```sh
# Focus on error details
hl -l e -h '*' -h '!message' -h '!error' -h '!stack' application.log

# Database query debugging
hl -h '*' -h '!query' -h '!duration' -h '!rows' application.log
```

### Performance Analysis

```sh
# Show only timing information
hl -h '*' \
   -h '!time' \
   -h '!duration' \
   -h '!method' \
   -h '!path' \
   application.log
```

### Security Audit

```sh
# Show authentication/authorization fields
hl -h '*' \
   -h '!user' \
   -h '!ip' \
   -h '!action' \
   -h '!result' \
   application.log
```

### Request Tracing

```sh
# Show trace information
hl -h '*' \
   -h '!trace_id' \
   -h '!span_id' \
   -h '!service' \
   -h '!message' \
   application.log
```

### HTTP API Logs

```sh
# Essential HTTP info
hl -h headers -h body \
   -h '!method' \
   -h '!path' \
   -h '!status' \
   -h '!duration' \
   application.log
```

## Combining with Other Options

### With Level Filtering

```sh
# Show only essential error fields
hl -l e -h '*' -h '!message' -h '!error' application.log
```

### With Time Range

```sh
# Recent logs with minimal fields
hl --since -1h -h headers -h metadata application.log
```

### With Field Filtering

```sh
# Specific service with minimal output
hl -f service=api -h '*' -h '!message' -h '!status' application.log
```

### With Queries

```sh
# Slow requests with timing info only
hl -q 'duration > 1.0' -h '*' -h '!duration' -h '!path' application.log
```

## Configuration File

Set default hidden fields in your config file:

```toml
# ~/.config/hl/config.toml
hide = ["headers", "metadata", "debug_info"]
hide-empty-fields = true
```

Or in YAML:

```yaml
# ~/.config/hl/config.yaml
hide:
  - headers
  - metadata
  - debug_info
hide-empty-fields: true
```

## Environment Variables

```sh
# Set default hidden fields
export HL_HIDE="headers,metadata"

# Hide empty fields by default
export HL_HIDE_EMPTY_FIELDS=true
```

## Field Visibility vs Field Expansion

These are independent controls:

```sh
# Hide fields (controls which fields appear)
hl -h headers application.log

# Field expansion (controls how fields are displayed)
hl -x always application.log

# Combine both
hl -h metadata -x inline application.log
```

See [Field Expansion](./field-expansion.md) for details on `-x` option.

## Performance Impact

Hiding fields has minimal performance impact:

- Fields are hidden during output formatting
- Parsing still processes all fields
- No performance benefit from hiding fields
- Use filtering (`-f`, `-q`) to reduce parsing

```sh
# Hiding doesn't improve performance
hl -h headers application.log

# Filtering improves performance
hl -f 'headers!~=' application.log  # Skip entries with headers
```

## Tips and Best Practices

1. **Start with `-h '*'` then reveal**:
   ```sh
   hl -h '*' -h '!message' -h '!level' application.log
   ```

2. **Hide verbose fields for readability**:
   ```sh
   hl -h headers -h body application.log
   ```

3. **Use patterns for related fields**:
   ```sh
   hl -h 'debug*' -h 'internal*' application.log
   ```

4. **Configure defaults in config file**:
   ```toml
   hide = ["headers", "metadata"]
   ```

5. **Combine with empty field hiding**:
   ```sh
   hl -e -h metadata application.log
   ```

6. **Override config with command line**:
   ```sh
   # Config hides headers, but show them for this run
   hl -h '!headers' application.log
   ```

## Examples by Use Case

### Development

```sh
# Minimal debug output
hl -l d -h '*' -h '!message' -h '!file' -h '!line' application.log
```

### Production Monitoring

```sh
# Essential production fields
hl -h '*' \
   -h '!time' \
   -h '!level' \
   -h '!service' \
   -h '!message' \
   application.log
```

### Error Investigation

```sh
# Error context only
hl -l e -h '*' \
   -h '!message' \
   -h '!error' \
   -h '!stack' \
   -h '!request.id' \
   application.log
```

### Performance Tuning

```sh
# Performance metrics only
hl -q 'duration > 0.1' -h '*' \
   -h '!method' \
   -h '!path' \
   -h '!duration' \
   -h '!db_queries' \
   application.log
```

## Troubleshooting

### Field Still Showing

If a field you tried to hide still appears:

1. **Check exact field name**:
   ```sh
   hl --raw application.log | head
   ```

2. **Check for nested structure**:
   ```sh
   # May need to hide nested field
   hl -h request.headers application.log
   ```

3. **Check wildcard pattern**:
   ```sh
   # Pattern might not match
   hl -h 'meta*' application.log  # Won't match 'metadata'
   ```

### All Fields Hidden

If all fields disappeared:

1. **Check for conflicting hide rules**:
   ```sh
   # Remove -h '*' or add reveals
   hl -h '*' -h '!message' application.log
   ```

2. **Check config file**:
   ```sh
   cat ~/.config/hl/config.toml
   ```

### Field Not Found

If revealing a field doesn't work:

1. **Verify field exists**:
   ```sh
   hl --raw application.log | grep "field_name"
   ```

2. **Check exact casing**:
   ```sh
   # Field names are case-sensitive
   hl -h '!Message' application.log  # Different from 'message'
   ```

## Limitations

1. **Cannot hide standard fields** - `time`, `level`, `message` always show (can be hidden with `-h '*'` and selective reveal)

2. **Pattern matching is simple** - Use `*` for wildcards, not regex

3. **Reveal requires prior hide** - `!field` only works if field was hidden

4. **No conditional hiding** - Cannot hide based on field value (use filtering for that)

## When to Use Field Hiding

### Use Field Hiding When:
- ✓ Reducing visual clutter
- ✓ Focusing on specific fields
- ✓ Creating cleaner reports
- ✓ Hiding sensitive data from display
- ✓ Simplifying output for presentations

### Use Filtering When:
- ✓ Excluding entries based on field values
- ✓ Improving performance
- ✓ Reducing data volume
- ✓ Finding specific log entries

## Related

- [Field Expansion](./field-expansion.md) - Control how fields are displayed
- [Filtering by Field Values](./filtering-fields.md) - Filter entries by field values
- [Output Formatting](./formatting.md) - General output options
- [Field Management Examples](../examples/field-management.md) - Real-world scenarios