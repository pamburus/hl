# Feature Specification: Pager Integration and Configuration

**Feature Branch**: `010-pager`
**Created**: 2025-01-15
**Status**: Draft
**Input**: User description: "Add feature describing how pager is handled, with configurable pager profiles, priority-based selection, and role-specific arguments for view and follow modes"

## Clarifications

### Session 2026-02-16

- Q: How should the pager configuration be structured to clearly distinguish profile references from environment variable references? → A: Use `pager.candidates` array with explicit `{ env = "VAR" }` and `{ profile = "name" }` objects. This provides clear structure and enables validation.
- Q: What should happen if both `env` and `profile` fields are specified in a candidate object? → A: Deserialize as externally-tagged enum - serde will reject configs where both fields are present with a clear error message.
- Q: Should the `pagers` section be moved under `pager` to group all pager-related configuration? → A: Yes, rename `pagers` to `pager.profiles` to clearly namespace all pager configuration under a single `[pager]` section.
- Q: What should the `pager` top-level option be renamed to for clarity? → A: Rename to `pager.candidates` - clearly indicates it's a search/fallback list and distinguishes it from the profiles themselves.
- Q: Should hard-coded environment variable checks (HL_PAGER, PAGER) be removed from code and made configurable? → A: Yes, move them into default config as `{ env = "..." }` entries in candidates. Users can reorder or remove these to customize precedence.
- Q: How should HL_FOLLOW_PAGER and HL_PAGER_DELIMITER be handled in the new structure? → A: Support both simple form `{ env = "PAGER" }` and structured form `{ env = { pager = "HL_PAGER", follow = "HL_FOLLOW_PAGER", delimiter = "HL_PAGER_DELIMITER" } }`. The structured form allows role-specific environment variables and delimiter override.
- Q: What happens when env.pager resolves to a profile reference (@profile)? → A: The entire candidate is resolved by that profile. Ignore env.follow and env.delimiter from the candidate to maintain profile coherence (delimiter must match pager command configuration).
- Q: What happens when env.pager resolves to a direct command (no @ prefix)? → A: Use the command for view mode. Check env.follow for follow mode (if omitted or unresolved, disable paging in follow mode). Check env.delimiter for delimiter override (if omitted or env var not found, use default newline).
- Q: Should env.delimiter override a profile's delimiter when a profile is selected? → A: No. Profiles are coherent units (command + args + delimiter). Breaking this coherence causes footguns. env.delimiter only applies to direct commands, never to profile references.
- Q: What is the migration path for existing configs? → A: Old `pager = ["fzf", "less"]` and `[pagers.X]` sections will be deprecated but supported temporarily. Document migration in changelog. New default config uses `pager.candidates` and `pager.profiles`.

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
- Q: What test coverage expectations should be documented in the Success Criteria? → A: Test coverage should not be less than the current test coverage on the target branch

### Session 2025-01-15

Note: Follow mode paging is now IN scope (not deferred), since this specification adds follow mode pager support via pager profiles and HL_FOLLOW_PAGER.

