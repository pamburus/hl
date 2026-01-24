# Field Expansion

Field expansion controls how `hl` displays nested objects, arrays, and complex field values in the formatted output.

## Overview

When log entries contain nested structures like objects or arrays, `hl` can display them in different ways:

- **Inline** — keep nested structures on the same line when they're short
- **Expanded** — display nested structures across multiple indented lines
- **Never expand** — always keep structures inline regardless of size
- **Always expand** — always break out nested structures
- **Auto** — let `hl` decide based on context

## Enabling Field Expansion

Use the `--expansion` (or `-x`) option:

```bash
# Never expand nested structures
hl --expansion never app.log

# Expand inline when short enough
hl --expansion inline app.log

# Auto mode (context-dependent)
hl --expansion auto app.log

# Always expand nested structures
hl --expansion always app.log
```

**Default:** `auto`

## Expansion Modes

### Never

`--expansion never` keeps all nested structures inline:

```bash
hl --expansion never app.log
```

Example output:
```
2024-01-15 10:30:45.123 INFO user: {id: 123, name: "Alice", roles: ["admin", "user"]}
```

This is most compact but can be hard to read for complex structures.

### Inline

`--expansion inline` expands structures only when they're short enough to fit comfortably inline:

```bash
hl --expansion inline app.log
```

Example output:
```
# Short structure stays inline
2024-01-15 10:30:45.123 INFO user: {id: 123, name: "Alice"}

# Long structure gets expanded
2024-01-15 10:30:45.124 INFO user:
  id: 123
  name: "Alice"
  email: "alice@example.com"
  roles: ["admin", "user", "developer"]
  metadata: {created: "2024-01-01", updated: "2024-01-15"}
```

This mode balances readability and compactness.

### Auto

`--expansion auto` is context-aware and makes intelligent decisions based on:

- Terminal width
- Entry complexity
- Field value types
- Overall entry size

```bash
hl --expansion auto app.log
```

Auto mode typically:
- Keeps simple objects inline
- Expands complex nested structures
- Adapts to your terminal width
- Considers the total line length

This is the default mode and works well for most use cases.

### Always

`--expansion always` expands all nested structures:

```bash
hl --expansion always app.log
```

Example output:
```
2024-01-15 10:30:45.123 INFO
  user:
    id: 123
    name: "Alice"
  action: "login"
```

Even simple objects are expanded, which can make output very verbose but maximizes readability.

## How Expansion Works

### Objects

JSON objects are expanded into indented key-value pairs:

Input:
```json
{"user": {"id": 123, "name": "Alice", "email": "alice@example.com"}}
```

With `--expansion always`:
```
user:
  id: 123
  name: "Alice"
  email: "alice@example.com"
```

With `--expansion never`:
```
user: {id: 123, name: "Alice", email: "alice@example.com"}
```

### Arrays

Arrays are expanded into indented lists:

Input:
```json
{"tags": ["important", "security", "audit"]}
```

With `--expansion always`:
```
tags:
  - important
  - security
  - audit
```

With `--expansion never`:
```
tags: ["important", "security", "audit"]
```

### Nested Structures

Deeply nested structures are expanded recursively:

Input:
```json
{
  "user": {
    "id": 123,
    "profile": {
      "name": "Alice",
      "contacts": {
        "email": "alice@example.com",
        "phone": "+1234567890"
      }
    }
  }
}
```

With `--expansion always`:
```
user:
  id: 123
  profile:
    name: "Alice"
    contacts:
      email: "alice@example.com"
      phone: "+1234567890"
```

## Interaction with Field Flattening

Field expansion interacts with the `--flatten` option:

```bash
# Flatten and never expand
hl --flatten always --expansion never app.log

# Don't flatten but expand
hl --flatten never --expansion always app.log
```

When `--flatten always` (default), nested objects are flattened into dot-notation before expansion rules are applied:

Input:
```json
{"user": {"id": 123, "name": "Alice"}}
```

With `--flatten always --expansion never`:
```
user.id: 123, user.name: "Alice"
```

With `--flatten never --expansion always`:
```
user:
  id: 123
  name: "Alice"
```

See [Field Visibility](./field-visibility.md) for more on flattening.

## Use Cases

### Debugging Complex Structures

When investigating detailed object structures, use `--expansion always`:

```bash
# See full structure of complex log entries
hl --expansion always -q 'exists(error-details)' app.log
```

### Monitoring Production Logs

For high-volume production monitoring, use `--expansion never` or `inline`:

```bash
# Compact output for quick scanning
hl -F --expansion never --level error /var/log/app.log
```

### Development and Testing

During development, `auto` or `inline` modes provide good readability:

```bash
# Balanced view for development
hl --expansion inline --local app.log
```

### Pipeline Processing

When piping to other tools, use `--raw` instead of controlling expansion:

```bash
# Use raw mode for JSON pipelines
hl --raw --level error app.log | jq '.user.id'
```

## Examples

### Compare Expansion Modes

```bash
# Same log file, different expansion modes
hl --expansion never app.log > never.txt
hl --expansion inline app.log > inline.txt
hl --expansion always app.log > always.txt

# Compare the outputs
diff never.txt always.txt
```

### Selective Expansion

Combine with field hiding to expand only specific fields:

```bash
# Hide most fields, expand only error details
hl --hide '*' \
   --hide '!level' --hide '!timestamp' --hide '!error' \
   --expansion always \
   -q 'exists(error)' \
   app.log
```

### Compact Production View

```bash
# Minimal expansion for production monitoring
hl -F \
   --expansion never \
   --hide-empty-fields \
   --level warn \
   /var/log/service-*.log
```

### Development Deep Dive

```bash
# Maximum detail for debugging
hl --expansion always \
   --local \
   --show-empty-fields \
   -q 'request-id = "abc-123"' \
   app.log
```

## Performance Considerations

Expansion mode has minimal performance impact:

- **Never/inline** — slightly faster (less formatting work)
- **Always** — slightly slower (more indentation and line breaks)
- **Auto** — adaptive (minimal overhead)

The difference is negligible for typical use cases. Choose based on readability needs, not performance.

## Configuration

Set default expansion mode in your config file:

```toml
# ~/.config/hl/config.toml
[formatting.expansion]
mode = "inline"
```

Or via environment variable:

```bash
export HL_EXPANSION=inline
hl app.log
```

## Related Topics

- [Output Formatting](./formatting.md) — overview of formatting options
- [Field Visibility](./field-visibility.md) — controlling which fields are shown
- [Raw Output](./raw-output.md) — outputting original JSON
- [Configuration Files](../customization/config-files.md) — saving preferences