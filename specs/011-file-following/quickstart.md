# Quickstart: `fsmon` crate

How a consumer uses the crate. The whole point is that the simple case needs no knowledge of classification, engine selection, or rotation handling.

## Simplest case — follow one file

```rust
use fsmon::follow;

let mut reader = follow("/var/log/app.log")?.into_reader();

// `reader` is a normal blocking `Read`. It yields appended bytes as they
// arrive and transparently continues across log rotation and truncation.
// On a network share it polls automatically; on local disk it uses native
// notifications. You don't configure any of that.
let mut buf = [0u8; 64 * 1024];
loop {
    let n = reader.read(&mut buf)?;
    if n == 0 { break; } // source ended (retry disabled)
    sink.write_all(&buf[..n])?;
}
```

## How hl uses it (per reader thread)

hl keeps its delimiter-aware tail-preload and its `Scanner`; the facade only
replaces the watch + reopen/offset bookkeeping.

```rust
// inside app.rs follow(), per input thread (sketch)
let mut input = input_ref.open()?.tail(self.options.tail, delimiter.clone())?; // unchanged preload
// ... emit the preloaded tail via the existing Scanner ...

// replace the old fsmon::run + manual reopen/offset block with:
// `into_reader()` returns `impl std::io::Read + Send`, and `Scanner::items`
// takes `&mut dyn Read` (src/scanning.rs:41) — so the facade reader plugs in
// directly. No `Stream`/`as_sequential()` wrapper is involved here: that is
// hl's own adapter (src/input.rs:392) for its dual-mode `Input::stream`, not
// part of the fsmon crate.
let mut reader = fsmon::follow(path.canonical.clone())?.into_reader();
for item in scanner.items(&mut reader).with_max_segment_size(self.options.max_message_size.into()) {
    txi.send((i, j, item?))?;
}
```

Rotation, truncation, deletion/recreation, network-share polling, and Windows
file-id rotation detection are all handled inside `into_reader()`.

> Handoff note: the tail-preload (`Input::tail`) reads the last N entries up to
> the current end of file; the facade reader must begin at that same end-of-file
> offset so the live stream continues with no gap or duplication. In the crate
> this means `into_reader()` opens at current EOF by default (the per-source
> state machine's initial `offset = last_size`).

## Following several files at once (multi-path)

```rust
use fsmon::{Follower, FollowOptions};

// Any iterable of path-like items works (&str, &Path, PathBuf, &[PathBuf], ...).
let mut f = Follower::new(["a.log", "b.log"], FollowOptions::default())?;

while let Some(chunk) = f.next_chunk()? {
    // chunk.source identifies which file; chunk.bytes are raw appended bytes
    route(chunk.source, &chunk.bytes);
}
```

## Tuning (only if you need it)

```rust
let opts = FollowOptions {
    poll_interval: Duration::from_millis(500), // faster polling on slow shares
    recheck_cadence: Duration::from_secs(2),   // catch same-size replacement sooner
    retry_missing: true,                       // keep waiting for a not-yet-existing file
    ..FollowOptions::default()
};
let f = Follower::new(paths, opts)?;
```

## Advanced — react to events yourself (core layer)

For consumers that want to render `tail`-style status ("file truncated",
"has appeared") or do their own reading:

```rust
use fsmon::watch::{Watcher, Event};

let w = Watcher::new(paths, FollowOptions::default())?;
loop {
    match w.recv()? {
        Event::Rotated(s)   => log::info!("{s:?}: following new file"),
        Event::Truncated(s) => log::info!("{s:?}: file truncated"),
        Event::Removed(s)   => log::info!("{s:?}: became inaccessible"),
        Event::Reappeared(s)=> log::info!("{s:?}: has appeared"),
        Event::DataAvailable(s) => { /* re-stat and read yourself */ }
    }
}
```

## Observability

Engine choice, conservative fallback, and runtime degradation are emitted via
the `log` crate (no stdout/stderr noise). In hl they surface through
`HL_DEBUG_LOG`:

```text
HL_DEBUG_LOG=debug hl -F /mnt/nfs/app.log
# ... fsmon: /mnt/nfs/app.log classified NotConfirmed (nfs) -> polling
```
