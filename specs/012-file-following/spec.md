# Feature Specification: Robust File Following (`tail -F` Parity)

**Feature Branch**: `012-file-following`
**Created**: 2026-06-08
**Updated**: 2026-11-25 (issue #1666: partial trailing entries and merge-window limits)
**Status**: Draft for robust follow-by-name. The existing CLI behavior is documented below.
**Input**: User description: "Replicate GNU `tail -F` follow behavior in hl: use OS-native filesystem notifications when they are reliable, and automatically fall back to polling when notifications are unreliable (network drives, FUSE mounts, or any filesystem whose reliability cannot be confirmed). All watcher/engine-selection/classification/platform logic must live in a new reusable standalone workspace crate with a simple, default-driven interface."
**Amendment input**: "Document when a partial trailing entry is shown before its delimiter arrives, including overlong entries, file replacement and pipe closure."
**Amendment input**: "Verify the current CLI behavior against the implementation and define the requirements for robust file following."

## Clarifications

### Session 2026-11-25

- Q: When can hl show an entry before its input delimiter arrives? → A: See [Partial trailing entry](#partial-trailing-entry) for the scanner and merge window rules, including file replacement and pipe EOF.
- Q: Which behavior is in scope for the draft? → A: The scenarios and requirements describe the proposed robust follower. The existing CLI rules under [Current behavior on develop](#current-behavior-on-develop) also apply to the proposed integration with hl, except where a requirement specifies a change.

### Session 2026-06-24

- Q: How is the memory-bounded guarantee preserved for content that never reaches a delimiter (an effectively unbounded single entry)? → A: Bounded by the consumer's existing maximum-segment-size cap (in hl, `--max-message-size`, default 64 MiB), enforced by the consumer's scanner over the facade's byte stream in all modes (including follow); the component itself does no unbounded in-memory accumulation of a single entry.

### Session 2026-06-08

- Q: What happens when a non-regular input (FIFO/pipe, directory, socket) is given in follow mode? → A: FIFOs and anonymous pipes are read as streams. A named FIFO can receive new writers after EOF. EOF ends an anonymous pipe. The component reports a clear error for directories and unsupported special files such as sockets and devices.
- Q: How must the component behave when a writer appends faster than the consumer drains? → A: Memory-bounded with backpressure and no committed-data loss. For regular files the data waits on disk and is read at the consumer's pace (no unbounded internal buffering); for pipe-like inputs a bounded buffer applies, with the OS pipe buffer providing natural backpressure to the writer.
- Q: Must engine selection and runtime fallback/degradation be observable, given the "no silent failures" principle? → A: Yes — observable via debug logging (routed through the consumer's debug-log channel, e.g., `HL_DEBUG_LOG` in hl); normal stdout/stderr stays clean. User-visible status messages remain deferred (OOS-001).
- Q: What is the default cadence of the same-size replacement re-check safety net? → A: 5 seconds by default (matching coreutils' effective default of 5 × the 1 s poll interval), configurable via options.

## Current behavior on develop

The `--follow` (`-F`) command preloads the last `--tail` entries (10 by default) from each seekable input. For a regular file, it continues reading appends from the same open stream. It resolves symlinks at startup and follows the resolved path. It requires the path to be accessible at startup. After a filesystem event, the application checks for truncation or a new file identity and starts reading a reopened file at the beginning. After a removal event, it can reopen the path when a subsequent event indicates that it has reappeared. When the CLI detects a replacement, it closes the old stream and submits any fragment already held by the scanner. Bytes still unread in the old file may be skipped.

Linux uses native notifications and macOS uses native vnode events. On Windows, native directory notifications report rotation, creation and deletion, and adaptive file-handle polling reports changes in file size or identity after it establishes an initial sample. This lets it detect appends from writers that keep a file open. The CLI currently chooses these mechanisms by platform. In particular, Windows polling supplements native notifications without classifying the filesystem. Per-path reliability classification, configurable periodic reconciliation, and automatic migration after loss of a native watch are proposed below.

Hl reads standard input and named FIFOs as streams, ending each input at EOF. The CLI rejects a directory when opening it. The proposed component also specifies how to handle other special files and how to continue waiting after a FIFO writer disconnects.

### Partial trailing entry

- **Regular file awaiting more data.** At EOF, hl holds back an unfinished entry that has stayed below `--max-message-size` (64 MiB by default). The next scan continues from that fragment when more bytes arrive.
- **Regular file with an overlong entry.** A fragment that reaches the cap can be shown before the input delimiter arrives. Further parts normally continue the same output entry. Hl treats the parts as unparsed text and shows them when no record filter or explicit input format suppresses unparsed text.
- **An open output entry with a missing continuation.** When the next part is available, it is written immediately after the preceding part. Otherwise, hl ends the entry with the output delimiter when any of these conditions applies.
  - The synchronization interval (`--sync-interval-ms`, 100 ms by default) has elapsed since the last part was taken for output and another queued entry is due.
  - The total data held by the merge window reaches twice its memory limit. The limit is the processing thread count times the effective buffer size, which is the smaller of `--buffer-size` and `--max-message-size`.
  - The next block of that input has been processed and contains no continuation, or the merge receives no further data because all inputs have closed.
- **A continuation after an early end.** When the output ended while still waiting for a continuation, a subsequent nonempty continuation is shown as a new output entry with the usual input prefix, if enabled. If no other entry is due and no other condition for ending the entry applies, the output entry stays open as it waits for its continuation.
- **Detected replacement, truncation or deletion of a regular file.** Hl submits any held fragment for output as the end of the old stream before reading the next stream. The configured output delimiter ends the old entry.
- **Pipe or FIFO EOF.** Hl scans and submits remaining text for output when the stream reaches EOF, even if it has no input delimiter. The configured output delimiter ends the entry.

## Proposed behavior

The following scenarios, requirements and success criteria define the proposed robust follower. The CLI behavior described above continues to apply unless a requirement specifies a change.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Live Tailing on a Local Disk (Priority: P1)

A user runs the log viewer in follow mode against a file on a local disk that another process is actively appending to. New entries appear in the viewer promptly and in order, without the user re-running the command, and without measurable CPU cost while the file is idle.

**Why this priority**: This is the everyday case and the core value of follow mode. If live tailing of a local file is not reliable and efficient, the feature has no value.

**Independent Test**: Start follow mode on a local file and append lines from a separate process. Verify that the follower delivers each appended line within the latency target, that the viewer displays it after its usual ordering delay, and that idle CPU stays negligible.

**Acceptance Scenarios**:

1. **Given** follow mode is active on a local file, **When** a writer appends a complete entry, **Then** the follower delivers the entry to the viewer within the responsiveness target (SC-001).
2. **Given** follow mode is active and no writes occur, **When** the file remains idle, **Then** the viewer consumes negligible CPU (no busy-waiting).
3. **Given** follow mode starts, **When** it begins, **Then** the last N preloaded entries are shown first and subsequent appends continue seamlessly from that point with no duplicated or skipped entries.

---

### User Story 2 - Survive Log Rotation, Truncation, and Deletion (Priority: P1)

A user follows an application log that is periodically rotated, truncated, or briefly removed and recreated by a logging framework or `logrotate`. The viewer keeps following the live file by name across these events without a restart and preserves entries that remain available to read (SC-002).

**Why this priority**: Real-world logs are rotated. This is precisely the behavior that distinguishes `tail -F` from `tail -f` and is the durability guarantee users depend on. Without it, follow mode silently stops showing new data after the first rotation.

**Independent Test**: Drive a file through append → rotate (rename + recreate) → append, append → truncate → append, and delete → recreate → append, keeping the entries readable until the follower can drain them and verifying that each appears once.

**Acceptance Scenarios**:

1. **Given** follow mode is active, **When** the file is rotated (renamed away and a new file created at the same path), **Then** the viewer begins following the new file from its start and continues showing newly appended entries.
2. **Given** follow mode is active, **When** the file is truncated (its size shrinks), **Then** the viewer re-reads the file from the beginning and continues following without error.
3. **Given** follow mode is active, **When** the file is deleted, **Then** the viewer keeps retrying the path and resumes following automatically once a file reappears at that path.
4. **Given** follow mode is active, **When** the original file is replaced by a different file of the same size, **Then** the viewer detects the replacement within the safety-net interval (see SC-005) and follows the new file.
5. **Given** follow mode is started against a path that does not yet exist, **When** a file later appears at that path, **Then** the viewer begins following it.

---

### User Story 3 - Automatic Fallback to Polling on Unreliable Filesystems (Priority: P1)

A user follows a log on a network share (NFS/SMB/CIFS), a FUSE-mounted filesystem, or any filesystem the application cannot positively confirm as local. The viewer still shows new entries, because it automatically uses polling for that path instead of relying on OS notifications that would silently miss remote writes. The user does not have to know or configure anything.

**Why this priority**: This is the central problem the feature exists to solve. OS notification mechanisms silently fail to report writes made by other hosts on network filesystems; without automatic fallback, follow mode appears to "work" but silently stops updating — the worst kind of failure.

**Independent Test**: Follow a file on a known-remote or unrecognized filesystem and append to it from another host or process. Verify that the follower detects the append within one polling interval after the data is readable to it, even though native notifications deliver nothing.

**Acceptance Scenarios**:

1. **Given** a followed path resides on an affirmatively-known-local filesystem, **When** following begins, **Then** native OS notifications are used for that path.
2. **Given** a followed path resides on a known-remote filesystem (e.g., NFS, SMB/CIFS, AFP), **When** following begins, **Then** polling is used for that path.
3. **Given** a followed path resides on a filesystem whose type cannot be determined, or the filesystem query fails, **When** following begins, **Then** polling is used for that path (conservative default).
4. **Given** polling is in effect for a path, **When** an update from any writer becomes readable to the follower, **Then** it detects the update within one polling interval (SC-003).

---

### User Story 4 - Following Multiple Files with Mixed Storage (Priority: P2)

A user follows several files at once where some live on local disk and others on a network share. Each file uses the most appropriate mechanism independently — native notifications for the local files, polling for the remote ones — with no degradation of the local files' responsiveness.

**Why this priority**: Following multiple inputs is a supported mode, and forcing all files onto the slowest common mechanism (as some tools do) needlessly penalizes local files. Per-path selection is a concrete improvement over the global all-or-nothing choice.

**Independent Test**: Follow one local and one remote file simultaneously; verify the local file updates at native latency while the remote file updates at polling cadence, each correct.

**Acceptance Scenarios**:

1. **Given** a mix of local and remote followed paths, **When** following begins, **Then** each path is routed to native or polling independently based on its own filesystem.
2. **Given** a mix of local and remote followed paths, **When** the local file is appended to, **Then** its update is delivered at native latency regardless of the remote file's slower cadence.

---

### User Story 5 - Graceful Runtime Degradation (Priority: P2)

While following a local file with native notifications, the underlying watch is lost at runtime (for example the filesystem is unmounted and remounted, or the OS drops the watch). The viewer transparently switches that path to polling and keeps following, without the user restarting it and without losing subsequent entries.

**Why this priority**: Native watches can be lost mid-session for reasons outside the application's control. Silent permanent failure after such an event would reintroduce exactly the "looks like it works but stopped updating" problem in a different form.

**Independent Test**: Force a native watch loss on a followed path during a session and verify following continues (via polling) and subsequent appends still appear.

**Acceptance Scenarios**:

1. **Given** a path is being followed via native notifications, **When** the native watch is reported lost or errors out, **Then** the path migrates to polling without restarting the session.
2. **Given** a path has migrated to polling after a watch loss, **When** the file is subsequently appended to, **Then** the follower detects the new data within one polling interval after it becomes readable (SC-003).

---

### User Story 6 - Reusable Component for Other Consumers (Priority: P2)

A developer (the application itself, or any other program in the workspace) needs the same robust follow behavior. They depend on a single self-contained component, ask it to follow one or more paths, and receive newly appended content per source — without having to understand or reimplement engine selection, filesystem classification, or rotation/truncation handling. When they need to, they can adjust behavior through options that all have sensible defaults.

**Why this priority**: The packaging requirement is explicit: the complexity must be encapsulated and reusable, not entangled in the application binary. This keeps the durable, platform-specific logic in one tested place and makes correct following available to any consumer.

**Independent Test**: From a minimal consumer using only default settings, follow a file through append/rotation/truncation and confirm correct delivered content without the consumer implementing any reopen, classification, or platform-specific logic.

**Acceptance Scenarios**:

1. **Given** a consumer using only default settings, **When** it follows one or more paths, **Then** it receives newly appended content per source with rotation, truncation, deletion/recreation, and engine selection handled internally.
2. **Given** a consumer with specific needs, **When** it overrides options (e.g., polling interval, fallback policy, whether to retry missing files, preloaded-tail amount), **Then** the component honors those overrides while leaving unspecified options at their defaults.
3. **Given** the application binary, **When** it adopts the component, **Then** its previous inlined file-monitoring logic is removed in favor of the component.

---

### Edge Cases

- **File replaced by a same-size file**: A pure size comparison misses this. The component periodically checks the identity of the file at the path on the configurable safety-net cadence (FR-006).
- **Rotation on Windows**: Windows has no inode/device pair. Rotation/replacement MUST still be detected, using the platform's file-identity information (volume + file id) as the equivalent of the Unix device/inode pair.
- **Writes on Windows from always-open writers**: Windows NTFS defers directory index entry updates (size, mtime) until `CloseHandle` or `FlushFileBuffers`. `ReadDirectoryChangesW` monitors the directory index and therefore silently misses writes from log writers that keep the file open without flushing (e.g., lumberjack-style appenders). The component MUST detect such writes without requiring the writer to flush, by using `GetFileInformationByHandle` on a freshly-opened handle — which reads the live MFT record, bypassing the directory cache — rather than relying solely on directory-change notifications.
- **Symlinked target**: When a followed path is a symlink, the component follows the underlying target and re-resolves on replacement.
- **Path is a FIFO/pipe**: The component streams both input types. A named FIFO can accept another writer after EOF. An anonymous pipe ends at EOF. File rotation, truncation and identity tracking apply only to regular files.
- **Path is a directory or other special file** (socket, device): The component rejects it with a clear, actionable error rather than attempting to tail it.
- **Rapid successive events / event queue overflow**: A burst of changes or a dropped/overflowed native event MUST NOT permanently desynchronize following; the component re-checks state so it converges to the current file contents.
- **Writes during a reopen**. The component reads from the correct offset so it does not skip bytes still readable in the old or new file. Bytes destroyed before the follower can read them, including in files created and removed entirely between checks, are outside this guarantee.
- **Filesystem query failure**: If the filesystem type cannot be determined, the default policy treats the path as unreliable and uses polling.
- **Partial trailing entry**. The hl integration preserves the [current partial-entry behavior](#partial-trailing-entry), including at stream boundaries.

## Requirements *(mandatory)*

### Functional Requirements

#### Follow Semantics (match `tail -F` = follow-by-name + retry)

- **FR-001**: The component MUST follow each path *by name*: after the file at a path is replaced, it MUST follow whatever file currently occupies that path, not the previously-open file.
- **FR-002**: The component MUST retry paths that are currently inaccessible (missing or unreadable), resuming automatically when a file becomes available at the path, including paths that do not exist when following begins.
- **FR-003**: When a followed file grows, the component MUST deliver the newly appended content in order.
- **FR-004**: When the component observes that a followed file has shrunk, it MUST resume delivery from the beginning of the current file.
- **FR-005**: When a followed file is rotated or replaced (its filesystem identity changes), the component MUST begin following the new file at the path from its start, after delivering unread content from the prior file that is still accessible to the follower.
- **FR-006**: The component MUST detect same-size replacement that a size comparison alone would miss, via a periodic re-check of the path on a configurable cadence (enabled by default, defaulting to 5 seconds — matching coreutils' effective default of 5 × the 1 s poll interval).
- **FR-007**: When a followed file is deleted, the component MUST keep the path under observation and resume following when a file reappears there.
- **FR-008**: On startup of following a path, the component MUST be able to preload the last N entries (configurable, with a sensible default) before delivering subsequent appends, with no duplicated or skipped entries across the handoff.

#### Filesystem Reliability Classification (conservative)

- **FR-009**: The component MUST classify each followed path's filesystem as either *affirmatively known-local* or *not confirmed reliable*.
- **FR-010**: By default, the component MUST use native OS notifications only for paths on an affirmatively-known-local filesystem.
- **FR-011**: By default, the component MUST use polling for any path that is known-remote (including NFS, SMB/CIFS, AFP), on a FUSE mount, on a Windows network drive, on a UNC path, on an unrecognized filesystem, or for which the filesystem query fails.
- **FR-012**: Classification and routing MUST be performed *per path*: when following multiple paths, some MAY use native notifications while others use polling, determined independently for each path.

#### Engine Selection and Runtime Behavior

- **FR-013**: The component MUST deliver equivalent follow semantics (FR-001–FR-008) regardless of whether a path is served by native notifications or by polling — the mechanism MUST be transparent to the consumer.
- **FR-014**: When a native watch for a path is lost or errors at runtime, the component MUST migrate that path to polling without restarting following and without losing subsequent content.
- **FR-015**: The component MUST NOT busy-wait; while followed files are idle, resource usage MUST remain negligible.
- **FR-015b**: The polling backend (used for network shares and other unreliable sources on all platforms) MUST use an IIR-based adaptive interval estimator per source. The estimator MUST tighten the polling interval toward a floor (10 ms) when a source is actively written, and MUST relax it toward a configurable ceiling (default 1 s) during idle periods. The same estimator algorithm MUST be used by the Windows hybrid polling thread, ensuring consistent adaptive behavior across all polling paths.
- **FR-015a**: When a writer appends faster than the consumer drains, the component MUST remain memory-bounded and MUST NOT drop data to relieve backpressure. For regular files this means reading at the consumer's pace with unread data left on disk (no unbounded internal buffering); for pipe-like inputs a bounded buffer applies and backpressure propagates to the writer via the OS pipe buffer. Content that never reaches a delimiter (an effectively unbounded single entry) is bounded by the consumer's maximum-segment-size cap applied during scanning (in hl, `--max-message-size`, default 64 MiB), so the memory-bounded guarantee holds in all modes.
- **FR-016**: The component MUST converge to current file contents after a dropped or overflowed native event burst (no permanent desynchronization).
- **FR-016a**: The component MUST make its per-path engine selection (native vs polling), its conservative fallback decisions, and any runtime degradation observable to the consumer through a diagnostic/logging interface, so that "no silent failures" holds without these decisions appearing in normal output. The consumer routes this to its own debug-log channel (in hl, the existing `HL_DEBUG_LOG` mechanism).

#### Platform Support

- **FR-017**: The component MUST support Linux, macOS, and Windows for both native notification and polling modes.
- **FR-018**: On Linux and macOS, classification MUST use the operating system's filesystem-type information.
- **FR-019**: On Windows, classification MUST treat network drives and UNC paths as not-confirmed-reliable, and rotation/replacement detection MUST use the platform's file-identity information as the device/inode equivalent.
- **FR-019a**: On Windows, the native backend for locally-classified paths MUST combine `ReadDirectoryChangesW` (for immediate rotation/deletion/creation events) with an adaptive `GetFileInformationByHandle` polling thread (for write detection on files held open by writers). The two threads share a single wake-signal channel; correctness is not compromised if one thread is slower than the other.
- **FR-020**: The macOS native path MUST be subject to the same reliability gate as other platforms (it MUST NOT bypass classification).

#### Reusability and Packaging

- **FR-021**: All watcher abstraction, native-vs-polling engine selection, filesystem-reliability classification, and platform-specific code MUST reside in a single, self-contained, reusable workspace component, separate from the application binary.
- **FR-022**: The component MUST expose a simple default-driven interface such that a consumer can follow one or more paths and receive newly appended content per source without configuring or understanding engine selection, classification, or rotation/truncation handling.
- **FR-023**: The component MUST expose advanced options for the polling ceiling, fallback policy, retrying missing paths, same-size re-check cadence, and preloaded-tail amount. Each option MUST have a default. A consumer can explicitly relax the conservative fallback policy.
- **FR-024**: The application binary MUST consume this component and MUST remove its previous inlined file-monitoring logic in favor of it.
- **FR-025**: Following multiple paths through the component MUST be supported through a single follow request, with content attributable to its source path.

#### Input Types

- **FR-026**: The component MUST follow regular files with full rotation/truncation/identity tracking (FR-001–FR-008). It MUST stream FIFOs and pipes without rotation, truncation or identity tracking. It MUST continue waiting for data on named FIFOs after a writer disconnects and treat EOF on anonymous pipes as the end of that input. It MUST reject directories and other special files (sockets, devices) with a clear, actionable error and MUST NOT panic or silently misbehave on any input type.

### Out of Scope (Future Work)

- **OOS-001**: User-visible diagnostic messages mirroring `tail` (e.g., "file truncated", "has appeared", "has become inaccessible") are deferred.
- **OOS-002**: A user-facing override flag/config to force polling or force native notifications (analogous to `tail ---disable-inotify`) is deferred. (The internal fallback-policy option is intended for consumers of the component.)
- **OOS-003**: Follow-by-descriptor semantics (the `tail -f` variant that keeps following a renamed file) are not provided; only follow-by-name (`-F`) is in scope.
- **OOS-004**: Rotation/truncation/identity tracking for pipe-like inputs is out of scope — FIFOs and pipes are streamed according to FR-026. Directories and other special files (sockets, devices) are rejected as unsupported.

### Key Entities

- **Followed Source**: A path the consumer asked to follow, together with the component's tracked state for it (last-known size, filesystem identity, current engine, accessibility).
- **Filesystem Reliability**: A per-path classification — *known-local* (native notifications permitted) vs *not-confirmed-reliable* (polled under the default policy).
- **Follow Engine**: The mechanism currently serving a source — native OS notifications or polling — selected per path and able to change at runtime.
- **Follow Options**: The consumer-tunable settings (polling interval, fallback policy, retry behavior, same-size re-check cadence, preloaded-tail amount), each with a default.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With native notifications on a local filesystem, appended data reaches the consumer within 200 ms under normal conditions. On Windows, SC-003a applies to writes observed by polling. Hl's output ordering window may add a separate delay.
- **SC-002**: Across repeated automated rotation, truncation, and delete-then-recreate cycles, the follower neither skips nor duplicates any entry that remains readable until it is observed and drained.
- **SC-003**: On a filesystem where native notifications deliver no events (network/unreliable), the polling engine detects appends within one polling interval of the data becoming readable to the polling process. The default ceiling is 1 second. For actively written files, the adaptive estimator SHOULD reduce observed detection latency well below the ceiling.
- **SC-003a**: On Windows, a write appended to a local file by a process that holds the file open without flushing MUST be detected and delivered on the next adaptive poll that can observe the change, even if `ReadDirectoryChangesW` delivers no event. The polling interval is bounded by the floor and ceiling in FR-015b.
- **SC-004**: While followed files are idle, the component adds no measurable sustained CPU load (no busy-wait), verified over a sustained idle period.
- **SC-005**: A same-size replacement of a followed file is detected and followed within the configured safety-net interval (default 5 seconds) in 100% of trials.
- **SC-006**: When following a mix of local and remote files, the local files' update latency matches the local-only latency target (SC-001) and is unaffected by the remote files' slower cadence.
- **SC-007**: A native watch loss during a session results in continued following with zero lost subsequent entries in 100% of trials.
- **SC-008**: A consumer can achieve correct robust following — across all behaviors above — using only default settings and without implementing any reopen, classification, or platform-specific logic of its own.
- **SC-009**: All robust-following behaviors are verified by automated tests on each supported platform (Linux, macOS, Windows), and overall test coverage is not less than the current coverage on the target branch.
- **SC-010**: Both engines deliver the same per-source data across append, preload, rotation, truncation and recreation scenarios. Each engine meets the detection and delivery timing applicable to it (SC-001, SC-003, SC-003a, SC-005 and SC-006).
- **SC-011**: Under a writer that sustainedly outpaces the consumer, the component's memory use stays bounded (does not grow with the volume of un-drained data) and backpressure drops no entries, verified over a sustained high-rate append test.

## Assumptions

- **Interface shape is a planning concern**: The exact surface the component exposes (content/byte stream vs. pre-split entries vs. semantic events) is an implementation decision deferred to the planning phase. The spec constrains only the *observable behavior* (FR-022/FR-023): a simple default-driven path that hides engine selection, classification, and rotation/truncation handling, plus tunable options with reasonable defaults.
- **Default polling ceiling** is 1 second, configurable via options.
- **Conservative classification** is the default component policy and the policy hl uses.
- **Preloaded tail** defaults to the application's existing default (last 10 entries) and is configurable.
- **Same-size re-check cadence** defaults to 5 seconds (coreutils' effective `--max-unchanged-stats` default of 5 × the 1 s poll interval) and is configurable.
- **Entry delimiter** is handled by the consumer. Hl splits content using its configured input delimiter and applies its [partial trailing entry rules](#partial-trailing-entry). The component must give the consumer enough information to apply those rules when a followed stream ends or is replaced.
- **Known-local filesystem set** is sourced from the same vetted classification data used by established tools (e.g., the filesystem-type tables coreutils/gnulib rely on) so the project inherits their real-world vetting rather than maintaining an ad-hoc list.
- **Symlinks** initially resolve to their target, as in the current CLI. The proposed follower re-resolves the original path when the symlink changes.
- **Single workspace**: the reusable component is a new member of the existing workspace and is built and tested alongside the application.

## Dependencies

- Operating-system filesystem-type / drive-type query capabilities on each platform (to classify reliability).
- Operating-system file-identity information on each platform (device/inode on Unix; volume + file id on Windows) to detect rotation/replacement.
- Native filesystem-notification facilities on each platform, plus a polling fallback mechanism.
- The application's existing entry-delimiter and tail-preloading concepts, which the component must interoperate with.
