# Feature Specification: Pager Integration for Output Display

**Feature Branch**: `010-pager`
**Created**: 2026-02-15
**Status**: Draft
**Input**: User description: "Add feature describing how pager is handled"

## Clarifications

### Session 2026-02-15

- Q: How should the application execute the pager program to prevent security vulnerabilities? → A: Execute pager directly from PATH without shell, validate it's executable before spawning
- Q: What should the user experience be when the specified pager program is not found or cannot be executed? → A: Write error to stderr, fall back to direct stdout (disable paging)
- Q: Which environment variable(s) should the application check for custom pager configuration, and in what order of precedence? → A: Check HL_PAGER first, then fall back to PAGER
- Q: When should the pager process be spawned - immediately when the application starts, or only after buffering some output? → A: Spawn pager immediately at application start (before generating output)
- Q: Which platforms must support pager integration? → A: All unix compatible systems and Windows. On Windows the default pager is also less.
- Q: What command-line option(s) should control paging behavior? → A: `--paging <mode>` where mode is `auto|always|never` (default: auto)
- Q: Should the application pass default arguments to the pager to enable color/formatting support? → A: Provide default options only if the pager is not overridden by environment variables (the default is used). Check the implementation for details.
- Q: What specific terminal state should be reset when the pager crashes or exits abnormally? → A: Reset color/formatting codes and restore echo mode (using `stty echo`)
- Q: How should the application handle SIGPIPE errors when the pager exits early (e.g., user quits before all output is written)? → A: Ignore SIGPIPE, treat write errors as normal termination signal
- Q: What should the error message say when the pager is killed or crashes? → A: "hl: pager killed" (matches current implementation)
- Q: What specific error messages should be shown when the pager executable is not found or lacks execute permissions? → A: "hl: unable to launch pager: <pager_name>: <system_error_message>"
- Q: What exit code should the application use when the pager crashes or is killed unexpectedly? → A: Exit with code 141 (SIGPIPE convention, matching broken pipe)
- Q: Which command-line option specifies the output file that disables paging (as mentioned in FR-008 and User Story 1)? → A: `--output <file>` or `-o <file>`
- Q: Is enabling paging in follow mode within the scope of this feature (010-pager), or is it explicitly deferred to a future feature? → A: Out of scope for this feature, explicitly deferred to future work
- Q: What test coverage expectations should be documented in the Success Criteria? → A: Test coverage should not be less than the current test coverage on the target branch

## Out of Scope

The following items are explicitly **out of scope** for this feature and deferred to future work:

- **Paging in follow mode**: Follow mode (`--follow`) currently disables paging entirely. Enabling paging support in follow mode (where pager exit would stop file monitoring and exit the application) is deferred to a future feature.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - View Many Lines of Output Interactively (Priority: P1)

When a user runs the log viewer application and the output contains more lines than fit on the terminal screen, they need to scroll through the results without losing earlier content. The application automatically pipes output to a pager when appropriate, allowing users to navigate through large volumes of log data line by line.

**Why this priority**: This is the core value proposition of pager integration - making outputs with many lines readable and navigable. Without this, users cannot effectively view log files with numerous entries.

**Independent Test**: Can be fully tested by running the application with a log file containing more than one screen's worth of content and verifying that a pager is automatically launched, allowing scroll navigation.

**Acceptance Scenarios**:

1. **Given** stdout is a terminal (TTY) and no explicit output file is specified and paging is not explicitly disabled, **When** the user runs the application, **Then** the output is automatically piped to a pager
2. **Given** the application runs with paging enabled, **When** the user quits the pager, **Then** the application exits cleanly without hanging
3. **Given** the user specifies an output file via `--output <file>` or `-o <file>`, **When** running the application, **Then** paging is disabled and output writes directly to the file
4. **Given** stdout is redirected or piped (not a TTY), **When** paging is set to "auto", **Then** paging is disabled and output writes directly to stdout

---

### User Story 2 - Control Paging Behavior (Priority: P2)

Users need explicit control over when paging occurs, allowing them to disable automatic paging when piping output to other tools or when they prefer direct output to the terminal.

**Why this priority**: Provides flexibility for advanced users and integration with other command-line tools. Essential for scriptability and automation.

**Independent Test**: Can be tested by running with paging flags (enable/disable) and verifying output behavior matches expectations.

**Acceptance Scenarios**:

