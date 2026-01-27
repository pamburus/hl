# Research: Pager Configuration System

**Feature Branch**: `008-pager-config`
**Date**: 2025-01-15

## Overview

This document captures research findings and technical decisions for implementing the pager configuration system.

## Research Topics

### 1. PATH Lookup for Executable Availability

**Question**: How to check if a pager command is available on the system?

**Decision**: Use the `which` crate for cross-platform PATH lookup.

**Rationale**:
- Cross-platform support (Unix and Windows)
- Well-maintained crate with minimal dependencies
- Simple API: `which::which("fzf")` returns `Result<PathBuf, Error>`
- Already handles PATH parsing and executable detection

**Alternatives Considered**:
- Manual PATH parsing: More code, platform-specific edge cases
- `std::process::Command::spawn` and check error: Wasteful, actually starts the process
- Shell `command -v`: Not portable to Windows

### 2. Configuration Deserialization Pattern

**Question**: How to handle the `pager` field accepting both string and array?

**Decision**: Use serde's `#[serde(untagged)]` enum pattern.

**Rationale**:
```rust
#[derive(Deserialize)]
#[serde(untagged)]
pub enum PagerConfig {
    Single(String),
    Priority(Vec<String>),
}
```
- Clean deserialization of both `pager = "fzf"` and `pager = ["fzf", "less"]`
- No custom deserializer needed
- Pattern already used elsewhere in the codebase (e.g., `ListOrCommaSeparatedList`)
- No `Default` impl needed - defaults come from `etc/defaults/config.toml` (embedded at compile time)

**Alternatives Considered**:
- Custom deserializer: More complex, not needed
- Always use array in config: Breaking change, less user-friendly

### 3. Environment Variable Parsing

**Question**: How to handle `HL_PAGER` that can be either a profile name or a command string?

**Decision**: Check if value matches a profile name first, then fall back to command string parsing.

**Rationale**:
- Profile names are known at config load time
- Simple string comparison for profile lookup
- Existing `shellwords::split` for command string parsing (already in codebase)
- Order: profile match → command string → fallback

**Implementation**:
```rust
fn resolve_pager_spec(value: &str, profiles: &HashMap<String, PagerProfile>) -> PagerSpec {
    if profiles.contains_key(value) {
        PagerSpec::Profile(value.to_string())
    } else {
        PagerSpec::Command(shellwords::split(value).unwrap_or_else(|_| vec![value.to_string()]))
    }
}
```

### 4. Debug Logging Strategy

**Question**: How to implement debug logging for pager selection (enabled via `HL_DEBUG_LOG`)?

**Decision**: Use the existing `log` crate with `log::debug!` macros.

**Rationale**:
- `log` crate already in dependencies
- Standard Rust logging pattern
- Can be enabled via `RUST_LOG=debug` or custom `HL_DEBUG_LOG` env var check
- No runtime overhead when disabled

**Implementation**:
- Check `HL_DEBUG_LOG` env var at startup
- Initialize logger with appropriate level
- Use `log::debug!` for selection decisions

### 5. Module Organization

**Question**: Where to place the new pager configuration code?

**Decision**: Create a new `src/pager/` module directory.

**Rationale**:
- Follows existing project patterns (`src/theme/`, `src/settings/`)
- Clear separation of concerns
- Allows growth (config, selection, execution can be separate files)
- Tests in `src/pager/tests.rs` following project convention

**Structure**:
```
src/pager/
├── mod.rs          # Re-exports
├── config.rs       # PagerConfig, PagerProfile, PagerRole
├── selection.rs    # PagerSelector, precedence logic
└── tests.rs        # Unit tests
```

### 6. Handling `follow.enabled` Default

**Question**: Should `follow.enabled` default to `false` or require explicit setting?

**Decision**: Default to `false` (no pager in follow mode unless explicitly enabled). This is handled via `#[serde(default)]` which gives `None`/`false` for missing fields.

**Rationale**:
- Safer default: streaming to an unprepared pager can cause issues
- Matches existing behavior where follow mode disables pager
- Explicit opt-in ensures user has configured appropriate follow.args
- Documented in spec (FR-014)
- No `Default` impl on structs - all profile defaults come from `etc/defaults/config.toml` (embedded at compile time)
- Only use `#[serde(default)]` for fields that should be empty/None when not specified in config

### 7. Environment Variable Processing in Pager

**Question**: How to pass custom `env` from profile to the pager process?

**Decision**: Use `std::process::Command::env()` method.

**Rationale**:
- Standard library support, no additional dependencies
- Already used in existing code for `LESSCHARSET`
- Simple iteration over profile's env map

**Implementation**:
```rust
for (key, value) in &profile.env {
    command.env(key, value);
}
```

### 8. Backward Compatibility: `HL_PAGER=""` Behavior Change

**Question**: Current behavior falls back to `less` when `HL_PAGER=""`. New behavior disables pager. How to handle?

**Decision**: Implement new behavior (disable pager) as specified in FR-019.

**Rationale**:
- New behavior is more intuitive (empty = disable)
- Current behavior is arguably a bug
- Document in CHANGELOG as behavior change
- Low impact: few users likely set `HL_PAGER=""`

**Migration**: Add note to CHANGELOG explaining the behavior change.

## Dependencies Summary

| Dependency | Version | Status | Purpose |
|------------|---------|--------|---------|
| `which` | ^6.0 | **New** | PATH lookup for executable availability |
| `serde` | existing | No change | Configuration deserialization |
| `shellwords` | existing | No change | Command string parsing |
| `log` | existing | No change | Debug logging |
| `config` | existing | No change | Configuration file loading |

## Open Questions

None. All technical questions resolved through research.

## Next Steps

1. Add `which` crate to `Cargo.toml`
2. Implement `src/pager/config.rs` with data structures
3. Implement `src/pager/selection.rs` with selection logic
4. Refactor `src/output.rs` to use new configuration
5. Update `src/main.rs` to wire everything together
6. Add comprehensive tests