# Research: Pager Configuration System

**Feature Branch**: `010-pager`
**Date**: 2025-01-15
**Updated**: 2025-01-27

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

**Decision**: Always parse as a command using `shellwords::split`, then check if it matches a profile at selection time.

**Rationale**:
- Simpler parsing logic - no heuristics about what "looks like" a profile name
- Profile names are checked at selection time against actual config
- Single-word values like `HL_PAGER=less` work as either:
  - A profile name (if `[pagers.less]` exists in config)
  - A direct command (if no matching profile)
- Multi-word values like `HL_PAGER=less -R` are always treated as commands
- Maintains backward compatibility - env vars don't require profile definitions

**Implementation**:
```rust
fn try_spec_as_profile_or_command(&self, cmd: &[String], role: PagerRole) -> Option<SelectedPager> {
    // If it's a single word and matches a profile name, use the profile
    if cmd.len() == 1 {
        let name = &cmd[0];
        if self.profiles.contains_key(name) {
            return self.try_profile(name, role);
        }
    }
    // Otherwise treat as a direct command
    self.try_command(cmd.to_vec())
}
```

### 4. Dependency Injection for Testing

**Question**: How to test pager selection without depending on actual environment variables and installed executables?

**Decision**: Use trait-based dependency injection for environment access and executable checking.

**Rationale**:
- Tests can use mock implementations that return controlled values
- No need to manipulate actual environment variables (which can cause test interference)
- No dependency on which executables are installed on the test machine
- Follows Rust idioms for testable code

**Implementation**:
```rust
pub trait EnvProvider {
    fn get(&self, name: &str) -> Option<String>;
}

pub trait ExeChecker {
    fn is_available(&self, executable: &str) -> bool;
}

pub struct PagerSelector<'a, E = SystemEnv, C = SystemExeChecker> {
    config: Option<&'a PagerConfig>,
    profiles: &'a HashMap<String, PagerProfile>,
    env_provider: E,
    exe_checker: C,
}
```

### 5. Debug Logging Strategy

**Question**: How to implement debug logging for pager selection?

**Decision**: Use the existing `log` crate with `log::debug!` macros.

**Rationale**:
- `log` crate already in dependencies
- Standard Rust logging pattern
- Can be enabled via `RUST_LOG=debug` or similar
- No runtime overhead when disabled

**Implementation**:
- Use `log::debug!` for selection decisions
- Log which profiles are tried and why they're skipped
- Log final selection result

### 6. Module Organization

**Question**: Where to place the new pager configuration code?

**Decision**: Create a new `src/pager/` module directory.

**Rationale**:
- Follows existing project patterns (`src/theme/`, `src/settings/`)
- Clear separation of concerns
- Allows growth (config, selection can be separate files)
- Tests in `src/pager/tests.rs` following project convention

**Structure**:
```
src/pager/
├── mod.rs          # Re-exports
├── config.rs       # PagerConfig, PagerProfile, PagerRole, PagerRoleConfig
├── selection.rs    # PagerSelector, PagerOverride, SelectedPager, traits
└── tests.rs        # Unit tests
```

### 7. Handling `follow.enabled` Default

**Question**: Should `follow.enabled` default to `false` or require explicit setting?

**Decision**: Default to `false` (no pager in follow mode unless explicitly enabled). This is handled via `#[serde(default)]` which gives `None`/`false` for missing fields.

**Rationale**:
- Safer default: streaming to an unprepared pager can cause issues
- Matches existing behavior where follow mode disables pager
- Explicit opt-in ensures user has configured appropriate follow.args
- Documented in spec (FR-014)
- Only use `#[serde(default)]` for fields that should be empty/None when not specified in config

### 8. Environment Variable Processing in Pager

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

### 9. Backward Compatibility: `HL_PAGER=""` Behavior Change

**Question**: Current behavior falls back to `less` when `HL_PAGER=""`. New behavior disables pager. How to handle?

**Decision**: Implement new behavior (disable pager) as specified in FR-019.

**Rationale**:
- New behavior is more intuitive (empty = disable)
- Current behavior is arguably a bug
- Document in CHANGELOG as behavior change
- Low impact: few users likely set `HL_PAGER=""`

**Migration**: Add note to CHANGELOG explaining the behavior change.

### 10. Special Handling for `less`

**Question**: Should `less` get special treatment when used via environment variable?

**Decision**: Yes, auto-add `-R` flag and set `LESSCHARSET=UTF-8` when `less` is used as a direct command (not via profile).

**Rationale**:
- Maintains backward compatibility with existing behavior
- `-R` enables ANSI color support which is essential for hl output
- `LESSCHARSET=UTF-8` ensures proper Unicode handling
- Profile-based usage is authoritative - no auto-modification

**Implementation**:
```rust
fn apply_less_defaults(mut command: Vec<String>) -> (Vec<String>, HashMap<String, String>) {
    let mut env = HashMap::new();
    if let Some(executable) = command.first() {
        let exe_name = Path::new(executable).file_stem().and_then(OsStr::to_str).unwrap_or(executable);
        if exe_name == "less" {
            if !command.iter().any(|arg| arg == "-R" || arg.starts_with("-R")) {
                command.push("-R".to_string());
            }
            env.insert("LESSCHARSET".to_string(), "UTF-8".to_string());
        }
    }
    (command, env)
}
```

### 11. Test Data Management

**Question**: How to structure test data for the pager module?

**Decision**: Use external TOML files in `src/testing/assets/pagers/` per constitution Principle VII.

**Rationale**:
- Constitution requires external data files instead of inline multiline strings
- External files can be validated with TOML linters
- Easier to reuse test data across multiple tests
- Tests embed files at compile time using `include_str!`

**Implementation**:
```rust
const SINGLE_PROFILE: &str = include_str!("../testing/assets/pagers/single-profile.toml");
const PRIORITY_LIST: &str = include_str!("../testing/assets/pagers/priority-list.toml");
// ... etc
```

## Dependencies Summary

| Dependency | Version | Status | Purpose |
|------------|---------|--------|---------|
| `which` | ^7.0 | **New** | PATH lookup for executable availability |
| `serde` | existing | No change | Configuration deserialization |
| `shellwords` | existing | No change | Command string parsing |
| `log` | existing | No change | Debug logging |
| `config` | existing | No change | Configuration file loading |

## Open Questions

None. All technical questions resolved through research and implementation.

## Implementation Status

- [x] Add `which` crate to `Cargo.toml`
- [x] Implement `src/pager/config.rs` with data structures
- [x] Implement `src/pager/selection.rs` with selection logic
- [x] Add `pager` and `pagers` fields to `Settings`
- [x] Create test assets in `src/testing/assets/pagers/`
- [x] Implement comprehensive unit tests
- [ ] Refactor `src/output.rs` to use new configuration
- [ ] Update `src/main.rs` to wire everything together
- [ ] Update README and CHANGELOG