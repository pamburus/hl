---
description: "Task list for robust file following (tail -F parity)"
---

# Tasks: Robust File Following (`tail -F` Parity)

**Input**: Design documents from `/specs/011-file-following/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/api.md

**Tests**: INCLUDED. The constitution mandates Test-First Development (Principle V) and SC-009 sets a coverage bar, so each user story has test tasks to be written before its implementation.

**Organization**: Tasks are grouped by user story. Note: this is a tightly-coupled library, so the Foundational phase is substantial (shared scaffold, classification, identity, watcher, facade skeleton) and several P1 stories extend the same core files — see Dependencies for the real ordering.

**Crate name**: `fsmon` is used throughout as the tentative crate name (see research.md §1); if renamed at implementation start, substitute consistently.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1–US6)
- File paths are exact; crate paths are under `crates/fsmon/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create the crate, wire it into the workspace, define the public option surface.

- [ ] T001 Create `crates/fsmon/Cargo.toml` with platform-gated deps (`notify` v8, `log`; `libc` on `cfg(unix)`; `kqueue` v1 on `cfg(macos)`; `windows-sys` with `Win32_Storage_FileSystem`/`Win32_System_IO`/`Win32_Foundation` on `cfg(windows)`) and add `"crates/fsmon"` to the `members` list in root `Cargo.toml`
- [ ] T002 Create `crates/fsmon/src/lib.rs` with module declarations, public `Error`/`Result` skeleton (`UnsupportedInput`/`Watch`/`Io` variants per contracts/api.md), and crate-level docs
- [ ] T003 [P] Define `FollowOptions` (poll_interval=1s, recheck_cadence=5s, retry_missing=true, fallback_policy=Conservative, read_buffer) with `Default`, and `FallbackPolicy { Conservative, Optimistic }`, in `crates/fsmon/src/options.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared platform primitives, the core watcher, and the facade skeleton that every user story builds on.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [ ] T004 [P] Define `FileId` abstraction in `crates/fsmon/src/identity/mod.rs` and the Unix impl (`(dev, ino)` via `libc`) in `crates/fsmon/src/identity/unix.rs`
- [ ] T005 [P] Implement Windows `FileId` (volume serial + 128-bit file id via `GetFileInformationByHandleEx`/`FILE_ID_INFO`, fallback `GetFileInformationByHandle` `nFileIndex*`) in `crates/fsmon/src/identity/windows.rs`
- [ ] T006 [P] Implement `Reliability { KnownLocal, NotConfirmed }` and the `classify(path)` dispatch with conservative default (any error/unknown → `NotConfirmed`) in `crates/fsmon/src/classify/mod.rs`
- [ ] T007 [P] Implement Linux classification (`statfs` `f_type` matched against the coreutils/gnulib known-local magic table) in `crates/fsmon/src/classify/linux.rs`
- [ ] T008 [P] Implement macOS classification (`statfs` `f_fstypename` known-local allowlist) in `crates/fsmon/src/classify/macos.rs`
- [ ] T009 [P] Implement Windows classification (`GetDriveTypeW` → `DRIVE_REMOTE` and UNC `\\server\share` detection → `NotConfirmed`) in `crates/fsmon/src/classify/windows.rs`
- [ ] T010 Define core `Event` enum (`DataAvailable`/`Rotated`/`Truncated`/`Removed`/`Reappeared` + internal `Tick`) and the `Watcher` skeleton (construct with `AsRef<Path>` items, `recv`, `engine`) in `crates/fsmon/src/watch/mod.rs`
- [ ] T011 [P] Implement the native backend wrapper over `notify` `RecommendedWatcher` in `crates/fsmon/src/watch/backend/native.rs`
- [ ] T012 [P] Implement the polling backend wrapper over `notify` `PollWatcher` (using `poll_interval`) in `crates/fsmon/src/watch/backend/poll.rs`
- [ ] T013 Implement engine routing (per-path `classify` → `Native`/`Polling`), raw-event coalescing to a per-path dirty signal, and the periodic tick at `min(applicable intervals)` in `crates/fsmon/src/watch/engine.rs` (depends on T006–T012)
- [ ] T014 Implement the input-type guard — classify path as `Regular` vs `Pipe`; reject directory/socket/device with `Error::UnsupportedInput` (FR-026) — in `crates/fsmon/src/follow/source.rs`
- [ ] T015 Implement the `FollowedSource` skeleton: open a regular file and read appended bytes from `offset`, with EOF-start default (`offset = last_size` at open); plus the FIFO/pipe streaming path (read-to-EOF then wait, bounded buffer, no identity tracking) in `crates/fsmon/src/follow/source.rs` (depends on T004, T014)
- [ ] T016 Implement `Follower` and the per-source blocking surface (`into_reader` → `impl Read + Send`; `next_chunk`) wiring the watcher to the source state machine in `crates/fsmon/src/follow/mod.rs` (depends on T015)

**Checkpoint**: Crate compiles; a local file can be opened and skeleton-followed. User stories can begin.

---

## Phase 3: User Story 1 - Live Tailing on a Local Disk (Priority: P1) 🎯 MVP

**Goal**: Follow a local file via native notifications; appends appear promptly and in order, idle is cheap, and the tail-preload handoff has no gap/duplication.

**Independent Test**: Append to a local file from another process and observe ordered delivery via `into_reader`; idle adds no busy-wait; pre-open content is not re-emitted.

### Tests for User Story 1

- [ ] T017 [P] [US1] Integration test: appends to a local file are delivered in order via `into_reader` (SC-001) in `crates/fsmon/tests/follow_regular.rs`
- [ ] T018 [P] [US1] Integration test: idle follow does not busy-wait — no output without writes, prompt delivery on the next append (FR-015, SC-004) in `crates/fsmon/tests/follow_regular.rs`
- [ ] T019 [P] [US1] Integration test: EOF-start handoff — content present before follow start is not re-emitted; only post-open appends are delivered (FR-008 handoff) in `crates/fsmon/tests/follow_regular.rs`

### Implementation for User Story 1

- [ ] T020 [US1] Wire the `KnownLocal → Native` engine path end-to-end so a local follow uses native notifications in `crates/fsmon/src/watch/engine.rs`
- [ ] T021 [US1] Deliver appended bytes at consumer pace (reconcile-on-wake append case; reads on the consumer thread) in `crates/fsmon/src/follow/source.rs`
- [ ] T022 [US1] Ensure idle efficiency: block on event/tick with no polling spin for native paths in `crates/fsmon/src/watch/mod.rs`

**Checkpoint**: MVP — `fsmon::follow(local_path)?.into_reader()` shows live appends.

---

## Phase 4: User Story 2 - Survive Log Rotation, Truncation, and Deletion (Priority: P1)

**Goal**: Keep following by name across rotation, truncation, deletion/recreation, and same-size replacement, losing no committed bytes.

**Independent Test**: Drive append→rotate→append, append→truncate→append, delete→recreate→append, and same-size replace; assert zero loss/duplication and continued following.

### Tests for User Story 2

- [ ] T023 [P] [US2] Integration test: rotation (rename away + recreate at path) loses no trailing bytes of the old file and follows the new file (FR-005, SC-002) in `crates/fsmon/tests/follow_regular.rs`
- [ ] T024 [P] [US2] Integration test: truncation (size shrinks) re-reads from start (FR-004) in `crates/fsmon/tests/follow_regular.rs`
- [ ] T025 [P] [US2] Integration test: delete→recreate resumes, and start-before-exists begins following on appearance (FR-002, FR-007) in `crates/fsmon/tests/follow_regular.rs`
- [ ] T026 [P] [US2] Integration test: same-size replacement detected within `recheck_cadence` (FR-006, SC-005) in `crates/fsmon/tests/follow_regular.rs`

### Implementation for User Story 2

- [ ] T027 [US2] Implement reconcile-from-`stat` transitions (append / truncate seek-0 / identity-change) in `FollowedSource` in `crates/fsmon/src/follow/source.rs`
- [ ] T028 [US2] Implement the `Draining` state — drain the old fd to EOF before closing/reopening by name (race-free handoff, research §3) in `crates/fsmon/src/follow/source.rs`
- [ ] T029 [US2] Implement the `Waiting`/retry path (deletion, start-before-exists, reappearance) in `crates/fsmon/src/follow/source.rs`
- [ ] T030 [US2] Drive the same-size re-check tick (`recheck_cadence`, default 5s) into reconcile in `crates/fsmon/src/watch/engine.rs`

**Checkpoint**: Following survives all rotation/truncation/deletion scenarios with no data loss.

---

## Phase 5: User Story 3 - Automatic Fallback to Polling on Unreliable Filesystems (Priority: P1)

**Goal**: Paths on network/unreliable/unknown filesystems are polled automatically (conservative default); behavior is identical to the native path.

**Independent Test**: Classify local vs remote vs unknown; on a polled path, appends appear within `poll_interval`; `engine()` reports `Polling`.

### Tests for User Story 3

- [ ] T031 [P] [US3] Unit tests: `classify` returns `KnownLocal` for known-local and `NotConfirmed` for remote/unknown/error (conservative) (FR-009–FR-011) in `crates/fsmon/tests/classification.rs`
- [ ] T032 [P] [US3] Integration test (environment-gated where a remote/unknown mount is unavailable): updates on a polled path appear within `poll_interval` and `engine()==Polling` for `NotConfirmed` (SC-003) in `crates/fsmon/tests/classification.rs`
- [ ] T033 [P] [US3] Integration test: behavior parity between native and polling backends for the same scenario (SC-010) in `crates/fsmon/tests/follow_regular.rs`

### Implementation for User Story 3

- [ ] T034 [US3] Wire `NotConfirmed → Polling` routing plus the conservative default into engine selection in `crates/fsmon/src/watch/engine.rs`
- [ ] T035 [US3] Gate the macOS kqueue backend to `KnownLocal`-only (route `NotConfirmed` to polling) in `crates/fsmon/src/watch/backend/kqueue_macos.rs` (FR-020)
- [ ] T036 [US3] Ensure the polling-driven reconcile produces identical `FollowedSource` behavior to native (shared reconcile path) in `crates/fsmon/src/follow/source.rs`

**Checkpoint**: Remote/unknown filesystems follow correctly via polling; local files unaffected.

---

## Phase 6: User Story 4 - Following Multiple Files with Mixed Storage (Priority: P2)

**Goal**: Follow several files at once with per-path engine selection; source-tagged output; local files keep native latency regardless of remote ones.

**Independent Test**: Follow one local + one remote file; assert independent routing and that local latency is unaffected by the remote cadence.

### Tests for User Story 4

- [ ] T037 [P] [US4] Integration test: multi-path `Follower` yields `Chunk`s correctly tagged by `SourceId` for each file (FR-025) in `crates/fsmon/tests/follow_regular.rs`
- [ ] T038 [P] [US4] Integration test: a local+remote mix is routed independently and local native latency is unaffected (SC-006) in `crates/fsmon/tests/classification.rs`

### Implementation for User Story 4

- [ ] T039 [US4] Implement multi-path `Follower::new` + `SourceId` tagging + `next_chunk` in `crates/fsmon/src/follow/mod.rs`
- [ ] T040 [US4] Ensure per-path independent engine selection across the followed set in `crates/fsmon/src/watch/engine.rs`

**Checkpoint**: Mixed-storage multi-file following works with independent per-path engines.

---

## Phase 7: User Story 5 - Graceful Runtime Degradation (Priority: P2)

**Goal**: When a native watch is lost mid-session, migrate that path to polling in place without restart or lost entries.

**Independent Test**: Force a native watch loss; assert following continues via polling with no lost subsequent entries.

### Tests for User Story 5

- [ ] T041 [P] [US5] Integration test: simulated native watch error/loss → following continues via polling with zero lost subsequent entries (FR-014, SC-007) in `crates/fsmon/tests/follow_regular.rs`

### Implementation for User Story 5

- [ ] T042 [US5] Detect native watch error/loss and migrate that single path `Native → Polling` in place (no restart) in `crates/fsmon/src/watch/engine.rs`
- [ ] T043 [US5] Emit a degradation diagnostic via `log` and guarantee no lost bytes across the migration in `crates/fsmon/src/watch/mod.rs`

**Checkpoint**: Runtime watch loss is survived transparently.

---

## Phase 8: User Story 6 - Reusable Component & hl Adoption (Priority: P2)

**Goal**: Finalize the simple default-driven API and observability; hl consumes the facade and drops its inlined `fsmon` logic.

**Independent Test**: A minimal default-only consumer follows correctly through append+rotation; hl `-F` works through the new crate.

### Tests for User Story 6

- [ ] T044 [P] [US6] Test: a minimal consumer using only `FollowOptions::default()` follows append+rotation correctly (FR-022, SC-008) in `crates/fsmon/tests/follow_regular.rs`
- [ ] T045 [P] [US6] hl follow integration test exercising the new crate path (append + rotation visible through `hl -F`) in an in-crate `#[cfg(test)]` module under `src/app.rs`

