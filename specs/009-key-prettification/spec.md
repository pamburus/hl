# Feature Specification: Key Prettification Config Option

**Feature Branch**: `009-key-prettification`
**Created**: 2026-02-04
**Status**: Draft
**Input**: User description: "Add an option to the config file for turning off the automatic replacement of underscores with hyphens in the field keys when formatting the output."

## Clarifications

### Session 2026-02-04

- Q: Should the option be exposed as CLI flags in addition to the config file? → A: No. Config-only. This is a set-once personal preference that does not need per-invocation override.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Preserve Original Field Keys via Config (Priority: P1)

A user works with log sources where field keys use underscores (e.g., `request_id`, `user_name`, `trace_id`). Currently, these are always displayed as `request-id`, `user-name`, `trace-id` in the formatted output. The user wants to see the original underscore-based keys because that matches their log schema and makes it easier to correlate fields with their source systems. The user sets a persistent configuration option to disable this replacement.

**Why this priority**: This is the core value of the feature. Without a config option, users must either accept the automatic replacement or use `--raw-fields`, which also disables field value unescaping — an overly broad workaround.

**Independent Test**: Can be fully tested by setting the configuration option and verifying that field keys with underscores appear unchanged in the formatted output, while all other formatting (value unescaping, nested key flattening) continues to work normally.

**Acceptance Scenarios**:

1. **Given** a config file with the underscore-to-hyphen replacement disabled, **When** the user processes a log entry with field key `request_id`, **Then** the output displays the key as `request_id`.
2. **Given** a config file with the underscore-to-hyphen replacement disabled, **When** the user processes a log entry with nested field keys like `http.response_code`, **Then** the output preserves underscores in all key segments (e.g., `http.response_code`, not `http.response-code`).
3. **Given** a config file without this option set (default behavior), **When** the user processes a log entry with field key `request_id`, **Then** the output displays the key as `request-id` (current behavior preserved).

---

### Edge Cases

- What happens when `--raw-fields` is used together with the new option? The `--raw-fields` flag disables all field formatting including value unescaping. It should take precedence: when `--raw-fields` is active, the key prettification option has no effect since raw output bypasses all formatting.
- What happens with field keys that contain no underscores? No change in behavior; keys without underscores are unaffected by this option.
- What happens with field keys that are entirely underscores (e.g., `___`)? When replacement is enabled, the key becomes `---`. When disabled, it remains `___`.
- What happens with nested/flattened keys where the dot-separated prefix contains underscores? The option applies to all key segments uniformly. For example, `parent_key.child_key` preserves underscores in both segments when replacement is disabled.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST provide a configuration file option to disable the automatic replacement of underscores with hyphens in field keys during output formatting.
- **FR-002**: The default value of this option MUST preserve the current behavior (underscores replaced with hyphens).
- **FR-003**: When the option is set to disable replacement, the system MUST preserve underscores in all field key segments, including nested and flattened key prefixes.
- **FR-004**: This option MUST be independent of `--raw-fields`. Disabling key prettification MUST NOT affect field value unescaping or any other formatting behavior.
- **FR-005**: When both `--raw-fields` and this option are active, `--raw-fields` MUST take precedence (all formatting bypassed).

### Out of Scope

- CLI flags for this option. The option is config-file only; no per-invocation override is needed.

### Key Entities

- **Formatting Configuration**: The set of user-configurable formatting preferences, extended with the new key prettification option.
- **Field Key**: A string identifier for a log record field, which may contain underscores and may be nested using dot-separated segments.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can disable underscore-to-hyphen replacement via a config file option and see original field keys preserved in 100% of formatted output.
- **SC-002**: Existing default behavior (underscores replaced with hyphens) is preserved for all users who do not set the new option.
- **SC-003**: No other formatting behaviors (value unescaping, key flattening, punctuation) are affected when only this option is changed.

## Assumptions

- The config file format is TOML, consistent with the existing configuration system.
- The option fits naturally within the existing formatting section of the config file.
- The naming convention for the option follows the existing kebab-case pattern used in the config file.
