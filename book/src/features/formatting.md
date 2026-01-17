# Output Formatting

`hl` provides extensive control over how log entries are displayed, allowing you to customize the output to match your needs and preferences.

## Overview

Output formatting in `hl` covers several aspects:

- **Field visibility** — choosing which fields to show or hide
- **Time display** — formatting timestamps in various ways
- **Field expansion** — controlling how nested objects and arrays are displayed
- **Raw output** — outputting original source entries instead of formatted output
- **Colors and themes** — visual styling and color schemes
- **Field value formatting** — controlling how values are escaped and displayed

## Quick Examples

```bash
# Hide verbose fields
hl --hide timestamp --hide host app.log

# Show only specific fields
hl --hide '*' --hide '!level' --hide '!message' app.log

# Use local timezone
hl --local app.log

# Expand nested objects
hl --expansion always app.log

# Output raw JSON
hl --raw app.log

# Use a different theme
hl --theme universal app.log
```

## Field Visibility

Control which fields appear in the output using the `--hide` (or `-h`) option:

```bash
# Hide specific fields
hl --hide timestamp --hide host --hide pid app.log

# Hide all fields except specific ones
hl --hide '*' --hide '!level' --hide '!message' --hide '!service' app.log
```

Field visibility rules:
- `--hide field` hides the field
- `--hide '!field'` reveals/shows the field (overrides previous hides)
- `--hide '*'` hides all fields (use with reveals to show only specific fields)
- Rules are processed in order

See [Field Visibility](./field-visibility.md) for detailed behavior and examples.

## Time Display

Customize how timestamps are displayed:

```bash
# Use local timezone instead of UTC
hl --local app.log

# Custom time format
hl --time-format '%Y-%m-%d %H:%M:%S' app.log

# Specific timezone
hl --time-zone 'America/New_York' app.log
```

Default format: `%Y-%m-%d %T.%3N` (e.g., `2024-01-15 10:30:45.123`)

See [Time Display](./time-display.md) for format specifications and timezone handling.

## Field Expansion

Control how nested objects and arrays are displayed:

```bash
# Never expand (keep nested objects inline)
hl --expansion never app.log

# Always expand (show nested objects on separate lines)
hl --expansion always app.log

# Expand inline when short enough
hl --expansion inline app.log

# Auto mode (context-dependent)
hl --expansion auto app.log
```

Expansion affects readability of complex nested structures.

See [Field Expansion](./field-expansion.md) for detailed behavior.

## Raw Output

Output the original JSON source instead of formatted output:

```bash
# Raw mode
hl --raw app.log

# Raw mode with filters (filters still apply)
hl --raw --level error --query '.user_id=123' app.log

# Disable raw mode (if enabled in config)
hl --no-raw app.log
```

Raw mode is useful for:
- Piping to other tools that expect JSON
- Preserving exact original format
- Re-processing filtered results

See [Raw Output](./raw-output.md) for more details.

## Field Value Formatting

Control how field values are displayed:

```bash
# Show raw field values without unescaping
hl --raw-fields app.log
```

By default, `hl` prettifies and unescapes field values for better readability. Use `--raw-fields` to see the exact values as they appear in the source.

## Colors and Themes

Control color usage and visual styling:

```bash
# Force colors even when piping
hl --color always app.log

# Disable colors
hl --color never app.log

# Use a different theme
hl --theme monokai app.log

# List available themes
hl --list-themes
```

See [Themes](../customization/themes.md) for available themes and customization options.

## Empty Field Handling

Control whether empty fields are displayed:

```bash
# Hide fields with null, empty string, empty object, or empty array values
hl --hide-empty-fields app.log

# Show empty fields (default)
hl --show-empty-fields app.log
```

This is useful for reducing clutter in logs with many optional fields.

## Input Info Display

When processing multiple files, show which file each entry came from:

```bash
# No input info
hl --input-info none *.log

# Minimal (just filename)
hl --input-info minimal *.log

# Compact (file number and name)
hl --input-info compact *.log

# Full (full path)
hl --input-info full *.log
```

Default: `none` for single file, `minimal` for multiple files.

See [Multiple Files](./multiple-files.md) for more details.

## Object Flattening

Control whether nested objects are flattened into dot-notation fields:

```bash
# Never flatten
hl --flatten never app.log

# Always flatten (default)
hl --flatten always app.log
```

Example:
```json
{"user": {"id": 123, "name": "Alice"}}
```

- With `--flatten always`: displayed as `user.id: 123, user.name: Alice`
- With `--flatten never`: displayed as `user: {id: 123, name: Alice}`

## ASCII-Only Mode

Restrict punctuation to ASCII characters only (useful for terminals with limited Unicode support):

```bash
# Force ASCII
hl --ascii always app.log

# Auto-detect based on terminal capabilities
hl --ascii auto app.log

# Never restrict (use Unicode box-drawing characters)
hl --ascii never app.log
```

Default: `auto` (uses Unicode if terminal supports it)

## Combining Formatting Options

All formatting options can be combined:

```bash
# Highly customized output
hl --hide '*' \
   --hide '!timestamp' --hide '!level' --hide '!message' \
   --local \
   --time-format '%H:%M:%S' \
   --hide-empty-fields \
   --theme universal \
   --expansion inline \
   app.log
```

## Configuration Files

All formatting options can be saved in configuration files to avoid repeating them:

```toml
# ~/.config/hl/config.toml
time-zone = "local"
time-format = "%Y-%m-%d %H:%M:%S"
theme = "monokai"
hide-empty-fields = true
hide = ["timestamp", "host", "pid"]
```

See [Configuration Files](../customization/config-files.md) for details.

## Environment Variables

Many formatting options can be set via environment variables:

```bash
export HL_TIME_ZONE=local
export HL_THEME=universal
export HL_HIDE_EMPTY_FIELDS=true

hl app.log
```

See [Environment Variables](../customization/environment.md) for the complete list.

## Examples

### Minimal Clean Output

```bash
# Show only level, time, and message
hl --hide '*' \
   --hide '!level' --hide '!timestamp' --hide '!message' \
   --hide-empty-fields \
   app.log
```

### Development-Friendly Format

```bash
# Local time, expanded objects, no empty fields
hl --local \
   --time-format '%T.%3N' \
   --expansion always \
   --hide-empty-fields \
   app.log
```

### Production Monitoring

```bash
# UTC time, compact format, show source files
hl --time-format '%Y-%m-%d %T' \
   --expansion never \
   --input-info compact \
   /var/log/service-*.log
```

### JSON Pipeline

```bash
# Filter and output raw JSON for further processing
hl --raw --level error --query '.service=api' app.log | jq '.message'
```

## Related Topics

- [Field Visibility](./field-visibility.md) — controlling which fields are shown
- [Time Display](./time-display.md) — timestamp formatting and timezones
- [Field Expansion](./field-expansion.md) — nested object display
- [Raw Output](./raw-output.md) — outputting original JSON
- [Themes](../customization/themes.md) — color schemes and styling
- [Configuration Files](../customization/config-files.md) — saving preferences