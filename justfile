# Justfile for hl project - convenient command runner
# Run `just --list` to see all available commands

# Default recipe, executed on `just` without arguments
[private]
default:
    @just --list

# Build the project in debug mode
build: (setup "build")
    cargo build

# Build the project in release mode
build-release: (setup "build")
    cargo build --release

# Run the application, example: `just run -- --help`
run *args: build
    cargo run -- {{args}}

# Run tests for all packages in the workspace
test: (setup "build")
    cargo test --workspace

# Check the code for errors without building an executable
check: (setup "build")
    cargo check --workspace --locked

# Run the Rust linter (clippy)
lint: (setup "lint")
    cargo clippy --workspace --all-targets --all-features

# Check for security vulnerabilities in dependencies
audit: (setup "audit")
    cargo audit

# Check for outdated dependencies
outdated: (setup "outdated")
    cargo outdated --workspace

# Format all Rust and Nix files
fmt: fmt-rust fmt-nix
    @echo "✓ All files formatted successfully"

# Format Rust code
fmt-rust: (setup "build-nightly")
    cargo +nightly fmt --workspace --all

# Format Nix files (gracefully skips if Nix is not installed)
fmt-nix:
    @if command -v nix > /dev/null; then \
        echo "Formatting Nix files..."; \
        nix fmt; \
    else \
        echo "Nix not found, skipping Nix formatting"; \
    fi

# Check formatting without applying changes (for CI)
fmt-check: fmt-check-rust fmt-check-nix
    @echo "✓ Formatting is correct"

# Check Rust formatting
fmt-check-rust: (setup "build-nightly")
    @cargo +nightly fmt --all --check

# Check Nix formatting
fmt-check-nix:
    @if command -v nix > /dev/null; then \
        nix fmt --check; \
    fi

# Clean build artifacts
clean:
    cargo clean
    @rm -f result*

# Run all CI checks locally
ci: test lint audit fmt-check check-schema check
    @echo "✓ All local CI checks passed"

# Generate code coverage
coverage: (setup "coverage")
    @bash build/ci/coverage.sh

# Show uncovered changed lines comparing to {{base}}
uncovered base="origin/master": (setup "coverage")
    @scripts/coverage-diff-analysis.py -q --ide-links {{base}}

# Run benchmarks
bench: (setup "build")
    cargo bench --workspace --locked

# Check schema validation
check-schema: (setup "schema")
    taplo check
    @.venv/bin/python build/ci/validate_yaml.py ./schema/json/config.schema.json etc/defaults/config{,-ecs,-k8s}.yaml
    @.venv/bin/python build/ci/validate_yaml.py ./schema/json/theme.schema.json etc/defaults/themes/*.yaml

# Install binary and man pages
install: (setup "build")
    cargo install --path . --locked

# Create screenshots
screenshots: (setup "screenshots") build
    @bash contrib/bin/screenshot.sh light cafe.log
    @bash contrib/bin/screenshot.sh dark cafe.log

# Nix-specific commands (require Nix to be installed)
nix-dev:
    nix develop

# Run all Nix flake checks
nix-check:
    nix flake check --all-systems --print-build-logs

# Update all Nix flake inputs
nix-update:
    nix flake update

# Build all defined Nix package variants
nix-build-all:
    nix build .#hl
    nix build .#hl-debug
    nix build .#hl-static
    nix build .#hl-minimal

# Show the dependency tree of the Nix derivation
nix-deps:
    @if command -v nix-tree > /dev/null; then \
        nix-tree ./result; \
    else \
        echo "nix-tree is not installed. Run 'nix develop' to enter a shell where it is available"; \
    fi

# Helper recipe to ensure required tools are available for a given task
[private]
setup tool:
    @contrib/bin/setup.sh {{tool}}
