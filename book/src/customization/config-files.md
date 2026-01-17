# Configuration Files

`hl` supports configuration files to customize default behavior without requiring command-line options every time. This allows you to set personal preferences and project-specific defaults.

## Configuration File Layering

`hl` uses a **layered configuration system** where multiple configuration files are merged together, with each layer able to override settings from previous layers.

### Configuration Layers

Configuration is built up in layers (from base to top):

1. **Embedded default configuration** (base layer, always present)
2. **System configuration(s)** (if found)
3. **User configuration** (if found at default location)
4. **`HL_CONFIG` environment variable** (if set, appends to the layer chain)
5. **`--config` option(s)** (each option appends to the layer chain)

Each layer **overrides only the specific settings it defines**, leaving other settings from previous layers intact. This allows partial configuration at each level.

### Configuration File Locations

**System configuration** (searched in order, all found files are loaded as layers):
- **Linux/macOS**: `/etc/hl/config`
- **Windows**: `%ProgramData%\hl\config` (if exists)

**User configuration** (first found is used):
- **Linux/macOS**: `~/.config/hl/config` (or `$XDG_CONFIG_HOME/hl/config`)
- **macOS alternative**: `~/.config/hl/config` (if `XDG_CONFIG_HOME` not set)
- **Windows**: `%APPDATA%\hl\config`

### Layer Examples

**Simple layering:**
```bash
# Layers loaded:
# 1. Embedded defaults
# 2. /etc/hl/config (if exists)
# 3. ~/.config/hl/config (if exists)
hl app.log
```

**With HL_CONFIG environment variable:**
```bash
# Layers loaded:
# 1. Embedded defaults
# 2. /etc/hl/config (if exists)
# 3. ~/.config/hl/config (if exists)
# 4. project-config.toml
HL_CONFIG=project-config.toml hl app.log
```

**Multiple --config options:**
```bash
# Layers loaded:
# 1. Embedded defaults
# 2. /etc/hl/config (if exists)
# 3. ~/.config/hl/config (if exists)
# 4. team-defaults.toml
# 5. project-specific.toml
hl --config team-defaults.toml --config project-specific.toml app.log
```

**Combining HL_CONFIG and --config:**
```bash
# Layers loaded:
# 1. Embedded defaults
# 2. /etc/hl/config (if exists)
# 3. ~/.config/hl/config (if exists)
# 4. base.toml (from HL_CONFIG)
# 5. override.toml (from --config)
HL_CONFIG=base.toml hl --config override.toml app.log
```

**Resetting the configuration chain:**
```bash
# Reset: skip system and user configs, use only embedded defaults
hl --config - app.log

# Reset and add specific config
# Layers loaded:
# 1. Embedded defaults
# 2. custom.toml (only)
hl --config - --config custom.toml app.log

# Reset then add multiple configs
# Layers loaded:
# 1. Embedded defaults
# 2. base.toml
# 3. override.toml
hl --config - --config base.toml --config override.toml app.log
```

### Important Notes

**File extension:**
Configuration files are loaded **without** extension. The actual files should be named:
- `config` (not `config.toml`)
- Or `config.toml`, `config.yaml`, etc. depending on your setup

**HL_CONFIG behavior:**
- `HL_CONFIG` **appends** to the layer chain (does not replace user config)
- Both `~/.config/hl/config` and the file specified by `HL_CONFIG` are loaded
- To skip the user config, use `--config -`

### How Layering Works

When multiple configuration files are loaded:

- **Base layer** (embedded defaults) provides all default values
- **Each subsequent layer** can override specific settings
- **Unspecified settings** in a layer are inherited from previous layers
- **Arrays and tables** in later layers completely replace those from earlier layers (not merged)

Example:

```toml
# Layer 1 (embedded): theme = "hl-dark", time-zone = "UTC"

# Layer 2 (/etc/hl/config):
# (empty or doesn't exist)

# Layer 3 (~/.config/hl/config):
theme = "universal"
# Inherits: time-zone = "UTC" from base

# Layer 4 (--config project.toml):
time-zone = "America/New_York"
# Inherits: theme = "universal" from Layer 3
# Final result: theme = "universal", time-zone = "America/New_York"
```

## Configuration File Format

Configuration files use TOML format, which is human-readable and easy to edit.

### Basic Example

