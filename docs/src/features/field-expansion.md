# Field Expansion

Field expansion controls how `hl` displays nested objects, arrays, and complex field values in the formatted output.

## Overview

When log entries contain many custom fields and nested objects, `hl` can display them in different ways:

- **Auto** — keep each entry on a single line but expand fields with multi-line values using consistent indentation
- **Never expand** — always keep each entry on a single line
- **Always expand** — display each field on its own indented line and expand all nested objects
- **Inline** — keep each entry on a single line but show multi-line values as raw data surrounded by backticks

## Enabling Field Expansion

Use the `--expansion` (or `-x`) option:

```sh
# Expand only multi-line fields using consistent indentation
hl --expansion auto app.log

# Never expand fields, keep each entry on a single line
hl --expansion never app.log

# Always expand all fields into multi-line format
hl --expansion always app.log

# Show multi-line values as raw data surrounded by backticks (legacy mode)
hl --expansion inline app.log
```

**Default:** `auto`

## Expansion Modes

### Never

`--expansion never` keeps all fields on a single line:

```sh
hl --expansion never app.log
```

Example output:
```
2024-01-15 10:30:45.123 [ERR] user registration failed › user.id=123 user.name=Alice user.roles=[admin user] error="failed to register user\nfailed to connect to database\n\tuser.go:123"
```

This is most compact but can be hard to read for complex structures.

### Auto

`--expansion auto` automatically expands multi-line fields with consistent indentation:

```sh
hl --expansion auto app.log
```

Example output:
```
2024-01-15 10:30:45.123 [ERR] user registration failed › user.id=123 user.name=Alice user.roles=[admin user]
                        [ ~ ]   > error=|=>
                        [ ~ ]       failed to register user
                        [ ~ ]       failed to connect to database
                        [ ~ ]           user.go:123
```

This is the default mode and works well for most use cases.

### Always

`--expansion always` expands all nested structures:

```sh
hl --expansion always app.log
```

Example output:
```
2024-01-15 10:30:45.123 [ERR] user registration failed
                        [ ~ ]   > user.id=123
                        [ ~ ]   > user.name=Alice
                        [ ~ ]   > user.roles=[admin user]
                        [ ~ ]   > error=|=>
                        [ ~ ]       failed to register user
                        [ ~ ]       failed to connect to database
                        [ ~ ]           user.go:123
```

Even simple objects are expanded, which can make output very verbose but maximizes readability.

### Inline

`--expansion inline` shows multi-line values as raw data surrounded by backticks, preserving newlines:

```sh
hl --expansion inline app.log
```

Example output:
```
2024-01-15 10:30:45.123 [ERR] user registration failed › user.id=123 user.name=Alice user.roles=[admin user] error=`failed to register user
failed to connect to database
        user.go:123`
```

This is legacy behavior prior to v0.35.0 and is useful for preserving original formatting.
Can be convenient for selecting and copying multi-line values in the terminal, but not as readable as expanded formats.

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

```sh
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

```sh
# See full structure of complex log entries
hl --expansion always -q 'exists(error-details)' app.log
```

### Monitoring Production Logs

For high-volume production monitoring, use `--expansion never` or `inline`:

```sh
# Compact output for quick scanning
hl -F --expansion never --level error /var/log/app.log
```

### Development and Testing

During development, `auto` or `inline` modes provide good readability:

```sh
# Balanced view for development
hl --expansion inline --local app.log
```

### Pipeline Processing

When piping to other tools, use `--raw` instead of controlling expansion:

```sh
# Use raw mode for JSON pipelines
hl --raw --level error app.log | jq '.user.id'
```

## Examples

### Compare Expansion Modes

```sh
# Same log file, different expansion modes
hl --expansion never app.log > never.txt
hl --expansion inline app.log > inline.txt
hl --expansion always app.log > always.txt

# Compare the outputs
diff never.txt always.txt
```

### Selective Expansion

Combine with field hiding to expand only specific fields:

```sh
# Hide most fields, expand only error details
hl --hide '*' \
   --hide '!level' --hide '!timestamp' --hide '!error' \
   --expansion always \
   -q 'exists(error)' \
   app.log
```

### Compact Production View

```sh
# Minimal expansion for production monitoring
hl -F \
   --expansion never \
   --hide-empty-fields \
   --level warn \
   /var/log/service-*.log
```

### Development Deep Dive

```sh
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

```sh
export HL_EXPANSION=inline
hl app.log
```

## Related Topics

- [Output Formatting](./formatting.md) — overview of formatting options
- [Field Visibility](./field-visibility.md) — controlling which fields are shown
- [Raw Output](./raw-output.md) — outputting original JSON
- [Configuration Files](../customization/config-files.md) — saving preferences
