# Themes

Themes control the visual appearance of `hl` output, including colors, text styles, and formatting. `hl` ships with a variety of built-in themes and supports custom theme creation.

## Overview

Themes in `hl` define:

- **Colors** for different log elements (timestamps, levels, fields, values)
- **Text styles** (bold, italic, underline, dim)
- **Visual modes** (reverse, strikethrough)
- **Element-specific styling** (numbers, strings, booleans, nulls)
- **Level-specific colors** (error, warning, info, debug, trace)

## Selecting a Theme

### Command Line

Use the `--theme` option:

```bash
# Use universal theme
hl --theme universal app.log

# Use monokai-inspired theme
hl --theme one-dark-24 app.log

# Use light theme
hl --theme ayu-light-24 app.log
```

### Environment Variable

Set a default theme:

```bash
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

```bash
# List all themes
hl --list-themes

# Filter by tags
hl --list-themes=dark
hl --list-themes=light
hl --list-themes=16color
hl --list-themes=256color
hl --list-themes=truecolor
```

Output example:
```
ayu-dark-24
ayu-light-24
classic
classic-light
hl-dark
hl-light
universal
one-dark-24
one-light-24
```

## Stock Themes

`hl` includes several stock themes optimized for different preferences and terminal capabilities.

### Universal Themes

**universal** (alias: `uni`)
- Works on both light and dark backgrounds
- Uses 16 basic ANSI colors
- Maximum compatibility
- Clean, readable output

```bash
hl --theme universal app.log
```

**universal-blue**
- Variant with blue accents
- Good for terminals with modified color schemes

### Classic Themes

**classic**
- Traditional log viewer appearance
- Dark background optimized
- Familiar color scheme

**classic-light**
- Light background variant
- Softer colors for reduced eye strain

**classic-plus**
- Enhanced classic theme with additional visual cues

### Modern 24-bit Themes

**ayu-dark-24**
- Based on the popular Ayu color scheme
- Requires 24-bit color (true color) support
- Modern, aesthetic appearance

**ayu-light-24**
- Light background variant of Ayu
- Excellent readability on light terminals

**ayu-mirage-24**
- Medium contrast variant
- Balanced between dark and light

**one-dark-24**
- Based on Atom's One Dark theme
- Popular among developers
- Requires true color support

**one-light-24**
- Light variant of One theme
- Clean and professional

### Color Capability Themes

**16-color themes**: `universal`, `classic`, `classic-light`
- Maximum compatibility
- Work on any color-capable terminal

**256-color themes**: `lsd`, `neutral`
- Enhanced color palette
- Requires 256-color terminal support

**24-bit themes**: `ayu-*`, `one-*`, `tc24*`
- Full RGB color support
- Requires true color terminal

### Special Purpose Themes

**neutral**
- Minimal color usage
- Focuses on content over aesthetics
- Good for environments with custom color schemes

**lsd**
- Inspired by the lsd (LSDeluxe) file listing tool
- Bright, colorful output

**dmt**
- Dark Monokai-inspired theme
- Popular with code editors

**frostline**
- Cool color palette
- Blue and cyan emphasis

See [Stock Themes](./themes-stock.md) for detailed descriptions and screenshots.

## Theme Overlays

Overlays are mini-themes that modify specific aspects without replacing the entire theme.

### Available Overlays

**@accent-italic**
- Applies italic style to accent elements
- Adds visual emphasis
- Default overlay in configuration

### Using Overlays

**Command line:**
```bash
# Apply italic accent overlay
hl --theme universal --theme-overlay @accent-italic app.log
```

**Configuration file:**
```toml
theme = "universal"
theme-overlays = ["@accent-italic"]
```

Multiple overlays can be applied in order:
```toml
theme-overlays = ["@accent-italic", "custom-overlay"]
```

See [Theme Overlays](./themes-overlays.md) for creating custom overlays.

## Theme Structure

Themes are TOML files with this structure:

```toml
#:schema https://raw.githubusercontent.com/pamburus/hl/v0.35.2/schema/json/theme.schema.v1.json

version = "1.1"
tags = ["dark", "16color"]

[styles]
# General purpose styles
[styles.accent]
foreground = "green"
modes = ["-faint"]

[styles.warning]
foreground = "yellow"

[elements]
# Element-specific styles
[elements.number]
foreground = "bright-blue"

[elements.string]
foreground = "green"

[levels]
# Level-specific styles
[levels.error.level]
style = ["level", "error"]
modes = ["reverse"]
```

## Color Specifications

Themes can use several color formats:

### Named ANSI Colors

Basic 16 colors:
```toml
foreground = "red"
foreground = "bright-blue"
foreground = "black"
```

Available: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`, and `bright-*` variants.

### 256-Color Palette

```toml
foreground = 214  # Orange-ish
background = 235  # Dark gray
```

### 24-bit RGB

```toml
foreground = "#FF5733"  # Hex notation
foreground = "rgb(255, 87, 51)"  # RGB function
```

### Special Values

```toml
foreground = "default"  # Terminal default color
```

## Text Styles and Modes

