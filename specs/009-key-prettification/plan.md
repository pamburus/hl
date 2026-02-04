# Implementation Plan: Key Prettification Config Option

**Branch**: `009-key-prettification` | **Date**: 2026-02-04 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/009-key-prettification/spec.md`

## Summary

Add a `prettify-field-keys` config option (default `true`) to control whether underscores in field keys are replaced with hyphens during output formatting. Config-only — no CLI flags. The implementation modifies the formatting pipeline to conditionally apply the existing `KeyPrettify` trait based on this option, while preserving all other formatting behaviors.

## Technical Context

**Language/Version**: Rust (stable, edition 2024)
**Primary Dependencies**: serde (config deserialization)
**Storage**: TOML config file (`etc/defaults/config.toml`)
**Testing**: cargo test (unit tests in `src/formatting/tests.rs`)
**Target Platform**: Cross-platform CLI (Linux, macOS, Windows)
**Project Type**: Single Rust crate (CLI application)
**Performance Goals**: No measurable regression; skipping `key_prettify` is a no-op performance gain
**Constraints**: Must preserve existing default behavior; must follow existing patterns for config options
**Scale/Scope**: 3 source files modified, ~30 lines added

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Performance First | PASS | Skipping string replacement is equal or faster than current behavior |
| II. Composability & Modularity | PASS | New option threads through existing config → builder → formatter pipeline |
| III. User Experience & Intuitiveness | PASS | Default preserves current behavior; option name is self-documenting |
| IV. Reliability & Robustness | PASS | Simple boolean flag; no new failure modes |
| V. Test-First Development & Quality | PASS | Tests planned for all acceptance scenarios |
| VI. Specification & Cross-Reference Integrity | PASS | No renumbering needed |
| VII. Test Data Management | PASS | Tests use inline assertions (<3 lines); no structured test data files needed |

No violations. No complexity tracking entries needed.

## Project Structure

### Documentation (this feature)

```text
specs/009-key-prettification/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (files to modify)

```text
src/
├── settings.rs          # Add prettify_field_keys field to Formatting struct
└── formatting.rs        # Conditionally apply KeyPrettify based on option
    └── tests.rs         # Add tests for new behavior

etc/defaults/
└── config.toml          # Add prettify-field-keys default
```

**Structure Decision**: Existing single-crate structure. All changes are additions to existing files following established patterns.

## Design Decisions

### Naming: `prettify-field-keys`

- **Config key**: `prettify-field-keys` (under `[formatting]` section, kebab-case per convention)
- **Rust field**: `prettify_field_keys: Option<bool>` in `Formatting`
- **Rationale**: The existing function is `KeyPrettify` and `--raw-fields` description says "prettifying". The term "prettify" is established codebase vocabulary. The name `prettify-field-keys` is specific enough to convey the behavior while being concise.

### Default value: `true`

Preserves current behavior (FR-002). Users who don't set the option see no change.

### Config-only (no CLI flags)

This option controls a personal preference that users set once and leave. Adding CLI flags would add unnecessary surface area to the help output for a rarely-toggled setting.

### Data flow

The `Formatting` struct is already passed to `RecordFormatterBuilder` via `with_options()` (app.rs:902). The builder stores it as `cfg: Option<Formatting>`. In `build()`, the new field is extracted alongside existing fields like `cfg.punctuation`. No changes needed to `Options`, `cli.rs`, `main.rs`, or `app.rs`.

### Interaction with `--raw-fields`

Per FR-005, when `raw_fields` is true, `prettify_field_keys` is forced to `false` in the builder's `build()` method. This matches the spec's requirement that `--raw-fields` takes precedence.

Note: Currently `--raw-fields` only disables value unescaping but does NOT disable key prettification. This implementation will make `--raw-fields` also disable key prettification, which is a minor behavior change but aligns with the documented intent ("without unescaping or prettifying") and FR-005.

## Implementation Flow

### 1. Config layer (`src/settings.rs`)

Add to `Formatting` struct (after line 397):
```rust
pub prettify_field_keys: Option<bool>,
```

Update `Sample for Formatting` to include the new field.

### 2. Default config (`etc/defaults/config.toml`)

Add under `[formatting]` section (after `flatten`):
```toml
# Prettify field keys by replacing underscores with hyphens. Options: [true, false].
prettify-field-keys = true
```

### 3. Formatter pipeline (`src/formatting.rs`)

**RecordFormatter** (line 379):
- Add field: `prettify_field_keys: bool`

**build()** (line 305):
- Extract from config and combine with raw_fields:
  ```rust
  prettify_field_keys: !self.raw_fields && cfg.prettify_field_keys.unwrap_or(true),
  ```

**KeyPrefix::push()** (line 896):
- Add `prettify: bool` parameter
- Conditionally call `key_prettify` or raw extend

**FieldFormatter::begin()** (line 1211):
- Replace unconditional `key.key_prettify(buf)` with conditional:
  ```rust
  if self.rf.prettify_field_keys {
      key.key_prettify(buf);
  } else {
      buf.extend_from_slice(key.as_bytes());
  }
  ```

**Flattened path** (line 1181):
- Pass prettify flag: `fs.key_prefix.push(key, self.rf.prettify_field_keys)`

### 4. Tests (`src/formatting/tests.rs`)

Add tests verifying:
1. Default behavior (prettify=true): `k_a` → `k-a` (existing test already covers this)
2. prettify=false: `k_a` stays `k_a`
3. Flattened with prettify=false: `k_a.va.kb` preserves underscores in all segments
4. raw_fields=true overrides prettify: keys not prettified
