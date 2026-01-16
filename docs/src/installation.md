# Installation

`hl` is available for macOS, Linux, Windows, and NixOS. Choose the installation method that works best for your system.

## macOS

### Using Homebrew (Recommended)

The easiest way to install `hl` on macOS is using [Homebrew](https://brew.sh):

```sh
brew install hl
```

### Using curl and tar

Download and extract the latest release directly:

```bash
curl -sSfL https://github.com/pamburus/hl/releases/latest/download/hl-macos.tar.gz | tar xz
```

This will extract the `hl` binary to your current directory. Move it to a directory in your PATH:

```bash
sudo mv hl /usr/local/bin/
```

## Linux

### Arch Linux

Install the official package from the extra repository:

```sh
pacman -S hl
```

### Using curl and tar (x86_64)

Download and extract the latest release:

```sh
curl -sSfL https://github.com/pamburus/hl/releases/latest/download/hl-linux-x86_64-musl.tar.gz | tar xz
sudo mv hl /usr/local/bin/
```

### Using curl and tar (ARM64/aarch64)

For ARM-based systems:

```sh
curl -sSfL https://github.com/pamburus/hl/releases/latest/download/hl-linux-arm64-musl.tar.gz | tar xz
sudo mv hl /usr/local/bin/
```

### Using Cargo

If you have Rust installed, you can build and install `hl` from source:

```sh
cargo install --locked --git https://github.com/pamburus/hl.git --rev latest
```

## Windows

### Using Scoop (Recommended)

[Scoop](https://scoop.sh) is the recommended way to install `hl` on Windows:

```powershell
scoop bucket add pamburus https://github.com/pamburus/scoop-bucket.git
scoop install hl
```

This automatically installs `hl` along with the `less` pager as a dependency.

> **Tip**: Use [Windows Terminal](https://aka.ms/terminal) for the best experience with `hl`.

> **Tip**: To enable mouse scrolling in the `less` pager, set the `LESS` environment variable to `-R --mouse`.

### Important Note About Pagers on Windows

`hl` relies on external pagers like `less` for displaying output. The Windows build of `less` distributed through WinGet has known issues with ANSI escape sequences and may not display `hl`'s colored output correctly.

**Recommended solutions:**
- Install `less` via Scoop (included automatically when installing `hl` via Scoop)
- Install `less` via Chocolatey

To verify which `less` is being used:
```cmd
where less
```

Or in PowerShell:
```powershell
Get-Command less
```

### Manual Download

Download the latest release from the [GitHub releases page](https://github.com/pamburus/hl/releases/latest) and extract it to a directory in your PATH.

## NixOS

### Run Without Installing

Try `hl` without installing (pre-built binary package):

```sh
nix run github:pamburus/hl/latest#bin
```

Or build from source code and run:

```sh
nix run github:pamburus/hl/latest
```

### Install with Nix Profile

Install `hl` to your profile (pre-built binary package):

```sh
nix profile add github:pamburus/hl/latest#bin
```

Or build from source code and install:

```sh
nix profile add github:pamburus/hl/latest
```

### NixOS System Configuration

Add `hl` to your NixOS system configuration using flakes.

## Using Cargo

If you have Rust installed, you can build and install `hl` from source:

```sh
cargo install --locked --git https://github.com/pamburus/hl.git --rev latest
```

This method works on all platforms but requires a Rust toolchain.

## Verifying Installation

After installation, verify that `hl` is working:

```sh
hl --version
```

You should see output showing the installed version of `hl`.

## Next Steps

Now that you have `hl` installed, proceed to the [Quick Start](./quick-start.md) guide to learn how to use it.
