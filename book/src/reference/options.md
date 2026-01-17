# Command-Line Options

This page provides a complete reference for all `hl` command-line options.

## Arguments

### `[FILE]...`

Files to process. If no files are specified, `hl` reads from standard input.

```/dev/null/example.sh#L1-2
# Read from file
hl application.log

# Read from multiple files
hl app1.log app2.log

# Read from stdin
kubectl logs my-pod | hl
```

## General Options

### `--config <FILE>`

Specify a custom configuration file path.

- **Environment variable**: `HL_CONFIG`
- **Example**: `hl --config ~/my-hl-config.yaml app.log`

See the [Configuration](../configuration/overview.md) section for details on configuration files.

### `--help[=<VERBOSITY>]`

Print help information.

- **Possible values**: `short`, `long`
- **Short form**: `--help` or `-h` (prints short help)
- **Long form**: `--help=long` (prints detailed help with descriptions)

```/dev/null/example.sh#L1-4
# Short help
hl --help

# Long help with detailed descriptions
hl --help=long
```

### `-V, --version`

Print version information.

```/dev/null/example.sh#L1
hl --version
```

## Sorting and Streaming Options

### `-s, --sort`

Sort log entries chronologically across all input files.

This option enables batch chronological sorting by building a timestamp index. Entries without recognized timestamps are discarded. The index includes timestamp ranges and level bitmasks, allowing very fast filtering by level.

```/dev/null/example.sh#L1-2
# Sort entries from multiple log files chronologically
hl --sort app1.log app2.log app3.log
```

See [Sorting](../features/sorting.md) for details.

### `-F, --follow`

Follow input streams and sort entries chronologically within the time window set by `--sync-interval-ms`.

This mode is designed for live log monitoring. It provides near-real-time output with sorting within a short time window.

```/dev/null/example.sh#L1-2
# Follow a log file with live updates
hl --follow application.log
```

See [Follow Mode](../features/follow-mode.md) for details.

### `--tail <N>`

Number of last entries to preload from each file in `--follow` mode.

- **Default**: `10`
- **Example**: `hl --follow --tail 50 app.log`

### `--sync-interval-ms <MILLISECONDS>`

Synchronization interval (in milliseconds) for live streaming mode enabled by `--follow`.

- **Default**: `100`
- **Example**: `hl --follow --sync-interval-ms 500 app.log`

This controls the time window within which entries are sorted in follow mode.

## Paging Options

### `--paging <WHEN>`

Control pager usage. The pager used is determined by the `HL_PAGER` or `PAGER` environment variables.

- **Environment variable**: `HL_PAGING`
- **Default**: `auto`
- **Possible values**: `auto`, `always`, `never`
  - `auto`: Use pager if output is a terminal and content exceeds screen size
  - `always`: Always use pager
  - `never`: Never use pager

```/dev/null/example.sh#L1-2
# Always use pager
hl --paging=always app.log
```

### `-P`

Shorthand alias for `--paging=never`. Overrides the `--paging` option.

```/dev/null/example.sh#L1-8
# Disable pager when outputting to terminal
hl -P app.log

# For streaming scenarios (input is piped, output to terminal)
kubectl logs -f my-pod | hl -P

# When piping hl's output, pager is auto-disabled
hl app.log | grep ERROR  # -P not needed here
```

**Note**: The pager only runs when `hl`'s **output** goes to a terminal. It auto-disables when output is piped or redirected.

## Filtering Options

### `-l, --level <LEVEL>`

Display only entries with log level greater than or equal to the specified level.

- **Environment variable**: `HL_LEVEL`
- **Common levels**: `trace`, `debug`, `info`, `warn`, `error`, `fatal`

```/dev/null/example.sh#L1-5
# Show only warnings and errors
hl --level warn app.log

# Using short form
hl -l error app.log
```

When used with `--sort`, the timestamp index's level bitmasks allow extremely fast filtering: file segments that don't contain the requested level(s) are skipped entirely without reading or parsing.

### `--since <TIME>`

Display entries with timestamp greater than or equal to the specified time.

The `--time-zone` and `--local` options are honored when parsing the time value.

```/dev/null/example.sh#L1-5
# Show entries since a specific time
hl --since "2024-01-15 10:00:00" app.log

# With timezone
hl --since "2024-01-15 10:00:00" --time-zone "America/New_York" app.log
```

### `--until <TIME>`

Display entries with timestamp less than or equal to the specified time.

The `--time-zone` and `--local` options are honored when parsing the time value.

