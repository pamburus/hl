# Theme Overlays

Theme overlays are small, focused theme modifications that can be applied on top of any base theme. They allow you to make targeted adjustments without creating a complete theme from scratch.

## What Are Theme Overlays?

A theme overlay is a special type of theme file that:

- Contains only the styles you want to modify.
- Is tagged with `"overlay"` in its tags list.
- Can be combined with any base theme.
- Multiple overlays can be stacked together.

Overlays are perfect for:

- Making small tweaks to stock themes (e.g., making accents italic).
- Applying consistent customizations across multiple base themes.
- Experimenting with modifications without editing the base theme.

## Using Theme Overlays

### Apply an Overlay

Use `--theme-overlay` to apply an overlay to the active theme:

```hl/dev/null/shell.sh#L1
hl --theme one-dark-24 --theme-overlay @accent-italic app.log
```

The base theme (`one-dark-24`) is loaded first, then the overlay (`@accent-italic`) is merged on top.

### Apply Multiple Overlays

You can stack multiple overlays (they are applied in order):

```hl/dev/null/shell.sh#L1
hl --theme classic --theme-overlay @accent-italic --theme-overlay @my-tweaks app.log
```

Each overlay is merged sequentially, so later overlays can override earlier ones.

### Overlays with Default Theme

If you don't specify a base theme, overlays are applied to the default theme (`universal`):

```hl/dev/null/shell.sh#L1
hl --theme-overlay @accent-italic app.log
```

### Environment Variable

Set a default overlay for all invocations:

```hl/dev/null/shell.sh#L1
export HL_THEME_OVERLAY=@accent-italic
hl app.log  # Applies @accent-italic to the default theme
```

### Configuration File

Add overlays to your config file:

```hl/dev/null/config.toml#L1
[theme]
name = "one-dark-24"
overlays = ["@accent-italic"]
```

## Built-in Overlays

`hl` includes the following built-in overlay:

### `@accent-italic`

Makes accent text (logger names, field names, etc.) display in italic:

```hl/dev/null/shell.sh#L1
hl --theme-overlay @accent-italic app.log
```

This overlay works with any theme and any terminal that supports italic text.

## Creating Custom Overlays

### Overlay File Location

Like regular themes, overlay files are stored in:

```hl/dev/null/path.txt#L1
~/.config/hl/themes/
```

Overlay filenames conventionally start with `@` (e.g., `@my-overlay.toml`), but this is not required.

### Overlay File Structure

An overlay is a theme file with the `"overlay"` tag:

```hl/dev/null/my-overlay.toml#L1
#:schema https://raw.githubusercontent.com/pamburus/hl/v0.35.2/schema/json/theme.schema.v1.json

version = "1.1"
tags = ["overlay", "dark", "light"]

[styles]
accent.modes = ["italic"]
```

The `"overlay"` tag tells `hl` that this theme should be merged with a base theme rather than used standalone.

### Minimal Overlay Example

Here's a simple overlay that makes error messages bold and underlined:

```hl/dev/null/@bold-errors.toml#L1
version = "1.1"
tags = ["overlay", "dark", "light"]

[levels.error]
message.modes = ["bold", "underline"]
```

Save this as `~/.config/hl/themes/@bold-errors.toml` and use it with:

```hl/dev/null/shell.sh#L1
hl --theme-overlay @bold-errors app.log
```

### Overlay for Multiple Themes

You can create overlays that work with both dark and light themes by tagging appropriately:

```hl/dev/null/@subtle-accents.toml#L1
version = "1.1"
tags = ["overlay", "dark", "light", "16color", "256color", "truecolor"]

[styles]
accent.modes = ["faint"]
accent-secondary.modes = ["faint"]
```

This overlay will work with any theme because it only modifies modes, not colors.

## Practical Examples

### Example 1: Underline All Keys

Create an overlay to underline all field keys:

```hl/dev/null/@underline-keys.toml#L1
version = "1.1"
tags = ["overlay", "dark", "light"]

[elements]
key.modes = ["underline"]
```

Usage:

```hl/dev/null/shell.sh#L1
hl --theme one-dark-24 --theme-overlay @underline-keys app.log
```

