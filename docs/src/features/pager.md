# Automatic Pager Integration

hl automatically integrates with a pager to make viewing large log files comfortable and efficient. This page explains how pager integration works and how to customize it.

## Configuration

| Method | Setting | Values |
|--------|---------|--------|
| CLI option | [`--paging`](../reference/options.md#paging), [`-P`](../reference/options.md#-p) | `auto` (default), `always`, `never` |
| Environment | [`HL_PAGING`](../customization/environment.md#hl-paging) | `auto` (default), `always`, `never` |
| Environment | [`HL_PAGER`](../customization/environment.md#hl-pager) | Program name (e.g., `less`, `bat`, `most`) |

## Default Behavior

When you run hl with a log file, it automatically opens the output in a pager:

```sh
hl app.log
```

By default, hl uses the `less` pager if available, which provides:
- Scrolling through large files
- Search functionality
- Pattern highlighting
- Easy navigation

## Pager Selection

hl determines which pager to use in this order:

1. The `HL_PAGER` environment variable
2. The `PAGER` environment variable
3. `less` (if available in PATH)
4. Falls back to direct output if no pager is found

### Using a Different Pager

Set your preferred pager using the `PAGER` environment variable:

```sh
PAGER=most hl app.log
```

Or use `HL_PAGER` to override `PAGER` specifically for hl:

```sh
HL_PAGER=bat hl app.log
```

## Pager Options

### Customizing less

You can customize the behavior of `less` using the `LESS` environment variable:

```sh
# Disable line wrapping
LESS=-SR hl app.log

# Enable mouse scrolling
LESS="-R --mouse" hl app.log

# Quit if output fits on one screen
LESS=-FX hl app.log
```

Common `less` options:
- `-S` - Chop long lines (no line wrapping)
- `-R` - Output raw ANSI escape sequences (for colors)
- `-F` - Quit if output fits on one screen
- `-X` - Don't clear screen on exit
- `-i` - Case-insensitive search
- `--mouse` - Enable mouse scrolling

### Setting Default Options

Make settings permanent by adding them to your shell profile:

```sh
# In ~/.bashrc or ~/.zshrc
export LESS="-R --mouse"
```

## Controlling Pager Usage

### Disable Pager

Use the `-P` flag to disable the pager entirely:

```sh
hl -P app.log
```

Or use the `--paging` option for more control:

```sh
# Never use pager
hl --paging=never app.log

# Always use pager
hl --paging=always app.log

# Automatic (default)
hl --paging=auto app.log
```

### Paging Modes

- **auto** (default) - Use pager when output is to a terminal and not in follow mode
- **always** - Always use pager, even when piping output
- **never** - Never use pager

### Environment Variable

Set the default paging mode:

```sh
export HL_PAGING=never
```

## When Pager is Disabled

The pager is automatically disabled in these scenarios:

1. **Follow mode** - When using `-F` or `--follow`
2. **Streaming mode** - When using `-P` explicitly
3. **Piped output** - When stdout is not a terminal
4. **Redirected output** - When using `-o` or `>` redirection

## Platform-Specific Notes

### macOS

macOS includes `less` by default, so paging works out of the box.

### Linux

Most Linux distributions include `less` by default. If not installed:

```sh
# Debian/Ubuntu
sudo apt-get install less

# Fedora/RHEL
sudo dnf install less

# Arch Linux
sudo pacman -S less
```

### Windows

On Windows, you need to install a pager separately:

#### Using Scoop (Recommended)

```powershell
scoop install less
```

This is automatically installed when you install hl via Scoop.

#### Enable Mouse Scrolling on Windows

```powershell
$env:LESS = "-R --mouse"
```

Add to your PowerShell profile to make it permanent.

## Alternative Pagers

### bat

[bat](https://github.com/sharkdp/bat) is a modern pager with syntax highlighting:

```sh
PAGER="bat --style=plain --paging=always" hl app.log
```

### most

[most](https://www.jedsoft.org/most/) is another alternative:

```sh
PAGER="most -w" hl app.log
```

### moar

[moar](https://github.com/walles/moar) is designed for ANSI escape sequences:

```sh
PAGER=moar hl app.log
```

## Troubleshooting

### Colors Not Showing

If colors aren't displaying correctly:

1. Make sure your pager supports ANSI escape sequences
2. For `less`, ensure `-R` option is set:
   ```sh
   LESS=-R hl app.log
   ```

3. Try forcing color output:
   ```sh
   hl -c app.log
   ```

### Mouse Scrolling Not Working

For `less`, enable mouse support:

```sh
LESS="--mouse" hl app.log
```

Note: This requires a terminal that supports mouse reporting.

### Pager Not Found

If hl can't find a pager:

1. Install `less`: See platform-specific instructions above
2. Or disable paging: `hl -P app.log`
3. Or specify a pager: `PAGER=cat hl app.log`

## Best Practices

1. **Use `-R` with less** - Ensures colors display correctly
2. **Enable mouse scrolling** - Makes navigation easier in modern terminals
3. **Set defaults** - Configure `LESS` in your shell profile
4. **Use `-P` for scripts** - Disable paging in automated workflows
5. **Use `-F` for small outputs** - Automatically quit if output fits on screen

## Examples

### Basic Usage with Custom Pager Settings

```sh
LESS="-RSi --mouse" hl app.log
```

### Disable Pager for Grep

```sh
hl -P app.log | grep ERROR
```

### Temporary Pager Override

```sh
HL_PAGER="most -w" hl app.log
```

### Set Permanent Defaults

```sh
# In ~/.bashrc or ~/.zshrc
export PAGER=less
export LESS="-R --mouse -i"
```

## Interrupt Handling

When viewing logs with a pager or piping from another application, `hl` provides interrupt tolerance to prevent premature termination.

### The --interrupt-ignore-count Option

By default, `hl` ignores the first 3 interrupt signals (Ctrl-C) before actually exiting:

```sh
# Default: ignore first 3 interrupts
hl app.log

# Adjust to ignore 5 interrupts
hl --interrupt-ignore-count 5 app.log

# Exit immediately on first Ctrl-C
hl --interrupt-ignore-count 0 app.log
```

When you press Ctrl-C, you'll see a message indicating how many more times you need to press it:

```
^C interrupted, press Ctrl-C 2 more times to exit
```

### Why This Matters

**When using a pager (like `less`):**

If you're viewing logs in `less` and using Shift+F to follow new data in real-time, pressing Ctrl-C tells `less` to stop loading and let you navigate the already-loaded buffer. Without interrupt ignore count, `hl` would terminate immediately, closing the pager before you could navigate.

```sh
# View large log file
hl large.log

# In less, press Shift+F to follow new data
# Press Ctrl-C to stop following and navigate buffer
# hl stays running so you can continue viewing
```

**When piping from an application:**

When you pipe an application's output to `hl`, pressing Ctrl-C sends the interrupt to both processes. The interrupt ignore count allows `hl` to continue running so you can see the application's graceful shutdown messages.

```sh
# Application receives Ctrl-C and starts graceful shutdown
myapp | hl -P

# Press Ctrl-C - myapp starts shutting down
# hl continues running and displays shutdown logs
# Press Ctrl-C again (3 times total) to force exit
```

This is especially useful for applications that log important cleanup or error messages during shutdown.

### When Interrupt Ignore Count Doesn't Apply

The `--interrupt-ignore-count` option is **ignored in follow mode** (`-F`):

```sh
# Follow mode exits immediately on Ctrl-C
hl -F app.log
```

Follow mode provides immediate exit because you're monitoring files directly and want quick termination when you're done.

### Configuration

Set a default interrupt ignore count:

```toml
# ~/.config/hl/config.toml
interrupt-ignore-count = 5
```

Or via environment variable:

```sh
export HL_INTERRUPT_IGNORE_COUNT=5
```

### Best Practices

- **Use default (3)** for interactive log viewing with a pager
- **Set to 0** in scripts or automated workflows for immediate exit
- **Increase to 5+** when working with applications that have long shutdown sequences
- **Don't rely on it in follow mode** - follow mode always exits immediately

## Next Steps

- [Live Streaming](./streaming.md) - View logs in real-time without a pager
- [Multiple Files](./multiple-files.md) - Navigate through multiple log sources