```/dev/null/example.sh#L1-2
# Show entries until a specific time
hl --until "2024-01-15 18:00:00" app.log
```

### `-f, --filter <FILTER>`

Filter entries by matching field values using simple field matching expressions.

**Format**: `<key> <operator> <value>`

**Operators**:
- `=`: Exact string match
- `~=`: Substring match
- `~~=`: Regular expression match

**Modifiers**:
- `!`: Negate the match (placed before operator): `k!=v`, `k!~=v`, etc.
- `?`: Include entry if the field is missing (placed after the key): `k?=v`, `k?!~=v`, etc.

```/dev/null/example.sh#L1-8
# Exact match
hl -f 'status=200' app.log

# Substring match
hl -f 'message~=timeout' app.log

# Regex match
hl -f 'user~~=^admin' app.log

# Negation
hl -f 'status!=200' app.log

# Multiple filters (AND logic)
hl -f 'status=500' -f 'method=POST' app.log
```

For complex filtering with boolean logic, comparisons, and more, use `--query` instead.

### `-q, --query <QUERY>`

Filter entries using complex query expressions.

Query expressions support:
- **All operators and modifiers from `--filter`**
- **Logical operators**: `and`, `or`, `not` (aliases: `&&`, `||`, `!`)
- **Comparison operators**: `<`, `>`, `<=`, `>=`, `=`, `!=`
- **Set membership**: `status in (500,503,504)`, `id in @ids.txt`, `id in @-`
- **String operations**: `message contains "timeout"`, `message matches "^Error.*timeout$"`
- **Existence checks**: `exists(user-id)`, `not exists(user-id)`
- **Grouping**: `(status>=500 and status<=504) or status==404`

```/dev/null/example.sh#L1-11
# Logical operators
hl -q 'status>=400 or duration>=15' app.log

# Comparison
hl -q 'status>=500 and status<600' app.log

# Set membership
hl -q 'status in (500,503,504)' app.log

# String operations
hl -q 'message contains "error" and level="error"' app.log

# Existence check
hl -q 'exists(user-id) and status>=400' app.log

# Complex expression with grouping
hl -q '(status>=500 and status<=504) or (status==404 and path contains "/api")' app.log
```

See [Query Syntax](./query-syntax.md) for complete syntax details.

## Output Options

### `--color [<WHEN>]`

Control ANSI color and style output.

- **Environment variable**: `HL_COLOR`
- **Default**: `auto`
- **Possible values**: `auto`, `always`, `never`
  - `auto`: Use colors if output is a terminal
  - `always`: Always use colors
  - `never`: Never use colors

```/dev/null/example.sh#L1-2
# Force colors even when piping
hl --color=always app.log | less -R
```

### `-c`

Shorthand alias for `--color=always`. Overrides the `--color` option.

### `--theme <THEME>`

Specify the color theme to use.

- **Environment variable**: `HL_THEME`
- **Default**: `hl-dark`

Run `hl --list-themes` to see all available themes.

```/dev/null/example.sh#L1-5
# Use a specific theme
hl --theme hl-light app.log

# List available themes
hl --list-themes
```

See [Themes](../configuration/themes.md) for details on themes and customization.

### `-r, --raw`

Output raw source entries instead of formatted entries.

This outputs the original JSON or logfmt for matching entries. Filtering still applies, but the output is in the original format rather than `hl`'s formatted representation.

```/dev/null/example.sh#L1-5
# Output raw JSON for matching entries
hl --raw -q 'status>=500' app.log

# Combine with --input-info json for JSON-only output
# (useful for strict JSON processors)
hl -r --input-info json -q 'status>=500' app.log | jq '.status'
```

See [Raw Output](../features/raw-output.md) for details.

### `--no-raw`

Disable raw output. Overrides the `--raw` option.

### `--raw-fields`

Output field values as-is, without unescaping or prettifying.

This is useful when you need the exact original field values without any transformations.

### `-h, --hide <KEY>`

Hide or reveal fields with the specified keys.

- Prefix with `!` to reveal a field
- Use `!*` to reveal all fields

```/dev/null/example.sh#L1-8
# Hide the 'caller' field
hl --hide caller app.log

# Hide multiple fields
hl --hide caller --hide pid app.log

# Reveal a specific field (if hidden by config)
hl --hide '!caller' app.log
```

See [Hiding Fields](../features/hiding-fields.md) for details.

### `--flatten <WHEN>`

Control whether to flatten nested objects.

- **Environment variable**: `HL_FLATTEN`
- **Default**: `always`
- **Possible values**: `never`, `always`