```toml
# ~/.config/hl/config.toml

# Time display settings
time-format = "%Y-%m-%d %H:%M:%S.%3N"
time-zone = "UTC"

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

# Time zone (IANA timezone name)
time-zone = "UTC"
# time-zone = "America/New_York"
# time-zone = "Europe/London"
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
time-zone = "UTC"
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

Command-line options override all configuration layers:

```bash
# Config layers set time-zone = "UTC"
# Command line overrides to local
hl --local app.log
```

Command-line options always take precedence over all configuration file layers.

## Precedence Order

Settings are applied in this order (lowest to highest priority):

```
1. Embedded default configuration (base layer)
2. System configuration file(s) (/etc/hl/config)
3. User configuration file (~/.config/hl/config)
4. HL_CONFIG environment variable (appends config file to layer chain)
5. --config option(s) (each appends a layer to the chain)
6. Other environment variables (HL_THEME, HL_LEVEL, etc.)
7. Command-line options (highest priority)
```

Within configuration file layers (1-5), later layers override earlier layers for the specific settings they define. Other environment variables (6) and command-line options (7) override all configuration layers.

See [Environment Variables](./environment.md) for available variables.

## Resetting Configuration

### Using --config -

Use `--config -` (or `--config ""`) to reset the configuration chain, discarding all previous layers except the embedded defaults:

```bash
# Skip system and user configs, use only embedded defaults
hl --config - app.log

# Reset and use only specific config(s)
hl --config - --config my-config.toml app.log
hl --config - --config base.toml --config override.toml app.log
```

This is useful when:
- You want to ignore system and user configuration files
- You need a clean slate for testing
- You want precise control over which configs are loaded
- You need to debug configuration issues

**Note:** Any `--config` options **before** the reset are discarded. `HL_CONFIG` is also ignored if `--config -` is used.

### Using HL_CONFIG to Disable Implicit Configs

Set `HL_CONFIG` to an empty string or `-` to skip system and user configuration files for all invocations:

```bash
# Skip system and user configs, use only embedded defaults
export HL_CONFIG=
# or
export HL_CONFIG=-

hl app.log  # Uses only embedded defaults
```

This applies to all `hl` commands in the current shell session, unlike `--config -` which only affects a single invocation.

You can combine this with `--config` to use specific configs while skipping the implicit ones:

```bash
# Skip system/user configs, use only custom config
export HL_CONFIG=-
hl --config ./project-config app.log
```

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
time-zone = "UTC"
theme = "universal"

[fields]
hide = ["host", "pid"]
```

### Use Comments

Document your configuration choices:

```toml
# Use UTC for consistency
time-zone = "UTC"

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

Leverage the layering system for project-specific settings:

```bash
# Use HL_CONFIG for project-specific settings
cd my-project
HL_CONFIG=./project-hl-config hl app.log

# Or explicitly layer team + project configs
hl --config team-defaults.toml --config project.toml app.log

# Reset to skip global configs, use only project config
hl --config - --config ./project-config app.log
```

### Understanding Layer Merging

Remember: each layer **replaces** complete settings, not merges them:

```toml
# ~/.config/hl/config
[fields]
hide = ["host", "pid"]

# project-config (via HL_CONFIG or --config)
[fields]
hide = ["version"]  # This REPLACES ["host", "pid"], not adds to it
# Final result: hide = ["version"] only
```

To extend settings, repeat all desired values:
```toml
# project-config - to extend the hide list
[fields]
hide = ["host", "pid", "version"]  # Include all desired values
```

## Common Configuration Patterns

### Development Setup

```toml
# Developer-friendly configuration
time-zone = "UTC"
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

### Multi-Layer Project Setup

```toml
# ~/.config/hl/config - Personal defaults
theme = "universal"
time-zone = "UTC"

# team-defaults - Team standards
time-format = "%Y-%m-%d %H:%M:%S"
[fields]
hide = ["host", "pid"]

# project-config - Project-specific
[fields]
hide = ["host", "pid", "internal_id"]  # Extends team defaults

# Usage (all three configs are layered):
# hl --config team-defaults --config project-config app.log
```

### Isolated Project Configuration

```bash
# Use only project config, skip all system and user configs
# Useful for reproducible builds or containerized environments

# Set in shell profile or project .envrc
export HL_CONFIG=-

# Then use project-specific config
hl --config ./project-config app.log

# Or in CI/CD
HL_CONFIG=- hl --config ./ci-config app.log
```

This ensures consistent behavior regardless of what's in `/etc/hl/config` or `~/.config/hl/config`.

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