1. **Given** the user specifies `--paging never`, **When** running the application, **Then** output writes directly to stdout without launching a pager regardless of TTY status
2. **Given** the user specifies `--paging always`, **When** running the application, **Then** output is piped to a pager regardless of TTY status or output length
3. **Given** the user specifies `--paging auto` or omits the flag (default), **When** stdout is a TTY and no output file is specified, **Then** a pager is launched

---

### User Story 3 - Graceful Exit When Pager Closes (Priority: P1)

When a user closes the pager (e.g., pressing 'q' in less), the application should detect the pager exit and terminate gracefully without error messages or hanging processes.

**Why this priority**: Critical for user experience - users expect quitting the pager to cleanly exit the application. Hanging or error messages create confusion and frustration.

**Independent Test**: Can be tested by running the application in follow mode or with large input, closing the pager, and verifying the application exits with status code 0.

**Acceptance Scenarios**:

1. **Given** the application is running with output piped to a pager, **When** the user quits the pager normally, **Then** the application process terminates within 1 second with exit code 0
2. **Given** the application is in follow mode watching a log file, **When** the user quits the pager, **Then** the application stops monitoring and exits cleanly
3. **Given** the pager crashes or is killed unexpectedly, **When** the application detects the abnormal exit, **Then** the application writes the error message "hl: pager killed" to stderr, resets ANSI color/formatting codes, restores terminal echo mode, and exits with code 141
4. **Given** the user quits the pager while the application is still writing output, **When** the write to pager's stdin fails with broken pipe error, **Then** the application stops processing and exits gracefully with exit code 0 without logging an error

**Note**: This user story applies to non-follow mode. Currently, follow mode explicitly disables paging (the `--follow` flag automatically disables the pager). Enabling paging in follow mode is out of scope for this feature and deferred to future work.

---

### User Story 4 - Custom Pager Selection (Priority: P3)

Advanced users want to specify their preferred pager program (less, most, fzf, etc.) through environment variables, allowing them to use their customized paging environment.

**Why this priority**: Nice-to-have for power users who have specific pager preferences, but not critical for core functionality.

**Independent Test**: Can be tested by setting the pager environment variable and verifying the specified pager is launched.

**Acceptance Scenarios**:

1. **Given** the user sets a custom pager via environment variable, **When** running the application with paging enabled, **Then** the specified pager is used instead of the default
2. **Given** no custom pager is specified, **When** running with paging enabled, **Then** the system default pager (typically `less`) is used
3. **Given** the specified custom pager is not found, **When** attempting to page output, **Then** the error message "hl: unable to launch pager: <pager_name>: <system_error>" is written to stderr and paging is disabled, with output writing to stdout
4. **Given** the specified custom pager lacks execute permissions, **When** attempting to page output, **Then** the error message "hl: unable to launch pager: <pager_name>: <system_error>" is written to stderr and paging is disabled, with output writing to stdout

---

### Edge Cases

