# Phase 0 Research: Robust File Following

This document records the design decisions that resolve the open questions from the plan's Technical Context and the user-provided architecture direction. Format per decision: **Decision / Rationale / Alternatives considered**.

## 1. Crate name and layering

**Decision**: One workspace crate (tentative name `fsmon`) under `crates/`, with two public layers: a core event layer (module `watch`, type `Watcher`, enum `Event`) and a facade layer (module `follow`, type `Follower` yielding per-source byte streams). The crate name is the single intentional bikeshed — easily changed at implementation start (alternatives: `ftail`, `tailfollow`, `filefollow`).

**Rationale**: The user requires all watcher/classification/platform logic in one reusable crate with a simple default-driven interface and an advanced surface for fine-tuning. Two layers satisfy both: the facade is the simple path; the core is the advanced/extension path (and what deferred `tail`-style messages, OOS-001, will consume). `fsmon` preserves continuity with the existing internal `mod fsmon` that hl developers already know. Mirrors the established `crates/pager` extraction precedent.

**Alternatives considered**: (a) Two separate crates (core + facade) — rejected; the user asked for *a* crate, and the layers share types tightly. (b) Keep logic inlined in hl — rejected; violates the explicit reusability requirement and constitution principle II.

## 2. Reconcile-from-`stat` as the correctness model

**Decision**: The facade treats core events as *advisory wake signals* and always re-derives ground truth by `stat`-ing the path (and `fstat`-ing the open fd) on every wake. Transitions (append / truncate / rotate / remove / reappear) are decided from the authoritative filesystem state, not solely from event kinds.

**Rationale**: Native event streams are lossy and platform-divergent; relying on exact event semantics is fragile. Re-deriving truth from `stat` makes behavior identical across native and polling backends (SC-010), and guarantees convergence after dropped/overflowed events (FR-016) because the periodic tick alone is sufficient to reconcile. This is precisely the GNU `tail` approach.

**Alternatives considered**: Trusting event kinds directly (e.g., act only on a `Rotated` event) — rejected; misses silent same-size replacement and desynchronizes on dropped events.

## 3. Race-free rotation handoff (drain-before-switch)

**Decision**: On detecting that the path's identity differs from the open fd's identity, the facade first reads the **old** fd to EOF (draining any trailing bytes written before/at rotation), and only then closes it, opens the new file by name at offset 0, and reads it. Truncation (size below offset on the same identity) seeks to 0 and re-reads.

**Rationale**: Resolves the explicit race-free-handoff design question. Bytes committed to the rotated-away file between the previous read and the switch are not lost (FR-005, no-data-loss). Matches `tail`'s "finish the old file, then follow the new name" behavior.

**Alternatives considered**: Switching to the new file immediately on identity change — rejected; drops un-read trailing bytes of the old file.

## 4. Backpressure model

**Decision**: Reading is performed on the **consumer's** thread inside the facade iterator's `next()`; the background watcher thread only delivers coalesced wake signals. For regular files, un-consumed data simply stays on disk (offset advances only as the consumer reads) — memory is O(read buffer), independent of backlog. For FIFOs/pipes (no disk backing), a bounded internal buffer is used and the OS pipe buffer applies backpressure to the writer. Core wake events are coalesced to a per-path "dirty" bit in a bounded channel, so a flood of events collapses to at most one pending reconcile.

**Rationale**: Resolves the backpressure design question and satisfies FR-015a / SC-011 (bounded memory, no loss). A slow consumer cannot cause unbounded buffering or dropped events because (a) file bytes live on disk and (b) events degrade to a single dirty bit.

**Alternatives considered**: Background thread reads ahead into an in-memory queue — rejected; unbounded memory under a fast writer, contradicts FR-015a.

## 5. Engine selection and runtime degradation

**Decision**: Classify each path independently at follow-start; `KnownLocal` → native backend, everything else → `PollWatcher`. On a native watch error/loss at runtime, migrate that single path to polling in place without restarting, and emit a diagnostic. macOS kqueue is used only for `KnownLocal` paths (same gate as other platforms).