- Q: How should HL_PAGER command strings be executed - via shell or direct exec? → A: Direct exec with shell-style argument splitting (using `shellwords::split`), no shell features. This matches the existing implementation.
- Q: How should pager crash/kill be handled? → A: Preserve existing behavior: Print message on SIGKILL, restore terminal echo.
- Q: How should pager selection failures be reported? → A: Silent fallback with debug log messages (enabled via `HL_DEBUG_LOG` env var). If all pagers fail, output goes to stdout without error indication.
- Q: When should invalid profile references be validated? → A: Lazy validation at pager invocation time. Missing profiles treated same as missing commands (skip/fallback).
- Q: Should special handling for `less` (auto-add `-R`, set `LESSCHARSET=UTF-8`) be preserved with profiles? → A: No, profile's `command` is used as-is. Magic behavior only applies to env var fallback. Default config's `less` profile should include these settings explicitly.
- Q: Should SC-005 (error messages for config errors) be updated to match silent fallback behavior? → A: Yes, update SC-005 to emphasize graceful fallback rather than error messages.
- Q: How should `--paging=always` interact with follow mode when `follow.enabled` is not set? → A: Profile wins. `follow.enabled = false` (or not set) means no pager in follow mode regardless of `--paging=always`.
- Q: Should HL_PAGER command strings (not matching a profile) be used in follow mode? → A: No, maintain existing behavior where follow mode disables pager. Added `HL_FOLLOW_PAGER` env var to allow overriding pager specifically for follow mode.
- Q: Should `HL_PAGER=""` disable pager for both view and follow modes? → A: No. Empty `HL_PAGER` is treated as "not set" and the candidate is skipped, continuing to the next one. Use `-P` / `--paging never` to explicitly disable paging.
- Q: Should `HL_FOLLOW_PAGER` be able to override `HL_PAGER=""`? → A: No. `HL_PAGER` is the sole candidate-selection gate for the structured env candidate. When `HL_PAGER` is empty or not set, the entire candidate is skipped — `HL_FOLLOW_PAGER` has no effect in that case. `HL_FOLLOW_PAGER` is only consulted when `HL_PAGER` resolves to a direct command and the role is follow mode.

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

**Independent Test**: Can be tested by running the application with large input, closing the pager, and verifying the application exits with status code 0.

**Acceptance Scenarios**:

1. **Given** the application is running with output piped to a pager, **When** the user quits the pager normally, **Then** the application process terminates within 1 second with exit code 0
2. **Given** the application is in follow mode with a pager enabled via profile configuration, **When** the user quits the pager, **Then** the application stops monitoring and exits cleanly
3. **Given** the pager crashes or is killed unexpectedly, **When** the application detects the abnormal exit, **Then** the application writes the error message "hl: pager killed" to stderr, resets ANSI color/formatting codes, restores terminal echo mode, and exits with code 141
4. **Given** the user quits the pager while the application is still writing output, **When** the write to pager's stdin fails with broken pipe error, **Then** the application stops processing and exits gracefully with exit code 0 without logging an error

---

### User Story 4 - Custom Pager Selection via Environment Variables (Priority: P2)

Advanced users want to specify their preferred pager program (less, most, fzf, etc.) through environment variables, allowing them to use their customized paging environment or temporarily override configuration.

**Why this priority**: Environment variable overrides are essential for scripting and temporary changes. Also provides backward compatibility with Unix conventions.

**Independent Test**: Can be tested by setting the pager environment variable and verifying the specified pager is launched.

**Acceptance Scenarios**:

1. **Given** a config with `pager = "fzf"` and `HL_PAGER=less` set, **When** the user runs `hl logfile.log`, **Then** `less` is used as the pager (HL_PAGER takes precedence over config)
2. **Given** `HL_PAGER=most -w` set (a command string, not a profile name), **When** the user runs `hl logfile.log`, **Then** `most -w` is used as the pager command
3. **Given** `HL_PAGER=fzf` set (matching a profile name), **When** the user runs `hl logfile.log`, **Then** the `fzf` profile from config is used with all its configured arguments
4. **Given** the specified custom pager is not found, **When** attempting to page output, **Then** the error message "hl: unable to launch pager: <pager_name>: <system_error>" is written to stderr and paging is disabled, with output writing to stdout
5. **Given** `HL_PAGER` is set to an empty string, **When** the user runs `hl logfile.log`, **Then** the pager is disabled entirely (both view and follow modes)

---

### User Story 5 - Configure Preferred Pager via Configuration File (Priority: P1)

As a user, I want to configure my preferred pager in the configuration file so that logs are displayed through my chosen pager without needing to set environment variables each time.

**Why this priority**: This is the core configuration functionality that enables profile-based pager features. Without the ability to configure a pager, profile-based features cannot work.

