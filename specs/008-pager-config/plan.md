# Implementation Plan: Pager Configuration System

**Branch**: `008-pager-config` | **Date**: 2025-01-15 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/008-pager-config/spec.md`

## Summary

Implement a configurable pager system that allows users to define named pager profiles with priority-based fallback selection and role-specific arguments for view and follow modes. The system will integrate with the existing configuration infrastructure (`Settings`) and replace the current hardcoded pager logic in `output.rs`.

## Technical Context

**Language/Version**: Rust (stable, as per project)
**Primary Dependencies**: serde (deserialization), shellwords (command parsing), config (configuration loading), which (PATH lookup)
**Storage**: TOML configuration files (existing infrastructure)
**Testing**: cargo test, integration tests for pager selection logic
**Target Platform**: Cross-platform (Unix + Windows)
**Project Type**: Single CLI application
**Performance Goals**: No measurable impact on startup time; PATH lookup should be cached
**Constraints**: Backward compatibility with existing `HL_PAGER` and `PAGER` environment variables
**Scale/Scope**: Single-user CLI tool

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Performance First | ✅ Pass | Pager selection happens once at startup; PATH lookup is O(n) where n is PATH entries |
| II. Composability & Modularity | ✅ Pass | New `pager` module with clear interfaces; `PagerConfig` separate from `Pager` execution |
| III. User Experience & Intuitiveness | ✅ Pass | Sensible defaults; silent fallback; debug logging available |
| IV. Reliability & Robustness | ✅ Pass | Graceful degradation to stdout; no panics on missing pagers |
| V. Test-First Development | ✅ Pass | Unit tests for config parsing, selection logic; integration tests for env var handling |
| VI. Specification Integrity | ✅ Pass | No renumbering needed; new module |
| VII. Test Data Management | ✅ Pass | Test configs stored in `src/testing/assets/` |

## Project Structure

### Documentation (this feature)

```text
specs/008-pager-config/
├── plan.md              # This file
├── spec.md              # Feature specification
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
└── checklists/
    └── requirements.md  # Specification quality checklist
```

### Source Code (repository root)

```text
src/
├── settings.rs          # MODIFY: Add pager and pagers fields to Settings
├── output.rs            # MODIFY: Refactor Pager to use config
├── pager.rs             # NEW: Pager configuration and selection logic
├── pager/               # NEW: Module directory
│   ├── mod.rs           # Module exports
│   ├── config.rs        # PagerConfig, PagerProfile, PagerRole structs
│   ├── selection.rs     # Priority-based pager selection logic
│   └── tests.rs         # Unit tests
├── main.rs              # MODIFY: Wire up new pager selection
└── testing/
    └── assets/
        └── pagers/      # NEW: Test configuration files
            ├── basic.toml
            ├── priority.toml
            └── follow-mode.toml

schema/json/
└── config.schema.json   # ALREADY MODIFIED: pager and pagers definitions
```

**Structure Decision**: Single project structure maintained. New `pager` module added alongside existing modules like `theme`, `settings`. Test assets follow existing pattern in `src/testing/assets/`.

## Implementation Phases

### Phase 1: Data Model & Configuration

**Goal**: Define configuration structures and integrate with Settings

1. Create `src/pager/config.rs`:
   - `PagerProfile` struct (command, env, view, follow)
   - `PagerRoleConfig` struct (enabled, args)
   - `PagerConfig` enum (single profile name or list)
   - Serde deserialization (no Default impls - defaults come from embedded `etc/defaults/config.toml`)
   - Use `#[serde(default)]` only for fields that should be empty/None when not specified

2. Modify `src/settings.rs`:
   - Add `pager: Option<PagerConfig>` field
   - Add `pagers: HashMap<String, PagerProfile>` field
   - No Default impls needed - defaults loaded from embedded config at runtime

3. Update default config (`etc/defaults/config.toml`):
   - Already done: `pager`, `[pagers.less]`, `[pagers.fzf]`

### Phase 2: Selection Logic

**Goal**: Implement priority-based pager selection

1. Create `src/pager/selection.rs`:
   - `PagerSelector` struct holding config reference
   - `select_for_view()` method implementing FR-020 precedence
   - `select_for_follow()` method implementing FR-020a precedence
   - `is_available(profile)` method using `which` crate for PATH lookup
   - Debug logging via `log` crate when `HL_DEBUG_LOG` is set

2. Environment variable handling:
   - `HL_PAGER` parsing (profile name vs command string)
   - `HL_FOLLOW_PAGER` parsing
   - `PAGER` fallback
   - Empty string handling (disable pager)

3. Special `less` handling:
   - Auto-add `-R` flag for command strings
   - Auto-set `LESSCHARSET=UTF-8` for command strings
   - Skip for profile-based usage

### Phase 3: Pager Execution

**Goal**: Refactor `output.rs` to use new configuration

1. Modify `src/output.rs`:
   - `Pager::new()` → `Pager::from_profile(profile, role)`
   - Accept `PagerProfile` and `PagerRole` enum
   - Apply `env` variables from profile
   - Append role-specific `args`
   - Preserve existing crash recovery logic

2. Create `PagerRole` enum:
   - `View` - uses `view.args`
   - `Follow` - uses `follow.args` (only if `follow.enabled`)

### Phase 4: Main Integration

**Goal**: Wire everything together in main.rs

1. Modify `src/main.rs`:
   - Create `PagerSelector` from settings
   - Call `select_for_view()` or `select_for_follow()` based on `--follow`
   - Handle `--paging=never` / `-P` flag (highest precedence)
   - Pass selected profile to `Pager::from_profile()`

2. Behavior changes:
   - `HL_PAGER=""` now disables pager (was: fallback to `less`)
   - Follow mode respects `follow.enabled` from profile

### Phase 5: Testing

**Goal**: Comprehensive test coverage

1. Unit tests (`src/pager/tests.rs`):
   - Config parsing (valid, invalid, defaults)
   - Profile selection with various priority lists
   - Environment variable precedence
   - Empty string handling
   - `less` special handling

2. Integration tests:
   - End-to-end pager selection with mock executables
   - Fallback to stdout when no pager available
   - Follow mode with/without `follow.enabled`

3. Test data files (`src/testing/assets/pagers/`):
   - `basic.toml` - single profile
   - `priority.toml` - multiple profiles with fallback
   - `follow-mode.toml` - follow.enabled scenarios

## Dependencies

### New Crates

| Crate | Version | Purpose |
|-------|---------|---------|
| `which` | latest | PATH lookup for executable availability |

### Existing Crates (already in use)

- `serde` - Configuration deserialization
- `shellwords` - Command string parsing
- `log` - Debug logging
- `config` - Configuration file loading

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking change: `HL_PAGER=""` behavior | Medium | Low | Document in CHANGELOG; behavior is more intuitive |
| Performance: PATH lookup on every run | Low | Low | Single lookup per pager in priority list; early termination |
| Complexity: Two precedence orders (view/follow) | Low | Medium | Clear separation in code; well-documented in spec |

## Milestones

1. **M1**: Configuration structures and Settings integration (Phase 1)
2. **M2**: Selection logic with environment variables (Phase 2)
3. **M3**: Pager execution refactor (Phase 3)
4. **M4**: Main integration and behavior changes (Phase 4)
5. **M5**: Full test coverage (Phase 5)

## Complexity Tracking

No constitution violations requiring justification.