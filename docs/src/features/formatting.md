# Output Formatting

`hl` provides extensive control over how log entries are displayed. This page provides an overview of the formatting options available, with links to detailed documentation for each feature.

## Overview

Output formatting in `hl` covers several aspects:

| Feature | Description | Documentation |
|---------|-------------|---------------|
| **Field visibility** | Choose which fields to show or hide | [Field Visibility](./field-visibility.md) |
| **Time display** | Format timestamps and set timezones | [Time Display](./time-display.md) |
| **Field expansion** | Control how multi-line values are displayed | [Field Expansion](./field-expansion.md) |
| **Raw output** | Output original JSON/logfmt instead of formatted output | [Raw Output](./raw-output.md) |
| **Colors and themes** | Visual styling and color schemes | [Themes](../customization/themes.md) |

## Quick Reference

| Option | Short | Description |
|--------|-------|-------------|
| `--hide <KEY>` | `-h` | Hide or reveal fields |
| `--time-format <FMT>` | `-t` | Set timestamp format |
| `--time-zone <TZ>` | `-Z` | Set display timezone |
| `--local` | `-L` | Use local timezone |
| `--expansion <MODE>` | `-x` | Control field expansion |
| `--raw` | `-r` | Output original format |
| `--theme <NAME>` | | Select color theme |
| `--hide-empty-fields` | `-e` | Hide fields with empty values |
| `--flatten <WHEN>` | | Control nested object flattening |

## Field Visibility

Control which fields appear in the output. You can hide specific fields, hide all fields except certain ones, or use patterns.

See [Field Visibility](./field-visibility.md) for syntax, patterns, and examples.

## Time Display

Customize how timestamps are displayed, including format and timezone.

- **Default format**: `%b %d %T.%3N` (e.g., `Jan 15 10:30:45.123`)
- **Default timezone**: UTC

See [Time Display](./time-display.md) for format specifiers and timezone handling.

## Field Expansion

Control how multi-line field values (such as stack traces or error details) are displayed. Available modes: `never`, `inline`, `auto` (default), `always`.

See [Field Expansion](./field-expansion.md) for mode descriptions and examples.

## Raw Output

Output the original JSON or logfmt instead of formatted output. Useful for piping to other tools like `jq`.

See [Raw Output](./raw-output.md) for details.

## Empty Field Handling

Hide fields with empty values (null, empty string, empty object, or empty array) using `-e` or `--hide-empty-fields`.

## Object Flattening

Control whether nested objects are flattened into dot-notation fields:

- `--flatten always` (default): `user.id=123 user.name=Alice`
- `--flatten never`: `user={ id=123 name=Alice }`

## Colors and Themes

Control color usage and visual styling with `--theme` and `--color`.

See [Themes](../customization/themes.md) for available themes and customization.

## Input Info Display

When processing multiple files, control how source file information is displayed with `--input-info`.

See [Multiple Files](./multiple-files.md) for details.

## Configuration

All formatting options can be saved in configuration files or set via environment variables.

See:
- [Configuration Files](../customization/config-files.md)
- [Environment Variables](../customization/environment.md)

## Related Topics

- [Field Visibility](./field-visibility.md) — controlling which fields are shown
- [Time Display](./time-display.md) — timestamp formatting and timezones
- [Field Expansion](./field-expansion.md) — multi-line value display
- [Raw Output](./raw-output.md) — outputting original format
- [Themes](../customization/themes.md) — color schemes and styling