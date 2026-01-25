# Field Visibility

Field visibility controls allow you to hide or reveal specific fields in log entries, helping you focus on what matters and reduce visual clutter.

## Basic Usage

Use the `-h` or `--hide` option to control field visibility:

```sh
# Hide a specific field
hl -h metadata app.log

# Hide multiple fields
hl -h headers -h metadata app.log
```

## Hiding Fields

### Single Field

```sh
# Hide the 'headers' field
hl -h headers app.log
```

### Multiple Fields

```sh
# Hide several fields
hl -h headers -h metadata -h debug_info app.log
```

### Nested Fields

```sh
# Hide nested field
hl -h request.headers app.log

# Hide deeply nested field
hl -h user.profile.preferences app.log
```

## Revealing Fields

Use the `!` prefix to reveal fields when others are hidden:

### Hide All Except Specific Fields

```sh
# Hide all fields, show only method and url
hl -h '*' -h '!method' -h '!url' app.log
```

### Reveal Nested Field

```sh
# Hide headers but show content-type
hl -h headers -h '!headers.content-type' app.log

# Hide request but show method and path
hl -h request -h '!request.method' -h '!request.path' app.log
```

## Wildcard Patterns

### Hide All Fields

```sh
# Hide all custom fields (keeps standard fields like time, level, message)
hl -h '*' app.log
```

### Pattern Matching

```sh
# Hide all fields starting with 'debug'
hl -h 'debug*' app.log

# Hide all fields containing 'internal'
hl -h '*internal*' app.log
```

## Common Patterns

### Minimal Output

Show only essential fields:

```sh
# Only method and url
hl -h '*' -h '!method' -h '!url' app.log
```

### Hide Verbose Fields

```sh
# Hide noisy fields
hl -h headers -h body -h metadata app.log

# Hide debug fields
hl -h 'debug-*' -h 'trace-*' app.log
```

### Focus on Errors

```sh
# Show errors with minimal fields
hl -l e -h '*' -h '!error' -h '!stack' app.log
```

### API Monitoring

```sh
# Show only request/response info
hl -h '*' \
   -h '!method' \
   -h '!url' \
   -h '!status' \
   -h '!duration' \
   app.log
```

## Hiding Empty Fields

Use the `-e` or `--hide-empty-fields` flag to automatically hide fields with empty values:

```sh
# Hide null, empty strings, empty objects, and empty arrays
hl -e app.log

# Combine with field hiding
hl -e -h metadata app.log
```

### Show Empty Fields

Use `-E` or `--show-empty-fields` to override config defaults:

```sh
# Explicitly show empty fields
hl -E app.log
```

## Nested Field Visibility

### Hide Parent, Show Child

```sh
# Hide entire request object except specific fields
hl -h request -h '!request.method' -h '!request.url' app.log
```

### Partial Object Display

```sh
# Hide some nested fields, keep others
hl -h user.profile -h user.settings app.log
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
hl -h tags app.log

# Hide nested array
hl -h user.roles app.log
```

## Use Cases

### Debugging Specific Issues

```sh
# Focus on error details
hl -l e -h '*' -h '!error' -h '!stack' app.log

# Database query debugging
hl -h '*' -h '!query' -h '!duration' -h '!rows' app.log
```

### Performance Analysis

```sh
# Show only timing information
hl -h '*' \
   -h '!time' \
   -h '!duration' \
   -h '!method' \
   -h '!url' \
   app.log
```

### Security Audit

```sh
# Show authentication/authorization fields
hl -h '*' \
   -h '!user' \
   -h '!ip' \
   -h '!action' \
   -h '!result' \
   app.log
```

### Request Tracing

```sh
# Show trace information
hl -h '*' \
   -h '!trace.id' \
   -h '!span.id' \
   -h '!service' \
   app.log
```

### HTTP API Logs

```sh
# Essential HTTP info
hl -h headers -h body \
   -h '!method' \
   -h '!url' \
   -h '!status' \
   -h '!duration' \
   app.log
```

