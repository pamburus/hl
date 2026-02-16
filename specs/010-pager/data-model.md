# Data Model: Pager Configuration System

**Feature Branch**: `010-pager`
**Date**: 2025-01-15
**Updated**: 2025-01-27

## Overview

This document defines the data structures for the pager configuration system, including configuration entities, their relationships, and validation rules.

## Entities

### 1. PagerConfig

Represents the top-level `pager` configuration option.

**Location**: `src/pager/config.rs`

```rust
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum PagerConfig {
    /// Single profile name: `pager = "fzf"`
    Single(String),
    /// Priority list: `pager = ["fzf", "less"]`
    Priority(Vec<String>),
}

impl PagerConfig {
    /// Returns profile names in priority order
    pub fn profiles(&self) -> impl Iterator<Item = &str> { ... }
}
```

**Validation Rules**:
- If `Single`, the string must not be empty
- If `Priority`, the vector may be empty (falls back to stdout)
- Profile names are validated lazily at selection time

---

### 2. PagerProfile

Represents a named pager profile in the `[pagers.<name>]` section.

**Location**: `src/pager/config.rs`

```rust
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PagerProfile {
    /// Base command and arguments: `command = ["fzf", "--ansi"]`
    #[serde(default)]
    pub command: Vec<String>,
    
    /// Environment variables to set: `env = { LESSCHARSET = "UTF-8" }`
    #[serde(default)]
    pub env: HashMap<String, String>,
    
    /// View mode configuration
    #[serde(default)]
    pub view: PagerRoleConfig,
    
    /// Follow mode configuration
    #[serde(default)]
    pub follow: PagerRoleConfig,
}

impl PagerProfile {
    /// Returns true if this profile has a valid command
    pub fn is_valid(&self) -> bool { ... }
    
    /// Returns the executable name (first element of command)
    pub fn executable(&self) -> Option<&str> { ... }
    
    /// Builds the full command for a given role
    pub fn build_command(&self, role: PagerRole) -> Vec<&str> { ... }
}
```

**Validation Rules**:
- `command` must not be empty for the profile to be usable
- `command[0]` must be a valid executable name or path
- `env` keys must be valid environment variable names
- Empty profile is valid but will be skipped during selection

---

### 3. PagerRoleConfig

Represents role-specific configuration (`view` or `follow`).

**Location**: `src/pager/config.rs`

```rust
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct PagerRoleConfig {
    /// Whether pager is enabled for this role (only meaningful for follow)
    #[serde(default)]
    pub enabled: Option<bool>,
    
    /// Additional arguments for this role
    #[serde(default)]
    pub args: Vec<String>,
}

impl PagerRoleConfig {
    /// Returns true if pager is enabled for the given role.
    /// For view: always returns true (implicit).
    /// For follow: only returns true if explicitly enabled.
    pub fn is_enabled(&self, role: PagerRole) -> bool { ... }
}
```

**Validation Rules**:
- `enabled` defaults to `None` (treated as `false` for follow mode)
- `args` may be empty

---

### 4. PagerRole

Enum representing the context in which a pager is used.

**Location**: `src/pager/config.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagerRole {
    /// Standard log viewing (non-follow)
    View,
    /// Live log streaming (--follow mode)
    Follow,
}
```

---

### 5. PagerOverride

Represents a pager override parsed from an environment variable.

**Location**: `src/pager/selection.rs`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PagerOverride {
    /// A value that could be a profile name or command (parsed from env var).
    Value(Vec<String>),
    /// Explicitly disabled (empty string in env var).
    Disabled,
}
```

**Resolution Logic**:
- When `Value` contains a single element matching a profile name in config, use that profile
- Otherwise, treat the entire `Value` as a direct command
- This allows `HL_PAGER=less` to use a `[pagers.less]` profile if defined, or run `less` directly

---

### 6. SelectedPager

Represents the final pager selection result.

**Location**: `src/pager/selection.rs`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectedPager {
    /// Use a pager with the given command, args, and env
    Pager {
        command: Vec<String>,
        env: HashMap<String, String>,
    },
    /// No pager, output to stdout
    None,
}
```

---

### 7. PagerSelector

Main selection logic with dependency injection for testing.

**Location**: `src/pager/selection.rs`

```rust
pub struct PagerSelector<'a, E = SystemEnv, C = SystemExeChecker> {
    config: Option<&'a PagerConfig>,
    profiles: &'a HashMap<String, PagerProfile>,
    env_provider: E,
    exe_checker: C,
}

impl<'a> PagerSelector<'a, SystemEnv, SystemExeChecker> {
    /// Creates a new pager selector with the given configuration.
    pub fn new(config: Option<&'a PagerConfig>, profiles: &'a HashMap<String, PagerProfile>) -> Self { ... }
}

impl<'a, E: EnvProvider, C: ExeChecker> PagerSelector<'a, E, C> {
    /// Creates a new pager selector with custom environment and executable checker.
    pub fn with_providers(...) -> Self { ... }
    
    /// Selects a pager for the given role.
    pub fn select(&self, role: PagerRole) -> SelectedPager { ... }
}
```

