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
