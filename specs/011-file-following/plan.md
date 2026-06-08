# Implementation Plan: Robust File Following (`tail -F` Parity)

**Branch**: `011-file-following` | **Date**: 2026-06-08 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/011-file-following/spec.md`

## Summary

Extract and harden `hl`'s file-following capability into a new reusable workspace crate that replicates GNU `tail -F` (follow-by-name + retry) semantics. The crate is organized in two layers: a thin **event-level core** that owns filesystem-reliability classification, per-path native-vs-polling engine selection, runtime degradation, cross-platform file-identity tracking, and the same-size re-check safety net; and a **byte-stream facade** that wraps the core, owns the read/reopen/offset state machine, and hands consumers a simple continuous per-source byte stream with rotation/truncation hidden. The `hl` binary consumes the facade — replacing the current `src/fsmon.rs` module and the manual reopen/offset block in `app.rs::follow()` — while keeping its existing `Scanner`/`Segment`/timestamp-merge pipeline and its per-entry tail-preload unchanged.

The defining correctness property is **conservative reconciliation from authoritative `stat`**: every wake (native event, poll tick, or safety-net tick) drives the facade to re-derive ground truth from the filesystem and to fully drain the old file descriptor before switching to a replacement, so no committed bytes are ever lost across rotation, truncation, deletion, or replacement — independent of whether the path is served by native notifications or polling.

## Technical Context

**Language/Version**: Rust (stable, edition 2024; workspace `rust-version = 1.86.0`)
**Primary Dependencies**: `notify` v8 (native backends + `PollWatcher`; already a workspace dependency), `libc` (Unix `statfs`/`fstatfs`, `st_dev`/`st_ino`), `windows-sys` (Windows `GetDriveTypeW`, `GetFileInformationByHandle`/`FILE_ID_INFO`, UNC detection; already transitive in the dependency tree), `kqueue` v1 (macOS, existing), `log` (diagnostics)
**Storage**: N/A (operates on filesystem paths; no persisted state)
**Testing**: `cargo test` (unit + integration), `rstest` (parametrized cases), temp-directory fixtures for filesystem behavior; cross-platform CI on Linux/macOS/Windows
**Target Platform**: Linux, macOS, Windows (native + polling on each)
**Project Type**: Single CLI application plus reusable workspace library crate
**Performance Goals**: Local append visible within 200 ms (SC-001); remote/polled append within one poll interval, default 1 s (SC-003); negligible idle CPU / no busy-wait (SC-004, FR-015)
**Constraints**: Memory-bounded under fast writers with backpressure and zero committed-data loss (FR-015a, SC-011); conservative classification (anything not affirmatively known-local is polled, FR-009–FR-012); mechanism transparent to the consumer (SC-010); maintain or exceed current test coverage (SC-009)
**Scale/Scope**: Following a handful of files per invocation (typical), each independently routed

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Performance First | ✅ Pass | Native notifications where reliable; polling only where needed; no busy-wait; reading at consumer pace leaves data on disk (bounded memory). Latency targets in SC-001/SC-003. |
| II. Composability & Modularity | ✅ Pass | Core logic extracted into a standalone, reusable crate with a clear two-layer interface (event core + byte-stream facade); mirrors the existing `crates/pager` extraction. Directly realizes this principle. |
| III. User Experience & Intuitiveness | ✅ Pass | Sensible defaults (conservative fallback, 1 s poll, 5 s re-check, default tail); "no silent failures" satisfied via debug-log observability (FR-016a) without polluting normal output. |
| IV. Reliability & Robustness | ✅ Pass | Reconcile-from-`stat` + drain-before-switch guarantees no data loss across rotation/truncation/deletion; converges after dropped/overflowed events (FR-016); no panics on any input type (FR-026). |
| V. Test-First Development | ✅ Pass | Behavior-driven tests for each follow scenario and classification path, written against the crate's public contract before/with implementation; property/stress test for backpressure (SC-011). |
| VI. Specification Integrity | ✅ Pass | New crate and new requirements; no identifier renumbering. Internal `mod fsmon` replaced by the crate of the same capability. |
| VII. Test Data Management | ✅ Pass | Filesystem scenarios use programmatic temp-dir fixtures (the data *is* filesystem state, not structured literals); no inline structured data. |

No violations requiring Complexity Tracking.

## Project Structure

### Documentation (this feature)

```text
specs/011-file-following/
├── plan.md              # This file
├── spec.md              # Feature specification (clarified)
├── research.md          # Phase 0 output — decisions & rationale
├── data-model.md        # Phase 1 output — entities & state machine
├── quickstart.md        # Phase 1 output — consumer usage
├── contracts/
│   └── api.md           # Phase 1 output — crate public API contract
└── checklists/
    └── requirements.md  # Specification quality checklist
