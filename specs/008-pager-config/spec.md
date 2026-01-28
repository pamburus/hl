# Feature Specification: Pager Configuration System

**Feature Branch**: `008-pager-config`  
**Created**: 2025-01-15  
**Status**: Draft  
**Input**: User description: "Configurable pager profiles with priority-based selection and role-specific arguments for view and follow modes"

## Clarifications

### Session 2025-01-15

- Q: How should HL_PAGER command strings be executed - via shell or direct exec? → A: Direct exec with shell-style argument splitting (using `shellwords::split`), no shell features. This matches the existing implementation.
- Q: How should pager crash/kill be handled? → A: Preserve existing behavior: Print message on SIGKILL, restore terminal echo.
- Q: How should pager selection failures be reported? → A: Silent fallback with debug log messages (enabled via `HL_DEBUG_LOG` env var). If all pagers fail, output goes to stdout without error indication.
- Q: When should invalid profile references be validated? → A: Lazy validation at pager invocation time. Missing profiles treated same as missing commands (skip/fallback).
- Q: Should special handling for `less` (auto-add `-R`, set `LESSCHARSET=UTF-8`) be preserved with profiles? → A: No, profile's `command` is used as-is. Magic behavior only applies to env var fallback. Default config's `less` profile should include these settings explicitly.
- Q: Should SC-005 (error messages for config errors) be updated to match silent fallback behavior? → A: Yes, update SC-005 to emphasize graceful fallback rather than error messages.
- Q: How should `--paging=always` interact with follow mode when `follow.enabled` is not set? → A: Profile wins. `follow.enabled = false` (or not set) means no pager in follow mode regardless of `--paging=always`.
- Q: Should HL_PAGER command strings (not matching a profile) be used in follow mode? → A: No, maintain existing behavior where follow mode disables pager. Added `HL_FOLLOW_PAGER` env var to allow overriding pager specifically for follow mode.
- Q: Should `HL_PAGER=""` disable pager for both view and follow modes? → A: Yes, `HL_PAGER=""` disables pager entirely (both modes). This is a behavior change from current implementation where empty HL_PAGER falls back to default `less`.
- Q: Should `HL_FOLLOW_PAGER` be able to override `HL_PAGER=""`? → A: Yes, `HL_FOLLOW_PAGER` can override `HL_PAGER=""` specifically for follow mode, giving users more flexibility.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Configure Preferred Pager (Priority: P1)

As a user, I want to configure my preferred pager in the configuration file so that logs are displayed through my chosen pager without needing to set environment variables each time.

**Why this priority**: This is the core functionality that enables all other pager features. Without the ability to configure a pager, no other pager-related features can work.

**Independent Test**: Can be fully tested by creating a config file with a pager profile and verifying that running `hl` opens the specified pager.

**Acceptance Scenarios**:

1. **Given** a config file with `pager = "less"` and a `[pagers.less]` profile defined, **When** the user runs `hl logfile.log`, **Then** the output is displayed through the `less` pager with the configured arguments.
2. **Given** a config file with `pager = "fzf"` and a `[pagers.fzf]` profile defined, **When** the user runs `hl logfile.log`, **Then** the output is displayed through `fzf` with the configured arguments.
3. **Given** a config file with a pager profile referencing a command that doesn't exist on the system, **When** the user runs `hl logfile.log`, **Then** the output is displayed directly to stdout without a pager.

---

### User Story 2 - Priority-Based Pager Fallback (Priority: P1)

As a user, I want to specify multiple pager profiles in priority order so that if my preferred pager is not installed, the system automatically falls back to an available alternative.

**Why this priority**: Many users work across different systems where their preferred pager (e.g., fzf) may not be installed. This ensures a good experience regardless of system configuration.

**Independent Test**: Can be fully tested by configuring `pager = ["fzf", "less"]` on a system without fzf and verifying less is used.

**Acceptance Scenarios**:

