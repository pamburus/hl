# Nix Configuration for hl

This directory contains Nix configuration files for building and developing the `hl` log viewer.

## Files Overview

- `package.nix` - Main package definition for hl
- `cargo-hashes.nix` - Output hashes for git dependencies in Cargo.lock
- `rust-toolchain.nix` - Rust toolchain configuration (reads from ../rust-toolchain.toml)
- `README.md` - This documentation file

## Usage

### Building the Package

```bash
# Build the default package
nix build

# Build specific variants
nix build .#hl-debug      # Debug build
nix build .#hl-static     # Static build for containers
nix build .#hl-minimal    # Minimal build without completions
```

### Development Environment

```bash
# Enter development shell with all tools
nix develop

# Or run directly without entering shell
nix develop -c cargo build
```

The development shell includes:
- Rust development tools (rust-analyzer, cargo-watch, etc.)
- Build tools (git, just, fd, ripgrep)
- Documentation tools (mdbook, taplo)
- Testing tools (hyperfine, valgrind)
- Nix development tools (nixpkgs-fmt, nil, nix-tree)

### Development Without Nix

If you don't have Nix installed, you can still develop the project using your system's Rust toolchain:

#### Prerequisites
- Rust 1.89 or later (install via [rustup](https://rustup.rs/))
- Git
- A C compiler (for some dependencies)

#### Setup
```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install 1.89
rustup default 1.89

# Clone and build
git clone https://github.com/pamburus/hl
cd hl
cargo build
```

#### Optional Tools
While Nix provides many development tools automatically, you can install them manually:
```bash
# Install just for convenient commands
cargo install just

# Install other useful tools
cargo install cargo-watch      # Auto-rebuild on changes
cargo install cargo-edit       # Manage dependencies
cargo install cargo-audit      # Security auditing
```

#### Using .envrc
The project includes a `.envrc` file that automatically detects your environment:
```bash
# Enable direnv for this directory
direnv allow
```

This will:
- Use Nix if available
- Fall back to system Rust toolchain
- Provide helpful setup instructions if neither is found

### Running the Application

```bash
# Run directly
nix run

# Using app aliases
nix run .#log-viewer
nix run .#logs
```

### Checking and Formatting

```bash
# Run all checks (including Nix formatting)
nix flake check

# Format Nix files
nix fmt

# Check specific components
nix build .#checks.package
nix build .#checks.nixpkgs-fmt
```

## Maintenance

### Updating Dependencies

1. **Flake inputs**: The GitHub Actions workflow automatically updates `flake.lock` weekly
2. **Cargo dependencies**: Update `Cargo.lock` normally, then update hashes if needed
3. **Git dependency hashes**: Update `cargo-hashes.nix` when git dependencies change

### Adding New Git Dependencies

When adding new git dependencies to `Cargo.toml`:

1. Add the dependency to `Cargo.toml`
2. Run `cargo update` to update `Cargo.lock`
3. Try building with Nix: `nix build`
4. If it fails with a hash mismatch, add the correct hash to `cargo-hashes.nix`

Example error:
```
error: hash mismatch in fixed-output derivation '/nix/store/...-new-dep-0.1.0.tar.gz.drv':
  specified: sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=
  got:       sha256-BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=
```

Add to `cargo-hashes.nix`:
```nix
{
  # ... existing hashes ...
  "new-dep-0.1.0" = "sha256-BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=";
}
```

### Rust Toolchain Updates

The Nix configuration automatically reads from `rust-toolchain.toml`. To update:

1. Update `rust-toolchain.toml` with the desired Rust version
2. Run `nix flake update` to update the rust-overlay input
3. Test the build: `nix build`

### Cross-compilation

The static build variant (`hl-static`) can be used for containers or cross-compilation:

```bash
# Build static binary
nix build .#hl-static

# The result will be in ./result/bin/hl
```

## Troubleshooting

### Common Issues

1. **Hash mismatches**: Update `cargo-hashes.nix` with correct hashes
2. **Rust version conflicts**: Ensure `rust-toolchain.toml` matches your needs
3. **Missing tools in dev shell**: Add them to the `packages` list in `flake.nix`

### Debugging Builds

```bash
# Build with detailed logs
nix build --print-build-logs

# Check what's in the build environment
nix develop -c env | grep -i rust

# Inspect the package derivation
nix show-derivation .#hl
```

### Getting Help

- Check the [Nix manual](https://nixos.org/manual/nix/stable/)
- Review [nixpkgs Rust documentation](https://nixos.org/manual/nixpkgs/stable/#rust)
- Look at the [rust-overlay documentation](https://github.com/oxalica/rust-overlay)