# Automatic Pager Integration

hl automatically integrates with a pager to make viewing large log files comfortable and efficient. This page explains how pager integration works and how to customize it.

## Default Behavior

When you run hl with a log file, it automatically opens the output in a pager:

```sh
hl application.log
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
PAGER=most hl application.log
```

Or use `HL_PAGER` to override `PAGER` specifically for hl:

```sh
HL_PAGER=bat hl application.log
```

## Pager Options

### Customizing less

You can customize the behavior of `less` using the `LESS` environment variable:

```sh
# Disable line wrapping
LESS=-SR hl application.log

# Enable mouse scrolling
LESS="-R --mouse" hl application.log

# Quit if output fits on one screen
LESS=-FX hl application.log
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
hl -P application.log
```

Or use the `--paging` option for more control:

```sh
# Never use pager
hl --paging=never application.log

# Always use pager
hl --paging=always application.log

# Automatic (default)
hl --paging=auto application.log
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
PAGER="bat --style=plain --paging=always" hl application.log
```

### most

[most](https://www.jedsoft.org/most/) is another alternative:

```sh
PAGER="most -w" hl application.log
```

### moar

[moar](https://github.com/walles/moar) is designed for ANSI escape sequences:

```sh
PAGER=moar hl application.log
```

## Troubleshooting

### Colors Not Showing

If colors aren't displaying correctly:

1. Make sure your pager supports ANSI escape sequences
2. For `less`, ensure `-R` option is set:
   ```sh
   LESS=-R hl application.log
   ```

3. Try forcing color output:
   ```sh
   hl -c application.log
   ```

### Mouse Scrolling Not Working

For `less`, enable mouse support:

```sh
LESS="--mouse" hl application.log
```

Note: This requires a terminal that supports mouse reporting.

### Pager Not Found

If hl can't find a pager:

1. Install `less`: See platform-specific instructions above
2. Or disable paging: `hl -P application.log`
3. Or specify a pager: `PAGER=cat hl application.log`

## Best Practices

1. **Use `-R` with less** - Ensures colors display correctly
2. **Enable mouse scrolling** - Makes navigation easier in modern terminals
3. **Set defaults** - Configure `LESS` in your shell profile
4. **Use `-P` for scripts** - Disable paging in automated workflows
5. **Use `-F` for small outputs** - Automatically quit if output fits on screen

## Examples

### Basic Usage with Custom Pager Settings

```sh
LESS="-RSi --mouse" hl application.log
```

### Disable Pager for Grep

```sh
hl -P application.log | grep ERROR
```

### Temporary Pager Override

```sh
HL_PAGER="most -w" hl application.log
```

### Set Permanent Defaults

```sh
# In ~/.bashrc or ~/.zshrc
export PAGER=less
export LESS="-R --mouse -i"
```

## Next Steps

- [Live Streaming](./streaming.md) - View logs in real-time without a pager
- [Multiple Files](./multiple-files.md) - Navigate through multiple log sources