1. **Given** a config with `pager = ["fzf", "less"]` and both pagers installed, **When** the user runs `hl logfile.log`, **Then** `fzf` is used as the pager.
2. **Given** a config with `pager = ["fzf", "less"]` and only `less` installed, **When** the user runs `hl logfile.log`, **Then** `less` is used as the pager.
3. **Given** a config with `pager = ["fzf", "less"]` and neither pager installed, **When** the user runs `hl logfile.log`, **Then** the output is displayed directly to stdout without a pager.

---

### User Story 3 - Role-Specific Pager Arguments (Priority: P2)

As a user, I want to configure different pager arguments for view mode versus follow mode so that each mode uses optimal settings for its use case.

**Why this priority**: Different modes have different requirements (e.g., follow mode may need streaming-friendly options), but the system works with a single configuration if this isn't implemented.

**Independent Test**: Can be fully tested by configuring view.args and follow.args differently and verifying the correct arguments are used in each mode.

**Acceptance Scenarios**:

1. **Given** a pager profile with `view.args = ["--layout=reverse-list"]`, **When** the user runs `hl logfile.log` (view mode), **Then** the pager is invoked with the base command plus `--layout=reverse-list`.
2. **Given** a pager profile with `follow.args = ["--tac", "--track"]`, **When** the user runs `hl --follow logfile.log`, **Then** the pager is invoked with the base command plus `--tac --track`.
3. **Given** a pager profile with `follow.enabled` not set or set to `false`, **When** the user runs `hl --follow logfile.log`, **Then** the pager is not used and output goes directly to stdout.
4. **Given** a pager profile with `follow.enabled = true` but no `follow.args`, **When** the user runs `hl --follow logfile.log`, **Then** the pager is invoked with only the base command (no additional args).

---

### User Story 4 - Environment Variable Override (Priority: P2)

As a user, I want to override the configured pager using the `HL_PAGER` environment variable so that I can temporarily use a different pager without modifying my configuration.

**Why this priority**: Environment variable overrides are essential for scripting and temporary changes, but the config-based system works without this.

**Independent Test**: Can be fully tested by setting HL_PAGER and verifying it takes precedence over config file settings.

**Acceptance Scenarios**:

1. **Given** a config with `pager = "fzf"` and `HL_PAGER=less` set, **When** the user runs `hl logfile.log`, **Then** `less` is used as the pager.
2. **Given** `HL_PAGER=most -w` set (a command string, not a profile name), **When** the user runs `hl logfile.log`, **Then** `most -w` is used as the pager command.
3. **Given** `HL_PAGER=fzf` set (matching a profile name), **When** the user runs `hl logfile.log`, **Then** the `fzf` profile from config is used with all its configured arguments.

---

### User Story 5 - Follow Mode Pager Override (Priority: P2)

As a user, I want to override the pager specifically for follow mode using the `HL_FOLLOW_PAGER` environment variable so that I can use a different pager (or command) for live streaming without affecting view mode configuration.

**Why this priority**: This provides flexibility for advanced users who want different pagers for different modes via environment variables, complementing the profile-based configuration.

**Independent Test**: Can be fully tested by setting HL_FOLLOW_PAGER and verifying it's used only in follow mode.

**Acceptance Scenarios**:

1. **Given** `HL_FOLLOW_PAGER=less` set and a config with `pager = "fzf"` (with `follow.enabled = true`), **When** the user runs `hl --follow logfile.log`, **Then** `less` is used as the pager (not fzf).
2. **Given** `HL_FOLLOW_PAGER=less` set, **When** the user runs `hl logfile.log` (view mode), **Then** `HL_FOLLOW_PAGER` is ignored and the normal pager selection applies.
3. **Given** `HL_FOLLOW_PAGER=fzf` set (matching a profile name), **When** the user runs `hl --follow logfile.log`, **Then** the `fzf` profile is used with `follow.args` (if defined) or base command.
4. **Given** `HL_FOLLOW_PAGER=""` (empty string) set, **When** the user runs `hl --follow logfile.log`, **Then** pager is disabled for follow mode (output to stdout).
5. **Given** a pager is being used in follow mode (e.g., `less` with `follow.enabled = true`), **When** the user closes the pager (e.g., presses 'q' in less), **Then** follow mode stops and the application exits gracefully.

