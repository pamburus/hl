# Environment Variables

`hl` supports configuration through environment variables, allowing you to set defaults without command-line options or configuration files.

## Overview

Environment variables are useful for:

- Setting system-wide defaults
- Configuring `hl` in containerized environments
- Temporary overrides without editing config files
- Shell-specific customization

## Precedence Order

Settings are applied in this order (lowest to highest priority):

1. **Embedded default configuration** — built-in defaults (base layer)
2. **System configuration file(s)** — `/etc/hl/config` or `%ProgramData%\hl\config` (if found)
3. **User configuration file** — `~/.config/hl/config` (if found)
4. **HL_CONFIG environment variable** — appends config file to layer chain
5. **--config option(s)** — each appends a layer to the chain (can repeat)
6. **Other environment variables** — `HL_THEME`, `HL_LEVEL`, etc.
7. **Command-line options** — explicit flags (highest priority)

Configuration files (1-5) are layered, with each layer overriding specific settings from previous layers. Other environment variables (6) and command-line options (7) override all configuration layers.

See [Configuration Files](./config-files.md) for details on the layered configuration system.

## Available Environment Variables

### Display and Formatting

#### HL_COLOR

Control ANSI color output.

```bash
export HL_COLOR=always
export HL_COLOR=never
export HL_COLOR=auto  # default
```

Values:
- `auto` — use colors when outputting to a terminal
- `always` — always use colors (even when piping)
- `never` — disable colors

Example:
```bash
# Force colors when piping to less
HL_COLOR=always hl app.log | less -R
```

#### HL_THEME

Select the color theme.

```bash
export HL_THEME=universal
export HL_THEME=monokai
export HL_THEME=ayu-dark-24
```

List available themes:
```bash
hl --list-themes
```

Example:
```bash
# Use universal theme by default
export HL_THEME=universal
```

#### HL_ASCII

Control Unicode vs ASCII characters for punctuation.

```bash
export HL_ASCII=auto    # default
export HL_ASCII=always  # ASCII only
export HL_ASCII=never   # always Unicode
```

Values:
- `auto` — detect terminal Unicode support
- `always` — restrict to ASCII characters only
- `never` — always use Unicode box-drawing characters

Example:
```bash
# Force ASCII mode for compatibility
export HL_ASCII=always
```

#### HL_EXPANSION

Control field expansion mode.

```bash
export HL_EXPANSION=auto    # default
export HL_EXPANSION=never
export HL_EXPANSION=inline
export HL_EXPANSION=always
```

Values:
- `never` — single-line output, escape newlines
- `inline` — preserve newlines/tabs as-is
- `auto` — expand multi-line content intelligently
- `always` — each field on its own line

Example:
```bash
# Always expand fields for readability
export HL_EXPANSION=always
```

#### HL_FLATTEN

Control whether nested objects are flattened to dot notation.

```bash
export HL_FLATTEN=always  # default
export HL_FLATTEN=never
```

Values:
- `always` — flatten nested objects (`user.id`)
- `never` — keep nested structure (`user: {id: 123}`)

Example:
```bash
# Keep nested structure
export HL_FLATTEN=never
```

### Time and Timezone

#### HL_TIME_FORMAT

Customize timestamp format using strftime syntax.

```bash
export HL_TIME_FORMAT="%Y-%m-%d %H:%M:%S.%3N"
export HL_TIME_FORMAT="%b %d %T.%3N"
export HL_TIME_FORMAT="%H:%M:%S"
```

Default: `%b %d %T.%3N`

Common patterns:
- `%Y-%m-%d %T.%3N` → `2024-01-15 10:30:45.123`
- `%b %d %T.%3N` → `Jan 15 10:30:45.123`
- `%H:%M:%S` → `10:30:45`

Example:
```bash
# Use concise time format
export HL_TIME_FORMAT="%H:%M:%S.%3N"
```

See [Time Display](../features/time-display.md) for format details.

#### HL_TIME_ZONE

Set the timezone for timestamp display.

```bash
export HL_TIME_ZONE=UTC              # default
export HL_TIME_ZONE=America/New_York
export HL_TIME_ZONE=Europe/London
```

