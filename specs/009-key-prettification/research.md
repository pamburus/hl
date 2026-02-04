# Research: Key Prettification Config Option

## Decision 1: Option naming convention

**Decision**: `prettify-field-keys` (config) / `prettify_field_keys` (Rust)

**Rationale**: The existing `KeyPrettify` trait (formatting.rs:1302) and the `--raw-fields` help text ("without unescaping or prettifying") establish "prettify" as the canonical term. The name is specific enough to convey underscore-to-hyphen replacement while remaining concise. Config uses kebab-case per `#[serde(rename_all = "kebab-case")]` on `Formatting` struct.

**Alternatives considered**:
- `replace-underscores-with-hyphens`: Maximally self-documenting but verbose
- `replace-underscores`: Ambiguous about replacement target
- `underscore-replacement`: Noun form, unclear as a boolean
- `key-prettification`: Not idiomatic for the existing codebase

## Decision 2: Config-only (no CLI flags)

**Decision**: Expose this option only in the config file, not as CLI flags.

**Rationale**: This is a personal preference that users set once and leave. It is unlikely to change between runs. Adding CLI flags would unnecessarily increase the help output surface area.

**Alternatives considered**:
- Dual-flag pattern (`--prettify-field-keys` / `--no-prettify-field-keys`): Following the `--hide-empty-fields` / `--show-empty-fields` pattern. Rejected as unnecessary for a set-once preference.

## Decision 3: `--raw-fields` interaction

**Decision**: When `raw_fields=true`, force `prettify_field_keys=false` in the builder's `build()` method.

**Rationale**: The `--raw-fields` help text says "without unescaping or prettifying" and FR-005 requires `--raw-fields` to take precedence. Currently `--raw-fields` does NOT disable key prettification (only value unescaping), so this is a minor behavior correction that aligns implementation with documented intent.

**Alternatives considered**:
- Keep `--raw-fields` behavior unchanged and make the options fully independent: Would violate FR-005 and contradict the documented meaning of `--raw-fields`

## Decision 4: Data flow via existing `Formatting` path

**Decision**: The option flows through the existing `Formatting` struct → `with_options()` → `build()` path. No new builder method, no changes to `Options`, `cli.rs`, `main.rs`, or `app.rs`.

**Rationale**: `Formatting` is already passed to `RecordFormatterBuilder` via `with_options()` (app.rs:902) and stored as `cfg: Option<Formatting>`. In `build()`, the new field is extracted from `cfg` alongside existing fields like `cfg.punctuation`. This minimizes the change footprint.

**Alternatives considered**:
- Adding a separate `with_prettify_field_keys()` builder method and threading through `Options`: Would require changes to 5+ files for no benefit when the existing config path works

## Decision 5: Where to gate the prettification

**Decision**: Two call sites modified — `FieldFormatter::begin()` (formatting.rs:1211) and `KeyPrefix::push()` (formatting.rs:901). Both check `prettify_field_keys` from `RecordFormatter`.

**Rationale**: These are the only two places `key_prettify` is called. The `KeyPrefix::push()` method gains a `prettify: bool` parameter rather than accessing the flag through a longer reference chain, keeping `KeyPrefix` simple and self-contained.

**Alternatives considered**:
- Adding `prettify_field_keys` to `FormattingState`: Would work but adds state that doesn't change during formatting; keeping it on `RecordFormatter` (which is immutable during formatting) is cleaner
- Single gate in `key_prettify` itself: Would require threading the flag through the trait, changing a well-optimized hot path
