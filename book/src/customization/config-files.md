# Configuration Files

`hl` supports configuration files to customize default behavior without requiring command-line options every time. This allows you to set personal preferences and project-specific defaults.

## Configuration File Location

`hl` looks for configuration files in the following locations (in order of precedence):

1. **Custom path** specified with `--config` option
2. **Current directory**: `./hl.toml` or `./.hl.toml`
3. **XDG config directory**: `~/.config/hl/config.toml` (Linux/macOS)
4. **User config directory**: `~/.hl/config.toml` (fallback)
5. **System config**: `/etc/hl/config.toml` (Linux)

The first configuration file found is used. Later files are not merged.

### Platform-Specific Paths

**Linux/macOS:**
```
~/.config/hl/config.toml
```

**Windows:**
```
%APPDATA%\hl\config.toml
```

## Configuration File Format

Configuration files use TOML format, which is human-readable and easy to edit.

### Basic Example

```toml
# ~/.config/hl/config.toml

# Time display settings
time-format = "%Y-%m-%d %H:%M:%S.%3N"
time-zone = "local"

# Default theme
theme = "universal"

# Field visibility
[fields]
hide = ["host", "pid", "version"]
```

## Configuration Schema

The configuration file supports a JSON schema for validation and autocompletion in editors:

```toml
#:schema https://raw.githubusercontent.com/pamburus/hl/v0.35.2/schema/json/config.schema.json
```

Add this at the top of your config file for editor support.

## Available Options

### Time and Timezone

Control how timestamps are displayed:

```toml
# Time format (strftime syntax)
time-format = "%b %d %T.%3N"

# Time zone (IANA timezone name or "UTC" or "local")
time-zone = "UTC"
# time-zone = "America/New_York"
# time-zone = "local"
```

Common time format patterns:
- `%Y-%m-%d %T.%3N` → `2024-01-15 10:30:45.123`
- `%b %d %T.%3N` → `Jan 15 10:30:45.123`
- `%H:%M:%S` → `10:30:45`

See [Time Display](../features/time-display.md) for format details.

### Theme Selection

```toml
# Current theme name
theme = "universal"

# Theme overlays to apply
theme-overlays = ["@accent-italic"]
```

Available themes can be listed with:
```bash
hl --list-themes
```

See [Themes](./themes.md) for more on themes and overlays.

### Input Information Display

Control how file information is shown when processing multiple files:

```toml
# Options: "auto", "none", "minimal", "compact", "full"
# Can specify multiple: ["auto", "minimal", "compact"]
input-info = "auto"
```

Modes:
- `auto` — automatically choose based on number of files and terminal width
- `none` — no file information
- `minimal` — file number only
- `compact` — file number and shortened path
- `full` — file number and full path

### ASCII Mode

Control Unicode vs ASCII characters for punctuation:

```toml
# Options: "auto", "never", "always"
ascii = "auto"
```

- `auto` — detect terminal Unicode support
- `never` — always use Unicode box-drawing characters
- `always` — restrict to ASCII characters only

### Field Configuration

#### Hiding and Ignoring Fields

```toml
[fields]
# Ignore fields matching wildcard patterns
ignore = ["_*", "internal.*"]

# Hide specific field names
hide = ["timestamp", "host", "pid"]
```

The difference:
- `ignore` — completely skip these fields during parsing (performance optimization)
- `hide` — parse but don't display (can still be used in queries)

#### Predefined Fields

Configure how `hl` recognizes standard log fields:

```toml
[fields.predefined.time]
# When to show time field: "always" or "auto"
show = "always"

# Field names to recognize as timestamp
names = [
  "ts",
  "time",
  "timestamp",
  "@timestamp"
]
```

**Logger field:**
```toml
[fields.predefined.logger]
names = ["logger", "Logger", "span.name"]
```

