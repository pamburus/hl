# Quickstart: Pager Configuration System

**Feature Branch**: `008-pager-config`
**Date**: 2025-01-15

## Overview

This guide provides a quick reference for implementing the pager configuration system. Follow the phases in order for best results.

## Prerequisites

- Rust toolchain (stable)
- Understanding of the existing `Settings` and `output.rs` modules
- Familiarity with serde deserialization

## Quick Implementation Steps

### Step 1: Add Dependency

Add to `Cargo.toml`:

```toml
[dependencies]
which = "6"
```

### Step 2: Create Pager Module

Create `src/pager/mod.rs`:

```rust
mod config;
mod selection;

pub use config::{PagerConfig, PagerProfile, PagerRole, PagerRoleConfig};
pub use selection::{PagerSelector, SelectedPager};

#[cfg(test)]
mod tests;
```

### Step 3: Implement Config Structs

Create `src/pager/config.rs` with:

- `PagerConfig` enum (Single/Priority)
- `PagerProfile` struct (command, env, view, follow)
- `PagerRoleConfig` struct (enabled, args)
- `PagerRole` enum (View/Follow)

**Important**: No `Default` impls on these structs. All defaults come from `etc/defaults/config.toml` (embedded at compile time). Only use `#[serde(default)]` for fields that should be empty/None when not specified in a profile.

See [data-model.md](./data-model.md) for full definitions.

### Step 4: Implement Selection Logic

Create `src/pager/selection.rs` with:

```rust
pub struct PagerSelector<'a> {
    config: Option<&'a PagerConfig>,
    profiles: &'a HashMap<String, PagerProfile>,
}

impl<'a> PagerSelector<'a> {
    pub fn new(settings: &'a Settings) -> Self { ... }
    
    pub fn select(&self, role: PagerRole) -> SelectedPager {
        match role {
            PagerRole::View => self.select_for_view(),
            PagerRole::Follow => self.select_for_follow(),
        }
    }
    
    fn select_for_view(&self) -> SelectedPager { ... }
    fn select_for_follow(&self) -> SelectedPager { ... }
    fn is_available(&self, profile: &PagerProfile) -> bool { ... }
}
```

### Step 5: Update Settings

Add to `src/settings.rs`:

```rust
use crate::pager::{PagerConfig, PagerProfile};

pub struct Settings {
    // ... existing fields ...
    
    #[serde(default)]
    pub pager: Option<PagerConfig>,
    
    #[serde(default)]
    pub pagers: HashMap<String, PagerProfile>,
}
```

### Step 6: Refactor Pager Struct

Update `src/output.rs`:

```rust
impl Pager {
    pub fn from_selection(selection: SelectedPager) -> Option<Result<Self>> {
        match selection {
            SelectedPager::Pager { command, env } => {
                Some(Self::spawn(command, env))
            }
            SelectedPager::None => None,
        }
    }
    
    fn spawn(command: Vec<String>, env: HashMap<String, String>) -> Result<Self> {
        // Build and spawn the process
    }
}
```

### Step 7: Wire Up in Main

Update `src/main.rs`:

```rust
use hl::pager::{PagerSelector, PagerRole};

// In run():
let role = if opt.follow { PagerRole::Follow } else { PagerRole::View };
let selector = PagerSelector::new(&settings);
let pager_selection = if opt.paging_never {
    SelectedPager::None
} else {
    selector.select(role)
};
```

## Environment Variable Precedence

### View Mode (FR-020)

1. `--paging=never` / `-P` → No pager
2. `HL_PAGER` → Profile or command
3. Config `pager` → Profile(s)
4. `PAGER` → Command
5. Fallback → stdout

### Follow Mode (FR-020a)

1. `--paging=never` / `-P` → No pager
2. `HL_FOLLOW_PAGER` → Profile or command
3. `HL_PAGER=""` → No pager
4. `HL_PAGER` → Profile (if `follow.enabled`)
5. Config `pager` → Profile (if `follow.enabled`)
6. Fallback → stdout

## Testing Checklist

- [ ] Config parsing: single profile
- [ ] Config parsing: priority list
- [ ] Profile selection: first available
- [ ] Profile selection: fallback to second
- [ ] Profile selection: all unavailable → stdout
- [ ] Environment: `HL_PAGER` as profile name
- [ ] Environment: `HL_PAGER` as command
- [ ] Environment: `HL_PAGER=""` disables
- [ ] Environment: `HL_FOLLOW_PAGER` override
- [ ] Follow mode: `follow.enabled = false` → stdout
- [ ] Follow mode: `follow.enabled = true` → pager with follow.args
- [ ] Special handling: `less` auto-adds `-R` (command string only)

## Common Pitfalls

1. **Don't forget `follow.enabled`**: Follow mode defaults to no pager
2. **Empty command array**: Skip profile, don't panic
3. **Profile not found**: Skip and try next, don't error
4. **`HL_PAGER=""` behavior change**: Now disables pager (document in CHANGELOG)

## Files Modified/Created

| File | Action | Description |
|------|--------|-------------|
| `Cargo.toml` | Modify | Add `which` dependency |
| `src/lib.rs` | Modify | Add `pub mod pager;` |
| `src/pager/mod.rs` | Create | Module exports |
| `src/pager/config.rs` | Create | Configuration structs |
| `src/pager/selection.rs` | Create | Selection logic |
| `src/pager/tests.rs` | Create | Unit tests |
| `src/settings.rs` | Modify | Add pager fields |
| `src/output.rs` | Modify | Refactor Pager |
| `src/main.rs` | Modify | Wire up selection |

## Next Steps

After implementation:

1. Run `cargo test` to verify all tests pass
2. Run `cargo clippy --workspace --all-targets --all-features` for linting
3. Test manually with various configurations
4. Update CHANGELOG with behavior changes
5. Update README with new configuration options