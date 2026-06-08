# Phase 1 Data Model: Robust File Following

Entities derived from the spec's Key Entities plus the design in `research.md`. These are conceptual; exact Rust signatures live in `contracts/api.md`.

## Entities

### FollowOptions

Consumer-tunable settings; every field has a default so the simple path sets nothing.

| Field | Type | Default | Spec |
|-------|------|---------|------|
| `poll_interval` | duration | 1 s | SC-003, Assumptions |
| `recheck_cadence` | duration | 5 s | FR-006, SC-005 |
| `retry_missing` | bool | true | FR-002, FR-007 |
| `fallback_policy` | `FallbackPolicy` | `Conservative` | FR-009–FR-011, FR-023 |
| `read_buffer` | size | implementation default | FR-015a |

`FallbackPolicy = Conservative | Optimistic`. Only `Conservative` is exercised by hl; `Optimistic` is provided for advanced consumers (FR-023). (Per-entry tail-preload is **not** here — it stays in hl, which is delimiter-aware.)

### Reliability

Per-path classification result.

`Reliability = KnownLocal | NotConfirmed`

- `KnownLocal` — affirmatively known-local filesystem → native backend permitted (FR-010).
- `NotConfirmed` — known-remote, FUSE, Windows network drive, UNC, unknown type, or `statfs` error → polling (FR-011). Conservative default.

### FileId

Cross-platform file identity for rotation/replacement detection (FR-005, FR-019).

- Unix: `(dev: u64, ino: u64)`.
- Windows: `(volume_serial: u64, file_id: u128)` from `FILE_ID_INFO` (fallback `nFileIndexHigh/Low`).
- Equality semantics: two opens of the "same file" compare equal; a replacement compares unequal.

### Engine

The mechanism currently serving a path; may change at runtime (FR-014).

`Engine = Native | Polling`

Transitions: initial `Native` (only if `KnownLocal`) or `Polling`; `Native → Polling` on watch loss (one-way in this iteration; re-promotion is future work).

### FollowedSource (state machine)

Per-path tracked state owned by the facade. This is the heart of the no-data-loss guarantee.

**Fields**: `path`, `kind` (`Regular | Pipe`), `engine`, `fd` (open handle or none), `offset` (bytes consumed), `last_size`, `identity: Option<FileId>`, `state` (below).

**States**:

- `Following` — fd open, reading appended bytes from `offset`.
- `Draining` — identity change detected; reading the **old** fd to EOF before switching (race-free handoff, research §3).
- `Waiting` — path inaccessible/missing; retrying per `retry_missing` (FR-002, FR-007).

**Transitions** (evaluated on every wake = native event | poll tick | recheck tick):

| From | Condition (from authoritative `stat`/`fstat`) | Action | To |
|------|-----------------------------------------------|--------|----|
| Following | size grew | read `offset`→EOF, emit | Following |
| Following | size < offset (same identity) | seek 0, `offset=0`, read→EOF | Following |
| Following | path identity ≠ fd identity | drain old fd→EOF, emit | Draining |
| Draining | old fd fully drained | close old, open name @0, set identity | Following |
| Following | path missing | close fd | Waiting |
| Waiting | path present again | open name @0, set identity | Following |
| Following/Waiting | recheck tick, same size, identity changed | treat as rotation | Draining |

Pipe sources (`kind = Pipe`) skip identity/size logic entirely: read→EOF, then block for more; bounded buffer; never `Draining`.

### Event (core layer)

Advisory wake signal emitted by the core `Watcher`; the facade reconciles authoritatively (research §2). Names tentative.

`Event = DataAvailable(SourceId) | Rotated(SourceId) | Truncated(SourceId) | Removed(SourceId) | Reappeared(SourceId)`

Plus an internal periodic `Tick` ensuring convergence (FR-016) and driving the `recheck_cadence` (FR-006). Each event also carries enough context (current `FileId`/size snapshot where known) to let the facade reopen without re-deriving everything, but the facade never *depends* on event accuracy.

### SourceId

Stable handle identifying one followed path within a multi-path `Follower` (FR-025); attaches every emitted byte chunk to its origin path.

## Relationships

```text
Follower (facade)
  ├── owns 1 Watcher (core)            // engine routing, events
  │      └── classifies each path → Reliability → Engine
  └── owns N FollowedSource            // one per path, the state machine
         └── holds FileId, offset, fd  // reconciled from stat each wake
```

## Validation rules

- A directory, socket, or device path is rejected at `Follower` construction with a clear error (FR-026); only `Regular` and `Pipe` proceed.
- `offset` never exceeds current file size except transiently before a truncation/rotation is reconciled.
- On any wake, the old fd is fully drained before its handle is dropped (invariant guaranteeing FR-005 no-loss).
- Memory held per source is bounded by `read_buffer` (regular) or the bounded pipe buffer (pipe), never by backlog size (FR-015a).
