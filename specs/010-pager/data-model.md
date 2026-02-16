# Data Model: Pager Configuration System

**Feature Branch**: `010-pager`
**Date**: 2025-01-15
**Updated**: 2026-02-16

## Overview

This document defines the data structures for the pager configuration system, including configuration entities, their relationships, and validation rules.

## Entities

### 1. PagerConfig

Represents the top-level `pager` configuration section.

**Location**: `src/pager/config.rs`

```rust
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct PagerConfig {
    /// List of pager candidates to try in order
    #[serde(default)]
    pub candidates: Vec<PagerCandidate>,
    
    /// Named pager profiles
    #[serde(default)]
    pub profiles: HashMap<String, PagerProfile>,
}

impl PagerConfig {
    /// Returns candidates in priority order
    pub fn candidates(&self) -> &[PagerCandidate] { ... }
    
    /// Gets a profile by name
    pub fn profile(&self, name: &str) -> Option<&PagerProfile> { ... }
}
```

**Validation Rules**:
- `candidates` may be empty (falls back to stdout)
- `profiles` may be empty
- Profile names in candidates are validated lazily at selection time

---

### 2. PagerCandidate

Represents a candidate in the `pager.candidates` array.

**Location**: `src/pager/config.rs`

```rust
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PagerCandidate {
    /// Simple environment variable reference: `{ env = "HL_PAGER" }`
    /// or structured reference: `{ env = { pager = "...", follow = "...", delimiter = "..." } }`
    Env(EnvReference),
    /// Reference to a profile: `{ profile = "fzf" }`
    Profile(String),
}
```

**Validation Rules**:
- Externally-tagged enum: only one field (`env` or `profile`) may be present at top level
- If both `env` and `profile` are specified, deserialization fails with clear error
- Empty strings are technically valid but will fail at runtime

**Examples**:
```toml
candidates = [
  # Structured form with role-specific vars
  { env = { pager = "HL_PAGER", follow = "HL_FOLLOW_PAGER", delimiter = "HL_PAGER_DELIMITER" } },
  { profile = "fzf" },
  { profile = "less" },
  # Simple form
  { env = "PAGER" },
]
```

---

### 2a. EnvReference

Represents an environment variable reference, either simple or structured.

**Location**: `src/pager/config.rs`

```rust
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum EnvReference {
    /// Simple form: just a variable name
    Simple(String),
    /// Structured form with role-specific variables
    Structured(StructuredEnvReference),
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct StructuredEnvReference {
    /// Environment variable for view mode (or both modes if follow not specified)
    #[serde(default)]
    pub pager: Option<String>,
    
    /// Environment variable for follow mode
    #[serde(default)]
    pub follow: Option<String>,
    
    /// Environment variable for delimiter override
    #[serde(default)]
    pub delimiter: Option<String>,
}
```

**Validation Rules**:
- Simple form: string must not be empty (validated at runtime)
- Structured form: all fields are optional
- If structured form has all fields `None`, it's valid but will be skipped at runtime
- Untagged enum: deserialization tries Simple first, then Structured

**Resolution Logic**:
- **Simple form**: Read env var, use for both view and follow modes (subject to follow-mode restrictions)
- **Structured form**:
  - If `pager` is `None` or env var not set/empty: skip candidate
  - If `pager` resolves to `@profile`: use profile entirely, ignore `follow` and `delimiter` fields
  - If `pager` resolves to direct command:
    - Use for view mode
    - Check `follow`: if set and resolves, use for follow mode; else disable follow paging
    - Check `delimiter`: if set and resolves, use it; else use default `newline`

---

### 3. PagerProfile

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

### 4. PagerRoleConfig

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

### 5. PagerRole

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