## Combining with Other Options

### With Level Filtering

```sh
# Show only essential error fields
hl -l e -h '*' -h '!error' app.log
```

### With Time Range

```sh
# Recent logs with minimal fields
hl --since -1h -h headers -h metadata app.log
```

### With Field Filtering

```sh
# Specific service with minimal output
hl -f service=api -h '*' -h '!message' -h '!status' app.log
```

### With Queries

```sh
# Slow requests with timing info only
hl -q 'duration > 1.0' -h '*' -h '!duration' -h '!path' app.log
```

## Configuration File

Set default hidden fields in your config file:

```toml
# ~/.config/hl/config.toml
hide = ["headers", "metadata", "debug-info"]
```

Or in YAML:

```yaml
# ~/.config/hl/config.yaml
hide:
  - headers
  - metadata
  - debug-info
```

## Environment Variables

```sh
# Hide empty fields by default
export HL_HIDE_EMPTY_FIELDS=true
```

## Performance Impact

Hiding fields has minimal performance impact:

- Fields are hidden during output formatting
- Parsing still processes all fields
- Minimal performance benefit from hiding fields

## Tips and Best Practices

1. **Start with `-h '*'` then reveal**:
   ```sh
   hl -h '*' -h '!method' -h '!url' app.log
   ```

2. **Hide verbose fields for readability**:
   ```sh
   hl -h headers -h body app.log
   ```

3. **Use patterns for related fields**:
   ```sh
   hl -h 'debug-*' -h 'internal-*' app.log
   ```

4. **Configure defaults in config file**:
   ```toml
   hide = ["headers", "metadata"]
   ```

5. **Combine with empty field hiding**:
   ```sh
   hl -e -h metadata app.log
   ```

6. **Override config with command line**:
   ```sh
   # Config hides headers, but show them for this run
   hl -h '!headers' app.log
   ```

## Examples by Use Case

### Development

```sh
# Minimal debug output
hl -l d -h '*' -h '!message' -h '!file' -h '!line' app.log
```

### Production Monitoring

```sh
# Essential production fields
hl -h '*' \
   -h '!service' \
   -h '!method' \
   -h '!url' \
   -h '!status' \
   app.log
```

### Error Investigation

```sh
# Error context only
hl -l e -h '*' \
   -h '!method' \
   -h '!error' \
   -h '!stack' \
   -h '!request.id' \
   app.log
```

### Performance Tuning

```sh
# Performance metrics only
hl -q 'duration > 0.1' -h '*' \
   -h '!method' \
   -h '!path' \
   -h '!duration' \
   -h '!db.queries' \
   app.log
```

## Troubleshooting

### Field Still Showing

If a field you tried to hide still appears:

1. **Check exact field name**:
   ```sh
   hl --raw app.log | head
   ```

2. **Check for nested structure**:
   ```sh
   # May need to hide nested field
   hl -h request.headers app.log
   ```

### All Fields Hidden

If all fields disappeared:

1. **Check for conflicting hide rules**:
   ```sh
   # Remove -h '*' or add reveals
   hl -h '*' -h '!method' app.log
   ```

2. **Check config file**:
   ```sh
   cat ~/.config/hl/config.toml
   ```

### Field Not Found

If revealing a field doesn't work:

1. **Verify field exists**:
   ```sh
   hl --raw app.log | grep "field-name"
   ```

2. **Check exact casing**:
   ```sh
   # Field names are case-sensitive
   hl -h '!Message' app.log  # Different from 'message'
   ```

## Limitations

1. **Cannot hide standard fields** - `time`, `level`, `message` are always shown (cannot be hidden with `-h '*'` and selective reveal)

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
- ✓ Reducing data volume
- ✓ Finding specific log entries

## Related

- [Field Expansion](./field-expansion.md) - Control how fields are displayed
- [Filtering by Field Values](./filtering-fields.md) - Filter entries by field values
- [Output Formatting](./formatting.md) - General output options
- [Field Management Examples](../examples/field-management.md) - Real-world scenarios