### Example 2: Dim Timestamps

Make timestamps less prominent:

```hl/dev/null/@dim-time.toml#L1
version = "1.1"
tags = ["overlay", "dark", "light"]

[elements]
time.modes = ["faint"]
```

### Example 3: Color-Specific Overlay

Create an overlay that changes number colors for dark themes only:

```hl/dev/null/@green-numbers-dark.toml#L1
version = "1.1"
tags = ["overlay", "dark", "truecolor"]

[elements]
number.foreground = "#00ff00"
```

This overlay should only be used with dark truecolor themes.

### Example 4: Multi-Element Overlay

Combine several modifications:

```hl/dev/null/@my-style.toml#L1
version = "1.1"
tags = ["overlay", "dark", "light"]

[styles]
accent.modes = ["italic"]

[elements]
key.modes = ["underline"]
caller.modes = ["faint", "italic"]
message.modes = ["bold"]

[levels.error]
message.foreground = "bright-red"
message.modes = ["bold", "underline"]
```

## How Overlays Merge

When an overlay is applied:

1. The base theme is loaded first.
2. The overlay is merged on top.
3. For each style/element in the overlay:
   - If it defines `foreground`, `background`, or `modes`, those properties **replace** the base theme's values.
   - If it defines `style` (inheritance), it **replaces** the base inheritance chain.
   - Undefined properties are left unchanged from the base theme.

### Merge Behavior Example

Base theme:

```hl/dev/null/base.toml#L1
[elements]
key.foreground = "blue"
key.modes = ["bold"]
```

Overlay:

```hl/dev/null/overlay.toml#L1
[elements]
key.modes = ["italic"]
```

Result:

```hl/dev/null/result.txt#L1
key.foreground = "blue"       # From base (unchanged)
key.modes = ["italic"]        # From overlay (replaces base modes)
```

If you want to **add** modes rather than replace them, you must specify all desired modes in the overlay:

```hl/dev/null/overlay-add.toml#L1
[elements]
key.modes = ["bold", "italic"]  # Explicitly include both
```

## Tips for Creating Overlays

- **Tag correctly** — Always include `"overlay"` in the tags. Include `"dark"` and/or `"light"` to indicate which base themes the overlay is designed for.
- **Be minimal** — Only define what you want to change. Leave everything else undefined so it inherits from the base theme.
- **Test with multiple bases** — If your overlay is intended to work with multiple themes, test it with several to ensure compatibility.
- **Use mode modifiers** — Prefer modifying `modes` over colors for maximum compatibility across different base themes.
- **Document your overlay** — Add comments to explain what the overlay does and which themes it works best with.

## Combining Overlays and Custom Themes

You can use overlays with your own custom themes:

```hl/dev/null/shell.sh#L1
hl --theme my-custom-theme --theme-overlay @accent-italic app.log
```

This allows you to create a base custom theme and then apply small modifications via overlays without duplicating theme files.

## Troubleshooting

### Overlay Not Applied

If an overlay doesn't seem to apply:

- Verify the `"overlay"` tag is present in the theme file.
- Check the overlay file location (`~/.config/hl/themes/`).
- Ensure the filename matches what you're passing to `--theme-overlay`.

### Unexpected Results

If the overlay produces unexpected results:

- Remember that overlay properties **replace** base theme properties (they don't merge element-wise for arrays like `modes`).
- Check the order of multiple overlays (later overlays override earlier ones).
- Test the overlay with different base themes to see if it's theme-specific.

### Overlay Conflicts

If multiple overlays conflict:

- The last overlay applied wins for any given property.
- Use `--theme-overlay` multiple times to control order:
  ```hl/dev/null/shell.sh#L1
  hl --theme-overlay @first --theme-overlay @second app.log
  ```
  `@second` will override any conflicting properties from `@first`.

## Next Steps

- [Stock Themes](themes-stock.md) — See built-in overlays and themes.
- [Custom Themes](themes-custom.md) — Learn the full theme file format.
- [Selecting Themes](themes-selecting.md) — Understand theme selection and priority.
