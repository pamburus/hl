# Themes

Themes control the visual appearance of `hl` output, including colors, text styles, and formatting. `hl` ships with a variety of built-in themes and supports custom theme creation.

## Overview

Themes in `hl` define:

- **Colors** for different log elements (timestamps, levels, fields, values)
- **Text styles** (bold, italic, underline, dim)
- **Visual modes** (reverse, strikethrough)
- **Element-specific styling** (numbers, strings, booleans, nulls)
- **Level-specific colors** (error, warning, info, debug, trace)

## Default Theme

The default theme is `uni`, which:

- Works on both light and dark backgrounds
- Uses 16 basic ANSI colors for maximum compatibility
- Uses blue accents

You can change the default in your [configuration file](./config-files.md).

## Selecting a Theme

### Command Line

Use the `--theme` option:

```sh
hl --theme universal app.log
hl --theme one-dark-24 app.log
hl --theme ayu-light-24 app.log
```

### Environment Variable

Set a default theme:

```sh
export HL_THEME=universal
hl app.log
```

### Configuration File

```toml
# ~/.config/hl/config.toml
theme = "universal"
```

## Listing Available Themes

View all built-in themes:

```sh
# List all themes
hl --list-themes

# Filter by tags
hl --list-themes=dark
hl --list-themes=light
hl --list-themes=16color
hl --list-themes=256color
hl --list-themes=truecolor
```

Themes are tagged by color capability and background preference, making it easy to find one that suits your terminal.

## Theme Overlays

Overlays are mini-themes that modify specific aspects without replacing the entire theme. They allow you to customize built-in themes without creating entirely new ones.

### Default Overlay

The default configuration applies the `@accent-italic` overlay, which adds italic styling to accent elements.

### Using Overlays

**Configuration file:**
```toml
theme = "universal"
theme-overlays = ["@accent-italic"]
```

Multiple overlays can be applied in order:
```toml
theme-overlays = ["@accent-italic", "@custom-overlay"]
```

Overlays are merged in order: base theme → main theme → overlays (in list order).

See [Theme Overlays](./themes-overlays.md) for creating custom overlays.

## Terminal Compatibility

Themes are categorized by color capability:

- **16-color themes** — Maximum compatibility, work on any color-capable terminal
- **256-color themes** — Enhanced color palette, requires 256-color terminal support
- **24-bit themes** — Full RGB color support, requires true color terminal

Use `hl --list-themes=16color` to find themes compatible with basic terminals.

### Troubleshooting Colors

**Colors not showing:**
```sh
# Force color output
hl --color always app.log

# Check terminal color support
echo $TERM
echo $COLORTERM
```

**Colors look wrong:**
```sh
# Try a 16-color theme for maximum compatibility
hl --theme uni app.log
```

**Colors in pager:**
```sh
# Ensure pager preserves colors
LESS=-R hl app.log
```

## Custom Themes

You can create custom themes to match your preferences or combine built-in themes with your own customizations.

### Creating a Theme

1. Create a TOML file: `~/.config/hl/themes/my-theme.toml`
2. Define theme structure (see below)
3. Use it: `hl --theme my-theme app.log`

Basic custom theme structure:
```toml
version = "1.1"
tags = ["dark", "custom"]

[styles]
[styles.accent]
foreground = "#00FF00"

[elements]
[elements.time]
foreground = "bright-blue"

[elements.level]
foreground = "yellow"
modes = ["bold"]
```

See [Custom Themes](./themes-custom.md) for a detailed guide on theme creation.

## Theme Structure Reference

Themes are TOML files with the following sections:

### Color Specifications

Themes support several color formats:

**Named ANSI colors:**
```toml
foreground = "red"
foreground = "bright-blue"
```

Available: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`, and `bright-*` variants.

**256-color palette:**
```toml
foreground = 214  # Orange-ish
```

**24-bit RGB:**
```toml
foreground = "#FF5733"
foreground = "rgb(255, 87, 51)"
```

**Default color:**
```toml
foreground = "default"  # Terminal default
```

### Text Modes

```toml
[styles.emphasis]
foreground = "yellow"
modes = ["bold", "italic"]
```

Available modes:
- `bold` — bold text
- `italic` — italic text
- `underline` — underlined text
- `dim` / `faint` — dimmed text
- `reverse` — swap foreground/background
- `strikethrough` — strikethrough text
- `-mode` — disable a mode (e.g., `-faint`)

### Stylable Elements

**General elements:** `number`, `string`, `boolean`, `null`, `key`, `punctuation`

**Log-specific elements:** `time`, `level`, `logger`, `caller`, `message`

**Level-specific styling:**
```toml
[levels.error.level]
foreground = "white"
background = "red"
modes = ["bold"]

[levels.warning.level]
foreground = "yellow"
modes = ["reverse"]
```

## Related Topics

- [Custom Themes](./themes-custom.md) — creating your own themes
- [Theme Overlays](./themes-overlays.md) — modifying themes with overlays
- [Configuration Files](./config-files.md) — persistent theme configuration