When flattening is enabled, nested objects are displayed with dot-notation field names (e.g., `user.name`, `error.details.code`).

```/dev/null/example.sh#L1-5
# Don't flatten nested objects
hl --flatten=never app.log

# Always flatten (default)
hl --flatten=always app.log
```

### `-t, --time-format <FORMAT>`

Specify the time format for displaying timestamps.

- **Environment variable**: `HL_TIME_FORMAT`
- **Default**: `"%Y-%m-%d %T.%3N"`
- **Format**: Uses `strftime` format specifiers (see `man date` or [strftime documentation](https://man7.org/linux/man-pages/man1/date.1.html))

```/dev/null/example.sh#L1-5
# ISO 8601 format
hl --time-format "%Y-%m-%dT%H:%M:%S%z" app.log

# Custom format
hl -t "%b %d %H:%M:%S" app.log
```

### `-Z, --time-zone <TZ>`

Specify the time zone for displaying timestamps.

- **Environment variable**: `HL_TIME_ZONE`
- **Default**: `UTC`
- **Format**: IANA time zone identifier (e.g., `America/New_York`, `Europe/Berlin`, `Asia/Shanghai`)

See the [list of tz database time zones](https://en.wikipedia.org/wiki/List_of_tz_database_time_zones) for valid identifiers.

```/dev/null/example.sh#L1-5
# Display times in New York timezone
hl --time-zone "America/New_York" app.log

# Display times in Shanghai timezone
hl -Z "Asia/Shanghai" app.log
```

**Note**: The value `local` is not a valid IANA timezone. Use `--local` instead.

### `-L, --local`

Use the local system time zone for displaying timestamps. Overrides the `--time-zone` option.

```/dev/null/example.sh#L1-2
# Display times in local timezone
hl --local app.log
```

### `--no-local`

Disable local time zone. Overrides the `--local` option.

### `-e, --hide-empty-fields`

Hide empty fields (null, empty string, empty object, empty array).

- **Environment variable**: `HL_HIDE_EMPTY_FIELDS`

```/dev/null/example.sh#L1-2
# Hide all empty fields
hl --hide-empty-fields app.log
```

### `-E, --show-empty-fields`

Show empty fields. Overrides the `--hide-empty-fields` option.

- **Environment variable**: `HL_SHOW_EMPTY_FIELDS`

### `--input-info <LAYOUTS>`

Control the display of input file information (file number and filename).

- **Default**: `none,minimal`
- **Possible values**: `auto`, `none`, `minimal`, `compact`, `full`
  - `none`: No input information
  - `minimal`: Minimal information (number only when needed)
  - `compact`: Compact format
  - `full`: Full filename path
  - `auto`: Automatically choose based on context

When processing multiple files or when combined with `--raw`, this option controls how file information is displayed.

```/dev/null/example.sh#L1-5
# Show full filenames
hl --input-info full app1.log app2.log

# No input info (useful for clean JSON output)
hl --raw --input-info json app.log | jq
```

The special value `json` can be used with `--raw` to ensure only valid JSON is output (no logfmt, no prefix text).

### `--ascii [<WHEN>]`

Control whether to restrict punctuation to ASCII characters only.

- **Environment variable**: `HL_ASCII`
- **Default**: `auto`
- **Possible values**: `auto`, `never`, `always`

When enabled, Unicode punctuation (like fancy quotes) is replaced with ASCII equivalents. The actual character mappings can be configured in the configuration file.

```/dev/null/example.sh#L1-2
# Force ASCII-only punctuation
hl --ascii=always app.log
```

### `-x, --expansion [<MODE>]`

Control how large field values and messages are formatted.

- **Environment variable**: `HL_EXPANSION`
- **Default**: `auto`
- **Possible values**: `never`, `inline`, `auto`, `always`

Higher expansion levels break up long content into multiple lines for better readability.

```/dev/null/example.sh#L1-5
# Never expand fields
hl --expansion=never app.log

# Always expand fields to multiple lines
hl -x always app.log
```

### `-o, --output <FILE>`

Write output to the specified file instead of stdout.

```/dev/null/example.sh#L1-2
# Write formatted output to file
hl --output formatted.log -q 'status>=500' app.log
```

## Input Options

### `--input-format <FORMAT>`

Specify the input log format.

- **Environment variable**: `HL_INPUT_FORMAT`
- **Default**: `auto`
- **Possible values**: `auto`, `json`, `logfmt`

```/dev/null/example.sh#L1-5
# Force JSON parsing
hl --input-format json app.log

# Force logfmt parsing
hl --input-format logfmt app.log
```

### `--unix-timestamp-unit <UNIT>`

Specify the unit for Unix timestamps.

- **Environment variable**: `HL_UNIX_TIMESTAMP_UNIT`
- **Default**: `auto`
- **Possible values**: `auto`, `s`, `ms`, `us`, `ns`
  - `s`: Seconds
  - `ms`: Milliseconds
  - `us`: Microseconds
  - `ns`: Nanoseconds

```/dev/null/example.sh#L1-2
# Treat numeric timestamps as milliseconds
hl --unix-timestamp-unit ms app.log
```

### `--allow-prefix`

Allow non-JSON prefixes before JSON log entries.

- **Environment variable**: `HL_ALLOW_PREFIX`

When enabled, `hl` will detect and skip text that appears before JSON objects on a line. The prefix text is preserved in the output.

```/dev/null/example.sh#L1-2
# Allow and preserve prefixes like "2024-01-15 10:30:45 {"level":"info",...}"
hl --allow-prefix app.log
```

See [Prefix Handling](../features/prefix-handling.md) for details.

### `--delimiter <DELIMITER>`

Specify the log entry delimiter.

- **Environment variable**: `HL_DELIMITER`
- **Default**: `auto`
- **Possible values**: `auto`, `cr`, `lf`, `crlf`, `nul`
  - `auto`: Auto-detect delimiter (LF or CRLF followed by empty line or line starting with character other than `}`, space, or tab)
  - `cr`: Carriage return (`\r`)
  - `lf`: Line feed (`\n`)
  - `crlf`: Carriage return + line feed (`\r\n`)
  - `nul`: Null character (`\0`)

The default auto-detection works well for most JSON and logfmt logs, including pretty-printed JSON.

```/dev/null/example.sh#L1-2
# Use null character as delimiter
hl --delimiter nul app.log
```

## Advanced Options

### `--interrupt-ignore-count <N>`

Number of interrupt signals (Ctrl-C / SIGINT) to ignore before exiting.

- **Environment variable**: `HL_INTERRUPT_IGNORE_COUNT`
- **Default**: `3`

This allows you to press Ctrl-C multiple times to force exit when `hl` is processing large files.

```/dev/null/example.sh#L1-2
# Ignore first 5 interrupts
hl --interrupt-ignore-count 5 large-file.log
```

### `--buffer-size <SIZE>`

Set the internal buffer size.

- **Environment variable**: `HL_BUFFER_SIZE`
- **Default**: `256 KiB`

```/dev/null/example.sh#L1-2
# Use 1 MiB buffer
hl --buffer-size "1 MiB" app.log
```

### `--max-message-size <SIZE>`

Set the maximum log entry size.

- **Environment variable**: `HL_MAX_MESSAGE_SIZE`
- **Default**: `64 MiB`

Entries larger than this will be truncated or skipped.

```/dev/null/example.sh#L1-2
# Allow up to 128 MiB per entry
hl --max-message-size "128 MiB" app.log
```

### `-C, --concurrency <N>`

Set the number of processing threads.

- **Environment variable**: `HL_CONCURRENCY`
- **Default**: Number of CPU cores

```/dev/null/example.sh#L1-2
# Use 4 threads
hl --concurrency 4 app.log
```

### `--shell-completions <SHELL>`

Print shell auto-completion script and exit.

- **Possible values**: `bash`, `elvish`, `fish`, `powershell`, `zsh`

```/dev/null/example.sh#L1-5
# Generate bash completions
hl --shell-completions bash > ~/.local/share/bash-completion/completions/hl

# Generate zsh completions
hl --shell-completions zsh > ~/.zsh/completions/_hl
```

### `--man-page`

Print man page and exit.

```/dev/null/example.sh#L1-2
# View man page
hl --man-page | man -l -
```

### `--list-themes[=<TAGS>]`

Print available themes, optionally filtered by tags.

- **Possible values**: `dark`, `light`, `16color`, `256color`, `truecolor`, `overlay`, `base`

```/dev/null/example.sh#L1-8
# List all themes
hl --list-themes

# List only dark themes
hl --list-themes=dark

# List only 256-color themes
hl --list-themes=256color
```

### `--dump-index`

Print debug index metadata (in `--sort` mode) and exit.

This is a debugging option that shows the structure of the timestamp index built for chronological sorting.

```/dev/null/example.sh#L1-2
# Show index structure
hl --sort --dump-index app.log
```