### Implementation for User Story 6

- [ ] T046 [US6] Finalize the public facade API (`follow()` and `Follower::new` with `AsRef<Path>` bounds, `into_reader`) and crate docs in `crates/fsmon/src/follow/mod.rs` and `crates/fsmon/src/lib.rs`
- [ ] T047 [US6] Wire observability: emit engine selection, conservative fallback, and degradation via the `log` crate (FR-016a) in `crates/fsmon/src/watch/engine.rs`
- [ ] T048 [US6] hl: add the `fsmon` path dependency in root `Cargo.toml`, remove `src/fsmon.rs`, and drop `mod fsmon;` from `src/lib.rs` (FR-024)
- [ ] T049 [US6] hl: rewrite `app.rs::follow()` to run the existing `Scanner` over the facade `into_reader`, removing the manual reopen/offset block (app.rs:636-662) while keeping the `Input::tail` preload (app.rs:618) and starting the facade at EOF, in `src/app.rs`
- [ ] T050 [US6] hl: map `fsmon::Error` into hl's `Error` in `src/error.rs` and route fsmon `log` diagnostics into the `HL_DEBUG_LOG` channel

**Checkpoint**: hl follows via the reusable crate; old module removed; other consumers can adopt it with defaults.

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Stress, edge-case, coverage, and documentation work spanning all stories.