Apply text attributes:

```toml
[styles.emphasis]
foreground = "yellow"
modes = ["bold", "italic"]

[styles.subtle]
foreground = "white"
modes = ["dim"]  # Faint/dim text
```

Available modes:
- `bold` — bold text
- `italic` — italic text (if terminal supports)
- `underline` — underlined text
- `dim` / `faint` — dimmed text
- `reverse` — swap foreground/background
- `strikethrough` — strikethrough text
- `-mode` — disable a mode (e.g., `-faint`)

## Theme Elements

Themes can style these elements:

### General Elements

- `number` — numeric values
- `string` — string values
- `boolean` — true/false values
- `null` — null values
- `key` — field keys
- `punctuation` — separators and delimiters

### Log-Specific Elements

- `time` — timestamps
- `level` — log level indicator
- `logger` — logger name
- `caller` — caller information
- `message` — log message

### Level-Specific Elements

Each log level can have custom styling:

```toml
[levels.error.time]
foreground = "bright-red"

[levels.error.level]
foreground = "white"
background = "red"
modes = ["bold"]

[levels.warning.level]
foreground = "yellow"
modes = ["reverse"]
```

## Terminal Compatibility

### Checking Color Support

Most modern terminals support at least 256 colors. To verify:

```bash
# Check TERM variable
echo $TERM

# Test 256 colors
hl --theme ayu-dark-24 app.log
```

If colors don't display correctly, use a 16-color theme:
```bash
hl --theme universal app.log
```

### Common Terminal Capabilities

| Terminal | 16-color | 256-color | 24-bit |
|----------|----------|-----------|--------|
| xterm-256color | ✓ | ✓ | ✓ |
| iTerm2 | ✓ | ✓ | ✓ |
| Alacritty | ✓ | ✓ | ✓ |
| Windows Terminal | ✓ | ✓ | ✓ |
| GNOME Terminal | ✓ | ✓ | ✓ |
| macOS Terminal.app | ✓ | ✓ | ✓ (10.12+) |
| tmux | ✓ | ✓ | ✓ (with config) |
| screen | ✓ | ✓ | ✗ |

### Troubleshooting Colors

**Colors not showing:**
```bash
# Force color output
hl --color always --theme universal app.log

# Check if terminal supports colors
echo $COLORTERM
```

**Wrong colors:**
```bash
# Use 16-color theme for compatibility
hl --theme universal app.log

# Check TERM setting
echo $TERM
```

**Colors in pager:**
```bash
# Ensure pager preserves colors
LESS=-R hl --theme universal app.log
```

## Custom Themes

Create your own themes for specific needs.

### Creating a Theme

1. Create a TOML file: `~/.config/hl/themes/my-theme.toml`
2. Define theme structure
3. Use it: `hl --theme my-theme app.log`

Basic custom theme:
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

See [Custom Themes](./themes-custom.md) for detailed guide.

## Theme Use Cases

### Development

```bash
# Readable on any background
hl --theme universal dev.log

# Modern aesthetic with full colors
hl --theme ayu-dark-24 dev.log
```

### Production Monitoring

```bash
# Clear, high-contrast for quick scanning
hl --theme classic-plus production.log

# Neutral colors to avoid alert fatigue
hl --theme neutral production.log
```

### Light Background

```bash
# Optimized for light terminals
hl --theme ayu-light-24 app.log
hl --theme one-light-24 app.log
hl --theme classic-light app.log
```

### Dark Background

```bash
# Dark theme options
hl --theme ayu-dark-24 app.log
hl --theme one-dark-24 app.log
hl --theme classic app.log
```

### Maximum Compatibility

```bash
# Works everywhere
hl --theme universal app.log

# Classic appearance
hl --theme classic app.log
```

## Best Practices

1. **Choose by terminal capabilities** — use 16-color themes for maximum compatibility
2. **Match your background** — use light themes for light terminals, dark for dark
3. **Test in actual environment** — colors may look different across terminals
4. **Use overlays for tweaks** — modify existing themes instead of creating new ones
5. **Set a default** — configure your preferred theme in config file

## Examples

### Personal Development Setup

```bash
# ~/.bashrc or ~/.zshrc
export HL_THEME=universal
```

```toml
# ~/.config/hl/config.toml
theme = "universal"
theme-overlays = ["@accent-italic"]
```

### Team Standard

```bash
# Project .hl.toml
theme = "one-dark-24"
```

### CI/CD Environment

```bash
# Disable colors in CI
export HL_COLOR=never
```

Or use a minimal theme:
```bash
export HL_THEME=neutral
```

### Light Terminal Setup

```toml
# ~/.config/hl/config.toml
theme = "ayu-light-24"
```

## Related Topics

- [Stock Themes](./themes-stock.md) — detailed descriptions of built-in themes
- [Selecting Themes](./themes-selecting.md) — theme selection and switching
- [Custom Themes](./themes-custom.md) — creating your own themes
- [Theme Overlays](./themes-overlays.md) — modifying themes with overlays
- [Configuration Files](./config-files.md) — persistent theme configuration