Use IANA timezone names (see [IANA timezone database](https://en.wikipedia.org/wiki/List_of_tz_database_time_zones)).

Example:
```bash
# Display timestamps in New York time
export HL_TIME_ZONE=America/New_York
```

### Filtering

#### HL_LEVEL

Set default minimum log level to display.

```bash
export HL_LEVEL=info
export HL_LEVEL=warn
export HL_LEVEL=error
```

Values: `trace`, `debug`, `info`, `warn`, `error` (or abbreviations: `t`, `d`, `i`, `w`, `e`)

Example:
```bash
# Only show warnings and errors by default
export HL_LEVEL=warn
```

### Field Visibility

#### HL_HIDE_EMPTY_FIELDS

Hide fields with empty values (null, empty string, empty object, empty array).

```bash
export HL_HIDE_EMPTY_FIELDS=true
export HL_HIDE_EMPTY_FIELDS=false  # default
```

Example:
```bash
# Hide empty fields by default
export HL_HIDE_EMPTY_FIELDS=true
```

#### HL_SHOW_EMPTY_FIELDS

Override to show empty fields (takes precedence over `HL_HIDE_EMPTY_FIELDS`).

```bash
export HL_SHOW_EMPTY_FIELDS=true
export HL_SHOW_EMPTY_FIELDS=false
```

### Input Options

#### HL_INPUT_FORMAT

Force specific input format.

```bash
export HL_INPUT_FORMAT=auto    # default
export HL_INPUT_FORMAT=json
export HL_INPUT_FORMAT=logfmt
```

Values:
- `auto` — automatically detect format
- `json` — expect JSON format
- `logfmt` — expect logfmt format

Example:
```bash
# Always expect JSON format
export HL_INPUT_FORMAT=json
```

#### HL_UNIX_TIMESTAMP_UNIT

Specify Unix timestamp unit for numeric timestamps.

```bash
export HL_UNIX_TIMESTAMP_UNIT=auto  # default
export HL_UNIX_TIMESTAMP_UNIT=s     # seconds
export HL_UNIX_TIMESTAMP_UNIT=ms    # milliseconds
export HL_UNIX_TIMESTAMP_UNIT=us    # microseconds
export HL_UNIX_TIMESTAMP_UNIT=ns    # nanoseconds
```

Example:
```bash
# Timestamps are in milliseconds
export HL_UNIX_TIMESTAMP_UNIT=ms
```

#### HL_ALLOW_PREFIX

Allow non-JSON prefixes before JSON entries (for Docker, systemd logs).

```bash
export HL_ALLOW_PREFIX=true
export HL_ALLOW_PREFIX=false  # default
```

Example:
```bash
# Enable prefix handling for Docker logs
export HL_ALLOW_PREFIX=true
```

#### HL_DELIMITER

Set log entry delimiter.

```bash
export HL_DELIMITER=auto   # default - smart newline + continuation detection
export HL_DELIMITER=lf     # line feed only (strict Unix)
export HL_DELIMITER=crlf   # smart newline (accepts LF or CRLF)
export HL_DELIMITER=cr     # carriage return only
export HL_DELIMITER=nul    # null byte
```

**Note:** `crlf` accepts **either** `\n` or `\r\n` (not strict CRLF only). Use this for files with mixed or Windows line endings.

Example:
```bash
# For logs with Unix or Windows line endings
export HL_DELIMITER=crlf
```

### Paging

#### HL_PAGING

Control pager usage.

```bash
export HL_PAGING=auto    # default
export HL_PAGING=always
export HL_PAGING=never
```

Values:
- `auto` — use pager for terminal output (not in follow mode)
- `always` — always use pager
- `never` — never use pager (same as `-P`)

Example:
```bash
# Disable pager by default
export HL_PAGING=never
```

#### HL_PAGER

Override the pager command (takes precedence over `PAGER`).

```bash
export HL_PAGER=less
export HL_PAGER="less -R"
export HL_PAGER=bat
export HL_PAGER=most
```

Example:
```bash
# Use bat as pager
export HL_PAGER="bat --style=plain --paging=always"
```

#### PAGER

Standard pager environment variable (used if `HL_PAGER` is not set).

```bash
export PAGER=less
export PAGER="less -R"
```

Example:
```bash
# Set default pager for all tools
export PAGER="less -R --mouse"
```

### Advanced Options

#### HL_CONFIG

Add a configuration file layer to the configuration chain.

```bash
export HL_CONFIG=/path/to/custom/config
export HL_CONFIG=./project-config
```

This adds a configuration layer **in addition to** system and user configs. The complete layer chain becomes:
1. Embedded defaults
2. System config (`/etc/hl/config` if exists)
3. User config (`~/.config/hl/config` if exists)
4. File specified by `HL_CONFIG`

Settings in `HL_CONFIG` override settings from previous layers, but all configs are loaded and layered.

**Disabling implicit configs:**

Set `HL_CONFIG` to an empty string or `-` to skip system and user configuration files:

```bash
# Skip system and user configs, use only embedded defaults
export HL_CONFIG=
# or
export HL_CONFIG=-

hl app.log  # Uses only embedded defaults
```

This is equivalent to using `--config -` but applies automatically to all `hl` invocations.

To skip implicit configs and use a specific config file:
```bash
# Skip system/user configs, use only embedded defaults + custom config
HL_CONFIG=- hl --config ./my-config app.log
```

Example:
```bash
# Add project config as a layer on top of system and user configs
# Loads: embedded defaults → /etc/hl/config → ~/.config/hl/config → ./project-config
export HL_CONFIG=./project-config
```

**Note:** Configuration files should be named without extension (e.g., `config`, not `config.toml`).

#### HL_INTERRUPT_IGNORE_COUNT

Number of interrupt signals (Ctrl-C) to ignore before exiting.

```bash
export HL_INTERRUPT_IGNORE_COUNT=3  # default
export HL_INTERRUPT_IGNORE_COUNT=0  # exit immediately
export HL_INTERRUPT_IGNORE_COUNT=5  # more tolerant
```

Note: This is ignored in follow mode (`-F`), which always exits immediately.

Example:
```bash
# Exit immediately on first Ctrl-C
export HL_INTERRUPT_IGNORE_COUNT=0
```

#### HL_BUFFER_SIZE

Set buffer size for reading input.

```bash
export HL_BUFFER_SIZE="256 KiB"  # default
export HL_BUFFER_SIZE="512 KiB"
export HL_BUFFER_SIZE="1 MiB"
```

Example:
```bash
# Increase buffer for high-volume logs
export HL_BUFFER_SIZE="1 MiB"
```

#### HL_MAX_MESSAGE_SIZE

Maximum log entry size.

```bash
export HL_MAX_MESSAGE_SIZE="64 MiB"  # default
export HL_MAX_MESSAGE_SIZE="128 MiB"
```

Example:
```bash
# Allow larger log entries
export HL_MAX_MESSAGE_SIZE="128 MiB"
```

#### HL_CONCURRENCY

Number of processing threads.

```bash
export HL_CONCURRENCY=4
export HL_CONCURRENCY=8
```

Default: automatically determined based on CPU cores.

Example:
```bash
# Limit to 4 threads
export HL_CONCURRENCY=4
```

## Common Configuration Patterns

### Development Setup

```bash
# ~/.bashrc or ~/.zshrc

# Developer-friendly defaults
export HL_TIME_FORMAT="%H:%M:%S.%3N"
export HL_THEME=universal
export HL_HIDE_EMPTY_FIELDS=true
```

### Production Monitoring

```bash
# Production environment

export HL_TIME_ZONE=UTC
export HL_TIME_FORMAT="%Y-%m-%d %H:%M:%S"
export HL_THEME=universal
export HL_PAGING=never
export HL_LEVEL=warn
```

### Container Environment

```bash
# Dockerfile or docker-compose.yml

ENV HL_COLOR=always
ENV HL_PAGING=never
ENV HL_TIME_ZONE=UTC
ENV HL_ALLOW_PREFIX=true
ENV HL_INPUT_FORMAT=json
```

### CI/CD Pipeline

```bash
# CI environment variables

export HL_COLOR=never
export HL_PAGING=never
export HL_INTERRUPT_IGNORE_COUNT=0
export HL_EXPANSION=never
```

### Minimal Output

```bash
# Clean, minimal output

export HL_EXPANSION=never
export HL_FLATTEN=always
export HL_HIDE_EMPTY_FIELDS=true
export HL_TIME_FORMAT="%H:%M:%S"
```

## Using with Shell Profiles

Add environment variables to your shell profile for persistent defaults.

### Bash

```bash
# ~/.bashrc or ~/.bash_profile

export HL_THEME=universal
export HL_PAGER="less -R --mouse"
```

### Zsh

```bash
# ~/.zshrc

export HL_THEME=universal
export HL_PAGER="less -R --mouse"
```

### Fish

```fish
# ~/.config/fish/config.fish

set -x HL_THEME universal
set -x HL_PAGER "less -R --mouse"
```

### PowerShell

```powershell
# $PROFILE

$env:HL_THEME = "universal"
$env:HL_PAGER = "less -R"
```

## Temporary Overrides

Set environment variables for a single command:

```bash
# Use different theme for one command
HL_THEME=monokai hl app.log

# Force colors when piping
HL_COLOR=always hl app.log | less -R

# Use local time for one command
hl -L app.log

# Use specific timezone temporarily
HL_TIME_ZONE=America/New_York hl app.log
```

## Debugging Environment Configuration

### Check Current Values

```bash
# Show all HL_* environment variables
env | grep HL_

# Check specific variable
echo $HL_THEME
echo $HL_TIME_ZONE
```

### Test Configuration

```bash
# Test with explicit override
HL_THEME=universal hl test.log

# Unset variable to test default
unset HL_THEME
hl test.log
```

## Best Practices

1. **Use shell profiles** for persistent personal preferences
2. **Use `.env` files** for project-specific settings
3. **Document** environment variables in project README
4. **Override sparingly** — prefer configuration files for complex settings
5. **Test changes** after modifying environment variables

## Troubleshooting

### Variables Not Taking Effect

**Check precedence:**
- Command-line options override environment variables
- Verify variable name (must be uppercase: `HL_THEME`, not `hl_theme`)
- Check if variable is actually set: `env | grep HL_`

**Restart shell:**
```bash
# After editing shell profile
source ~/.bashrc  # or ~/.zshrc
```

### Syntax Errors

Some variables expect specific values. Check help output:
```bash
hl --help
```

### Conflicts

If behavior is unexpected:
```bash
# Unset all HL_* variables
unset $(env | grep HL_ | cut -d= -f1)

# Test default behavior
hl app.log
```

## Related Topics

- [Configuration Files](./config-files.md) — persistent configuration
- [Themes](./themes.md) — color scheme customization
- [Automatic Pager Integration](../features/pager.md) — pager configuration
- [Time Display](../features/time-display.md) — time format options