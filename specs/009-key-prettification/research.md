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

---

## Amendment 2026-10-21: Key Quoting (#1609)

## Decision 6: Trigger set for keys

**Decision**: Quote when the key is empty or its bytes contain any of the `Control`, `DoubleQuote`, `SingleQuote`, `Backslash`, `Backtick`, `Space`, `Tab`, `Newline` or `EqualSign` character classes. This is the value `NON_PLAIN` mask.

**Rationale**: It is exactly the set values use to detect characters that break `key=value` parsing (FR-006). Values additionally quote number-like strings, `true`/`false`/`null` and strings that start with `{`/`[`, because those could be mistaken for other value types. A key in key position cannot be mistaken that way (FR-007).

**Alternatives considered**:
- Reusing `ValueFormatAuto` as is: it trims trailing whitespace and quotes number-like keys, which would break the edge case for keys with trailing spaces and change output for keys like `0` or `true`.
- Adding `.` to the set: it would quote common dotted keys such as `http.method` (rejected in spec Assumptions).

## Decision 7: Decide on the original key, not the prettified one

**Decision**: Analyze the original key bytes, then write the prettified key between the quotes.

**Rationale**: Prettification maps `_` to `-`, and neither is in the trigger set or the quote-style conditions, so the analysis gives the same result before and after. Analyzing first means the call sites write the opening quote first and never rotate or re-copy the buffer. That matters for `KeyPrefix`, whose buffer is not a contiguous `Vec<u8>`.

**Alternatives considered**:
- Prettify, then analyze the output and rotate the quote into place (as the value formats do): needs a contiguous slice, which `KeyPrefix` does not have.

## Decision 8: Quote-style cascade

**Decision**: Use the value cascade with keys treated as always single-line: `"` → `'` → `` ` `` → JSON-escaped `"`. Backticks are used only when the key has no tab or newline, matching the value behaviour under `ExtendedSpaceAction::Escape`.

**Rationale**: FR-008 requires the same rules as values. FR-009 requires keys to stay on one line, which is the `Escape` mode for values.

## Decision 9: Escape fallback

**Decision**: For the JSON-escaped fallback, append the original key as a JSON string to a temporary buffer with `encstr`'s JSON appender, then prettify the escaped bytes when prettification is on. JSON escape sequences never contain `_`, so the order does not change the result, and the prettification rule stays in one place.

**Rationale**: The fallback needs control characters, which real keys almost never contain, so a temporary allocation there costs nothing in practice and keeps the common path allocation-free.

## Decision 10: Gate on `unescape_fields`

**Decision**: Quote keys only when `RecordFormatter::unescape_fields` is `true`.

**Rationale**: It is `false` exactly when `--raw-fields` is active, and FR-005/FR-013 say `--raw-fields` displays keys as is. No new state is needed.

## Decision 11: One shared key writer for both call sites

**Decision**: Add a single key-writing helper used by `FieldFormatter::begin()` and `KeyPrefix::push()`. It takes the key, the prettify flag, the quote flag and a `Push<u8>` buffer.

**Rationale**: Both sites already duplicate the prettify-or-copy branch. Folding quoting into one helper keeps their behaviour identical, which FR-010 and FR-011 rely on.
