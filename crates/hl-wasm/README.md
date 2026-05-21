# hl-wasm

Browser-based log viewer powered by the `hl` parser/formatter compiled to WebAssembly.

The crate exposes a tiny WASM module with two entry points:

- `init()` — initialize the renderer (call once).
- `format_line(bytes)` — render one or more JSON / logfmt log records to HTML.

The companion `www/` directory contains an HTML/JS shell that fetches log files via HTTP Range
requests, indexes line offsets lazily, and renders the visible window with a recycled DOM pool.

## Building

```sh
# install once
cargo install wasm-pack

# build the WASM module into www/pkg/
wasm-pack build crates/hl-wasm --target web --release --out-dir www/pkg
```

If you prefer not to use `wasm-pack`, the equivalent flow with `wasm-bindgen-cli`:

```sh
# install wasm-bindgen-cli matching the Cargo.lock version
cargo install --version $(cargo tree -p hl-wasm -i wasm-bindgen -e =features | head -n1 | awk '{print $2}' | tr -d 'v') wasm-bindgen-cli

# build
cargo build --release --target wasm32-unknown-unknown --manifest-path crates/hl-wasm/Cargo.toml

# generate JS bindings
wasm-bindgen \
  crates/hl-wasm/target/wasm32-unknown-unknown/release/hl_wasm.wasm \
  --target web \
  --out-dir crates/hl-wasm/www/pkg
```

## Running

Serve `crates/hl-wasm/www/` over any static HTTP server, then open the page in a browser:

```sh
python3 -m http.server --directory crates/hl-wasm/www 8080
# open http://localhost:8080/?src=<absolute-url-to-a-log-file>
```

The log URL must be reachable with CORS allowed and (for large files) must support
`Range` requests. If the server doesn't honor ranges the viewer falls back to a full GET.

## Performance budget

- Bundle size target: < 200 KiB gzipped (WASM + JS glue).
- Scroll: 60 fps on a 100 k-line log with a recycled row pool sized to viewport + 4.
- Fetcher prefetches two viewport pages above and below the visible window.
- An LRU caches the last 32 fetched 256 KiB chunks (~8 MiB total).

## Limitations (v1)

- No filtering / search UI (the underlying `hl::Query` does compile in, but no UI surfaces it).
- No follow-tail.
- No compressed sources (`.gz` / `.xz`).
- Variable line heights aren't currently corrected — every row is exactly `ROW_HEIGHT_PX`.
  Very long lines are clipped to a single row.