- [ ] T051 [P] Backpressure stress test: a fast writer outpacing the consumer keeps memory bounded with zero loss (FR-015a, SC-011) in `crates/fsmon/tests/backpressure.rs`
- [ ] T052 [P] FIFO/pipe streaming test and directory/special-file rejection test (FR-026) in `crates/fsmon/tests/follow_pipe.rs`
- [ ] T053 Run `just uncovered` and add tests for changed lines lacking coverage; document any environment-gated exclusions (SC-009, constitution V)
- [ ] T054 [P] Update `CHANGELOG.md` and crate README/docs describing the new robust-follow behavior (no source-internals per commit guidelines)
- [ ] T055 Validate `quickstart.md` snippets against the built crate and against `hl -F` on a local file
- [ ] T056 Cross-platform verification: `cargo test` on Linux, macOS, and Windows (note the dev Windows box's missing CI tooling is tracked separately; `cargo test` is the gate for this feature)

---

## Dependencies & Execution Order

### Phase dependencies

- **Setup (P1)** → **Foundational (P2)** → **User Stories (P3–P8)** → **Polish (P9)**.
- Foundational BLOCKS all user stories.

### User story dependencies (be honest about coupling)

- **US1 (P1)** depends only on Foundational. It is the MVP.
- **US2 (P1)** depends on Foundational and is most natural after US1 — both extend `follow/source.rs`, so they serialize at the file level even though each is an independent test increment.
- **US3 (P1)** depends on Foundational; extends `watch/engine.rs` and shares the reconcile path — serialize after US2 for clean merges. Conceptually independent (its tests pass on their own).
- **US4 (P2)** depends on Foundational; builds on the multi-path seams in `follow/mod.rs` / `watch/engine.rs`.
- **US5 (P2)** depends on Foundational; extends `watch/engine.rs` (degradation).
- **US6 (P2)** depends on US1–US3 being functional (hl consumption needs append+rotation+fallback working); finalizes API + integrates hl.

### Within each story

- Write the story's tests first (they should fail), then implement.
- For shared-file tasks (`follow/source.rs`, `watch/engine.rs`) within a story, execute sequentially (no `[P]`).

### Parallel opportunities

- Setup: T003 is independent of T001/T002 once the crate exists.
- Foundational: T004–T009 and T011–T012 are `[P]` (distinct files: identity/*, classify/*, backend/*). T010, T013–T016 serialize on shared files.
- Per story, all `[P]` test tasks can be written together; implementation tasks on the same file serialize.
- Polish: T051, T052, T054 are `[P]` (distinct files).

---

## Parallel Example: Foundational platform primitives

```bash
# Distinct files, no inter-dependencies — can be implemented in parallel:
Task: "T004 Unix FileId in crates/fsmon/src/identity/unix.rs"
Task: "T005 Windows FileId in crates/fsmon/src/identity/windows.rs"
Task: "T007 Linux classification in crates/fsmon/src/classify/linux.rs"
Task: "T008 macOS classification in crates/fsmon/src/classify/macos.rs"
Task: "T009 Windows classification in crates/fsmon/src/classify/windows.rs"
```

## Parallel Example: User Story 2 tests

```bash
# All US2 test files/cases authored together, then implement to make them pass:
Task: "T023 rotation no-loss test"
Task: "T024 truncation re-read test"
Task: "T025 delete→recreate + start-before-exists test"
Task: "T026 same-size replacement test"
```

---

## Implementation Strategy

### MVP first (User Story 1)

1. Phase 1 Setup → Phase 2 Foundational (the bulk of the shared crate).
2. Phase 3 US1 → **STOP and VALIDATE**: live local tailing through `into_reader`.

### Incremental delivery (priority order)

1. Foundation → US1 (MVP: local tailing).
2. US2 (rotation/truncation durability) — the headline `tail -F` value.
3. US3 (polling fallback) — the network-share correctness the feature was created for.
4. US4 (multi-file mixed) → US5 (degradation) → US6 (hl adoption + reusable API).
5. Polish: backpressure/pipe/coverage/docs/cross-platform.

### Notes

- `[P]` = different files, no dependencies.
- Constitution Principle V: write tests before implementation for each story.
- Commit after each task or logical group (`docs`/`feat`/`fix` per the constitution's commit guidelines); do not push from this environment.
- Implementation tasks are to be executed on the Sonnet model.