---

### User Story 6 - Backward Compatibility with PAGER (Priority: P3)

As a user migrating from other tools, I want `hl` to respect the standard `PAGER` environment variable when `HL_PAGER` is not set and no config is present, so that it integrates with my existing workflow.

**Why this priority**: This maintains backward compatibility with the existing behavior and Unix conventions, but is lower priority than new features.

**Independent Test**: Can be fully tested by unsetting HL_PAGER, removing pager config, setting PAGER, and verifying it's used.

**Acceptance Scenarios**:

1. **Given** no `HL_PAGER` set, no `pager` in config, and `PAGER=most` set, **When** the user runs `hl logfile.log`, **Then** `most` is used as the pager.
2. **Given** `HL_PAGER=less` and `PAGER=most` both set, **When** the user runs `hl logfile.log`, **Then** `less` is used (HL_PAGER takes precedence).
3. **Given** config has `pager = "fzf"` and `PAGER=most` set (no HL_PAGER), **When** the user runs `hl logfile.log`, **Then** `fzf` profile is used (config takes precedence over PAGER).

---

### Edge Cases

- What happens when `pager` references a non-existent profile name? → System should skip it and try the next profile in the list (log to debug). Validation is lazy (at invocation time, not config load time).
- What happens when `command` array is empty in a profile? → Profile should be treated as invalid and skipped (log to debug).
- What happens when the pager command exists but fails to start? → Log to debug and fall back to stdout silently.
- What happens when follow mode is used but `follow.enabled` is not set for the selected profile? → Pager should not be used in follow mode; output goes to stdout.
- What happens when `HL_PAGER` is set to an empty string? → Should be treated as "disable pager" (output to stdout).
- What happens when `HL_FOLLOW_PAGER` is set to an empty string? → Should be treated as "disable pager for follow mode" (output to stdout).
- What happens when all configured pagers are unavailable? → Output goes to stdout without any error indication (log to debug).
- What happens when `HL_FOLLOW_PAGER` is set but not in follow mode? → It is ignored; normal pager selection applies.
- What happens when the pager is closed by the user in follow mode? → Follow mode should stop and the application should exit gracefully.

## Requirements *(mandatory)*

### Functional Requirements

#### Configuration

- **FR-001**: System MUST support a `pager` configuration option that accepts either a single profile name (string) or a list of profile names (array of strings).
- **FR-002**: System MUST support a `[pagers]` configuration section containing named pager profiles.
- **FR-003**: Each pager profile MUST support a `command` property containing an array of strings representing the pager command and its base arguments.
- **FR-004**: Each pager profile MUST support optional `env` property containing a map of environment variables to set when invoking the pager.
- **FR-005**: Each pager profile MUST support optional `view.args` property containing additional arguments for view mode.
- **FR-006**: Each pager profile MUST support optional `follow.enabled` property (boolean) to enable pager usage in follow mode.
- **FR-007**: Each pager profile MUST support optional `follow.args` property containing additional arguments for follow mode.

#### Priority-Based Selection

- **FR-008**: When `pager` is a list, system MUST try each profile in order until one with an available command is found.
- **FR-009**: System MUST check if a pager command is available by searching for the executable in the system PATH.
- **FR-010**: System MUST skip profiles whose command executable is not found and continue to the next profile.
- **FR-011**: If no profile has an available command, system MUST fall back to outputting directly to stdout without error indication.
- **FR-011a**: System SHOULD log pager selection decisions and failures to debug logs when `HL_DEBUG_LOG` environment variable is set.

#### Role-Based Arguments

- **FR-012**: When in view mode (non-follow), system MUST use the pager with base `command` plus `view.args` (if defined), and set environment variables from `env` (if defined).
- **FR-013**: When in follow mode and `follow.enabled = true` for the selected profile, system MUST use the pager with base `command` plus `follow.args` (if defined), and set environment variables from `env` (if defined).
- **FR-014**: When in follow mode and `follow.enabled` is not set or is `false`, system MUST NOT use a pager and output directly to stdout, regardless of `--paging=always` CLI flag.