**Independent Test**: Can be fully tested by creating a config file with a pager profile and verifying that running `hl` opens the specified pager.

**Acceptance Scenarios**:

1. **Given** a config file with `pager.candidates = [{ profile = "less" }]` and a `[[pager.profiles]]` entry with `name = "less"` defined, **When** the user runs `hl logfile.log`, **Then** the output is displayed through the `less` pager with the configured arguments.
2. **Given** a config file with `pager.candidates = [{ profile = "fzf" }]` and a `[[pager.profiles]]` entry with `name = "fzf"` defined, **When** the user runs `hl logfile.log`, **Then** the output is displayed through `fzf` with the configured arguments.
3. **Given** a config file with a pager profile referencing a command that doesn't exist on the system, **When** the user runs `hl logfile.log`, **Then** the output is displayed directly to stdout without a pager.

---

### User Story 6 - Priority-Based Pager Fallback (Priority: P1)

As a user, I want to specify multiple pager profiles in priority order so that if my preferred pager is not installed, the system automatically falls back to an available alternative.

**Why this priority**: Many users work across different systems where their preferred pager (e.g., fzf) may not be installed. This ensures a good experience regardless of system configuration.

**Independent Test**: Can be fully tested by configuring `pager = ["fzf", "less"]` on a system without fzf and verifying less is used.

**Acceptance Scenarios**:

1. **Given** a config with `pager.candidates = [{ profile = "fzf" }, { profile = "less" }]` and both pagers installed, **When** the user runs `hl logfile.log`, **Then** `fzf` is used as the pager.
2. **Given** a config with `pager.candidates = [{ profile = "fzf" }, { profile = "less" }]` and only `less` installed, **When** the user runs `hl logfile.log`, **Then** `less` is used as the pager.
3. **Given** a config with `pager.candidates = [{ profile = "fzf" }, { profile = "less" }]` and neither pager installed, **When** the user runs `hl logfile.log`, **Then** the output is displayed directly to stdout without a pager.

---

### User Story 7 - Role-Specific Pager Arguments (Priority: P2)

As a user, I want to configure different pager arguments for view mode versus follow mode so that each mode uses optimal settings for its use case.

**Why this priority**: Different modes have different requirements (e.g., follow mode may need streaming-friendly options), but the system works with a single configuration if this isn't implemented.

**Independent Test**: Can be fully tested by configuring view.args and follow.args differently and verifying the correct arguments are used in each mode.

**Acceptance Scenarios**:

1. **Given** a pager profile with `view.args = ["--layout=reverse-list"]`, **When** the user runs `hl logfile.log` (view mode), **Then** the pager is invoked with the base command plus `--layout=reverse-list`.
2. **Given** a pager profile with `follow.args = ["--tac", "--track"]`, **When** the user runs `hl --follow logfile.log`, **Then** the pager is invoked with the base command plus `--tac --track`.
3. **Given** a pager profile with `follow.enabled` not set or set to `false`, **When** the user runs `hl --follow logfile.log`, **Then** the pager is not used and output goes directly to stdout.
4. **Given** a pager profile with `follow.enabled = true` but no `follow.args`, **When** the user runs `hl --follow logfile.log`, **Then** the pager is invoked with only the base command (no additional args).

---

### User Story 8 - Follow Mode Pager Override (Priority: P2)

As a user, I want to override the pager specifically for follow mode using the `HL_FOLLOW_PAGER` environment variable so that I can use a different pager (or command) for live streaming without affecting view mode configuration.

**Why this priority**: This provides flexibility for advanced users who want different pagers for different modes via environment variables, complementing the profile-based configuration.

**Independent Test**: Can be fully tested by setting HL_FOLLOW_PAGER and verifying it's used only in follow mode.

**Acceptance Scenarios**:

1. **Given** `HL_PAGER=fzf` (direct command) and `HL_FOLLOW_PAGER=less` set, **When** the user runs `hl --follow logfile.log`, **Then** `less` is used as the follow-mode pager (HL_FOLLOW_PAGER overrides the follow-mode command for direct-command env candidates).
2. **Given** `HL_FOLLOW_PAGER=less` set, **When** the user runs `hl logfile.log` (view mode), **Then** `HL_FOLLOW_PAGER` is ignored and the normal pager selection applies.
3. **Given** `HL_FOLLOW_PAGER=fzf` set (matching a profile name), **When** the user runs `hl --follow logfile.log`, **Then** the `fzf` profile is used with `follow.args` (if defined) or base command.
4. **Given** `HL_FOLLOW_PAGER=""` (empty string) set, **When** the user runs `hl --follow logfile.log`, **Then** pager is disabled for follow mode (output to stdout).
5. **Given** a pager is being used in follow mode (e.g., `less` with `follow.enabled = true`), **When** the user closes the pager (e.g., presses 'q' in less), **Then** follow mode stops and the application exits gracefully.

---

### User Story 9 - Backward Compatibility with PAGER (Priority: P3)

As a user migrating from other tools, I want `hl` to respect the standard `PAGER` environment variable when `HL_PAGER` is not set and no config is present, so that it integrates with my existing workflow.

**Why this priority**: This maintains backward compatibility with the existing behavior and Unix conventions, but is lower priority than new features.

**Independent Test**: Can be fully tested by unsetting HL_PAGER, removing pager config, setting PAGER, and verifying it's used.

**Acceptance Scenarios**:

1. **Given** no `HL_PAGER` set, no `pager` in config, and `PAGER=most` set, **When** the user runs `hl logfile.log`, **Then** `most` is used as the pager.
2. **Given** `HL_PAGER=less` and `PAGER=most` both set, **When** the user runs `hl logfile.log`, **Then** `less` is used (HL_PAGER takes precedence).
3. **Given** config has `pager = "fzf"` and `PAGER=most` set (no HL_PAGER), **When** the user runs `hl logfile.log`, **Then** `fzf` profile is used (config takes precedence over PAGER).

---

### Edge Cases