**Level field:**
```toml
[fields.predefined.level]
show = "always"

# Common level fields
[[fields.predefined.level.variants]]
names = ["level", "Level", "severity"]

[fields.predefined.level.variants.values]
error = ["error", "err", "fatal", "critical", "panic"]
warning = ["warning", "warn"]
info = ["info", "information"]
debug = ["debug"]
trace = ["trace"]

# Systemd priority levels
[[fields.predefined.level.variants]]
names = ["PRIORITY"]

[fields.predefined.level.variants.values]
error = [3, 2, 1]
warning = [5, 4]
info = [6]
debug = [7]
```

**Message field:**
```toml
[fields.predefined.message]
names = ["msg", "message", "MESSAGE"]
```

**Caller field:**
```toml
[fields.predefined.caller]
names = ["caller", "Caller"]
```

### Formatting Options

#### Field Flattening

```toml
[formatting]
# Flatten nested objects to dot notation: "always" or "never"
flatten = "always"
```

With `flatten = "always"`:
- `{"user": {"id": 123}}` → `user.id: 123`

With `flatten = "never"`:
- `{"user": {"id": 123}}` → `user: {id: 123}`

#### Message Format

```toml
[formatting.message]
# Options: "auto-quoted", "always-quoted", "always-double-quoted", "delimited", "raw"
format = "delimited"
```

Modes:
- `auto-quoted` — quote messages when needed for clarity
- `always-quoted` — always quote messages (most appropriate quotes)
- `always-double-quoted` — always use double quotes
- `delimited` — use delimiter instead of quotes
- `raw` — no quotes or delimiters

#### Punctuation Customization

```toml
[formatting.punctuation]
logger-name-separator = ":"
field-key-value-separator = "="
string-opening-quote = "'"
string-closing-quote = "'"
caller-name-file-separator = " @ "
hidden-fields-indicator = "..."
level-left-separator = "["
level-right-separator = "]"
array-separator = " "

# Unicode vs ASCII variants
message-delimiter = { ascii = "::", unicode = "›" }
source-location-separator = { ascii = "-> ", unicode = "→ " }
input-number-right-separator = { ascii = " | ", unicode = " │ " }
input-name-right-separator = { ascii = " | ", unicode = " │ " }
```

Each punctuation item can be:
- A string (same for ASCII and Unicode mode)
- An object with `ascii` and `unicode` keys (different based on mode)

#### Field Expansion

```toml
[formatting.expansion]
# Options: "never", "inline", "auto", "always"
mode = "auto"
```

Expansion modes:
- `never` — single-line, escape newlines as `\n`
- `inline` — preserve newlines/tabs as-is
- `auto` — expand multi-line content intelligently
- `always` — each field on its own line

See [Field Expansion](../features/field-expansion.md) for details.

## Complete Example Configuration

```toml
#:schema https://raw.githubusercontent.com/pamburus/hl/v0.35.2/schema/json/config.schema.json

# Display settings
time-format = "%Y-%m-%d %H:%M:%S.%3N"
time-zone = "local"
theme = "universal"
theme-overlays = ["@accent-italic"]
input-info = "auto"
ascii = "auto"

# Field configuration
[fields]
# Ignore internal fields
ignore = ["_*", "internal.*"]

# Hide verbose fields
hide = ["host", "hostname", "pid", "version"]

# Time field configuration
[fields.predefined.time]
show = "always"
names = ["ts", "time", "timestamp", "@timestamp"]

# Level field configuration
[fields.predefined.level]
show = "always"

[[fields.predefined.level.variants]]
names = ["level", "severity"]

[fields.predefined.level.variants.values]
error = ["error", "err", "fatal", "critical"]
warning = ["warning", "warn"]
info = ["info"]
debug = ["debug"]
trace = ["trace"]

# Message field configuration
[fields.predefined.message]
names = ["msg", "message"]

# Formatting settings
[formatting]
flatten = "always"

[formatting.message]
format = "delimited"

[formatting.expansion]
mode = "auto"

[formatting.punctuation]
field-key-value-separator = "="
message-delimiter = { ascii = "::", unicode = "›" }
```