### 6. PagerOverride

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
- `HL_PAGER=less` runs `less` command directly (with automatic `-R` flag added)
- `HL_PAGER=@less` uses `[pagers.less]` profile (fails if profile doesn't exist)
- Multi-word values like `HL_PAGER=less -R` are always treated as direct commands

---

### 7. SelectedPager

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

### 8. PagerSelector

Main selection logic with dependency injection for testing.

**Location**: `src/pager/selection.rs`

```rust
pub struct PagerSelector<'a, E = SystemEnv, C = SystemExeChecker> {
    config: &'a PagerConfig,
    env_provider: E,
    exe_checker: C,
}

impl<'a> PagerSelector<'a, SystemEnv, SystemExeChecker> {
    /// Creates a new pager selector with the given configuration.
    pub fn new(config: &'a PagerConfig) -> Self { ... }
}

impl<'a, E: EnvProvider, C: ExeChecker> PagerSelector<'a, E, C> {
    /// Creates a new pager selector with custom environment and executable checker.
    pub fn with_providers(...) -> Self { ... }
    
    /// Selects a pager for the given role.
    pub fn select(&self, role: PagerRole) -> Result<SelectedPager, Error> { ... }
}
```

---

### 9. Dependency Injection Traits

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
    └── pager: PagerConfig
            │
            ├── candidates: Vec<PagerCandidate>
            │       │
            │       └── PagerCandidate (enum)
            │               ├── Env(String)
            │               └── Profile(String)
            │
            └── profiles: HashMap<String, PagerProfile>
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
│(pager.candidates│
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Candidate loop: │
│ For each        │◄───────┐
│ candidate       │        │
└────────┬────────┘        │
         │                 │
         ▼                 │
┌─────────────────┐        │
│ Candidate type? │        │
└────┬───────┬────┘        │
     │       │             │
 Env │       │ Profile     │
     │       │             │
     ▼       ▼             │
┌─────┐   ┌──────┐         │
│Read │   │Lookup│         │
│ var │   │profile│        │
└──┬──┘   └───┬──┘         │
   │          │            │
   │          ▼            │
   │    ┌─────────────┐    │
   │    │Profile      │    │
   │    │exists?      │─No─►│
   │    └──────┬──────┘    │
   │           │Yes        │
   └───────┬───┘           │
           ▼               │
    ┌─────────────┐        │
    │Command      │        │
    │valid?       │──No───►│
    └──────┬──────┘        │
           │Yes            │
           ▼               │
    ┌─────────────┐        │
    │Executable   │        │
    │in PATH?     │──No───►┘
    └──────┬──────┘
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
[pager]
# Candidate search list (checked in order)
candidates = [
  # Structured env reference with role-specific vars
  { env = { pager = "HL_PAGER", follow = "HL_FOLLOW_PAGER", delimiter = "HL_PAGER_DELIMITER" } },
  { profile = "fzf" },
  { profile = "less" },
  # Simple env reference
  { env = "PAGER" },
]

# Pager profiles
[pager.profiles.less]
command = ["less", "-R", "--mouse"]
env = { LESSCHARSET = "UTF-8" }

[pager.profiles.fzf]
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
    
    /// Pager configuration (candidates and profiles)
    #[serde(default)]
    pub pager: PagerConfig,
}
```

## Test Data Files

Located in `src/testing/assets/pagers/`:

| File | Purpose |
|------|---------|
| `basic.toml` | Basic configuration with one profile candidate |
| `simple-env-candidate.toml` | Simple env candidate referencing one variable |
| `structured-env-candidate.toml` | Structured env candidate with role-specific vars |
| `profile-candidate.toml` | Candidate referencing a profile |
| `mixed-candidates.toml` | Mix of env and profile candidates |
| `follow-enabled.toml` | Profile with follow mode enabled |
| `minimal-profile.toml` | Minimal profile (tests defaults) |
| `profile-with-env.toml` | Profile with environment variables |
| `profile-with-view-args.toml` | Profile with view-specific args |
| `empty-candidates.toml` | Empty candidates list (falls back to stdout) |
| `unavailable-first.toml` | First candidate unavailable (tests fallback) |
| `env-and-profile-error.toml` | Invalid: both env and profile fields (deserialization error test) |