```

### Source Code (repository root)

```text
crates/fsmon/                    # NEW reusable crate (name tentative; see research.md)
├── Cargo.toml                   # notify, libc (unix), windows-sys (windows), kqueue (macos), log
├── src/
│   ├── lib.rs                   # Crate root; re-exports watch + follow; Options, Error
│   ├── options.rs               # FollowOptions (poll interval, fallback policy, retry, recheck cadence, tail) with Defaults
│   ├── watch/                   # CORE: event-level layer
│   │   ├── mod.rs               # Watcher, Event (DataAvailable/Rotated/Truncated/Removed/Reappeared), engine routing
│   │   ├── engine.rs            # Per-path native-vs-poll selection + runtime degradation
│   │   └── backend/
│   │       ├── native.rs        # notify RecommendedWatcher wrapper
│   │       ├── poll.rs          # notify PollWatcher wrapper
│   │       └── kqueue_macos.rs  # existing kqueue loop, gated by classification
│   ├── follow/                  # FACADE: byte-stream layer
│   │   ├── mod.rs               # Follower (single + multi path), per-source byte stream iterator
│   │   └── source.rs            # Per-source read/reopen/offset state machine (reconcile-from-stat, drain-before-switch)
│   ├── classify/                # Filesystem-reliability classification
│   │   ├── mod.rs               # Reliability enum + classify(path) dispatch + conservative default
│   │   ├── linux.rs             # statfs f_type against known-local magic table
│   │   ├── macos.rs             # statfs f_fstypename allowlist
│   │   └── windows.rs           # GetDriveTypeW + UNC detection
│   └── identity/                # File-identity (rotation/replacement detection)
│       ├── mod.rs               # FileId abstraction
│       ├── unix.rs              # (dev, ino) via libc
│       └── windows.rs           # (volume serial, file id) via GetFileInformationByHandle/FILE_ID_INFO
└── tests/
    ├── follow_regular.rs        # append/rotate/truncate/delete-recreate/same-size-replace
    ├── follow_pipe.rs           # FIFO streaming; directory/special-file rejection
    ├── classification.rs        # local vs remote vs unknown (where env permits)
    └── backpressure.rs          # fast-writer bounded-memory / no-loss stress

src/                             # hl binary changes
├── fsmon.rs                     # REMOVE — superseded by the crate
├── lib.rs                       # MODIFY — drop `mod fsmon;`, depend on the crate
├── app.rs                       # MODIFY — follow(): run Scanner over the facade stream; remove manual reopen/offset block (app.rs:636-662); keep Input::tail preload (app.rs:618)
└── error.rs                     # MODIFY — map the crate's error into hl's Error (replace direct notify::Error mapping)

