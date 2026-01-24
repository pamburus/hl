# Custom Themes

`hl` allows you to create custom themes to match your preferences and terminal environment. Themes are defined in TOML files and stored in your configuration directory.

## Theme File Location

Custom themes are stored in platform-specific locations:

| OS      | Location                                                |
| ------- | ------------------------------------------------------- |
| macOS   | `~/.config/hl/themes/` (or `$XDG_CONFIG_HOME/hl/themes/` if `XDG_CONFIG_HOME` is set to an absolute path) |
| Linux   | `~/.config/hl/themes/` (or `$XDG_CONFIG_HOME/hl/themes/` if set) |
| Windows | `%APPDATA%\hl\themes\`                                  |

**Note**: On macOS, `XDG_CONFIG_HOME` is only respected if it contains an absolute path (e.g., `/Users/username/my-config`). Relative paths are ignored and `~/.config` is used instead. This absolute path requirement is specific to macOS; on Linux, the `dirs` crate handles `XDG_CONFIG_HOME` according to the XDG Base Directory specification.

Theme files can use any of the following formats:
- `.toml` — TOML format (recommended, default)
- `.yaml` or `.yml` — YAML format
- `.json` — JSON format

The theme name is the filename without the extension.

For example, any of these files define a theme named `my-theme`:
- `~/.config/hl/themes/my-theme.toml`
- `~/.config/hl/themes/my-theme.yaml`
- `~/.config/hl/themes/my-theme.json`

You can activate the theme with:

```hl/dev/null/shell.sh#L1
hl --theme my-theme app.log
```

## Theme File Structure

A theme file is written in TOML format and consists of several sections:

- **`version`** — Theme format version (use `"1.1"`).
- **`tags`** — Optional tags describing the theme (e.g., `["dark", "256color"]`).
- **`[styles]`** — General-purpose named styles used throughout the theme.
- **`[elements]`** — Styles for specific log elements (keys, values, timestamps, etc.).
- **`[levels]`** — Styles for different log levels and their components.
- **`[indicators]`** — Styles for status indicators.

### Minimal Example

Here's a minimal custom theme:

```hl/dev/null/my-theme.toml#L1
version = "1.1"
tags = ["dark", "16color"]

[styles]
primary.foreground = "cyan"
accent.foreground = "green"

