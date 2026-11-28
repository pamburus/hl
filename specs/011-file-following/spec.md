# Feature Specification: Robust File Following (`tail -F` Parity)

**Feature Branch**: `011-file-following`
**Created**: 2026-06-08
**Status**: Draft
**Input**: User description: "Replicate GNU `tail -F` follow behavior in hl: use OS-native filesystem notifications when they are reliable, and automatically fall back to polling when notifications are unreliable (network drives, FUSE mounts, or any filesystem whose reliability cannot be confirmed). All watcher/engine-selection/classification/platform logic must live in a new reusable standalone workspace crate with a simple, default-driven interface."

## Clarifications

### Session 2026-11-28

- Q: Where may follow mode differ from `tail -F`? → A: Only where doing what `tail -F` does is not possible, or would break another requirement of this spec. Each difference is listed under **Differences from `tail -F`**, with its reason (FR-039). The behavior of `tail -F` is that of GNU coreutils 9.1.
- Q: What happens with a path that is a device, a directory or a socket? → A: What `tail -F` does, as following starts and when one takes the place of a followed file. A character device, such as a terminal or `/dev/null`, is followed: what it has is read without waiting, and again each time its modification time changes, checked once per poll interval, until the path refers to another file. A block device or a directory is reported as a file that cannot be followed, and waited for until a file takes its place. A socket cannot be opened, so it is waited for as an inaccessible path (FR-002). The other inputs are followed meanwhile. This replaces the rejection of directories and other special files decided on 2026-06-08 and 2026-11-27, which carried devices and sockets along with directories, with no reason of its own, and gave no reason to differ from `tail -F` for directories.
- Q: How is a file followed whose size stays zero while it has content, such as one in procfs or sysfs? → A: It is polled, as such files change with no notification, and read again from its start once per poll interval. This differs from `tail -F`, which shows it twice and then never again with native notifications, and reads it again and again with no pause when polling.
- Q: Is a missing file given by a bare name, which is in the current directory, waited for with native notifications? → A: Yes, as is any path on a file system confirmed local (FR-010).

### Session 2026-11-27

- Q: What happens when a followed path is missing or cannot be read as following starts? → A: It is waited for, as with `tail -F`, and once it can be read, the file is shown from its start (FR-002). The other paths are followed meanwhile.
- Q: Is a directory at a followed path rejected, as `tail -F` waits for it? → A: Revised on 2026-11-28: a directory is waited for until a file takes its place, as with `tail -F`, as following starts and later (FR-026).
- Q: Where do the notices that `tail -F` writes to its standard error go, and does a failing input stop the others? → A: To the debug log, as warnings for a path that cannot be opened and as information for other changes, dropped unless `HL_DEBUG_LOG` turns them on, while the other inputs keep being followed (FR-016b). An error reading an open input ends following with a failure, as `tail -F` exits on one (FR-016c).
- Q: How does Windows meet FR-035 and FR-036? → A: A stop cancels a blocked read of a pipe, but a read of the console may not end. A closed output pipe is found by checking its state periodically, as Windows reports it only once something is written.

### Session 2026-11-26

- Q: What does the component deliver for each followed input? → A: A sequence of byte streams, one for each generation of the file at the path, that is, the file between two replacements. A read of a stream waits while the file may still grow. A stream ends when its file is replaced, truncated or deleted, once the old file is read to its end, or, for a FIFO, once it has no writer left; the next stream then starts at the beginning of the new file, or with the next writer. The consumer splits the bytes into entries, so holding back a partial entry and handling entries longer than the maximum message size stay in the consumer.
- Q: How does following a path start? → A: Either with the path alone, or with the path and a handle already open on it. With a handle, delivery continues from the handle's current position, with no gap and no duplicate, so the consumer can preload the last entries from that handle first. With the path alone, the component opens the file itself and delivers what is appended after its current end.
- Q: What happens when the last writer of a FIFO closes it? → A: Text after its last delimiter is shown as a complete entry, and following goes on with the next writer of the FIFO. Standard input still ends at its end.
- Q: What happens when the output of follow mode is closed, for example by `hl -F app.log | head -1`? → A: hl stops following and exits, also while no input changes. The component offers a stop that ends all streams of a follow request at once, including a read that waits for data. A wait for data ends as soon as data comes or the stop is requested, not at the next tick of a timer.

### Session 2026-11-25

- Q: Is a partial trailing entry ever shown before its delimiter arrives? → A: Yes, in the cases listed under the **Partial trailing entry** edge case, when waiting longer would lose it, hold back other inputs or exceed the memory bound. These cases are intended behavior.

### Session 2026-06-24

