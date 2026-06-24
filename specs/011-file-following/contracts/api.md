# Phase 1 API Contract: `fsmon` crate

The "contract" for a library feature is its public Rust surface. Signatures below are the proposed contract; types map to `data-model.md`. Names are indicative and may be refined in implementation, but the **shape** (two layers, default-driven facade, advisory-event core) is fixed by the plan.

## Facade layer (the simple, default path)

```rust
// crates/fsmon/src/follow/mod.rs

/// Follow one path with default options. The simplest entry point.
/// Accepts anything path-like (`&str`, `&Path`, `PathBuf`, ...), matching std.
pub fn follow(path: impl AsRef<Path>) -> Result<Follower>;

/// Follow one or more paths with explicit options (FR-022, FR-023, FR-025).
pub struct Follower { /* ... */ }

impl Follower {
    /// Accepts any iterable of path-like items: `Vec<PathBuf>`, `&[PathBuf]`
    /// (since `&PathBuf: AsRef<Path>`), `["a.log", "b.log"]`, `[&Path]`, etc.
    pub fn new<I>(paths: I, options: FollowOptions) -> Result<Self>
    where
        I: IntoIterator,
        I::Item: AsRef<Path>;

    /// Blocking iterator of newly-appended bytes, tagged by source (FR-003, FR-013).
    /// Rotation/truncation/reopen are hidden; reading happens here so the
    /// consumer's pace governs (backpressure, FR-015a).
    pub fn next_chunk(&mut self) -> Result<Option<Chunk>>; // None = all sources ended (non-retry) 
}

/// A run of bytes from one source. `bytes` are raw; the consumer splits on its
/// own delimiter (hl keeps its Scanner).
pub struct Chunk {
    pub source: SourceId,
    pub bytes: Bytes,
}

pub struct SourceId(/* opaque, stable per Follower */);
```

Single-source convenience (what hl uses per reader thread):

```rust
impl Follower {
    /// True when this Follower tracks exactly one source; lets hl treat the
    /// stream as an unlabeled byte reader feeding its existing Scanner.
    pub fn into_reader(self) -> impl std::io::Read + Send; // blocks for more data; spans reopens
}
```

`Read::read` returns appended bytes as they become available and transparently continues across rotation/truncation; it returns `Ok(0)` only when the source has permanently ended (and retry is disabled).

## Options

```rust
// crates/fsmon/src/options.rs

#[derive(Clone)]
pub struct FollowOptions {
    pub poll_interval: Duration,   // default 1s
    pub recheck_cadence: Duration, // default 5s
    pub retry_missing: bool,       // default true
    pub fallback_policy: FallbackPolicy, // default Conservative
    pub read_buffer: usize,        // default implementation-chosen
}

impl Default for FollowOptions { /* the defaults above */ }

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FallbackPolicy { Conservative, Optimistic } // hl uses Conservative
```

## Core layer (advanced / extension surface)

```rust
// crates/fsmon/src/watch/mod.rs

/// Event-level watcher: classification + per-path engine routing + degradation.
/// Consumers at this layer own their own reading (this is what the deferred
/// tail-style messages, OOS-001, will use).
pub struct Watcher { /* ... */ }

impl Watcher {
    pub fn new<I>(paths: I, options: FollowOptions) -> Result<Self>
    where
        I: IntoIterator,
        I::Item: AsRef<Path>;

    /// Block for the next advisory event. Events are hints; authoritative state
    /// must be re-derived via stat by the consumer (research §2).
    pub fn recv(&self) -> Result<Event>;

    /// Current engine for a source (Native | Polling), for diagnostics/tests.
    pub fn engine(&self, source: SourceId) -> Engine;
}

pub enum Event {
    DataAvailable(SourceId),
    Rotated(SourceId),
    Truncated(SourceId),
    Removed(SourceId),
    Reappeared(SourceId),
}

pub enum Engine { Native, Polling }
```

## Classification (exposed for testing / advanced use)

```rust
// crates/fsmon/src/classify/mod.rs
pub enum Reliability { KnownLocal, NotConfirmed }
pub fn classify(path: &Path) -> Reliability; // conservative: errors/unknown => NotConfirmed
```

## Errors

```rust
// crates/fsmon/src/lib.rs
pub type Result<T> = std::result::Result<T, Error>;

#[non_exhaustive]
pub enum Error {
    /// Unsupported input type (directory / socket / device) — FR-026.
    UnsupportedInput { path: PathBuf, kind: &'static str },
    /// Underlying watch backend failure.
    Watch(/* notify::Error */),
    /// I/O error while opening/reading a followed file.
    Io(std::io::Error),
}
```

## Observability contract (FR-016a)

The crate emits, via the `log` crate, at least:

- engine selection per path at follow-start (`debug`),
- conservative fallback decisions with the classification reason (`debug`),
- runtime native→polling degradation events (`info`/`warn`).

The consumer (hl) routes these to its existing `HL_DEBUG_LOG` channel. Nothing is written to stdout/stderr by the crate itself (clean default output).

## Contract test obligations

Each contract element maps to required tests (Phase 6):

| Contract element | Test |
|------------------|------|
| `follow()` / `into_reader()` append | bytes appear in order (SC-001) |
| reader across rotation | no loss, continues on new file (FR-005, SC-002) |
| reader across truncation | re-reads from 0 (FR-004) |
| reader across delete→recreate | resumes (FR-007) |
| same-size replacement | detected within `recheck_cadence` (FR-006, SC-005) |
| `classify()` | KnownLocal vs NotConfirmed incl. conservative unknown (FR-009–FR-011) |
| `UnsupportedInput` | directory/socket rejected; FIFO accepted as stream (FR-026) |
| backpressure | bounded memory + no loss under fast writer (FR-015a, SC-011) |
| engine transparency | identical behavior native vs polling (SC-010) |
| degradation | follow continues after watch loss (FR-014, SC-007) |
