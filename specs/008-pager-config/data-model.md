# Data Model: Pager Configuration System

**Feature Branch**: `008-pager-config`
**Date**: 2025-01-15

## Overview

This document defines the data structures for the pager configuration system, including configuration entities, their relationships, and validation rules.

## Entities

### 1. PagerConfig

Represents the top-level `pager` configuration option.

**Location**: `src/pager/config.rs`

```rust
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PagerConfig {
    /// Single profile name: `pager = "fzf"`
    Single(String),
    /// Priority list: `pager = ["fzf", "less"]`
    Priority(Vec<String>),
}

impl PagerConfig {
    /// Returns profile names in priority order
    pub fn profiles(&self) -> impl Iterator<Item = &str> {
        match self {
            PagerConfig::Single(name) => std::iter::once(name.as_str()).chain(std::iter::empty()),
            PagerConfig::Priority(names) => names.iter().map(|s| s.as_str()),
        }
    }
}

// Note: No Default impl - defaults come from etc/defaults/config.toml (embedded at compile time)
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
// Note: No Default derive - defaults come from etc/defaults/config.toml (embedded at compile time)
#[derive(Debug, Clone, Deserialize, PartialEq)]
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
    pub fn is_valid(&self) -> bool {
        !self.command.is_empty()
    }
    
    /// Returns the executable name (first element of command)
    pub fn executable(&self) -> Option<&str> {
        self.command.first().map(|s| s.as_str())
    }
    
    /// Builds the full command for a given role
    pub fn build_command(&self, role: PagerRole) -> Vec<&str> {
        let mut cmd: Vec<&str> = self.command.iter().map(|s| s.as_str()).collect();
        let args = match role {
            PagerRole::View => &self.view.args,
            PagerRole::Follow => &self.follow.args,
        };
        cmd.extend(args.iter().map(|s| s.as_str()));
        cmd
    }
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
// Note: No Default derive - defaults come from etc/defaults/config.toml (embedded at compile time)
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PagerRoleConfig {
    /// Whether pager is enabled for this role (only meaningful for follow)
    #[serde(default)]
    pub enabled: Option<bool>,
    
    /// Additional arguments for this role
    #[serde(default)]
    pub args: Vec<String>,
}

impl PagerRoleConfig {
    /// Returns true if this role is enabled
    /// For view: always true (implicit)
    /// For follow: only if explicitly enabled
    pub fn is_enabled(&self, role: PagerRole) -> bool {
        match role {
            PagerRole::View => true,
            PagerRole::Follow => self.enabled.unwrap_or(false),
        }
    }
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

### 5. PagerSpec

Represents a resolved pager specification from environment variables.

**Location**: `src/pager/selection.rs`

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum PagerSpec {
    /// References a named profile
    Profile(String),
    /// Direct command (parsed from env var)
    Command(Vec<String>),
    /// Explicitly disabled (empty string in env var)
    Disabled,
}
```

---

### 6. SelectedPager

Represents the final pager selection result.

**Location**: `src/pager/selection.rs`

```rust
#[derive(Debug, Clone)]
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
         │ not set or profile
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

### basic.toml
```toml
pager = "less"

[pagers.less]
command = ["less", "-R"]
```

### priority.toml
```toml
pager = ["nonexistent", "less"]

[pagers.less]
command = ["less", "-R"]
```

### follow-mode.toml
```toml
pager = "fzf"

[pagers.fzf]
command = ["fzf", "--ansi"]
view.args = ["--layout=reverse-list"]
follow.enabled = true
follow.args = ["--tac"]
```