Cargo.toml                       # MODIFY — add crates/fsmon to workspace members; add path dep; notify/libc move under the crate where appropriate
```

**Structure Decision**: Single binary plus a new `crates/fsmon` library, following the established `crates/pager` precedent. The crate is self-contained (all classification, engine selection, and platform-specific code live inside it) and exposes two layers so simple consumers use the facade while advanced consumers (and the deferred `tail`-style messages, OOS-001) can use the core event API.

## Implementation Phases

### Phase 1: Crate scaffold & options

**Goal**: Create the crate, wire it into the workspace, define the public option surface with defaults.

1. Add `crates/fsmon` to workspace members; create `Cargo.toml` with platform-gated deps (`notify`, `log`; `libc` on unix, `kqueue` on macOS, `windows-sys` with `Win32_Storage_FileSystem`/`Win32_System_IO`/`Win32_Foundation` features on windows).
2. `options.rs`: `FollowOptions { poll_interval=1s, fallback_policy=Conservative, retry_missing=true, recheck_cadence=5s, tail=<consumer-set> }` with a `Default` impl; `FallbackPolicy { Conservative, Optimistic }` (only Conservative is exercised by hl; Optimistic exists for advanced consumers per spec FR-023 / Assumptions).
3. `lib.rs`: error type, re-exports, crate docs.

### Phase 2: Classification & identity (platform layer)

**Goal**: Per-path reliability classification and cross-platform file identity (FR-009–FR-012, FR-018/FR-019).

1. `identity/`: `FileId` from open handle/metadata — `(dev, ino)` on unix; `(volume serial, file id)` via `GetFileInformationByHandle`/`FILE_ID_INFO` on windows.
2. `classify/`: `Reliability { KnownLocal, NotConfirmed }`; Linux `statfs` `f_type` matched against the coreutils/gnulib known-local magic table; macOS `statfs` `f_fstypename` allowlist (`apfs`,`hfs`,…); Windows `GetDriveTypeW` (`DRIVE_REMOTE`→NotConfirmed) + UNC path detection. Any error or unknown type → `NotConfirmed` (conservative).
3. Unit tests per platform for the classification table and identity equality/inequality across simulated rotation.

### Phase 3: Core watcher (event layer)

**Goal**: Per-path engine selection, event emission, runtime degradation, convergence (FR-010–FR-016, FR-016a).

1. `watch/engine.rs`: route each path by `Reliability` — `KnownLocal`→native, else→poll; per-path independence (FR-012).
2. `watch/backend/`: thin wrappers over `notify` `RecommendedWatcher` (native) and `PollWatcher` (poll, `poll_interval`); macOS kqueue loop retained but only used for `KnownLocal` paths (FR-020). On a native watch error/loss, migrate that path to poll in place (FR-014) and emit an observable diagnostic (FR-016a).
3. `watch/mod.rs`: coalesce raw backend events into a per-path "dirty" signal plus lifecycle hints; emit `Event` { `DataAvailable`, `Rotated`, `Truncated`, `Removed`, `Reappeared` } (semantic, but advisory — the facade re-derives truth from `stat`). Emit a periodic tick at `min(applicable intervals)` so missed/overflowed events still converge (FR-016) and the 5 s same-size re-check runs (FR-006).
4. Diagnostics routed through `log` so the consumer (hl) surfaces them via `HL_DEBUG_LOG`.

### Phase 4: Facade (byte-stream layer)

**Goal**: Per-source continuous byte stream hiding reopen; memory-bounded backpressure; input-type handling (FR-001–FR-008, FR-013, FR-015a, FR-026).

1. `follow/source.rs`: per-source state machine — track `FileId`, offset, last size. On each wake: drain current fd from offset to EOF (emit bytes); then reconcile via `stat`: identity change → **drain old fd fully, then** close/reopen by name from offset 0 (FR-005, race-free handoff); size shra­nk → seek 0 (FR-004); missing → mark waiting, retry (FR-002, FR-007); reappeared → open from 0 (FR-007). Reading happens on the consumer's thread so consumer pace governs (backpressure; data waits on disk).
2. Input types (FR-026): regular files → full tracking; FIFO/pipe → continuous read-to-EOF-then-wait with a bounded buffer (no identity tracking); directory/socket/device → reject with a clear error at follow-start.
3. `follow/mod.rs`: `Follower` exposing both single-path (one source) and multi-path (single request, source-tagged output, FR-025) construction; a blocking iterator yielding `(SourceId, bytes)` / per-source readers.

### Phase 5: hl integration

**Goal**: Consume the facade; delete the old module; preserve hl's pipeline.

1. `Cargo.toml`/`lib.rs`: add the path dependency; remove `mod fsmon;` and `src/fsmon.rs`.
2. `app.rs::follow()`: keep `Input::tail` preload (app.rs:618); replace the `fsmon::run` + manual reopen/offset block (app.rs:636-662) with a facade byte stream that the existing `Scanner` consumes. Preserve the `thread::scope` reader/worker/merger structure — each reader thread drives one single-path `Follower` and feeds its `Scanner`.
3. `error.rs`: map the crate's error type into hl's `Error` (replacing the direct `notify::Error` conversion).
4. Wire diagnostics into the `HL_DEBUG_LOG` channel.

### Phase 6: Testing & coverage

**Goal**: Verify every behavior on every platform; meet SC-009.

1. Crate integration tests (temp-dir fixtures): append, rotate (rename+recreate), truncate, delete→recreate, same-size replace, start-before-exists; FIFO streaming and directory/special rejection; backpressure stress (fast writer, assert bounded memory + zero loss).
2. Classification unit tests (local always; remote/unknown where the environment allows, otherwise documented as environment-gated per constitution coverage policy).
3. hl-level follow integration test through the new path; run `just uncovered` and add tests for changed lines.

## Dependencies

### New crate dependencies (inside `crates/fsmon`)

| Crate | Scope | Purpose |
|-------|-------|---------|
| `notify` v8 | all | Native backends + `PollWatcher` |
| `log` | all | Observable diagnostics (FR-016a) |
| `libc` | `cfg(unix)` | `statfs`/`fstatfs`, `st_dev`/`st_ino` |
| `kqueue` v1 | `cfg(macos)` | Existing native macOS loop |
| `windows-sys` | `cfg(windows)` | `GetDriveTypeW`, `GetFileInformationByHandle`/`FILE_ID_INFO`, UNC detection |

No new third-party crates enter the workspace that are not already present transitively; `notify`/`libc`/`kqueue` move/extend into the crate.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Race on rotation losing trailing bytes of the old file | Medium | High | Drain old fd to EOF *before* switching; reconcile from `stat`; covered by rotation tests |
| Missed/overflowed native events stall following | Low | High | Periodic reconcile tick regardless of events (FR-016); safety-net re-check |
| Windows file-id semantics differ from inode | Medium | Medium | Use `FILE_ID_INFO` (128-bit) where available, fall back to `nFileIndex*`; explicit identity tests |
| Classification false-negative (local seen as remote) | Low | Low | Conservative by design — worst case is polling a local file (correct, slightly higher latency) |
| Behavior divergence native vs poll | Medium | Medium | Facade reconciles identically from `stat` regardless of engine; SC-010 transparency test |
| Cross-platform CI gaps on the dev's fresh Windows env | Medium | Low | Tooling (markdownlint/audit/tombi/taplo/nightly) addressed separately; `cargo test` is the gate for this feature |

## Milestones

1. **M1**: Crate scaffold + options (Phase 1)
2. **M2**: Classification + identity, per-platform (Phase 2)
3. **M3**: Core watcher with engine routing + degradation (Phase 3)
4. **M4**: Facade byte-stream state machine + input types (Phase 4)
5. **M5**: hl integration; old module removed (Phase 5)
6. **M6**: Full cross-platform tests + coverage (Phase 6)

## Complexity Tracking

No constitution violations requiring justification.