- What happens when the pager crashes or is killed externally?
  - Application should detect the abnormal exit, write the error message "hl: pager killed" to stderr, reset ANSI color/formatting codes (ESC[0m), restore terminal echo mode (using `stty echo`), and exit with code 141

- How does the system handle terminal resize while paging?
  - Pager handles resize (application passes through terminal control)

- What happens when output is redirected but user forces paging?
  - Respects user's explicit choice (always page) over auto-detection

- How does follow mode interact with paging?
  - **Current behavior**: Follow mode explicitly disables paging (the `--follow` flag automatically sets paging to false, overriding user preferences)
  - **Future behavior**: Enabling paging in follow mode is out of scope for this feature and deferred to future work

- What if the pager program doesn't support streaming input?
  - Application should detect and handle appropriately or document supported pagers

- How should SIGPIPE be handled when user quits pager early?
  - Ignore SIGPIPE signal, treat write errors (broken pipe) as normal termination condition, exit gracefully with status code 0

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST automatically detect when output is directed to a terminal (TTY) versus being redirected or piped
- **FR-002**: System MUST launch a pager automatically when stdout is a TTY, no output file is specified, and paging mode is "auto" (default)
- **FR-003**: System MUST provide a `--paging` command-line option accepting values `auto`, `always`, or `never` with `auto` as the default
- **FR-003b**: System MUST respect user-specified paging preferences through the `--paging` option (always, never, auto)
- **FR-003a**: System MUST spawn the pager process immediately at application start when paging is enabled, before generating any output
- **FR-004**: System MUST check the HL_PAGER environment variable first for custom pager configuration
- **FR-004a**: System MUST fall back to the PAGER environment variable if HL_PAGER is not set
- **FR-004b**: System MUST use the default system pager if neither HL_PAGER nor PAGER is set
- **FR-004c**: System MUST support shell-style quoting in environment variable values (e.g., `PAGER="less -X"`) using shellwords parsing without full shell execution
- **FR-004d**: System MUST execute the pager program directly from PATH without shell interpretation to prevent command injection vulnerabilities
- **FR-004e**: System MUST validate that the specified pager executable exists and has execute permissions before attempting to spawn the process
- **FR-004f**: System MUST support pager integration on all Unix-compatible systems (Linux, macOS, BSD) and Windows
- **FR-004g**: System MUST use `less` as the default pager on Windows when no custom pager is specified
- **FR-004h**: System MUST pass `-R` flag and set `LESSCHARSET=UTF-8` environment variable when the default `less` pager is used (not overridden by environment variables)
- **FR-004i**: System MUST NOT add default arguments when a custom pager is specified via HL_PAGER or PAGER environment variables
- **FR-005**: System MUST fall back to system default pager (`less`) if custom pager is not specified
- **FR-005a**: System MUST write an error message to stderr in the format "hl: unable to launch pager: <pager_name>: <system_error_message>" when the specified pager program is not found or cannot be executed
- **FR-005b**: System MUST disable paging and fall back to direct stdout output when the specified pager program is not found or cannot be executed
- **FR-006**: System MUST detect when the pager process exits or terminates
- **FR-007**: System MUST terminate gracefully within 1 second when the pager exits with exit code 0
- **FR-007a**: System MUST ignore SIGPIPE signals when writing to the pager
- **FR-007b**: System MUST treat write errors to pager stdin (broken pipe) as a normal termination signal, not as an error condition
- **FR-007c**: System MUST exit with status code 0 when pager exits early and write operations fail with broken pipe
- **FR-008**: System MUST provide `--output` (or `-o`) command-line option to specify an output file
- **FR-008a**: System MUST disable paging when output is explicitly directed to a file via `--output` or `-o` option
- **FR-009**: System MUST not leave zombie processes when the pager exits
- **FR-010**: System MUST detect pager crashes or unexpected terminations, write the error message "hl: pager killed" to stderr, reset terminal state, and exit with code 141
- **FR-010a**: System MUST reset ANSI color/formatting codes (using ESC[0m or \x1bm) when recovering from pager crash
- **FR-010b**: System MUST restore terminal echo mode (using `stty echo`) when recovering from pager crash on TTY systems
- **FR-010c**: System MUST exit with code 141 when pager crashes or is killed unexpectedly, following SIGPIPE convention
- **FR-011**: System MUST stop all background processing (e.g., follow mode) when pager exits
- **FR-012**: System MUST preserve pager color/formatting capabilities by passing appropriate flags to the default pager (e.g., `-R` for less)
- **FR-013**: System MUST disable paging when follow mode is active, overriding user paging preferences (enabling paging in follow mode is out of scope and deferred to future work)

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can view log files exceeding 10,000 lines with smooth pager navigation
- **SC-001a**: Pager interface appears immediately (within 100ms) after application launch when paging is enabled
- **SC-002**: Application exits within 1 second of pager termination in 100% of normal cases
- **SC-003**: No zombie processes remain after application exit in follow mode with pager
- **SC-004**: Users can successfully pipe output to other tools by disabling paging
- **SC-005**: Application works with all common pager programs (less, most, fzf) without modification
- **SC-006**: Test coverage for pager-related code must not be less than the current test coverage on the target branch

## Assumptions

- Terminal detection relies on standard TTY checking mechanisms (POSIX on Unix-like systems, Windows Console API on Windows)
- Pager programs follow standard Unix conventions for input/output handling
- Users have appropriate permissions to execute pager programs
- The application checks HL_PAGER environment variable first, then falls back to PAGER, following standard Unix tool conventions
- Default pager (`less`) is available on target systems including Windows
- Pager process lifecycle can be monitored through standard process management APIs
- Follow mode currently operates without paging due to the continuous streaming nature of the output (enabling paging in follow mode is out of scope for this feature)
- Cross-platform compatibility is required for all Unix-compatible systems (Linux, macOS, BSD) and Windows

## Dependencies

- Access to standard pager programs (less, most, fzf, etc.) on the target system
- Terminal capability detection (isatty on Unix-like systems, Windows Console API on Windows)
- Process monitoring capabilities to detect pager exit on all supported platforms
- Environment variable access for custom pager configuration
- `less` pager availability on Windows systems
