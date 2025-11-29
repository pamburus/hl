# styled-help

A proc macro for adding styled text to clap help output using doc comments.

## Overview

This crate provides a `#[styled_help]` attribute macro that transforms doc comments containing style markers into `help` and `long_help` attributes that use `color_print::cstr!` for styling. Doc comments without style markers are left untouched for clap to process normally, ensuring compatibility with clap's default behavior (like automatic period trimming in short help).

## Features

- Write styled help text directly in doc comments
- Only processes doc comments that contain style markers
- Doc comments without style markers are left for clap to process normally
- Preserves existing `help` and `long_help` attributes (doesn't override them)
- Automatically handles period trimming for short vs long help
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

The `#[styled_help]` macro will transform only the doc comments with style markers:

```rust
/// Sort messages using <c>--sync-interval-ms</> option
#[arg(long)]
sort: bool,
// Becomes:
#[arg(long, help = color_print::cstr!("..."), long_help = color_print::cstr!("..."))]
sort: bool,

/// Enable verbose mode
#[arg(short, long)]
verbose: bool,
// Left as-is (no style markers, clap handles it)
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

Doc comments without style markers are left untouched for clap to process:

```rust
#[styled_help]
#[derive(Parser)]
struct Opt {
    /// This is plain help text without styling
    #[arg(long)]
    plain: bool,
}
```

The doc comment remains as-is, and clap handles it normally (including automatic period trimming in short help).

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
   - It checks if any doc comment contains style markers
   - **If style markers found**:
     - Removes the doc comments
     - Generates both `help` and `long_help` attributes with `color_print::cstr!(...)`
     - Strips trailing period from `help` (short help), keeps it in `long_help`
   - **If no style markers**: Leaves doc comments untouched for clap to process normally

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
- **Smart processing**: Only intervenes when styling is needed
- **Clap-compatible**: Plain doc comments are handled by clap itself
- **Period handling**: Correctly strips periods in short help, keeps them in long help

## License

MIT