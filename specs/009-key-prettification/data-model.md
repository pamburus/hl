# Data Model: Key Prettification Config Option

## Modified Entities

### Formatting (settings.rs)

Config-level representation of formatting preferences.

| Field | Type | Default | Notes |
|-------|------|---------|-------|
| flatten | `Option<FlattenOption>` | `None` | Existing |
| expansion | `ExpansionOptions` | `default()` | Existing |
| message | `MessageFormatting` | `default()` | Existing |
| punctuation | `Punctuation` | `default()` | Existing |
| **prettify_field_keys** | **`Option<bool>`** | **`None`** | **New. `None` = use default (`true`). Deserialized from TOML as `prettify-field-keys`.** |

### RecordFormatter (formatting.rs)

Immutable formatter used during record processing.

| Field | Type | Notes |
|-------|------|-------|
| ... | ... | Existing fields unchanged |
| **prettify_field_keys** | **`bool`** | **New. Set to `!raw_fields && cfg.prettify_field_keys.unwrap_or(true)` during `build()`.** |

## Modified Functions

### KeyPrefix::push()

| Parameter | Type | Notes |
|-----------|------|-------|
| key | `&str` | Existing |
| **prettify** | **`bool`** | **New. When true, applies `key_prettify`; when false, copies key bytes verbatim.** |

## Config File (TOML)

```toml
[formatting]
prettify-field-keys = true   # New field, default true
```

## State Transitions

N/A — This is a stateless boolean flag. No lifecycle transitions.

## Validation Rules

- `prettify-field-keys` in TOML must be a boolean (`true` or `false`)
- Invalid values cause a deserialization error (handled by serde)

---

## Amendment 2026-10-21: Key Quoting (#1609)

No new config fields and no new formatter state. Quoting is gated on the existing `RecordFormatter::unescape_fields`.

### Key quote style (derived per key)

| Style | Chosen when the key… | Example |
|-------|----------------------|---------|
| Plain | is non-empty and has none of the trigger classes (research Decision 6) | `user-id` |
| Double-quoted | has no `"`, backslash, control, tab or newline | `"user name"` |
| Single-quoted | has `"` but no `'`, backslash, control, tab or newline | `'say "hi"'` |
| Backtick-quoted | has `"` and `'` but no backtick, control, tab or newline | `` `it's "x"` `` |
| Escaped | anything else | `"line\nbreak"` |

An empty key is double-quoted: `""`.

### Modified functions

| Function | Change |
|----------|--------|
| `KeyPrefix::push()` | Gains a quote flag; quotes the pushed segment when needed |
| `FieldFormatter::begin()` | Writes the key through the shared key writer |