**Rationale**: Per-path routing (FR-012) is strictly better than `tail`'s global all-or-nothing and is natural given hl's per-input threads. Runtime degradation (FR-014) covers watches lost mid-session (unmount/remount, OS limits). Gating kqueue (FR-020) closes the macOS bypass.

**Alternatives considered**: Global engine choice (like coreutils) — rejected; needlessly penalizes local files when one input is remote (US4).

## 6. Conservative classification data source

**Decision**: Linux uses `statfs` `f_type` matched against the known-local magic-number table that coreutils/gnulib maintain (ext*, btrfs, xfs, zfs, tmpfs, etc.); macOS uses `statfs` `f_fstypename` against a small known-local allowlist (`apfs`, `hfs`); Windows uses `GetDriveTypeW` (`DRIVE_REMOTE` → not confirmed) plus UNC (`\\server\share`) detection. Any `statfs` error or unrecognized type → `NotConfirmed` → poll.

**Rationale**: Inherits coreutils' decades of real-world vetting rather than maintaining an ad-hoc list (spec Assumptions). Conservative default (FR-011) ensures correctness on exotic/unknown mounts at the cost of occasionally polling a local FS we failed to recognize — a safe trade (Risk table).

**Alternatives considered**: Behavioral probe (write a sentinel, wait for an event) — rejected; adds startup latency and is racy under load.

## 7. Cross-platform file identity

**Decision**: `FileId` = `(st_dev, st_ino)` on Unix via `libc`; on Windows, `FILE_ID_INFO` (volume serial number + 128-bit file id) via `GetFileInformationByHandle`/`GetFileInformationByHandleEx`, falling back to `dwVolumeSerialNumber` + `nFileIndexHigh/Low` where the newer API is unavailable.

**Rationale**: Gives Windows the rotation/replacement detection it currently lacks (today's check is `#[cfg(unix)]` only). `FILE_ID_INFO` is the modern, ReFS-safe analog of the inode pair.

**Alternatives considered**: Path/name-only tracking on Windows — rejected; cannot distinguish same-name replacement.

## 8. Dependency strategy

**Decision**: Reuse `notify` v8 (native + `PollWatcher`) already in the workspace; keep `kqueue` v1 for macOS; use `libc` on unix and `windows-sys` (already transitive; add as a direct dep of the crate with the needed Win32 feature groups) on windows. No new third-party crates enter the workspace.

**Rationale**: Minimizes dependency surface and audit burden; `notify`'s `PollWatcher` already exists and is the natural polling backend, eliminating a hand-rolled poller.

**Alternatives considered**: Hand-rolled inotify/ReadDirectoryChangesW bindings — rejected; `notify` already abstracts these and is battle-tested. A separate polling crate — rejected; `notify::PollWatcher` suffices.

## 9. Multi-path & threading model

**Decision**: The facade supports both a single-path `Follower` (one source) and a multi-path `Follower` (one request, source-tagged output) per FR-025. hl initially uses one single-path `Follower` per existing reader thread, preserving its `thread::scope` reader/worker/merger structure with minimal churn. Internally, a single background watcher thread services wake signals; reconciliation + reading run on the consumer thread.

**Rationale**: Preserves hl's proven concurrency structure and timestamp-merge while still offering the multi-path API other consumers want. Keeps reads on the consumer thread for backpressure (decision 4).

**Alternatives considered**: Force hl onto a single multi-path follower with internal fan-out — rejected; larger, riskier change to app.rs for no immediate benefit; multi-path remains available for new consumers.

## 10. Open items intentionally deferred

- Exact crate name (decision 1) — confirm at implementation start.
- User-visible `tail`-style messages and a `--poll`/`--no-inotify` CLI flag — out of scope (OOS-001/OOS-002); the core event API and the `FallbackPolicy` option are the seams that make them cheap to add later.