---

### 8. Dependency Injection Traits

Traits for testability without mocking the real environment.

**Location**: `src/pager/selection.rs`

```rust
/// Trait for providing environment variable access.
pub trait EnvProvider {
    fn get(&self, name: &str) -> Option<String>;
}

/// Default implementation using std::env.
pub struct SystemEnv;

/// Trait for checking executable availability.
pub trait ExeChecker {
    fn is_available(&self, executable: &str) -> bool;
}

/// Default implementation using the `which` crate.
pub struct SystemExeChecker;
```

---

## Relationships

```
Settings
    │
    ├── pager: Option<PagerConfig>
    │       │
    │       └── profiles() → Iterator<&str>
    │
    └── pagers: HashMap<String, PagerProfile>
                    │
                    └── PagerProfile
                            │
                            ├── command: Vec<String>
                            ├── env: HashMap<String, String>
                            ├── view: PagerRoleConfig
                            │           ├── enabled: Option<bool>
                            │           └── args: Vec<String>
                            │
                            └── follow: PagerRoleConfig
                                        ├── enabled: Option<bool>
                                        └── args: Vec<String>

PagerSelector<E, C>
    │
    ├── config: &PagerConfig
    ├── profiles: &HashMap<String, PagerProfile>
    ├── env_provider: E (implements EnvProvider)
    └── exe_checker: C (implements ExeChecker)
```

## State Transitions

### Pager Selection Flow

```
┌─────────────────┐
│  Check CLI flag │
│ (--paging=never)│
└────────┬────────┘
         │ not set
         ▼
┌─────────────────┐
│ Check env vars  │
│ (HL_PAGER, etc) │
└────────┬────────┘
         │ not set
         ▼
┌─────────────────┐
│ Check config    │
│ (pager option)  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Priority loop:  │
│ For each profile│◄───────┐
└────────┬────────┘        │
         │                 │
         ▼                 │
┌─────────────────┐        │
│ Profile exists? │──No───►│
└────────┬────────┘        │
         │ Yes             │
         ▼                 │
┌─────────────────┐        │
│ Command valid?  │──No───►│
└────────┬────────┘        │
         │ Yes             │
         ▼                 │
┌─────────────────┐        │
│ Executable in   │──No───►┘
│ PATH?           │
└────────┬────────┘
         │ Yes
         ▼
┌─────────────────┐
│ Role enabled?   │──No──► SelectedPager::None
│ (follow mode)   │
└────────┬────────┘
         │ Yes
         ▼
┌─────────────────┐
│SelectedPager::  │
│ Pager { ... }   │
└─────────────────┘
```

### Environment Variable Resolution

```
┌─────────────────┐
│ Get env var     │
│ value           │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Empty string?   │──Yes──► PagerOverride::Disabled
└────────┬────────┘
         │ No
         ▼
┌─────────────────┐
│ Parse with      │
│ shellwords      │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ PagerOverride:: │
│ Value(parts)    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Single word &   │──Yes──► Use profile
│ matches profile?│
└────────┬────────┘
         │ No
         ▼
┌─────────────────┐
│ Treat as direct │
│ command         │
└─────────────────┘
```

## Configuration Example

```toml
# Top-level pager selection (priority list)
pager = ["fzf", "less"]

# Pager profiles
[pagers.less]
command = ["less", "-R", "--mouse"]
env = { LESSCHARSET = "UTF-8" }

[pagers.fzf]
command = [
  "fzf",
  "--ansi",
  "--no-sort",
  "--exact"
]
view.args = ["--layout=reverse-list"]
follow.enabled = true
follow.args = [
  "--tac",
  "--track"
]
```

## Integration with Settings

**Modified `src/settings.rs`**:

```rust
pub struct Settings {
    // ... existing fields ...
    
    /// Pager profile(s) to use for output pagination
    #[serde(default)]
    pub pager: Option<PagerConfig>,
    
    /// Named pager profiles
    #[serde(default)]
    pub pagers: HashMap<String, PagerProfile>,
}
```

## Test Data Files

Located in `src/testing/assets/pagers/`:

| File | Purpose |
|------|---------|
| `basic.toml` | Basic single profile configuration |
| `single-profile.toml` | Single profile with env vars |
| `priority-list.toml` | Multiple profiles in priority order |
| `follow-enabled.toml` | Profile with follow mode enabled |
| `minimal-profile.toml` | Minimal profile (tests defaults) |
| `profile-with-env.toml` | Profile with environment variables |
| `profile-with-view-args.toml` | Profile with view-specific args |
| `empty-priority.toml` | Empty priority list (falls back to stdout) |
| `unavailable-first.toml` | First pager unavailable (tests fallback) |