[elements]
message.style = "primary"
key.style = "accent"
```

## Style Definition

Each style can define:

- **`foreground`** — Foreground color.
- **`background`** — Background color (rarely used).
- **`modes`** — Text modes (e.g., `["bold"]`, `["italic"]`, `["underline"]`).
- **`style`** — Inherit from another named style (string or array of strings).

### Color Values

Colors can be specified as:

- **Named ANSI colors** — `"black"`, `"red"`, `"green"`, `"yellow"`, `"blue"`, `"magenta"`, `"cyan"`, `"white"`, `"bright-black"`, `"bright-red"`, etc.
- **Numeric (256-color palette)** — `42`, `214`, etc.
- **Hex RGB (truecolor)** — `"#ff5733"`, `"#abb2bf"`, etc.
- **`"default"`** — Use the terminal's default foreground/background color.

### Text Modes

Available modes:

- `"bold"` — Bold text.
- `"faint"` / `"dim"` — Faint/dim text.
- `"italic"` — Italic text.
- `"underline"` — Underlined text.
- `"reverse"` — Reverse video (swap foreground and background).
- `"strikethrough"` — Strikethrough text.

To **remove** a mode inherited from another style, prefix it with `-`:

```hl/dev/null/example.toml#L1
[styles]
primary.modes = ["bold", "italic"]
secondary.style = "primary"
secondary.modes = ["-bold"]  # Inherits italic, removes bold
```

### Style Inheritance

Styles can inherit from other styles using the `style` property:

```hl/dev/null/example.toml#L1
[styles]
primary.foreground = "cyan"
accent.style = "primary"
accent.modes = ["bold"]
```

You can inherit from multiple styles (they are merged in order):

```hl/dev/null/example.toml#L1
[styles]
primary.foreground = "white"
strong.modes = ["bold"]
accent.style = ["primary", "strong"]
```

The following diagram illustrates how style inheritance works in the theme system:

![Style Inheritance Diagram](../images/theme-style-roles.svg)

This shows how styles can reference other styles to build a hierarchy of reusable style definitions.

## Sections in Detail

### `[styles]` — Named Styles

Define general-purpose styles that can be referenced elsewhere. Common style names used by the base theme:

- `default` — Base default style.
- `primary` — Primary text color.
- `secondary` — Secondary/muted text.
- `strong` — Strong emphasis (often bold).
- `muted` — De-emphasized text.
- `accent` — Accent color for highlights.
- `accent-secondary` — Secondary accent.
- `message` — Log message text.
- `syntax` — Syntax elements (braces, brackets).
- `status` — Status indicators.
- `key` — Field keys.
- `value` — Field values.
- `level` — Log level styling.
- `unknown`, `trace`, `debug`, `info`, `warning`, `error` — Level-specific styles.

Example:

```hl/dev/null/styles-example.toml#L1
[styles]
primary.foreground = "#abb2bf"
secondary.foreground = "#636d83"
secondary.modes = ["faint"]
strong.style = "primary"
strong.modes = ["bold"]
accent.foreground = "#61afef"
debug.foreground = "#c678dd"
info.foreground = "#56b6c2"
warning.foreground = "#e5c07b"
error.foreground = "#e06c75"
```

### `[elements]` — Log Element Styles

Define styles for specific parts of log entries.

The following diagram illustrates how element styles inherit from base styles and can be overridden at the level-specific configuration:

![Element Inheritance Diagram](../images/theme-element-inheritance.svg)

This shows the complete inheritance chain from base styles through elements to level-specific overrides.

#### Available Elements


- `input` — Input source indicator (filename or stream name).
- `time` — Timestamp field.
- `level` — Log level label (outer container).
- `level-inner` — Log level label (inner text).
- `logger` — Logger name.
- `caller` — Caller location (file:line).
- `message` — Log message text.
- `message-delimiter` — Delimiter between message and fields.
- `field` — Field name-value pair container.
- `key` — Field key/name.
- `value` — Field value (generic).
- `ellipsis` — Ellipsis indicator for truncated content.
- `bullet` — Bullet point for multiline fields.
- `value-expansion` — Expanded field indicator.
- `object` — JSON object braces `{}`.
- `array` — JSON array brackets `[]`.
- `string` — String values.
- `number` — Numeric values.
- `boolean` — Boolean values (`true`/`false`).
- `boolean-true` — `true` value specifically.
- `boolean-false` — `false` value specifically.
- `null` — `null` values.

Example:

```hl/dev/null/elements-example.toml#L1
[elements]
input.style = "secondary"
time.style = "secondary"
level.style = "muted"
level-inner.style = "level"
logger.style = "accent"
caller.style = "secondary"
caller.modes = ["italic"]
message.style = "strong"
key.style = "accent"
key.modes = ["underline"]
number.foreground = "#98c379"
boolean-true.foreground = "#56b6c2"
boolean-false.foreground = "#e06c75"
null.foreground = "#e06c75"
```

### `[levels]` — Per-Level Overrides

Customize styles for specific log levels. You can override any element style for a given level:

```hl/dev/null/levels-example.toml#L1
[levels]
[levels.warning]
time.style = "warning"
message.style = ["message", "warning"]

[levels.error]
time.style = "error"
level.modes = ["reverse"]
message.style = ["message", "error"]
message.foreground = "#e06c75"
```

Supported level names: `unknown`, `trace`, `debug`, `info`, `warning`, `error`.

### `[indicators]` — Status Indicators

Customize status indicators (rarely needed):

```hl/dev/null/indicators-example.toml#L1
[indicators]
[indicators.sync]
synced.text = "✓"
failed.text = "✗"
failed.inner.style = "error"
failed.inner.modes = ["bold"]
```

## Complete Example

Here's a complete custom theme based on a fictional "midnight" color scheme:

```hl/dev/null/midnight.toml#L1
#:schema https://raw.githubusercontent.com/pamburus/hl/v0.35.2/schema/json/theme.schema.v1.json

version = "1.1"
tags = ["dark", "truecolor"]