- Q: How is the memory-bounded guarantee preserved for content that never reaches a delimiter (an effectively unbounded single entry)? → A: Bounded by the consumer's existing maximum-segment-size cap (in hl, `--max-message-size`, default 64 MiB), enforced by the consumer's scanner over the facade's byte stream in all modes (including follow); the component itself does no unbounded in-memory accumulation of a single entry.

### Session 2026-06-08

- Q: What happens when a non-regular input (FIFO/pipe, directory, socket) is given in follow mode? → A: FIFOs/pipes are followed by streaming (read to EOF, then keep waiting for more, like `tail`). Rotation/truncation tracking does not apply to pipe-like inputs. Revised on 2026-11-28: directories, devices and sockets are handled as `tail -F` handles them (FR-026), instead of being rejected.
- Q: How must the component behave when a writer appends faster than the consumer drains? → A: Memory-bounded with backpressure and no committed-data loss. For regular files the data waits on disk and is read at the consumer's pace (no unbounded internal buffering); for pipe-like inputs a bounded buffer applies, with the OS pipe buffer providing natural backpressure to the writer.
- Q: Must engine selection and runtime fallback/degradation be observable, given the "no silent failures" principle? → A: Yes — observable via debug logging (routed through the consumer's debug-log channel, e.g., `HL_DEBUG_LOG` in hl); normal stdout/stderr stays clean. User-visible status messages remain deferred (OOS-001).
- Q: What is the default cadence of the same-size replacement re-check safety net? → A: 5 seconds by default (matching coreutils' effective default of 5 × the 1 s poll interval), configurable via options.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Live Tailing on a Local Disk (Priority: P1)

A user runs the log viewer in follow mode against a file on a local disk that another process is actively appending to. New entries appear in the viewer promptly and in order, without the user re-running the command, and without measurable CPU cost while the file is idle.

**Why this priority**: This is the everyday case and the core value of follow mode. If live tailing of a local file is not reliable and efficient, the feature has no value.

**Independent Test**: Start follow mode on a local file, append lines from a separate process, and verify each appended line appears in the viewer within the latency target while idle CPU stays negligible.

**Acceptance Scenarios**:

1. **Given** follow mode is active on a local file, **When** a writer appends a complete entry, **Then** the entry appears in the viewer within the responsiveness target (see SC-001).
2. **Given** follow mode is active and no writes occur, **When** the file remains idle, **Then** the viewer consumes negligible CPU (no busy-waiting).
3. **Given** follow mode starts, **When** it begins, **Then** the last N preloaded entries are shown first and subsequent appends continue seamlessly from that point with no duplicated or skipped entries.

---

### User Story 2 - Survive Log Rotation, Truncation, and Deletion (Priority: P1)

A user follows an application log that is periodically rotated, truncated, or briefly removed and recreated by a logging framework or `logrotate`. The viewer keeps following the live file by name across all of these events without losing entries and without the user restarting it.

**Why this priority**: Real-world logs are rotated. This is precisely the behavior that distinguishes `tail -F` from `tail -f` and is the durability guarantee users depend on. Without it, follow mode silently stops showing new data after the first rotation.

**Independent Test**: Drive a file through append → rotate (rename + recreate) → append, append → truncate → append, and delete → recreate → append, verifying that entries written to the current file-by-name continue to appear and none are lost across each transition.

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

**Independent Test**: Follow a file on a known-remote or unrecognized filesystem, append to it from another host/process, and verify new entries still appear (within the polling interval) even though native notifications deliver nothing.

**Acceptance Scenarios**:

1. **Given** a followed path resides on an affirmatively-known-local filesystem, **When** following begins, **Then** native OS notifications are used for that path.
2. **Given** a followed path resides on a known-remote filesystem (e.g., NFS, SMB/CIFS, AFP), **When** following begins, **Then** polling is used for that path.
3. **Given** a followed path resides on a filesystem whose type cannot be determined, or the filesystem query fails, **When** following begins, **Then** polling is used for that path (conservative default).
4. **Given** polling is in effect for a path, **When** the file is updated by any writer (including a remote host), **Then** the update appears within the configured polling interval.

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
2. **Given** a path has migrated to polling after a watch loss, **When** the file is subsequently appended to, **Then** the new entries appear within the polling interval.

---

### User Story 6 - Reusable Component for Other Consumers (Priority: P2)

A developer (the application itself, or any other program in the workspace) needs the same robust follow behavior. They depend on a single self-contained component, ask it to follow one or more paths, and receive newly appended content per source — without having to understand or reimplement engine selection, filesystem classification, or rotation/truncation handling. When they need to, they can adjust behavior through options that all have sensible defaults.

**Why this priority**: The packaging requirement is explicit: the complexity must be encapsulated and reusable, not entangled in the application binary. This keeps the durable, platform-specific logic in one tested place and makes correct following available to any consumer.

**Independent Test**: From a minimal consumer using only default settings, follow a file through append/rotation/truncation and confirm correct delivered content without the consumer implementing any reopen, classification, or platform-specific logic.

**Acceptance Scenarios**:

1. **Given** a consumer using only default settings, **When** it follows one or more paths, **Then** it receives newly appended content per source with rotation, truncation, deletion/recreation, and engine selection handled internally.
2. **Given** a consumer with specific needs, **When** it overrides options (e.g., polling interval, fallback policy, whether to retry missing files), **Then** the component honors those overrides while leaving unspecified options at their defaults.
3. **Given** the application binary, **When** it adopts the component, **Then** its previous inlined file-monitoring logic is removed in favor of the component.
4. **Given** a consumer that has read the last entries of a file from a handle it opened, **When** it starts following the path with that handle, **Then** it receives the content after the handle's position, with nothing skipped or repeated.
5. **Given** a consumer reading a stream that waits for data, **When** it requests a stop, **Then** the read ends at once, whether or not the file changes.

---

### User Story 7 - Several Inputs Merged by Time (Priority: P1)

A user follows the logs of several services at once and reads them as one stream in time order. Entries of all inputs are interleaved by their timestamps, a stack trace stays right after the record it belongs to, a long entry is never cut by the entries of other inputs, and each record shows whether it came in time order.

**Why this priority**: Following several inputs at once is what `hl -F` is used for most, and a merged stream that drops, splits or misplaces entries is wrong output.

**Independent Test**: Follow two files, append records with interleaved timestamps to both within the sync interval, plus a multi-line text after one record and an entry longer than `--max-message-size`, and check the order and integrity of the output.

**Acceptance Scenarios**:

1. **Given** two followed inputs, **When** records with interleaved timestamps are appended to both within the sync interval, **Then** they are shown in timestamp order (FR-027).
2. **Given** a followed input, **When** several records with the same timestamp are appended in separate writes, **Then** all of them are shown (FR-027).
3. **Given** a record followed by text that is not a record, such as a stack trace, **When** records of other inputs come meanwhile and the text comes within the sync interval of its record, **Then** the text is shown right after its record (FR-029).
4. **Given** a record that reaches the file in several writes, **When** its end arrives, **Then** it is shown as one formatted entry (FR-030).
5. **Given** the output of `hl -F` is closed by its reader, **When** no input changes, **Then** hl exits (FR-035).

---

### Edge Cases

- **File replaced by a same-size file**: A pure size comparison misses this. The component periodically re-checks the path by re-opening it (a configurable safety-net cadence, analogous to coreutils `--max-unchanged-stats`) and detects replacement via identity change.
- **Rotation on Windows**: Windows has no inode/device pair. Rotation/replacement MUST still be detected, using the platform's file-identity information (volume + file id) as the equivalent of the Unix device/inode pair.
- **Writes on Windows from always-open writers**: Windows NTFS defers directory index entry updates (size, mtime) until `CloseHandle` or `FlushFileBuffers`. `ReadDirectoryChangesW` monitors the directory index and therefore silently misses writes from log writers that keep the file open without flushing (e.g., lumberjack-style appenders). The component MUST detect such writes without requiring the writer to flush, by checking the size of the open file through its handle (`GetFileInformationByHandle`), which bypasses the directory cache, at the adaptive interval of FR-015b, rather than relying solely on directory-change notifications (FR-019a).
- **Symlinked target**: When a followed path is a symlink, the component follows the underlying target and re-resolves on replacement.
- **Path is a FIFO**: The component follows it by streaming, without rotation, truncation or identity tracking, which do not apply to pipe-like inputs. When its last writer closes it, the stream ends and the next one starts with the next writer, so the FIFO is followed across writers. The stream ends only once the FIFO has no writer: a writer that opens it before the last one closes it, or before what that one wrote is read to its end, writes to the same stream, as a FIFO reads as ended only then, so text after the last delimiter of the earlier writer is joined with what the next one writes. Standard input is read to its end, and following it stops there. So is a pipe that has no name, such as one given by process substitution, as it can have no other writer.
- **Path is a device, a directory or a socket**: Handled as by `tail -F`, as following starts and when one takes the place of a followed file, while the other inputs are followed meanwhile (FR-026):
  - **A character device**, such as a terminal or `/dev/null`, is followed: what it has is read without waiting, and again each time its modification time changes, checked once per poll interval. Once the path refers to another file, found at the same-size re-check cadence (FR-006), that file is followed from its start. Data that comes to a device with no change of its modification time, such as the input typed in a terminal, is not read, as with `tail -F`.
  - **A block device or a directory** is reported as a file that cannot be followed (FR-016b), and waited for until a file takes its place.
  - **A socket** cannot be opened, so it is waited for as an inaccessible path (FR-002).
- **File whose size stays zero while it has content**, such as one in procfs or sysfs: such files change with no notification, so they are polled (FR-011), and the file is read again from its start once per poll interval, as it is found shorter than the content read from it (FR-004). See **Differences from `tail -F`**.
- **File whose size cannot be relied on**, such as one in procfs that cannot be seeked from its end, or one in sysfs that is shorter than its size: hl finds its last entries (FR-008) by reading it from its start, as `tail -F` shows them, rather than failing.
- **Rapid successive events / event queue overflow**: A burst of changes or a dropped/overflowed native event MUST NOT permanently desynchronize following; the component re-checks state so it converges to the current file contents.
- **Truncation outgrown before it is noticed**: A file truncated in place keeps its identity, so truncation is detected only by its size shrinking (FR-004). If the file grows past its last-known size again before the component checks it, for example when `copytruncate` rotation is followed by a burst of writes, the truncation cannot be told from an append: the content after the last-known size is delivered as appended, and the content before it is not. `tail -F` behaves the same way, and this is an accepted limitation.
- **Writes during a reopen**: Entries written between detecting a rotation/truncation and reopening MUST NOT be lost; the component reads from the correct offset of the correct file so no committed bytes are skipped.
- **Filesystem query failure**: If the filesystem type cannot be determined, the path is treated as unreliable and polled (never assumed local).
- **Partial trailing entry**: hl holds back text at the end of a followed regular file that has no delimiter yet, as its writer has usually not finished it, and shows it as one entry once its delimiter arrives (FR-030). Content that never reaches a delimiter is bounded by the maximum message size (`--max-message-size`, default 64 MiB), so it cannot grow memory without bound. The text is shown before its delimiter arrives only when waiting longer would lose it, hold back other inputs, or exceed that bound:
  - **The text reaches the maximum message size**: the part read so far is shown, and the rest is shown as it comes, continuing the same entry, with no entries of other inputs in between (FR-031).
  - **Other inputs are followed**: an entry longer than the maximum message size that is still being written is ended early, when its next part has not come for the sync interval and an entry of another input is due, when the entries held in the merge window reach twice its memory limit (FR-032), or when all inputs are closed. Its rest is shown as it comes, as a new entry that starts with the input's badge, or with the placeholder of the sync indicator when badges are not shown. When a single input is followed, the entry waits for its rest, like `tail -f`.
  - **The file is replaced, truncated or deleted**: the rest of the old file will not come, so its text is shown as a complete entry before the new file is read.
  - **The last writer of a FIFO closes it**: text after its last delimiter is shown as a complete entry, ended by the output delimiter, and the FIFO is followed on with its next writer. A writer that opens it before then writes to the same stream (see **Path is a FIFO**).
  - **A filter or `--input-format` is set**: an entry longer than the maximum message size is hidden, like other text that is not a record (FR-029), so none of the cases above shows it.

## Requirements *(mandatory)*

### Functional Requirements

#### Follow Semantics (match `tail -F` = follow-by-name + retry)

- **FR-001**: The component MUST follow each path *by name*: after the file at a path is replaced, it MUST follow whatever file currently occupies that path, not the previously-open file.
- **FR-002**: The component MUST retry paths that are currently inaccessible (missing or unreadable), resuming automatically when a file becomes available at the path, including paths that do not exist when following begins.
- **FR-003**: When a followed file grows, the component MUST deliver the newly appended content in order.
- **FR-004**: When a followed file is truncated (its size shrinks below the last-known size), the component MUST reopen it and resume delivery from the beginning of the current file.
- **FR-005**: When a followed file is rotated or replaced (its filesystem identity changes), the component MUST begin following the new file at the path from its start, without losing content committed before the switch.
- **FR-006**: The component MUST detect same-size replacement that a size comparison alone would miss, via a periodic re-check of the path on a configurable cadence (enabled by default, defaulting to 5 seconds — matching coreutils' effective default of 5 × the 1 s poll interval).
- **FR-007**: When a followed file is deleted, the component MUST keep the path under observation and resume following when a file reappears there.
- **FR-008**: The component MUST start following a path either from the path alone, delivering what is appended after the current end of the file, or from the path together with a handle already open on it, delivering from the handle's current position. With a handle, nothing between that position and later appends is skipped or delivered twice, so that the consumer can preload the last entries from the handle first (in hl, the last `--tail` entries, default 10) and continue from there seamlessly.
- **FR-039**: Follow mode MUST do what GNU `tail -F` (coreutils 9.1) does wherever this spec does not state otherwise. Each difference from it MUST be listed under **Differences from `tail -F`**, with its reason, which is that doing the same is not possible, or would break another requirement of this spec.
- **FR-008a**: The component MUST deliver the content of each followed path as a sequence of byte streams, one for each generation of the file at the path. A stream MUST end when its file is replaced, truncated or deleted, after the content of the old file is delivered to its end, or, for a FIFO, once it has no writer left; the next stream MUST start at the beginning of the new file, or with the next writer of the FIFO. Reading a stream MUST wait while more data may come, instead of reporting its end.

#### Filesystem Reliability Classification (conservative)

- **FR-009**: The component MUST classify each followed path's filesystem as either *affirmatively known-local* or *not confirmed reliable*.
- **FR-010**: The component MUST use native OS notifications ONLY for paths on an affirmatively-known-local filesystem.
- **FR-011**: The component MUST use polling for any path that is known-remote (including NFS, SMB/CIFS, AFP), on a FUSE mount, on a Windows network drive, on a UNC path, on an unrecognized/unknown filesystem, on a file system whose files change with no notification (procfs and sysfs on Linux), or for which the filesystem query fails. (Conservative default: anything not positively confirmed local is polled.)
- **FR-011a**: FR-010 and FR-011 describe the default, conservative fallback policy. A consumer MAY choose another policy through the options (FR-023): an optimistic one, which uses native notifications for any path where they can be set up, for file systems that are local but not recognized as such, or one that polls every path, as `tail -F` does with `---disable-inotify`.
- **FR-012**: Classification and routing MUST be performed *per path*: when following multiple paths, some MAY use native notifications while others use polling, determined independently for each path.

#### Engine Selection and Runtime Behavior

- **FR-013**: The component MUST deliver equivalent follow semantics (FR-001–FR-008) regardless of whether a path is served by native notifications or by polling — the mechanism MUST be transparent to the consumer.
- **FR-014**: When a native watch for a path is lost or errors at runtime, the component MUST migrate that path to polling without restarting following and without losing subsequent content.
- **FR-015**: The component MUST NOT busy-wait; while followed files are idle, resource usage MUST remain negligible.
- **FR-015b**: The polling backend (used for network shares and other unreliable sources on all platforms) MUST use an IIR-based adaptive interval estimator per source. The estimator MUST tighten the polling interval toward a floor (10 ms) when a source is actively written, and MUST relax it toward a configurable ceiling (default 1 s) during idle periods. The same estimator MUST set the interval of the checks of the open file in the Windows native backend (FR-019a), so that all periodic checks adapt the same way.
- **FR-015a**: When a writer appends faster than the consumer drains, the component MUST remain memory-bounded and MUST NOT lose committed data. For regular files this means reading at the consumer's pace with unread data left on disk (no unbounded internal buffering); for pipe-like inputs a bounded buffer applies and backpressure propagates to the writer via the OS pipe buffer. Content that never reaches a delimiter (an effectively unbounded single entry) is bounded by the consumer's maximum-segment-size cap applied during scanning (in hl, `--max-message-size`, default 64 MiB), so the memory-bounded guarantee holds in all modes.
- **FR-016**: The component MUST converge to current file contents after a dropped or overflowed native event burst (no permanent desynchronization).
- **FR-016a**: The component MUST make its per-path engine selection (native vs polling), its conservative fallback decisions, and any runtime degradation observable to the consumer through a diagnostic/logging interface, so that "no silent failures" holds without these decisions appearing in normal output. The consumer routes this to its own debug-log channel (in hl, the existing `HL_DEBUG_LOG` mechanism).
- **FR-016b**: The component MUST report through the same interface what `tail -F` reports on its standard error while following, and keep following the other inputs meanwhile: as a warning, a path that cannot be opened (missing, unreadable, or not a file that can be followed), with the reason, once until a file can be opened there; as information, a file that is truncated, replaced or removed, and a file at the path that is followed from its start. In hl, these go to the debug log, so they are dropped unless `HL_DEBUG_LOG` turns them on, and do not land over the pager's screen.
- **FR-016c**: An error reading an input that is open, as opposed to opening one (FR-002), MUST end following, with the error reported as hl exits with a failure status, as `tail -F` exits on a read error. In hl, this includes an error decoding a compressed input (FR-038).
- **FR-037**: The native notifications of a process MUST NOT use a resource for each followed path that the operating system limits to a small number per user, so that following many local paths keeps native notifications for all of them. On Linux, all paths MUST share one inotify instance, as a user may have only 128 of them by default.

#### Platform Support

- **FR-017**: The component MUST support Linux, macOS, and Windows for both native notification and polling modes.
- **FR-018**: On Linux and macOS, classification MUST use the operating system's filesystem-type information.
- **FR-019**: On Windows, classification MUST treat network drives and UNC paths as not-confirmed-reliable, and rotation/replacement detection MUST use the platform's file-identity information as the device/inode equivalent.
- **FR-019a**: On Windows, the native backend for locally-classified paths MUST combine `ReadDirectoryChangesW` (for immediate rotation/deletion/creation events) with checks of the open file through its handle (`GetFileInformationByHandle`) at the adaptive interval of FR-015b (for writes by writers that keep the file open, whose notifications may be delayed). A wait for a change MUST end at whichever comes first, a notification or the next check; correctness does not depend on which one it is.
- **FR-020**: The macOS native path MUST be subject to the same reliability gate as other platforms (it MUST NOT bypass classification).

#### Reusability and Packaging

- **FR-021**: All watcher abstraction, native-vs-polling engine selection, filesystem-reliability classification, and platform-specific code MUST reside in a single, self-contained, reusable workspace component, separate from the application binary.
- **FR-022**: The component MUST expose a simple default-driven interface such that a consumer can follow one or more paths and receive newly appended content per source without configuring or understanding engine selection, classification, or rotation/truncation handling.
- **FR-023**: The component MUST expose advanced options — at minimum polling interval, fallback policy, whether to retry missing paths, and the same-size re-check cadence — each with a reasonable default so that consumers only set what they need.
- **FR-024**: The application binary MUST consume this component and MUST remove its previous inlined file-monitoring logic in favor of it.
- **FR-025**: Following multiple paths through the component MUST be supported through a single follow request, with content attributable to its source path.

#### Input Types

- **FR-026**: The component MUST follow regular files with full rotation/truncation/identity tracking (FR-001–FR-008). It MUST follow FIFOs without rotation/truncation/identity tracking, across writers: when the last writer closes the FIFO, its stream ends (FR-008a) and the next stream starts with the next writer. Standard input MUST be read to its end, where following it stops. So MUST a pipe that has no name, such as one given by process substitution. As following starts and when one takes the place of a followed file, it MUST, as `tail -F` does: follow a character device, reading what it has without waiting, and again each time its modification time changes, checked once per poll interval, until the path refers to another file; report a block device or a directory as a file that cannot be followed (FR-016b) and wait for a file to take its place; and wait for a socket as an inaccessible path (FR-002). On Windows, a device, such as `NUL` or the console, MUST be read to its end as the only stream, as standard input is. It MUST NOT panic or silently misbehave on any input type.
- **FR-038**: hl MUST follow a compressed file, in any format it decodes when not following, by showing it decoded from its start, as it is shown when not followed, whatever the number of last entries asked for. It MUST show each entry of the file once, also in a format whose compressed data ends before the end of the file.

#### Merging Inputs (hl)

These requirements apply to hl in follow mode, which reads the content delivered by the component, splits it into entries and shows the entries of all inputs as one stream. How text that is not a record is recognized is defined by the input format specs ([JSON](../004-json-input-format/spec.md), [logfmt](../005-logfmt-input-format/spec.md)); what follow mode does with it is defined here.

- **FR-027**: hl MUST show the entries of all followed inputs ordered by timestamp within the sync interval (`--sync-interval-ms`, default 100 ms): each entry is held for up to the sync interval after it is read, and the held entry with the earliest timestamp is shown first. Entries with equal timestamps MUST all be shown, in the order of their inputs and then of their position. An entry with no timestamp to be ordered by, such as text before the first record of an input, is shown without waiting.
- **FR-028**: Each record MUST start with the sync indicator, showing whether its timestamp is in order with the record shown before it. Text that is not a record, and the rest of an entry that was ended early, start with a blank placeholder of the same width instead. When badges are shown, the indicator or placeholder is followed by the input's badge.
- **FR-029**: Text that is not a record, such as a stack trace, MUST be shown right after the record before it in its input, never before that record, nor after entries of other inputs that came after it. Text that comes after entries of other inputs have already been shown is shown after them, as what is shown is never reordered, and, like any entry, is held for no longer than the sync interval (FR-027). Text before the first record of an input MUST be ended by the output delimiter. When a filter or `--input-format` is set, text that is not a record MUST be hidden, as without follow mode.
- **FR-030**: A record that reaches a followed file in several writes MUST be shown as one formatted entry once its delimiter arrives, also when the first write carries enough complete records to reach the maximum message size.
- **FR-031**: An entry longer than the maximum message size MUST be shown in its place as one entry, part by part as it is read, with no entries of other inputs in between, also when the timestamps of its input are out of order, except in the cases listed under the **Partial trailing entry** edge case. When a filter or `--input-format` is set, such an entry MUST be hidden.
- **FR-032**: The entries held to be ordered MUST be bounded by a memory limit of `--concurrency` × the buffer size, which is `--buffer-size` capped at `--max-message-size`. Once the held entries reach it, they MUST be shown without waiting for the rest of the sync interval. While an entry longer than the maximum message size waits for its next part, entries of other inputs MAY be held up to twice the limit, after which that entry is ended early.
- **FR-033**: While entries wait to be shown, hl MUST NOT busy-wait; it waits until the next entry is due or new content comes. Once all inputs are closed, the held entries MUST be shown at once.
- **FR-034**: Entries MUST reach the output as soon as they are shown, whatever the output delimiter, so that a program reading the output through a pipe gets them without delay.

#### Stopping

- **FR-035**: hl MUST stop following and exit once its output is closed, for example by the reader of a pipe, also while no followed input changes (SC-013). Where the platform does not report that the reader of a pipe has gone until something is written to it (Windows), hl checks the state of the output pipe periodically, often enough to meet SC-013. An output whose closing is reported only to a write, such as a network connection closed by its peer, is found closed at the next write.
- **FR-036**: The component MUST offer a stop that ends all streams of a follow request at once, including a read that waits for data. Waiting for data MUST end as soon as data comes, the stream ends or the stop is requested, rather than at the next tick of a timer; periodic checks remain only where this spec requires them (FR-006, polling of paths that are not confirmed local, the output check of FR-035 on Windows). On Windows, a read of the console may not end on a stop, as it cannot be cancelled reliably there.

### Out of Scope (Future Work)

- **OOS-001**: Diagnostic messages mirroring `tail` (e.g., "file truncated", "has appeared", "has become inaccessible") in normal output or on the standard error by default are out of scope; they go only to the debug log (FR-016b).
- **OOS-002**: A user-facing override flag/config to force polling or force native notifications (analogous to `tail ---disable-inotify`) is deferred. (The internal fallback-policy option exists for consumers, but no end-user CLI surface is added in this iteration.)
- **OOS-003**: Follow-by-descriptor semantics (the `tail -f` variant that keeps following a renamed file) are not provided; only follow-by-name (`-F`) is in scope.
- **OOS-004**: Rotation/truncation/identity tracking for pipe-like inputs is out of scope — FIFOs/pipes are followed only as continuous streams (read-to-EOF then wait). Block devices and directories are not read, as by `tail -F`: they are waited for to be replaced by a file (FR-026).

### Differences from `tail -F`

Follow mode differs from GNU `tail -F` (coreutils 9.1) only in these cases (FR-039):

- **Engine per path** (FR-012): `tail -F` polls all files once one of them is not confirmed local or is not a regular file or FIFO, such as `/dev/null`, so an append to a local file then shows after up to the poll interval instead of at once. hl decides for each path, so a local file keeps the latency of SC-001 (SC-006).
- **Notices** (FR-016b, OOS-001): `tail -F` writes them to its standard error. hl writes them to the debug log, as on the standard error they would land over the pager's screen and mix with the output.
- **Partial trailing entry** (FR-030): `tail -F` writes bytes as they come. hl shows entries it formats, which it can do only with the whole entry, so it holds back text that has no delimiter yet, except in the cases listed under the **Partial trailing entry** edge case.
- **Several inputs** (FR-027 to FR-029): `tail -F` writes each read under a header naming its file. hl shows the entries of all inputs as one stream ordered by time, with the input's badge, as it does without follow mode.
- **Compressed files** (FR-038): `tail -F` writes their compressed bytes. hl shows them decoded, from their start, as without follow mode, as the last entries of compressed data cannot be found without decoding it all.
- **File whose size stays zero while it has content**, such as one in procfs or sysfs: `tail -F` shows it twice and then never again with native notifications, which such a file does not send, and reads it again and again with no pause when polling. hl polls it (FR-011) and reads it again once per poll interval, so its changes are shown with no busy wait (FR-015).
- **Devices on Windows** (FR-026): GNU `tail -F` does not run on Windows natively, so there is no behavior there to match. A device, such as `NUL` or the console, is read to its end as the only stream, as standard input is, which is what a user typing into the console expects, rather than leaving the input typed in a terminal unread, as `tail -F` does on Unix.

### Key Entities

- **Followed Source**: A path the consumer asked to follow, together with the component's tracked state for it (last-known size, filesystem identity, current engine, accessibility).
- **Filesystem Reliability**: A per-path classification — *known-local* (native notifications permitted) vs *not-confirmed-reliable* (polled).
- **Follow Engine**: The mechanism currently serving a source — native OS notifications or polling — selected per path and able to change at runtime.
- **Follow Options**: The consumer-tunable settings (polling interval, fallback policy, retry behavior, same-size re-check cadence), each with a default.
- **File Generation Stream**: The bytes of one file at a followed path, from where delivery starts to where the file is replaced, truncated or deleted, or, for a FIFO, the bytes written to it until it has no writer.
- **Merge Window**: In hl, the entries of all inputs held for up to the sync interval to be shown in timestamp order, bounded by its memory limit.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: On a local filesystem, an appended entry appears to the consumer within 200 ms of being committed by a writer, under normal conditions. A write on Windows that is not notified is covered by SC-003a instead.
- **SC-002**: Across log rotation, truncation, and delete-then-recreate sequences, zero committed entries written to the current file-by-name are lost or duplicated, verified over repeated automated cycles. A truncation that the file outgrows before it is noticed is excepted, as an accepted limitation (see the **Truncation outgrown before it is noticed** edge case), and so is a file that is created, written and removed or replaced between two checks of a polled path, which is never seen, and data that a truncation erases before it is read, which is no longer there to be read, both as with `tail -F`.
- **SC-003**: On a filesystem where native notifications deliver no events (network/unreliable), appended entries still appear within one polling interval (default 1 second) in 100% of trials. For actively-written files, the adaptive estimator SHOULD reduce observed latency well below the ceiling.
- **SC-003a**: On Windows, a write appended to a local file by a process that holds the file open without flushing MUST be detected and delivered at the next check of the open file, whether or not a notification comes for that write: within one polling interval (default 1 second) for a file that has been idle, and sooner for a file that is actively written, as with SC-003.
- **SC-004**: While followed files are idle, the component does not busy-wait: hl following 10 idle files uses less than 1% of one CPU core, measured as its CPU time over 60 seconds.
- **SC-005**: A same-size replacement of a followed file is detected and followed within the configured safety-net interval (default 5 seconds) in 100% of trials.
- **SC-006**: When following a mix of local and remote files, the local files' update latency matches the local-only latency target (SC-001) and is unaffected by the remote files' slower cadence.
- **SC-007**: A native watch loss during a session results in continued following with zero lost subsequent entries in 100% of trials.
- **SC-008**: A consumer can achieve correct robust following — across all behaviors above — using only default settings and without implementing any reopen, classification, or platform-specific logic of its own.
- **SC-009**: All robust-following behaviors are verified by automated tests on each supported platform (Linux, macOS, Windows), and overall test coverage is not less than the current coverage on the target branch.
- **SC-010**: The behaviors in SC-002, SC-004, SC-005, SC-007 and SC-008 hold identically whether a path is served by native notifications or by polling (mechanism is transparent to the consumer), and appended entries appear within the latency target of the mechanism: SC-001 for native notifications, with SC-003a for a write on Windows that is not notified, and SC-003 for polling.
- **SC-011**: Under a writer that sustainedly outpaces the consumer, the component's memory use stays bounded (does not grow with the volume of un-drained data) and no committed entry is lost, verified over a sustained high-rate append test.
- **SC-012**: An entry longer than the maximum message size that is written to a followed file is shown in about the time it takes to show the same data read from standard input, whatever the maximum message size; the time grows no faster than the length of the entry.
- **SC-013**: Once the output of follow mode is closed by its reader, such as the reader of a pipe, hl exits within 1 second, whether or not any followed input changes. An output whose closing is reported only to a write (FR-035) is excepted.
- **SC-014**: A record appended to one of several followed local files is shown within the sync interval plus the latency of SC-001.

## Assumptions

- **Interface shape**: The component delivers a sequence of byte streams per path (FR-008a), not entries or events; the exact types and names are a planning concern. The spec also constrains the *observable behavior* (FR-022/FR-023): a simple default-driven path that hides engine selection, classification, and rotation/truncation handling, plus tunable options with reasonable defaults.
- **Default polling interval** is 1 second, configurable via options, matching the long-standing `tail` default.
- **Conservative classification** is the default policy: anything not positively confirmed local is polled. The fallback-policy option lets advanced consumers relax it, or poll every path (FR-011a), but the default is conservative.
- **Preloaded tail** is read by the consumer from the handle it passes to the component (FR-008); in hl it defaults to the last 10 entries (`--tail`).
- **Same-size re-check cadence** defaults to 5 seconds (coreutils' effective `--max-unchanged-stats` default of 5 × the 1 s poll interval) and is configurable.
- **Entry delimiter** is the consumer's concern: the component delivers bytes, and the consumer splits them on its delimiter (hl supports newline and NUL delimiters) and holds back a partial trailing entry of a regular file, except in the cases listed under the **Partial trailing entry** edge case.
- **Known-local filesystem set** is sourced from the same vetted classification data used by established tools (e.g., the filesystem-type tables coreutils/gnulib rely on) so the project inherits their real-world vetting rather than maintaining an ad-hoc list.
- **Symlinks** are resolved again each time the followed path is checked for a replacement, as `tail -F` does, so a link pointed at another file is followed to it (see the **Symlinked target** edge case).
- **Single workspace**: the reusable component is a new member of the existing workspace and is built and tested alongside the application.

## Dependencies

- Operating-system filesystem-type / drive-type query capabilities on each platform (to classify reliability).
- Operating-system file-identity information on each platform (device/inode on Unix; volume + file id on Windows) to detect rotation/replacement.
- Native filesystem-notification facilities on each platform, plus a polling fallback mechanism.
- The application's existing entry-delimiter and tail-preloading concepts, which the component must interoperate with.
