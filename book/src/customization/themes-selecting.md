# Selecting Themes

`hl` provides multiple ways to select and activate themes. You can choose themes at runtime via command-line options, set defaults through environment variables, or configure persistent preferences in configuration files.

## Command-Line Selection

### Basic Theme Selection

Use the `--theme` option to select a theme:

```hl/dev/null/shell.sh#L1
hl --theme one-dark-24 app.log
```

## Theme Overlays

Theme overlays modify the active theme. To use overlays, add them to your configuration file (see below).

## Environment Variables

### Setting a Default Theme

Set the `HL_THEME` environment variable to choose a default theme for all `hl` invocations:

```hl/dev/null/shell.sh#L1
export HL_THEME=one-dark-24
hl app.log  # Uses one-dark-24
```

The command-line `--theme` option overrides the environment variable:

```hl/dev/null/shell.sh#L1
export HL_THEME=one-dark-24
hl --theme classic app.log  # Uses classic, not one-dark-24
```

## Configuration Files

### Setting Theme in Config

You can set a default theme in your configuration file (`~/.config/hl/config.toml` or similar):

```hl/dev/null/config.toml#L1
theme = "one-dark-24"
```

To apply theme overlays (configuration file only):

```hl/dev/null/config.toml#L1
theme = "ayu-dark-24"
theme-overlays = ["@accent-italic"]
```

### Priority and Layering

Configuration settings are layered with the following priority (lowest to highest):

1. Embedded default configuration (theme = `universal`)
2. System configuration files (e.g., `/etc/hl/config.toml`)
3. User configuration file (e.g., `~/.config/hl/config.toml`)
4. `HL_THEME` environment variable
5. Command-line `--theme` option

Later layers override earlier ones for the theme name. Theme overlays are only configurable via the configuration file's `theme-overlays` array.

## Discovering Available Themes

### List All Themes

To see all available themes:

```hl/dev/null/shell.sh#L1
hl --list-themes
```

This displays each theme name and its tags (e.g., `dark`, `light`, `16color`, `256color`, `truecolor`).

### Filter by Tags

To list only themes with specific tags:

```hl/dev/null/shell.sh#L1
# Show only dark themes
hl --list-themes=dark

# Show 256-color themes
hl --list-themes=256color

# Show truecolor themes
hl --list-themes=truecolor
```

## Choosing the Right Theme

Consider the following when selecting a theme:

- **Terminal color support** — Use `--list-themes` with tag filters to find themes compatible with your terminal (16-color, 256-color, or truecolor).
- **Background color** — Choose a `dark` theme for dark backgrounds, `light` for light backgrounds, or `base` for either.
- **Personal preference** — Try different themes to find one that suits your taste and readability needs.

### Testing Themes

You can quickly test a theme on sample logs:

```hl/dev/null/shell.sh#L1
# Test one-dark-24
hl --theme one-dark-24 app.log | head -n 20

# Compare two themes side-by-side (in separate terminals)
hl --theme ayu-dark-24 app.log
hl --theme one-dark-24 app.log
```

### Checking Terminal Color Support

Most modern terminals support 256 colors or truecolor. To check your terminal's capabilities:

```hl/dev/null/shell.sh#L1
# Check TERM environment variable
echo $TERM

# Common values:
# xterm-256color, screen-256color → 256-color support
# xterm-truecolor, alacritty, kitty → truecolor support
```

If you're unsure, start with a 16-color theme like `universal` or `classic` for maximum compatibility.

## Examples

### Temporary Theme Change

Use a different theme for a single command without changing defaults:

```hl/dev/null/shell.sh#L1
hl --theme ayu-dark-24 /var/log/app.log
```

### Set a Persistent Default

Add to your shell profile (`~/.bashrc`, `~/.zshrc`, etc.):

```hl/dev/null/shell.sh#L1
export HL_THEME=one-dark-24
```

And configure theme overlays in `~/.config/hl/config.toml`:

```hl/dev/null/config.toml#L1
theme = "one-dark-24"
theme-overlays = ["@accent-italic"]
```

### Switching Themes by Time of Day

You can use shell aliases or functions to switch themes based on context:

```hl/dev/null/shell.sh#L1
# In ~/.bashrc or ~/.zshrc
alias hl-dark='hl --theme one-dark-24'
alias hl-light='hl --theme one-light-24'
```

## Next Steps

- [Stock Themes](themes-stock.md) — Browse all available built-in themes.
- [Custom Themes](themes-custom.md) — Create your own themes.
- [Theme Overlays](themes-overlays.md) — Learn how overlays work and create custom overlays.
