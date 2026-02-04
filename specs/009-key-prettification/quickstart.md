# Quickstart: Key Prettification Config Option

## Usage

### Disable underscore-to-hyphen replacement via config

Add to your `~/.config/hl/config.toml`:

```toml
[formatting]
prettify-field-keys = false
```

Field keys like `request_id` will now appear as `request_id` instead of `request-id`.

### Verify behavior

```bash
# With a JSON log like: {"ts":"2024-01-01T00:00:00Z","level":"info","msg":"test","request_id":"abc"}

# Default (prettify enabled):
hl app.log
# Output: ... request-id='abc'

# With prettify disabled in config:
hl app.log
# Output: ... request_id='abc'
```

## Development

### Build and test

```bash
cargo test
cargo clippy
```

### Key files

| File | Purpose |
|------|---------|
| `src/settings.rs` | `Formatting` struct with `prettify_field_keys` field |
| `src/formatting.rs` | Conditional `key_prettify` application |
| `src/formatting/tests.rs` | Unit tests |
| `etc/defaults/config.toml` | Default config with new option |
