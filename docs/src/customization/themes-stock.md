# Stock Themes

`hl` comes with a variety of built-in themes optimized for different terminal color capabilities and personal preferences. This page describes all available stock themes and their characteristics.

## Theme Categories

Themes are organized by color capability:

- **16-color themes** — Work with standard 16-color terminals; maximum compatibility.
- **256-color themes** — Require 256-color terminal support; richer color palette.
- **Truecolor (24-bit) themes** — Require truecolor terminal support; full RGB color range.

Most themes are also tagged as either:

- **Dark** — Optimized for dark terminal backgrounds.
- **Light** — Optimized for light terminal backgrounds.

Special tags include:

- **Base** — Base theme used as a foundation for other themes.
- **Overlay** — Theme overlays that modify existing themes.

## Available Themes

### Universal (Multi-Mode)

#### **uni** {#uni}
- Tags: **`dark`** **`light`** **`16-color`**
- The default theme; works on all terminals with both dark and light backgrounds.
- Balances readability and compatibility.
- Uses only the standard 16 ANSI colors.

<picture>
  <source srcset="https://raw.githubusercontent.com/pamburus/hl-extra/refs/heads/main/screenshot/uni/dark.svg" media="(prefers-color-scheme: dark)">
  <img src="https://raw.githubusercontent.com/pamburus/hl-extra/refs/heads/main/screenshot/uni/light.svg" alt="screenshot-uni">
</picture>

#### **universal** {#universal}
- Tags: **`dark`** **`light`** **`16-color`**
- Variant of **uni** with green accent colors and reversed warning and errors levels.
- Works on both dark and light backgrounds.
- Balances readability and compatibility.
- Uses only the standard 16 ANSI colors.

<picture>
  <source srcset="https://raw.githubusercontent.com/pamburus/hl-extra/refs/heads/main/screenshot/universal/dark.svg" media="(prefers-color-scheme: dark)">
  <img src="https://raw.githubusercontent.com/pamburus/hl-extra/refs/heads/main/screenshot/universal/light.svg" alt="screenshot-universal">
</picture>

#### **universal-blue** {#universal-blue}
- Tags: **`dark`** **`light`** **`16-color`**
- Variant of **universal** with blue accent colors.
- Works on both dark and light backgrounds.

<picture>
  <source srcset="https://raw.githubusercontent.com/pamburus/hl-extra/refs/heads/main/screenshot/universal-blue/dark.svg" media="(prefers-color-scheme: dark)">
  <img src="https://raw.githubusercontent.com/pamburus/hl-extra/refs/heads/main/screenshot/universal-blue/light.svg" alt="screenshot-universal-blue">
</picture>

#### **neutral** {#neutral}
- Tags: **`dark`** **`light`** **`16-color`**
- Minimal colorization; emphasizes log levels, warnings and errors only.
- Most content displayed in terminal default colors.
- Suitable for minimal or monochrome aesthetics.

<picture>
  <source srcset="https://raw.githubusercontent.com/pamburus/hl-extra/refs/heads/main/screenshot/neutral/dark.svg" media="(prefers-color-scheme: dark)">
  <img src="https://raw.githubusercontent.com/pamburus/hl-extra/refs/heads/main/screenshot/neutral/light.svg" alt="screenshot-neutral">
</picture>

#### **frostline** {#frostline}
- Tags: **`dark`** **`light`** **`16-color`**
- Cool cyan-based theme with bold syntax highlighting.
- Works on both dark and light backgrounds.
- Distinguishes `true`/`false` boolean values with different colors.

<picture>
  <source srcset="https://raw.githubusercontent.com/pamburus/hl-extra/refs/heads/main/screenshot/frostline/dark.svg" media="(prefers-color-scheme: dark)">
  <img src="https://raw.githubusercontent.com/pamburus/hl-extra/refs/heads/main/screenshot/frostline/light.svg" alt="screenshot-frostline">
</picture>

### Dark Themes (256-color)

**`hl-dark`** (dark, 256-color)
- Modern dark theme with a teal/cyan primary color palette.
- Faint level labels with distinct inner level colors.
- Distinct colors for `true` vs `false` booleans.

### Light Themes (256-color)

**`hl-light`** (light, 256-color)
- Modern light theme with dark text on light backgrounds.
- Faint level labels with distinct inner level colors.
- Distinct colors for `true` vs `false` booleans.

### Dark Themes (Truecolor / 24-bit)

**`one-dark-24`** (dark, truecolor)
- Inspired by the popular "One Dark" editor theme.
- Cyan primary colors with blue accents.
- Distinct color coding for different value types.

**`ayu-dark-24`** (dark, truecolor)
- Based on the Ayu color scheme (dark variant).
- Soft, balanced colors with green numbers and purple debug level.
- Warning and error messages inherit level colors.

**`ayu-mirage-24`** (dark, truecolor)
- Ayu color scheme (mirage variant); a softer, mid-tone dark theme.
- Blue-gray accents with balanced contrast.

**`tc24d-blue`** (dark, truecolor)
- Dark truecolor theme with blue accents and reverse video level highlighting.
- Bold syntax; error-styled null values.

**`tc24d-b2`** (dark, truecolor)
- Dark truecolor theme variant with bold syntax and balanced colors.

### Light Themes (Truecolor / 24-bit)

**`one-light-24`** (light, truecolor)
- Light counterpart to `one-dark-24`.
- Royal blue accents with dark text on light backgrounds.

**`ayu-light-24`** (light, truecolor)
- Ayu color scheme (light variant).
- Clear, readable colors optimized for light backgrounds.
- Warning and error messages inherit level colors; red error messages.

**`tc24l-b2`** (light, truecolor)
- Light truecolor theme with royal blue accents.
- Bold syntax highlighting; dark text on light background.

**`tc24l-blue`** (light, truecolor)
- Light truecolor theme variant with blue color scheme.


## Listing Available Themes

To see all available themes with their tags:

```sh
hl --list-themes
```

To see themes filtered by specific tags (e.g., only dark themes):

```sh
hl --list-themes=dark
```

To see dark 256-color themes:

```sh
hl --list-themes=dark,256color
```

## Theme Overlays

In addition to full themes, `hl` provides **theme overlays** that modify the active theme:

**`@accent-italic`** (overlay)
- Makes accent text (e.g., logger names, field names) italic.
- Apply by configuring in your config file: `theme-overlays = ["@accent-italic"]`.

**`@base`** (base)
- The fundamental base theme defining the default styling structure.
- Typically not used directly; serves as the foundation for other themes.

See [Theme Overlays](themes-overlays.md) for more details on using overlays.

## Next Steps

- [Selecting Themes](themes-selecting.md) — Learn how to choose and activate themes.
- [Custom Themes](themes-custom.md) — Create your own themes.
- [Theme Overlays](themes-overlays.md) — Modify themes with overlays.
