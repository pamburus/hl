# Field Expansion

Field expansion controls how `hl` displays nested objects, arrays, and complex field values in the formatted output.

## Overview

When log entries contain fields with multi-line values (such as stack traces or error details), `hl` can display them in different ways:

- **Auto** — expand only fields with multi-line values into indented blocks, keep single-line fields inline
- **Never** — keep everything on a single line, escape newlines as `\n`
- **Always** — display each field on its own indented line, expand multi-line values
- **Inline** — preserve actual newlines in multi-line values, surrounded by backticks

## Configuration

| Method | Setting |
|--------|---------|
| Config file | [`formatting.expansion.mode`](../customization/config-files.md#formatting-expansion-mode) |
| CLI option | [`-x, --expansion`](../reference/options.md#expansion) |
| Environment | [`HL_EXPANSION`](../customization/environment.md#hl-expansion) |

**Values:** `never`, `inline`, `auto` (default), `always`

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

`--expansion never` keeps all fields on a single line, escaping newlines and tabs:

```
10:30:45 [ERR] database connection failed › attempt=3 error="connection refused\n\tretry 1/3\n\tretry 2/3"
```

This is most compact but can be hard to read when fields contain multi-line values.

### Auto

`--expansion auto` expands only fields with multi-line values, keeping single-line fields inline:

```
10:30:45 [ERR] database connection failed › attempt=3
         [ ~ ]   > error=|=>
         [ ~ ]      	connection refused
         [ ~ ]      		retry 1/3
         [ ~ ]      		retry 2/3
```

This is the default mode, providing a good balance between compactness and readability.

### Always

`--expansion always` displays each field on its own line, regardless of content:

```
10:30:45 [ERR] database connection failed
         [ ~ ]   > attempt=3
         [ ~ ]   > error=|=>
         [ ~ ]      	connection refused
         [ ~ ]      		retry 1/3
         [ ~ ]      		retry 2/3
```

This mode maximizes readability but produces more verbose output.

### Inline

`--expansion inline` shows multi-line values surrounded by backticks, preserving actual newlines:

```
10:30:45 [ERR] database connection failed › attempt=3 error=`connection refused
	retry 1/3
	retry 2/3`
```

This legacy mode (default prior to v0.35.0) is convenient for selecting and copying multi-line values in the terminal.

## How Expansion Works

### Multi-line Field Values

The primary purpose of expansion is handling multi-line string values (containing newlines or tabs), such as stack traces or error details.

With `--expansion never`, newlines and tabs are escaped as `\n` and `\t`:

```
10:30:45 [ERR] connection failed › error="dial tcp 10.0.0.5:5432: connection refused\n\tretry 1/3 failed\n\tretry 2/3 failed"
```

With `--expansion inline`, multi-line values are shown with backticks, preserving actual newlines:

```
10:30:45 [ERR] connection failed › error=`dial tcp 10.0.0.5:5432: connection refused
	retry 1/3 failed
	retry 2/3 failed`
```

With `--expansion auto` or `--expansion always`, multi-line values are expanded into indented blocks marked with `|=>`:

```
10:30:45 [ERR] connection failed
         [ ~ ]   > error=|=>
         [ ~ ]      	dial tcp 10.0.0.5:5432: connection refused
         [ ~ ]      		retry 1/3 failed
         [ ~ ]      		retry 2/3 failed
```

The `[ ~ ]` indicator replaces the level badge on continuation lines, and each line of the value is prefixed with a tab character for consistent indentation.

### Field Placement

With `--expansion always`, all fields appear on separate lines regardless of whether they contain multi-line values:

```
10:30:45 [INF] request completed
         [ ~ ]   > method=GET
         [ ~ ]   > path=/api/users
         [ ~ ]   > status=200
         [ ~ ]   > latency-ms=42
```

With `--expansion auto`, only fields with multi-line values trigger expansion. Single-line fields remain inline:

```
10:30:45 [INF] request completed › method=GET path=/api/users status=200 latency-ms=42
```

### Multi-line Messages

Multi-line messages (e.g., stack traces in the message field) are also handled by expansion.

With `--expansion inline`, multi-line messages are shown directly, which may break visual alignment:

```
10:30:45 [ERR] panic: runtime error
	goroutine 1 [running]:
	main.main() › request-id=abc-123
```

With `--expansion never`, newlines are escaped:

```
10:30:45 [ERR] "panic: runtime error\n\tgoroutine 1 [running]:\n\tmain.main()" › request-id=abc-123
```

With `--expansion auto`, the message is expanded separately from inline fields:

```
10:30:45 [ERR] › request-id=abc-123
         [ ~ ]   > msg=|=>
         [ ~ ]      	panic: runtime error
         [ ~ ]      		goroutine 1 [running]:
         [ ~ ]      		main.main()
```

With `--expansion always`, the message placeholder `~` appears where the message would normally be, and all fields are expanded:

```
10:30:45 [ERR] ~
         [ ~ ]   > msg=|=>
         [ ~ ]      	panic: runtime error
         [ ~ ]      		goroutine 1 [running]:
         [ ~ ]      		main.main()
         [ ~ ]   > request-id=abc-123
```

## Interaction with Field Flattening

Field expansion interacts with the `--flatten` option, which controls how nested objects are displayed.

With `--flatten always` (default), nested objects become dot-notation keys:

```
10:30:45 [INF] user logged in › user.id=42 user.name=alice
```

```
10:30:45 [INF] user logged in
         [ ~ ]   > user.id=42
         [ ~ ]   > user.name=alice
```

With `--flatten never`, nested objects are preserved as hierarchical structures:

```
10:30:45 [INF] user logged in › user={ id=42 name=alice }
```

```
10:30:45 [INF] user logged in
         [ ~ ]   > user:
         [ ~ ]     > id=42
         [ ~ ]     > name=alice
```

Note: Arrays are always displayed inline (e.g., `tags=[admin user]`) regardless of expansion mode.

See [Field Visibility](./field-visibility.md) for more on flattening.

## Related Topics

- [Output Formatting](./formatting.md) — overview of formatting options
- [Field Visibility](./field-visibility.md) — controlling which fields are shown
- [Raw Output](./raw-output.md) — outputting original JSON
- [Configuration Files](../customization/config-files.md#formatting-expansion-mode) — persistent configuration