#### Follow Mode Pager Behavior

- **FR-014a**: When using a pager in follow mode and the pager process is closed (e.g., user presses 'q' in less, or the pager process terminates), system MUST stop follow mode and exit the application gracefully.
- **FR-014b**: System MUST detect pager closure by monitoring the pager's stdin pipe (write failure indicates pager has closed).

#### Environment Variable Handling

- **FR-015**: System MUST check `HL_PAGER` environment variable before using config file settings.
- **FR-016**: If `HL_PAGER` value matches a defined profile name, system MUST use that profile with all its configured arguments and environment variables.
- **FR-017**: If `HL_PAGER` value does not match any profile name, system MUST parse it using shell-style argument splitting (e.g., `shellwords::split`) and execute directly without invoking a shell (for backward compatibility and security).
- **FR-017a**: When using HL_PAGER or PAGER as a command string (not a profile), system MUST apply special handling for `less`: automatically add `-R` flag and set `LESSCHARSET=UTF-8`.
- **FR-017b**: When using HL_PAGER or PAGER as a command string (not a profile) in follow mode, system MUST NOT use a pager and output directly to stdout (unless overridden by HL_FOLLOW_PAGER).
- **FR-017c**: In follow mode, system MUST check `HL_FOLLOW_PAGER` environment variable before other pager settings.
- **FR-017d**: If `HL_FOLLOW_PAGER` value matches a defined profile name, system MUST use that profile with base `command` plus `follow.args` (if defined).
- **FR-017e**: If `HL_FOLLOW_PAGER` value does not match any profile name, system MUST treat it as a command string (same parsing as HL_PAGER, including special `less` handling).
- **FR-017f**: If `HL_FOLLOW_PAGER` is set to an empty string, system MUST disable pager usage for follow mode.
- **FR-018**: System MUST check `PAGER` environment variable only when both `HL_PAGER` is not set and no `pager` config option is defined.
- **FR-019**: If `HL_PAGER` is set to an empty string, system MUST disable pager usage entirely (both view and follow modes). Note: This is a behavior change from the current implementation.

#### Precedence

- **FR-020**: Pager selection for view mode MUST follow this precedence order (highest to lowest):
  1. `--paging=never` / `-P` CLI flag (disables pager)
  2. `HL_PAGER` environment variable
  3. `pager` config file option
  4. `PAGER` environment variable
  5. No pager (stdout)

- **FR-020a**: Pager selection for follow mode MUST follow this precedence order (highest to lowest):
  1. `--paging=never` / `-P` CLI flag (disables pager)
  2. `HL_FOLLOW_PAGER` environment variable (can override `HL_PAGER=""`)
  3. `HL_PAGER` set to empty string (disables pager)
  4. `HL_PAGER` environment variable (only if it matches a profile with `follow.enabled = true`)
  5. `pager` config file option (only if profile has `follow.enabled = true`)
  6. No pager (stdout)

### Key Entities

- **Pager Profile**: A named configuration containing a base command and optional role-specific arguments. Identified by its name in the `[pagers.<name>]` config section.
- **Pager Role**: The context in which a pager is used, either "view" (standard log viewing) or "follow" (live log streaming with `--follow` flag).
- **Pager Command**: An array of strings where the first element is the executable and subsequent elements are arguments.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can configure a pager profile and have it work on first use without additional setup.
- **SC-002**: Users with multiple systems can use a single config file, and the system automatically selects an available pager on each system.
- **SC-003**: Users can use fzf or similar interactive pagers in both view and follow modes with appropriate settings for each mode.
- **SC-004**: Existing users relying on `HL_PAGER` or `PAGER` environment variables experience no change in behavior.
- **SC-005**: Configuration issues (invalid profile names, missing commands) are handled gracefully with silent fallback to the next available option or stdout, with details available via debug logging.