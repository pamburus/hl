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

---

## Amendment 2026-10-21: Key Quoting (#1609)

### Verify behavior

```bash
printf '%s\n' '{"ts":"2026-10-21T10:00:00Z","level":"info","msg":"hello","user name":"alice smith","a=b":"x","":"empty","n":{"inner key":3}}' | hl -P --color never
# 2026-10-21 10:00:00.000 [INF] hello › "user name"="alice smith" "a=b"=x ""=empty n."inner key"=3

# Same input, no flattening:
#   … n={ "inner key"=3 }

# With --raw-fields, keys are shown as is:
#   … user name="alice smith" a=b="x" …

printf '%s\n' '{"ts":"2026-10-21T10:00:00Z","level":"info","msg":"hello","n":{"say \"hi\"":1,"line\nbreak":2}}' | hl -P --color never
# … n.'say "hi"'=1 n."line\nbreak"=2
```

Plain keys, such as `request_id` → `request-id` or `http.method`, are displayed exactly as before.