## Project-Specific Configuration

Place an `hl.toml` or `.hl.toml` file in your project directory for project-specific settings:

```toml
# ./hl.toml - Project-specific configuration

# Project uses specific time format
time-format = "%Y-%m-%d %H:%M:%S"

# Hide project-specific internal fields
[fields]
hide = ["build_id", "deployment_id", "trace_context"]

# Project uses custom level field names
[[fields.predefined.level.variants]]
names = ["log_level"]

[fields.predefined.level.variants.values]
error = ["ERROR"]
warning = ["WARN"]
info = ["INFO"]
debug = ["DEBUG"]
```

## Specialized Configurations

`hl` ships with specialized configuration presets:

### Kubernetes Logs

```bash
hl --config /path/to/hl/etc/defaults/config-k8s.toml pod.log
```

Optimized for Kubernetes/container logs with appropriate field mappings.

### ECS Logs

```bash
hl --config /path/to/hl/etc/defaults/config-ecs.toml ecs.log
```

Configured for AWS ECS log format.

## Command-Line Override

Configuration file settings can be overridden by command-line options:

```bash
# Config file sets time-zone = "UTC"
# Command line overrides to local
hl --local app.log
```

Command-line options always take precedence over configuration files.

## Environment Variables

Environment variables take precedence over configuration files but are overridden by command-line options:

```
Precedence (lowest to highest):
1. Configuration file
2. Environment variables
3. Command-line options
```

See [Environment Variables](./environment.md) for available variables.

## Validation and Debugging

### Check Current Configuration

There's no built-in command to show current configuration, but you can test settings:

```bash
# Test time format
hl --help | grep "time-format"

# List available themes
hl --list-themes
```

### Schema Validation

Use an editor with TOML and JSON schema support (VS Code, IntelliJ, etc.) to validate your configuration file:

1. Add the schema line at the top of your config file
2. The editor will provide validation and autocompletion

### Syntax Errors

If your configuration file has syntax errors, `hl` will report them:

```
Error: failed to parse configuration file: ...
```

Fix syntax errors and try again.

## Configuration Tips

### Start Simple

Begin with a minimal configuration and add settings as needed:

```toml
# Minimal starter config
time-zone = "local"
theme = "universal"

[fields]
hide = ["host", "pid"]
```

### Use Comments

Document your configuration choices:

```toml
# Show local time for easier debugging
time-zone = "local"

# Hide noisy fields that don't help debugging
[fields]
hide = ["host", "pid", "version"]
```

### Test Changes

After modifying your configuration:

```bash
# Test with a sample log file
hl test.log
```

### Per-Project Configs

Use local configuration files for project-specific needs while keeping global defaults in `~/.config/hl/config.toml`.

## Common Configuration Patterns

### Development Setup

```toml
# Developer-friendly configuration
time-zone = "local"
time-format = "%H:%M:%S.%3N"
theme = "universal"

[fields]
hide = ["host", "pid"]

[formatting.expansion]
mode = "auto"
```

### Production Monitoring

```toml
# Production monitoring configuration
time-zone = "UTC"
time-format = "%Y-%m-%d %H:%M:%S"
theme = "universal"
input-info = "compact"

[fields]
# Hide less critical fields
hide = ["version", "build"]
```

### Minimal Output

```toml
# Minimal, clean output
[fields]
hide = ["host", "pid", "version", "logger"]

[formatting.message]
format = "raw"

[formatting.expansion]
mode = "never"
```

## Related Topics

- [Environment Variables](./environment.md) — configuration via environment
- [Themes](./themes.md) — color scheme customization
- [Time Display](../features/time-display.md) — time format options
- [Field Visibility](../features/field-visibility.md) — controlling field display