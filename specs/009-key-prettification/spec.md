# Feature Specification: Field Key Display

**Feature Branch**: `009-key-prettification`
**Created**: 2026-02-04
**Updated**: 2026-10-21 (issue #1609: quoting of ambiguous keys)
**Status**: Draft
**Input**: User description: "Add an option to the config file for turning off the automatic replacement of underscores with hyphens in the field keys when formatting the output."

**Amendment input** (issue #1609): "In the default output, field keys are printed as is, even when they contain spaces, `=` or other special characters, or are empty. This makes the output ambiguous. A key is quoted when it is empty or contains characters that make the output ambiguous, such as spaces, `=`, quotes or control characters. Keys use the same quoting style as values, including the choice of quote character. For nested fields, each path segment is quoted on its own (`n.\"inner key\"`). Plain keys stay unquoted, so typical output does not change."

## Clarifications

### Session 2026-02-04

- Q: Should the option be exposed as CLI flags in addition to the config file? → A: No. Config-only. This is a set-once personal preference that does not need per-invocation override.

### Session 2026-10-21

- Q: Should key quoting be a new feature spec? → A: No. It fixes how field keys are displayed, so it amends this spec.
- Q: Does key quoting apply with `--raw-fields`? → A: No. FR-005 keeps applying: `--raw-fields` displays keys as is.

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

### User Story 2 - Read Fields Whose Keys Contain Spaces or Separators (Priority: P1)

A user views logs whose records have field keys with spaces, `=`, quotes or other special characters, or an empty key. Before this fix such keys were printed as is, so the user could not tell where a key ends and its value starts (`a=b=x`, `user name="alice smith"`, `=empty`). Each such key is now quoted, so every field reads as exactly one key and one value.

**Why this priority**: This is the bug reported in issue #1609. Ambiguous output can make a user misread which value belongs to which key.

**Independent Test**: Pipe the record from the issue through `hl -P` and compare the output with the expected line.

**Acceptance Scenarios**:

1. **Given** a record with the field `"user name": "alice smith"`, **When** it is displayed, **Then** the field reads `"user name"="alice smith"`.
2. **Given** a record with the field `"a=b": "x"`, **When** it is displayed, **Then** the field reads `"a=b"=x`.
3. **Given** a record with an empty key, `"": "empty"`, **When** it is displayed, **Then** the field reads `""=empty`.
4. **Given** the record `{"ts":"2026-10-21T10:00:00Z","level":"info","msg":"hello","user name":"alice smith","a=b":"x","":"empty","n":{"inner key":3}}`, **When** it is displayed with `hl -P`, **Then** the output is `2026-10-21 10:00:00.000 [INF] hello › "user name"="alice smith" "a=b"=x ""=empty n."inner key"=3`.

---

### User Story 3 - Plain Keys Stay Unquoted (Priority: P1)

A user views ordinary logs whose keys are plain identifiers such as `user_id`, `http.method`, `request-id` or `status`. The output is exactly the same as before the fix.

**Why this priority**: Almost all real keys are plain. Quoting them would add clutter and break expectations about `hl` output.

**Independent Test**: Run the existing output tests and sample logs, and check that only lines with special keys differ from the previous release.

**Acceptance Scenarios**:

1. **Given** a record whose keys contain only letters, digits, and printable non-space characters other than those listed in FR-006 (such as `_`, `-`, `.`, `:`, `/`, `@`), **When** it is displayed, **Then** no key is quoted.
2. **Given** key prettification is on (the default), **When** a record with the key `user_id` is displayed, **Then** the key reads `user-id`, unquoted, as before.
3. **Given** a key that looks like a number or a literal, such as `123`, `true` or `null`, **When** it is displayed, **Then** it is not quoted, because in key position it cannot be mistaken for a value.

---

### User Story 4 - Nested Keys Quote Each Path Segment (Priority: P2)

A user views records with nested objects whose keys contain special characters. With flattening, the dotted path quotes only the segments that need it. Without flattening, the keys inside the braces are quoted the same way.

**Why this priority**: Nested fields are common in structured logs, and the issue covers them explicitly. They come after flat fields, which are the main case.

**Independent Test**: Display `{"n":{"inner key":3}}` and `{"outer key":{"x":1}}` with flattening on and off, and check the output.

**Acceptance Scenarios**:

1. **Given** the field `"n": {"inner key": 3}` and flattening on, **When** it is displayed, **Then** it reads `n."inner key"=3`.
2. **Given** the field `"outer key": {"x": 1}` and flattening on, **When** it is displayed, **Then** it reads `"outer key".x=1`.
3. **Given** the field `"n": {"inner key": 3}` and flattening off, **When** it is displayed, **Then** the inner key inside the braces reads `"inner key"=3`.
4. **Given** a record shown in the expanded (multi-line) layout, **When** a key needs quoting, **Then** it is quoted the same way as on a single line.

---

### User Story 5 - Quote Character Adapts to the Key (Priority: P3)

A user views a key that itself contains a double quote, or other characters that clash with a quote character. The key is quoted the way a value with the same text would be: the same quote character and, when needed, the same escaping.

**Why this priority**: Such keys are rare, but they must still be readable and unambiguous. Matching how values are quoted keeps the output predictable.

**Independent Test**: Display a record with each key below, and compare it with how the same text is displayed as a value.

**Acceptance Scenarios**:

1. **Given** the key `say "hi"`, **When** it is displayed, **Then** it reads `'say "hi"'`.
2. **Given** a key that contains both `"` and `'`, but no backtick or control characters, **When** it is displayed, **Then** it is wrapped in backticks.
3. **Given** a key that contains a newline, a tab or another control character, **When** it is displayed, **Then** it is shown double-quoted with escape sequences, such as `"line\nbreak"`, and stays on one line.

---

### Edge Cases

- What happens when `--raw-fields` is used together with the new option? The `--raw-fields` flag disables all field formatting including value unescaping. It should take precedence: when `--raw-fields` is active, the key prettification option has no effect since raw output bypasses all formatting.
- What happens with field keys that contain no underscores? No change in behavior; keys without underscores are unaffected by this option.
- What happens with field keys that are entirely underscores (e.g., `___`)? When replacement is enabled, the key becomes `---`. When disabled, it remains `___`.
- What happens with nested/flattened keys where the dot-separated prefix contains underscores? The option applies to all key segments uniformly. For example, `parent_key.child_key` preserves underscores in both segments when replacement is disabled.
- What happens with a key that consists only of spaces? It is quoted and its spaces are kept: `"  "=x`.
- What happens with a key that has leading or trailing spaces? It is quoted and the spaces are kept, so `" a"` and `"a"` stay distinct.
- What happens when prettification and quoting both apply? Prettification runs first and quoting looks at the result: `user_name` becomes `user-name`, unquoted, while `user name` becomes `"user name"`.
- What happens with a key that contains a `.`? It is not quoted, even though `.` also joins nested path segments (see Assumptions).
- What happens with a key that contains non-ASCII letters, such as `имя` or `名前`? It is not quoted.
- What happens with a key that contains a backslash? It is quoted, as values are.
- What happens to field filters such as `-h`? They keep matching the original key. Quoting changes only how keys are displayed.
- What happens with `--raw`? Records are output in their original form, and key quoting has no effect.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST provide a configuration file option to disable the automatic replacement of underscores with hyphens in field keys during output formatting.
- **FR-002**: The default value of this option MUST preserve the current behavior (underscores replaced with hyphens).
- **FR-003**: When the option is set to disable replacement, the system MUST preserve underscores in all field key segments, including nested and flattened key prefixes.
- **FR-004**: This option MUST be independent of `--raw-fields`. Disabling key prettification MUST NOT affect field value unescaping or any other formatting behavior.
- **FR-005**: When both `--raw-fields` and this option are active, `--raw-fields` MUST take precedence (all formatting bypassed).
- **FR-006**: In the human-readable output, the system MUST quote a field key when the key is empty, or when its displayed text contains any of: a space, a tab, a newline, any other control character, `=`, `"`, `'`, a backtick, or a backslash.
- **FR-007**: The system MUST NOT quote a key that does not meet the conditions in FR-006. In particular, keys that look like numbers, booleans or `null`, or that start with `{` or `[`, MUST stay unquoted.
- **FR-008**: When a key is quoted, the system MUST choose the quote character and escaping by the same rules it uses for string field values: double quotes when possible, otherwise single quotes, otherwise backticks, otherwise double quotes with escape sequences.
- **FR-009**: A quoted key MUST always be displayed on a single line. A key never uses the multi-line layout that a long or multi-line value may use.
- **FR-010**: For flattened nested fields, the system MUST decide on and apply quoting to each path segment separately, and join the segments with `.` outside the quotes, as in `n."inner key"`.
- **FR-011**: For non-flattened nested fields, the system MUST apply the same quoting rules to the keys inside the nested object.
- **FR-012**: The decision to quote MUST be based on the key as displayed, after key prettification when it is on.
- **FR-013**: Key quoting MUST apply in single-line and expanded layouts alike. Per FR-005, it MUST NOT apply when `--raw-fields` is active.
- **FR-014**: Key quoting MUST NOT change field filtering, hiding, or any other key matching. It affects only how keys are displayed.
- **FR-015**: The quotes around a key MUST be styled as part of the key, so the key keeps the active theme's key style.
- **FR-016**: Key quoting MUST NOT measurably slow down processing of logs whose keys are all plain.

### Out of Scope

- CLI flags for this option. The option is config-file only; no per-invocation override is needed.
- An option to turn key quoting off. Quoting fixes ambiguous output and changes nothing for plain keys.
- Parsing such keys from logfmt input. That is handled by #1607.

### Key Entities

- **Formatting Configuration**: The set of user-configurable formatting preferences, extended with the new key prettification option.
- **Field Key**: A string identifier for a log record field, which may contain underscores and may be nested using dot-separated segments. It is displayed plain or quoted.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can disable underscore-to-hyphen replacement via a config file option and see original field keys preserved in 100% of formatted output.
- **SC-002**: Existing default behavior (underscores replaced with hyphens) is preserved for all users who do not set the new option.
- **SC-003**: No other formatting behaviors (value unescaping, key flattening, punctuation) are affected when only this option is changed.
- **SC-004**: The reproduction command from issue #1609 produces exactly the expected line given in the issue.
- **SC-005**: For every record in the test suite that has only plain keys, the output is byte-for-byte the same as before key quoting was added.
- **SC-006**: For every key in the edge-case and acceptance set, a reader can split each displayed field into exactly one key and one value without seeing the original input.
- **SC-007**: Throughput on the standard benchmarks, whose logs have plain keys, does not regress by more than the project's 5% threshold.

## Assumptions

- The config file format is TOML, consistent with the existing configuration system.
- The option fits naturally within the existing formatting section of the config file.
- The naming convention for the option follows the existing kebab-case pattern used in the config file.
- Keys are quoted by the same rules as string values, except that keys skip the value-only checks that keep strings from being mistaken for numbers, literals, objects or arrays. Those checks only matter in value position. #1481 quotes keys in the structured output formats in the same spirit.
- `.` does not trigger quoting. Dotted keys such as `http.method` are very common, and quoting them would change typical output. A literal `a.b` key and a nested `a` → `b` path therefore still look the same. This ambiguity already exists and is out of scope.
- The trigger set uses `=` regardless of the configured key-value separator. Keys that contain a custom separator are not quoted unless they also meet FR-006.
- Key quoting is a bug fix. It changes output only for keys with special characters, so it is a patch-level change.