- What happens when the pager crashes or is killed externally?
  - Application should detect the abnormal exit, write the error message "hl: pager killed" to stderr, reset ANSI color/formatting codes (ESC[0m), restore terminal echo mode (using `stty echo`), and exit with code 141

- How does the system handle terminal resize while paging?
  - Pager handles resize (application passes through terminal control)

- What happens when output is redirected but user forces paging?
  - Respects user's explicit choice (always page) over auto-detection

- What if the pager program doesn't support streaming input?
  - Application should detect and handle appropriately or document supported pagers

- How should SIGPIPE be handled when user quits pager early?
  - Ignore SIGPIPE signal, treat write errors (broken pipe) as normal termination condition, exit gracefully with status code 0

- What happens when `pager` references a non-existent profile name?
  - System should skip it and try the next profile in the list (log to debug). Validation is lazy (at invocation time, not config load time).

- What happens when `command` array is empty in a profile?
  - Profile should be treated as invalid and skipped (log to debug).

- What happens when the pager command exists but fails to start?
  - Log to debug and fall back to stdout silently.

- What happens when follow mode is used but `follow.enabled` is not set for the selected profile?
  - Pager should not be used in follow mode; output goes to stdout.

- What happens when `HL_PAGER` is set to an empty string?
  - Should be treated as "disable pager" (output to stdout).

- What happens when `HL_FOLLOW_PAGER` is set to an empty string?
  - Should be treated as "disable pager for follow mode" (output to stdout).

- What happens when all configured pagers are unavailable?
  - Output goes to stdout without any error indication (log to debug).

- What happens when `HL_FOLLOW_PAGER` is set but not in follow mode?
  - It is ignored; normal pager selection applies.

- What happens when the pager is closed by the user in follow mode?
  - Follow mode should stop and the application should exit gracefully.

## Requirements *(mandatory)*

### Functional Requirements

#### TTY Detection and Paging Modes

- **FR-001**: System MUST automatically detect when output is directed to a terminal (TTY) versus being redirected or piped
- **FR-002**: System MUST launch a pager automatically when stdout is a TTY, no output file is specified, and paging mode is "auto" (default)
- **FR-003**: System MUST provide a `--paging` command-line option accepting values `auto`, `always`, or `never` with `auto` as the default
- **FR-003a**: System MUST spawn the pager process immediately at application start when paging is enabled, before generating any output
- **FR-003b**: System MUST respect user-specified paging preferences through the `--paging` option (always, never, auto)
- **FR-004**: System MUST provide `--output` (or `-o`) command-line option to specify an output file
- **FR-004a**: System MUST disable paging when output is explicitly directed to a file via `--output` or `-o` option

#### Pager Execution and Security

- **FR-005**: System MUST execute the pager program directly from PATH without shell interpretation to prevent command injection vulnerabilities
- **FR-005a**: System MUST support shell-style quoting in environment variable values (e.g., `PAGER="less -X"`) using shellwords parsing without full shell execution
- **FR-005b**: System MUST validate that the specified pager executable exists and has execute permissions before attempting to spawn the process
- **FR-005c**: System MUST support pager integration on all Unix-compatible systems (Linux, macOS, BSD) and Windows
- **FR-005d**: System MUST use `less` as the default pager on all platforms (including Windows) when no custom pager is specified

#### Default Pager Behavior

- **FR-006**: System MUST pass `-R` flag and set `LESSCHARSET=UTF-8` environment variable when the default `less` pager is used (not overridden by environment variables or profiles)
- **FR-006a**: System MUST NOT add default arguments when a custom pager is specified via HL_PAGER or PAGER environment variables (as raw command strings, not matching a profile)

#### Configuration

- **FR-007**: System MUST support a `pager.candidates` configuration option that accepts an array of candidate objects, where each candidate is either:
  - `{ env = "VAR_NAME" }` - simple environment variable reference (view mode only)
  - `{ env = { pager = "VAR1", follow = "VAR2", delimiter = "VAR3" } }` - structured environment variable reference with role-specific vars (all fields optional)
  - `{ profile = "profile_name" }` - profile reference
- **FR-007a**: System MUST reject configuration if a candidate object contains both `env` and `profile` fields at the top level with a clear deserialization error message.
- **FR-007b**: For structured env candidates `{ env = { pager = "...", follow = "...", delimiter = "..." } }`:
  - If `pager` field is omitted or the environment variable is not set/empty, skip this candidate
  - If the candidate has `profiles = true` and `pager` resolves to a value starting with `@`, use that profile entirely (command, args, env, delimiter from profile) and IGNORE `follow` and `delimiter` fields from the candidate
  - If `pager` resolves to a direct command (no `@` prefix, or `profiles = false`), use it for view mode
  - If `follow` field is specified and the environment variable exists/non-empty, use it for follow mode; otherwise disable paging in follow mode
  - If `delimiter` field is specified and the environment variable exists/non-empty, use it as delimiter; otherwise use default `newline` (only applies to direct commands, not profile references)
- **FR-007c**: For simple env candidates `{ env = "VAR_NAME" }`:
  - Apply to view mode only; always skip in follow mode
  - Read the environment variable and apply same parsing logic as structured form's `pager` field
  - If the candidate has `profiles = true` and the value starts with `@`, treat it as a profile reference (see FR-014c); otherwise treat as a direct command
  - Use default `newline` delimiter
- **FR-007e**: Each env candidate MAY include a `profiles` boolean field (default: `false`). When `true`, values starting with `@` in the resolved environment variable are treated as explicit profile references (see FR-014c). When `false` (the default), the `@` character is treated literally as part of a command name.
- **FR-007d**: System MUST process candidates in array order, trying each until a usable pager is found.
- **FR-008**: System MUST support a `[pager.profiles]` configuration section containing named pager profiles.
- **FR-008a**: System SHOULD support legacy `pager` (string or array of strings) and `[pagers]` configuration for backward compatibility, with deprecation warning logged when `HL_DEBUG_LOG` is set.
- **FR-009**: Each pager profile MUST have a `name` property (string) that uniquely identifies it within the `pager.profiles` array.
- **FR-009a**: Each pager profile MUST support a `command` property (string) containing the pager executable name or path.
- **FR-009b**: Each pager profile MUST support an `args` property (array of strings) containing base arguments appended to the command for all modes.
- **FR-010**: Each pager profile MUST support optional `env` property containing a map of environment variables to set when invoking the pager.
- **FR-011**: Each pager profile MUST support optional `modes.view.args` property containing additional arguments for view mode.
- **FR-012**: Each pager profile MUST support optional `modes.follow.enabled` property (boolean) to enable pager usage in follow mode.
- **FR-013**: Each pager profile MUST support optional `modes.follow.args` property containing additional arguments for follow mode.
- **FR-013a**: Each pager profile MUST support optional `delimiter` property (`"newline"` or `"nul"`) specifying the output entry delimiter when this pager is used.
- **FR-013b**: Each pager profile MUST support optional `conditions` array, where each entry has an `if` condition string and optional `args` (array of strings) and `env` (map) fields applied when the condition matches. Supported condition strings: `os:macos`, `os:linux`, `os:windows`, `os:unix`, `mode:view`, `mode:follow`, with optional `!` negation prefix. Multiple matching conditions are applied in order.

#### Candidate-Based Selection

- **FR-014**: System MUST process `pager.candidates` array in order, trying each candidate until a usable pager is found.
- **FR-014a**: For `{ profile = "name" }` candidates, system MUST select the profile if it exists in `pager.profiles` and its command executable is available in PATH. Selection is mode-agnostic: the first available candidate wins regardless of the current mode. Mode-specific behaviour (whether paging is enabled, which arguments to use) is determined after selection.
- **FR-014b**: For env candidates (both simple and structured forms), system MUST apply shell-style argument splitting using `shellwords::split` when parsing environment variable values.
- **FR-014c**: For env candidates with `profiles = true`, if the value starts with `@` (e.g., `@less`), system MUST treat the remainder as an explicit profile reference. If the profile does not exist or the executable is not available, system MUST display an error message and exit with non-zero status (no fallback to other candidates). When `profiles = false` (default), `@` is treated as a literal character in the command name.
- **FR-014d**: For env candidates, if the value does not start with `@`, system MUST treat it as a direct command. If the command is not available, system MUST display an error message and exit with non-zero status (no fallback to other candidates).
- **FR-015**: System MUST check if a pager command is available by searching for the executable in the system PATH.
- **FR-016**: System MUST skip candidates whose referenced profile doesn't exist or whose command executable is not found, continuing to the next candidate.
- **FR-017**: If no candidate yields a usable pager, system MUST fall back to outputting directly to stdout without error indication.
- **FR-017a**: System SHOULD log pager selection decisions and failures to debug logs when `HL_DEBUG_LOG` environment variable is set.

#### Role-Based Arguments

- **FR-018**: When in view mode (non-follow), system MUST use the pager with base `command` plus `view.args` (if defined), and set environment variables from `env` (if defined).
- **FR-019**: When in follow mode and `follow.enabled = true` for the selected profile, system MUST use the pager with base `command` plus `follow.args` (if defined), and set environment variables from `env` (if defined).
- **FR-020**: When in follow mode and the selected profile has `follow.enabled` not set or `false`, system MUST disable paging and output directly to stdout. The selector MUST NOT continue evaluating lower-priority candidates — the selected profile's follow configuration is final. This applies regardless of the `--paging=always` CLI flag.

#### Follow Mode Pager Behavior

- **FR-021**: When using a pager in follow mode and the pager process is closed (e.g., user presses 'q' in less, or the pager process terminates), system MUST stop follow mode and exit the application gracefully.
- **FR-021a**: System MUST detect pager closure by monitoring the pager's stdin pipe (write failure indicates pager has closed).

#### Environment Variable Handling

- **FR-022**: When using an environment variable as a direct command string (not a profile reference), system MUST invoke the command as-is with no implicit argument injection or environment variable modification.
- **FR-023**: When a simple env candidate `{ env = "VAR" }` is evaluated in follow mode, system MUST skip it and continue to the next candidate.
- **FR-024**: When using a structured env candidate's `pager` field with a direct command (not `@profile`) in follow mode:
  - If `follow` field is specified and resolves to a value, use it for follow mode
  - If `follow` field is not specified or does not resolve, disable paging in follow mode
- **FR-025**: The default configuration SHOULD include `{ env = { pager = "HL_PAGER", follow = "HL_FOLLOW_PAGER", delimiter = "HL_PAGER_DELIMITER" }, profiles = true }` as the first candidate, followed by profile candidates, and finally `{ env = "PAGER" }` (with `profiles = false`, the default) to maintain backward compatibility. The `PAGER` candidate is intentionally `profiles = false` because it is a universal system variable shared with other programs that are unaware of `hl` profile names.
- **FR-026**: If an environment variable referenced in candidates is set to an empty string, system MUST treat it as "not set" and:
  - For simple form: skip to next candidate (best-effort fallback)
  - For structured form: treat that specific field as unset (e.g., empty `follow` disables follow mode paging for this candidate; empty `pager` skips the candidate)
- **FR-027**: Config file `pager.candidates` uses best-effort fallback for profile references: if a candidate's profile doesn't exist or executable is not available, system MUST try the next candidate. If none succeed, system MUST fall back to stdout (no error).
- **FR-028**: For env candidates that resolve to direct commands or profile references with `@` prefix: if the command/profile is not available, system MUST display an error message and exit with non-zero status (no fallback to next candidate).

#### Precedence

- **FR-029**: Pager selection for view mode MUST follow this precedence order (highest to lowest):
  1. `--paging=never` / `-P` CLI flag (disables pager)
  2. `pager.candidates` config option processed in array order (best-effort fallback for profile references, exits on error for env var references with `@` prefix or unavailable direct commands)
  3. No pager (stdout)

- **FR-029a**: Pager selection for follow mode MUST follow this precedence order (highest to lowest):
  1. `--paging=never` / `-P` CLI flag (disables pager)
  2. `pager.candidates` config option processed in array order (same semantics as view mode, with follow-mode specific handling per FR-023 and FR-024)
  3. No pager (stdout)

- **FR-029b**: The default `pager.candidates` SHOULD be ordered as: `[{ env = { pager = "HL_PAGER", follow = "HL_FOLLOW_PAGER", delimiter = "HL_PAGER_DELIMITER" }, profiles = true }, { profile = "fzf" }, { profile = "less" }, { env = "PAGER" }]` to maintain backward compatibility while allowing users to customize order.

#### Pager Exit Code Handling

- **FR-034**: When the pager process exits with a non-zero exit code, system MUST exit with status code 141 (following Unix SIGPIPE convention and matching behavior of `git`, `bat`, and other tools).
- **FR-035**: When the pager process exits successfully (exit code 0), system MUST exit with status code 0.
- **FR-036**: When writing to the pager fails with a broken pipe error (pager closed stdin prematurely), system MUST exit with status code 141.

#### Pager Lifecycle and Error Handling

- **FR-037**: System MUST detect when the pager process exits or terminates.
- **FR-038**: System MUST terminate gracefully within 1 second when the pager exits with exit code 0.
- **FR-038a**: System MUST ignore SIGPIPE signals when writing to the pager.
- **FR-038b**: System MUST treat write errors to pager stdin (broken pipe) as a normal termination signal, not as an error condition.
- **FR-038c**: System MUST exit with status code 0 when pager exits early and write operations fail with broken pipe.
- **FR-039**: System MUST not leave zombie processes when the pager exits.
- **FR-040**: System MUST detect pager crashes or unexpected terminations, write the error message "hl: pager killed" to stderr, reset terminal state, and exit with code 141.
- **FR-040a**: System MUST reset ANSI color/formatting codes (using ESC[0m or \x1bm) when recovering from pager crash.
- **FR-040b**: System MUST restore terminal echo mode (using `stty echo`) when recovering from pager crash on TTY systems.
- **FR-040c**: System MUST exit with code 141 when pager crashes or is killed unexpectedly, following SIGPIPE convention.
- **FR-041**: System MUST stop all background processing (e.g., follow mode) when pager exits.
- **FR-042**: System MUST write an error message to stderr in the format "hl: unable to launch pager: <pager_name>: <system_error_message>" when the specified pager program is not found or cannot be executed.
- **FR-042a**: System MUST disable paging and fall back to direct stdout output when the specified pager program is not found or cannot be executed.

### Key Entities

- **Pager Profile**: A named configuration containing a base command, optional base arguments, environment variables, conditional argument sets, and role-specific arguments. Defined as an entry in the `[[pager.profiles]]` array, identified by its `name` field.
- **Pager Role**: The context in which a pager is used, either "view" (standard log viewing) or "follow" (live log streaming with `--follow` flag).
- **Pager Command**: An array of strings where the first element is the executable and subsequent elements are arguments.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can view log files exceeding 10,000 lines with smooth pager navigation
- **SC-001a**: Pager interface appears immediately (within 100ms) after application launch when paging is enabled
- **SC-002**: Application exits within 1 second of pager termination in 100% of normal cases
- **SC-003**: No zombie processes remain after application exit in follow mode with pager
- **SC-004**: Users can successfully pipe output to other tools by disabling paging
- **SC-005**: Application works with all common pager programs (less, most, fzf) without modification
- **SC-006**: Test coverage for pager-related code must not be less than the current test coverage on the target branch
- **SC-007**: Users can configure a pager profile and have it work on first use without additional setup
- **SC-008**: Users with multiple systems can use a single config file, and the system automatically selects an available pager on each system
- **SC-009**: Users can use fzf or similar interactive pagers in both view and follow modes with appropriate settings for each mode
- **SC-010**: Existing users relying on `HL_PAGER` or `PAGER` environment variables experience no change in behavior
- **SC-011**: Configuration issues (invalid profile names, missing commands) are handled gracefully with silent fallback to the next available option or stdout, with details available via debug logging

## Assumptions

- Terminal detection relies on standard TTY checking mechanisms (POSIX on Unix-like systems, Windows Console API on Windows)
- Pager programs follow standard Unix conventions for input/output handling
- Users have appropriate permissions to execute pager programs
- Pager selection order is driven by the `pager.candidates` array in config; the default config places the `HL_PAGER`/`HL_FOLLOW_PAGER` env candidate first, followed by profile candidates, then the `PAGER` env candidate, maintaining backward compatibility while allowing customization
- Default pager (`less`) is available on target systems including Windows
- Pager process lifecycle can be monitored through standard process management APIs
- Cross-platform compatibility is required for all Unix-compatible systems (Linux, macOS, BSD) and Windows
- Follow mode paging is supported via pager profiles with `follow.enabled = true` and via the `HL_FOLLOW_PAGER` environment variable

## Dependencies

- Access to standard pager programs (less, most, fzf, etc.) on the target system
- Terminal capability detection (isatty on Unix-like systems, Windows Console API on Windows)
- Process monitoring capabilities to detect pager exit on all supported platforms
- Environment variable access for custom pager configuration
- `less` pager availability on Windows systems
- Configuration file parsing for pager profiles
