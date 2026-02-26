# Data Model: Pager Configuration System

**Feature Branch**: `010-pager`
**Date**: 2025-01-15
**Updated**: 2026-02-24

## Overview

This document defines the data structures for the pager configuration system, including configuration entities, their relationships, and validation rules.

## Entities

### 1. PagerConfig

Represents the top-level `pager` configuration section.

**Location**: `src/pager/config/mod.rs`

```rust
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct PagerConfig {
    /// List of pager candidates to try in order
    #[serde(default)]
    pub candidates: Vec<PagerCandidate>,

    /// Named pager profiles
    #[serde(default)]
    pub profiles: Vec<PagerProfile>,
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

**Location**: `src/pager/config/mod.rs`

```rust
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PagerCandidate {
    /// The kind of candidate (env or profile).
    #[serde(flatten)]
    pub kind: PagerCandidateKind,

    /// Optional condition; if set the candidate is only considered when it matches.
    #[serde(default)]
    pub r#if: Option<Condition>,

    /// Whether `@profile` references are supported in environment variable values (default: false).
    /// When true, values starting with `@` are treated as profile references.
    /// When false, `@` is treated as a literal character in the command name.
    /// Only meaningful for env candidates; ignored for profile candidates.
    #[serde(default)]
    pub profiles: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PagerCandidateKind {
    /// Simple environment variable reference: `{ env = "HL_PAGER" }`
    /// or structured reference: `{ env = { pager = "...", follow = "...", delimiter = "..." } }`
    Env(EnvReference),
    /// Reference to a profile: `{ profile = "fzf" }`
    Profile(String),
}
```

**Validation Rules**:
- `kind` is flattened into the parent struct; only one of `env` or `profile` may be present
- If both `env` and `profile` are specified, deserialization fails with a clear error
- `if` is optional; when absent the candidate is always considered
- `profiles` is ignored for `profile` candidates; only meaningful for `env` candidates
- Empty strings are technically valid but will fail at runtime

**Examples**:
```toml
candidates = [
  # Structured form — profiles = true enables @profile references in HL_PAGER/HL_FOLLOW_PAGER
  { env = { pager = "HL_PAGER", follow = "HL_FOLLOW_PAGER", delimiter = "HL_PAGER_DELIMITER" }, profiles = true },
  { profile = "fzf" },
  { profile = "less" },
  # Simple form — profiles = false (default); PAGER is a universal system variable
  { env = "PAGER" },
]
```

---

### 2a. EnvReference

Represents an environment variable reference, either simple or structured.

**Location**: `src/pager/config/mod.rs`

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

Represents a named pager profile in the `[[pager.profiles]]` array.

**Location**: `src/pager/config/mod.rs`

```rust
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PagerProfile {
    /// Profile name (used to reference the profile from candidates).
    pub name: String,

    /// Base command (executable): `command = "fzf"`
    #[serde(default)]
    pub command: String,

    /// Base arguments: `args = ["--ansi", "--exact"]`
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables to set: `env = { LESSCHARSET = "UTF-8" }`
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Output entry delimiter when this pager is used.
    #[serde(default)]
    pub delimiter: Option<OutputDelimiter>,

    /// Mode-specific configuration.
    #[serde(default)]
    pub modes: PagerModes,

    /// Conditional arguments based on platform and mode.
    #[serde(default)]
    pub conditions: Vec<ConditionalArgs>,
}

impl PagerProfile {
    /// Returns the executable name.
    pub fn executable(&self) -> Option<&str> { ... }

    /// Builds the full command for a given role.
    pub fn build_command(&self, role: PagerRole) -> Vec<&str> { ... }

    /// Builds the environment variables for a given role.
    pub fn build_env(&self, role: PagerRole) -> HashMap<String, String> { ... }
}
```

**Validation Rules**:
- `command` must not be empty for the profile to be usable
- `command` must be a valid executable name or path
- `env` keys must be valid environment variable names
- Empty profile (empty `command`) is valid but will be skipped during selection

---

### 3a. PagerModes

Wraps mode-specific configuration for a profile.

**Location**: `src/pager/config/mod.rs`

```rust
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct PagerModes {
    /// View mode configuration.
    #[serde(default)]
    pub view: PagerRoleConfig,

    /// Follow mode configuration.
    #[serde(default)]
    pub follow: PagerRoleConfig,
}
```

---

### 3b. ConditionalArgs

Represents additional arguments and environment variables applied when a condition is met.

**Location**: `src/pager/config/mod.rs`

```rust
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ConditionalArgs {
    /// Condition that must be met for these args to apply.
    pub r#if: Condition,

    /// Arguments to append when the condition is met.
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables to set when the condition is met.
    #[serde(default)]
    pub env: HashMap<String, String>,
}
```

**Examples**:
```toml
[[pager.profiles]]
name = "less"
command = "less"
args = ["-R"]

[[pager.profiles.conditions]]
if = { os = "windows" }
env = { LESSCHARSET = "UTF-8" }
```

---

### 4. PagerRoleConfig

Represents role-specific configuration (`view` or `follow`).

**Location**: `src/pager/config/mod.rs`

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

**Location**: `src/pager/config/mod.rs`

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
    /// Use a pager with the given command, env, and optional delimiter.
    Pager {
        command: Vec<String>,
        env: HashMap<String, String>,
        delimiter: Option<OutputDelimiter>,
    },
    /// No pager, output to stdout.
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
            │       └── PagerCandidate (struct)
            │               ├── kind: PagerCandidateKind (enum)
            │               │         ├── Env(EnvReference)
            │               │         └── Profile(String)
            │               └── if: Option<Condition>
            │
            └── profiles: Vec<PagerProfile>
                            │
                            └── PagerProfile
                                    │
                                    ├── name: String
                                    ├── command: String
                                    ├── args: Vec<String>
                                    ├── env: HashMap<String, String>
                                    ├── delimiter: Option<OutputDelimiter>
                                    ├── modes: PagerModes
                                    │           ├── view: PagerRoleConfig
                                    │           │         ├── enabled: Option<bool>
                                    │           │         └── args: Vec<String>
                                    │           └── follow: PagerRoleConfig
                                    │                     ├── enabled: Option<bool>
                                    │                     └── args: Vec<String>
                                    └── conditions: Vec<ConditionalArgs>
                                                        ├── if: Condition
                                                        ├── args: Vec<String>
                                                        └── env: HashMap<String, String>

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
  # Structured env reference — profiles = true allows @profile in HL_PAGER
  { env = { pager = "HL_PAGER", follow = "HL_FOLLOW_PAGER", delimiter = "HL_PAGER_DELIMITER" }, profiles = true },
  { profile = "fzf" },
  { profile = "less" },
  # Simple env reference — view mode only, no profile references
  { env = "PAGER" },
]

# Pager profiles
[[pager.profiles]]
name = "less"
command = "less"
args = ["-R", "--mouse"]
env = { LESSCHARSET = "UTF-8" }

[[pager.profiles]]
name = "fzf"
command = "fzf"
args = ["--ansi", "--no-sort", "--exact"]
modes.view.args = ["--layout=reverse-list"]
modes.follow.enabled = true
modes.follow.args = ["--tac", "--track"]
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
| `single-profile.toml` | Single profile for US1 deserialization tests |
| `priority-list.toml` | Multiple profiles for US2 priority-based selection tests |
| `follow-enabled.toml` | Profile with follow mode enabled |
| `follow-enabled-no-env.toml` | Follow mode enabled without env candidates |
| `minimal-profile.toml` | Minimal profile (tests defaults) |
| `profile-with-env.toml` | Profile with environment variables |
| `profile-with-view-args.toml` | Profile with view-specific args |
| `conditional-args.toml` | Profile with condition-based argument sets |
| `candidate-with-if.toml` | Candidate with conditional `if` field |
| `unavailable-first.toml` | Priority list with an unavailable first pager |
| `empty-priority.toml` | Empty candidate list (falls back to stdout) |
| `empty-candidates.toml` | Empty candidates list (falls back to stdout) |
| `unavailable-first.toml` | First candidate unavailable (tests fallback) |
| `env-and-profile-error.toml` | Invalid: both env and profile fields (deserialization error test) |