#
# Styles define general purpose styles used throughout the theme.
#
[styles]
[styles.primary]
foreground = "#d0d0f0"

[styles.secondary]
foreground = "#606080"
modes = ["faint"]

[styles.strong]
style = "primary"
modes = ["bold"]

[styles.accent]
foreground = "#80a0ff"

[styles.message]
style = "strong"

[styles.syntax]
foreground = "#a0a0c0"
modes = ["bold"]

[styles.debug]
foreground = "#c080ff"

[styles.info]
foreground = "#60c0e0"

[styles.warning]
foreground = "#ffb060"

[styles.error]
foreground = "#ff6060"

#
# Elements define specific styles for different log elements.
#
[elements]
[elements.input]
style = "secondary"

[elements.time]
style = "secondary"

[elements.level]
style = "secondary"

[elements.level-inner]
style = "level"

[elements.logger]
style = "accent"
modes = ["italic"]

[elements.caller]
style = "secondary"
modes = ["italic"]

[elements.message]
style = "message"

[elements.key]
style = "accent"

[elements.number]
foreground = "#80ff80"

[elements.boolean-true]
foreground = "#60e060"

[elements.boolean-false]
foreground = "#ff8080"

[elements.null]
foreground = "#ff6060"

#
# Levels define styles for different log levels.
#
[levels]
[levels.unknown.level-inner]
style = ["level", "secondary"]

[levels.trace.level-inner]
style = ["level", "secondary"]

[levels.debug.level-inner]
style = ["level", "debug"]

[levels.info.level-inner]
style = ["level", "info"]

[levels.warning]
time.style = "warning"
level-inner.style = ["level", "warning"]
message.style = ["message", "warning"]

[levels.error]
time.style = "error"
level-inner.style = ["level", "error"]
message.style = ["message", "error"]
```

Save this as `~/.config/hl/themes/midnight.toml` and activate it with:

```hl/dev/null/shell.sh#L1
hl --theme midnight app.log
```

## Starting from a Stock Theme

The easiest way to create a custom theme is to start from a stock theme:

1. Find a stock theme you like:
   ```hl/dev/null/shell.sh#L1
   hl --list-themes
   ```

2. Copy it to your themes directory:
   ```hl/dev/null/shell.sh#L1
   # Find the embedded theme file in hl source or docs
   # Or create a new file based on examples above
   cp one-dark-24.toml ~/.config/hl/themes/my-theme.toml
   ```

3. Edit `my-theme.toml` to customize colors and styles.

4. Test your theme:
   ```hl/dev/null/shell.sh#L1
   hl --theme my-theme app.log
   ```

## Tips and Best Practices

- **Use the schema comment** — Include the `#:schema` line at the top for editor autocomplete and validation.
- **Tag your theme** — Add appropriate tags (`dark`, `light`, `16color`, `256color`, `truecolor`) to help filter and categorize.
- **Test on real logs** — Use actual log files to ensure readability across different log levels and field types.
- **Inherit from base** — Custom themes automatically inherit from the `@base` theme, so you only need to override what you want to change.
- **Use named styles** — Define colors in `[styles]` and reference them in `[elements]` for easier maintenance.
- **Consider accessibility** — Ensure sufficient contrast between foreground and background colors.

## Troubleshooting

### Theme Not Found

If `hl` reports "theme not found":

- Verify the file exists in `~/.config/hl/themes/`.
- Check the filename matches the theme name (without the extension).
- Ensure the file has a supported extension: `.toml`, `.yaml`, `.yml`, or `.json`.

### Colors Not Showing

If colors don't appear as expected:

- Check your terminal's color support (`echo $TERM`).
- Try a simpler color (e.g., `"red"` instead of `"#ff0000"`) to test.
- Verify the TOML syntax is correct (use a TOML linter).

### Styles Not Applying

If styles aren't applied:

- Ensure the `version = "1.1"` line is present.
- Check for typos in element or style names.
- Verify style inheritance chains are correct (no circular references).

## Next Steps

- [Stock Themes](themes-stock.md) — Browse built-in themes for inspiration.
- [Theme Overlays](themes-overlays.md) — Learn how to create small overlay themes to tweak existing themes.
- [Configuration Files](config-files.md) — Set a default theme in your config file.
