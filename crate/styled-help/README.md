# styled-help

A proc macro for adding styled text to clap help output using doc comments.

## Overview

This crate provides a `#[styled_help]` attribute macro that transforms doc comments containing style markers into `help` attributes that use `color_print::cstr!` for styling. This allows you to write styled help text directly in doc comments without the boilerplate of separate `help` attributes.

## Features

- Write styled help text directly in doc comments
- Automatically converts style markers to `color_print::cstr!` format
- Preserves existing `help` and `long_help` attributes (doesn't override them)
- Only applies styling when style markers are detected
- Works seamlessly with clap's derive API

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
color-print = "0.3"
styled-help = { path = "./crate/styled-help" }
```

## Usage

### Basic Example

```rust
use clap::Parser;
use styled_help::styled_help;

#[styled_help]
#[derive(Parser)]
struct Opt {
    /// Sort messages using <c>--sync-interval-ms</> option
    #[arg(long)]
    sort: bool,

    /// Enable <c>verbose</> mode with <g>colored</> output
    #[arg(short, long)]
    verbose: bool,
}
```

The `#[styled_help]` macro will transform the doc comments into:

```rust
#[arg(long, help = color_print::cstr!("Sort messages using <c>--sync-interval-ms</> option"))]
sort: bool,

#[arg(short, long, help = color_print::cstr!("Enable <c>verbose</> mode with <g>colored</> output"))]
verbose: bool,
```

### Supported Style Markers

The macro supports all `color-print` style markers:

- `<c>text</>` - Cyan (for commands/options)
- `<r>text</>` - Red
- `<g>text</>` - Green
- `<b>text</>` - Blue
- `<y>text</>` - Yellow
- `<m>text</>` - Magenta
- `<k>text</>` - Black
- `<white>text</>` - White
- `<cyan>text</>` - Cyan (alternative)
- `<s>text</>` - Bold/strong
- `<u>text</>` - Underline

See the [color-print documentation](https://docs.rs/color-print) for all available markers.

### Multi-line Doc Comments

The macro automatically combines multi-line doc comments:

```rust
#[styled_help]
#[derive(Parser)]
struct Opt {
    /// This is the first line of help text.
    /// This is the second line with <c>styled</> text.
    /// And this is the third line.
    #[arg(long)]
    field: String,
}
```

### Plain Doc Comments

Doc comments without style markers are converted to plain `help` attributes:

```rust
#[styled_help]
#[derive(Parser)]
struct Opt {
    /// This is plain help text without styling
    #[arg(long)]
    plain: bool,
}
```

This becomes:

```rust
#[arg(long, help = "This is plain help text without styling")]
plain: bool,
```

### Preserving Existing Help Attributes

If a field already has a `help` or `long_help` attribute, the macro will not process its doc comments:

```rust
#[styled_help]
#[derive(Parser)]
struct Opt {
    /// This doc comment will be ignored
    #[arg(long, help = "Explicit help text takes precedence")]
    field: String,
}
```

## How It Works

1. The `#[styled_help]` attribute macro is applied to your clap Parser struct
2. For each field in the struct:
   - It checks if there's already a `help` or `long_help` attribute (if so, skips the field)
   - It collects all doc comments (`///` or `#[doc = "..."]`)
   - It combines them into a single string
   - If style markers are detected, it generates `help = color_print::cstr!("...")`
   - If no style markers, it generates a plain `help = "..."`
   - It removes the original doc comments to avoid duplication

## Comparison

### Without styled-help

```rust
mod help {
    pub const SORT: &str = cstr!("Sort messages using <c>--sync-interval-ms</> option");
    pub const FOLLOW: &str = cstr!("Follow logs with <c>--tail</> support");
}

#[derive(Parser)]
struct Opt {
    #[arg(long, help = help::SORT)]
    sort: bool,

    #[arg(long, help = help::FOLLOW)]
    follow: bool,
}
```

### With styled-help

```rust
#[styled_help]
#[derive(Parser)]
struct Opt {
    /// Sort messages using <c>--sync-interval-ms</> option
    #[arg(long)]
    sort: bool,

    /// Follow logs with <c>--tail</> support
    #[arg(long)]
    follow: bool,
}
```

## Benefits

- **Less boilerplate**: No need for separate help constants or modules
- **Co-location**: Help text lives next to the field definition
- **Standard syntax**: Uses familiar doc comment syntax
- **Flexibility**: Automatically handles both styled and plain text
- **Compatible**: Works with all clap derive features

## License

MIT