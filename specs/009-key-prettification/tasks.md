# Tasks: Key Prettification Config Option

**Input**: Design documents from `/specs/009-key-prettification/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, quickstart.md

**Organization**: Tasks are grouped by user story to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1)
- Include exact file paths in descriptions

## Path Conventions

- Single Rust crate: `src/`, `tests/` at repository root
- Config: `etc/defaults/` for default configuration

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and validation

- [X] T001 Verify Rust toolchain and cargo are available
- [X] T002 Run cargo test to validate existing test suite baseline
- [X] T003 [P] Run cargo clippy to validate code quality baseline

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before user story implementation

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T004 Read and understand existing KeyPrettify trait implementation in src/formatting.rs:1302
- [X] T005 [P] Read and understand Formatting struct in src/settings.rs (around line 397)
- [X] T006 [P] Read and understand RecordFormatterBuilder::with_options() usage in src/app.rs:902
- [X] T007 Read and understand current raw_fields handling in RecordFormatterBuilder::build() in src/formatting.rs

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Preserve Original Field Keys via Config (Priority: P1) 🎯 MVP

**Goal**: Allow users to disable automatic underscore-to-hyphen replacement in field keys via config file option, preserving original field key formatting while maintaining all other formatting behaviors

**Independent Test**: Set `prettify-field-keys = false` in config, process logs with underscored field keys (e.g., `request_id`), verify keys appear as `request_id` not `request-id` while all other formatting (value unescaping, key flattening, punctuation) remains unchanged

### Implementation for User Story 1

- [X] T008 [P] [US1] Add prettify_field_keys: Option<bool> field to Formatting struct in src/settings.rs (after line 397)
- [X] T009 [P] [US1] Update Sample implementation for Formatting struct to include prettify_field_keys field in src/settings.rs
- [X] T010 [P] [US1] Add prettify-field-keys = true config option with comment to etc/defaults/config.toml under [formatting] section
- [X] T011 [US1] Add prettify_field_keys: bool field to RecordFormatter struct in src/formatting.rs (around line 379)
- [X] T012 [US1] Update RecordFormatterBuilder::build() to extract prettify_field_keys from config and set to !raw_fields && cfg.prettify_field_keys.unwrap_or(true) in src/formatting.rs (around line 305)
- [X] T013 [US1] Add prettify: bool parameter to KeyPrefix::push() method in src/formatting.rs (around line 896)
- [X] T014 [US1] Update KeyPrefix::push() implementation to conditionally apply key_prettify based on prettify parameter in src/formatting.rs
- [X] T015 [US1] Replace unconditional key.key_prettify(buf) in FieldFormatter::begin() with conditional check of self.rf.prettify_field_keys in src/formatting.rs (around line 1211)
- [X] T016 [US1] Update flattened path call to pass prettify flag to fs.key_prefix.push() in src/formatting.rs (around line 1181)
- [X] T017 [P] [US1] Add test for prettify=false with single underscore key (k_a stays k_a) in src/formatting/tests.rs
- [X] T018 [P] [US1] Add test for prettify=false with flattened nested keys (k_a.va.kb preserves underscores) in src/formatting/tests.rs
- [X] T019 [P] [US1] Add test for raw_fields=true overriding prettify (keys not prettified) in src/formatting/tests.rs
- [X] T020 [P] [US1] Add test confirming default behavior still works (prettify=true, k_a becomes k-a) in src/formatting/tests.rs

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently

---

## Phase 4: Polish & Cross-Cutting Concerns

**Purpose**: Validation and finalization

- [X] T021 Run cargo test to validate all tests pass
- [X] T022 Run cargo clippy to validate code quality
- [X] T023 Manually validate quickstart.md scenarios using actual config file
- [X] T024 Verify edge cases: empty underscores field (___), no underscores, nested keys

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS user story
- **User Story 1 (Phase 3)**: Depends on Foundational phase completion
- **Polish (Phase 4)**: Depends on User Story 1 completion

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories

### Within User Story 1

1. Config layer tasks (T008-T010) can run in parallel [P]
2. T011 (add field to RecordFormatter) must wait for T008 (config field defined)
3. T012 (builder logic) must wait for T008 and T011
4. T013-T016 (formatting logic updates) must be sequential (they modify interconnected code)
5. T017-T020 (tests) can run in parallel [P] after implementation complete

### Parallel Opportunities

- Phase 1: T002 and T003 can run in parallel
- Phase 2: T005 and T006 can run in parallel after T004
- Phase 3: T008, T009, T010 can run in parallel (different files)
- Phase 3: T017, T018, T019, T020 can run in parallel (independent tests)

---

## Parallel Example: User Story 1

```bash
# Launch all config layer tasks together (Phase 3 start):
Task: "Add prettify_field_keys field to Formatting struct in src/settings.rs"
Task: "Update Sample implementation in src/settings.rs"
Task: "Add prettify-field-keys config option to etc/defaults/config.toml"

# Launch all test tasks together (Phase 3 end):
Task: "Test prettify=false single key in src/formatting/tests.rs"
Task: "Test prettify=false nested keys in src/formatting/tests.rs"
Task: "Test raw_fields override in src/formatting/tests.rs"
Task: "Test default behavior in src/formatting/tests.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (validate baseline)
2. Complete Phase 2: Foundational (understand existing code)
3. Complete Phase 3: User Story 1 (implement feature)
4. Complete Phase 4: Polish (validate and test)
5. **STOP and VALIDATE**: Run all tests, manually verify config behavior
6. Ready for PR

### Incremental Delivery

This feature consists of a single user story, so delivery is atomic:

1. Complete Setup + Foundational → Understand existing architecture
2. Implement User Story 1 → Feature complete
3. Polish → Test and validate
4. Deploy/Demo

---

## Summary

- **Total Tasks**: 24
- **User Story 1**: 13 implementation tasks (T008-T020)
- **Parallel Opportunities**:
  - Phase 1: 2 tasks (T002, T003)
  - Phase 2: 2 tasks (T005, T006)
  - Phase 3 Start: 3 tasks (T008, T009, T010)
  - Phase 3 End: 4 tasks (T017, T018, T019, T020)
- **Independent Test Criteria**: Set config option, verify underscored keys preserved while other formatting unchanged
- **MVP Scope**: User Story 1 (the entire feature)

---

## Notes

- [P] tasks = different files, no dependencies
- [US1] label maps task to User Story 1 for traceability
- This feature modifies existing files following established patterns
- No new files created, only additions to existing structs/functions
- Tests validate all acceptance scenarios from spec.md
- Default behavior preserved (prettify=true) for backward compatibility
- Commit after logical groups (config layer, formatter layer, tests)

---

# Amendment 2026-10-21: Key Quoting (#1609)

**Input**: User Stories 2-5, FR-006 to FR-016 in [spec.md](spec.md); Amendment sections of [plan.md](plan.md), [research.md](research.md) (Decisions 6-11), [data-model.md](data-model.md), [quickstart.md](quickstart.md)

**Tests**: Included. The constitution requires test-first development (Principle V).

## Phase 5: Foundational (Blocking Prerequisites)

**Purpose**: The shared key writer used by every story below.

- [ ] T025 Add a key quote-style decision in the `string` module of src/formatting.rs: analyze the key bytes with the existing `Analyze` trait and return Plain, DoubleQuoted, SingleQuoted, Backtick or Escaped per research.md Decisions 6 and 8 (empty key → DoubleQuoted; trigger mask = `Control | DoubleQuote | SingleQuote | Backslash | Backtick | Space | Tab | Newline | EqualSign`; backticks only when the key has no tab, newline or control character)
- [ ] T026 Add a shared key writer in src/formatting.rs that takes the key, the prettify flag, the quote flag and a `Push<u8>` buffer, and writes the opening quote, the (prettified) key and the closing quote; for Escaped, prettify into a temporary buffer when needed and append it JSON-escaped via `encstr` (research.md Decisions 7, 9, 11)
- [ ] T027 Add unit tests for the quote-style decision and the key writer in src/formatting/tests.rs: plain, empty, spaces only, leading/trailing spaces, `=`, `"`, `'` and `"` together, backtick, backslash, newline, tab, control character, non-ASCII letters, `123`, `true`, `null`, `{x`, `http.method`, `user_id` with prettify on and off

**Checkpoint**: Key writer tested in isolation.

---

## Phase 6: User Story 2 - Read Fields Whose Keys Contain Spaces or Separators (Priority: P1) 🎯 MVP

**Goal**: Top-level keys that are empty or contain trigger characters are quoted.

**Independent Test**: The record from issue #1609 displays as `hello › "user name"="alice smith" "a=b"=x ""=empty n."inner key"=3`.

- [ ] T028 [US2] Add tests in src/formatting/tests.rs for acceptance scenarios 1-4 of User Story 2 (`"user name"="alice smith"`, `"a=b"=x`, `""=empty`, and the full issue record)
- [ ] T029 [US2] Use the shared key writer in `FieldFormatter::begin()` in src/formatting.rs, with quoting gated on `self.rf.unescape_fields` (research.md Decision 10); keep the writes inside the `Element::Key` batch (FR-015)
- [ ] T030 [US2] Add a test in src/formatting/tests.rs that `--raw-fields` (`with_raw_fields(true)`) displays such keys as is (FR-013)

**Checkpoint**: The issue repro is fixed for top-level keys.

---

## Phase 7: User Story 3 - Plain Keys Stay Unquoted (Priority: P1)

**Goal**: Output for plain keys is unchanged.

**Independent Test**: All existing formatter tests pass unchanged; plain-key records format byte-for-byte as before.

- [ ] T031 [US3] Add tests in src/formatting/tests.rs for acceptance scenarios 1-3 of User Story 3 (`user_id` → `user-id`, `http.method`, `a:b/c@d`, `123`, `true`, `null` all unquoted)
- [ ] T032 [US3] Run `cargo test` and confirm no existing expected outputs had to change, apart from keys that now legitimately need quotes (SC-005); review any such change

**Checkpoint**: No regressions for typical logs.

---

## Phase 8: User Story 4 - Nested Keys Quote Each Path Segment (Priority: P2)

**Goal**: Flattened paths quote only the segments that need it; non-flattened and expanded layouts quote keys the same way.

**Independent Test**: `{"n":{"inner key":3}}` → `n."inner key"=3`; `{"outer key":{"x":1}}` → `"outer key".x=1`; with flattening off → `n={ "inner key"=3 }`.

- [ ] T033 [US4] Add tests in src/formatting/tests.rs for acceptance scenarios 1-4 of User Story 4 (flattened inner and outer segment, non-flattened, expanded layout)
- [ ] T034 [US4] Give `KeyPrefix::push()` in src/formatting.rs a quote flag and write the segment through the shared key writer; pass `self.rf.unescape_fields` from `FieldFormatter::begin()` (FR-010)
- [ ] T035 [US4] Verify non-flattened and expanded output need no further change, since both use `FieldFormatter::begin()` (FR-011, FR-013)

**Checkpoint**: Nested fields unambiguous in every layout.

---

## Phase 9: User Story 5 - Quote Character Adapts to the Key (Priority: P3)

**Goal**: Keys with quotes or control characters pick the right quote style and stay on one line.

**Independent Test**: Nested keys `say "hi"` → `'say "hi"'`; `"` and `'` together → backticks; `line\nbreak` → `"line\nbreak"` on one line.

- [ ] T036 [US5] Add tests in src/formatting/tests.rs for acceptance scenarios 1-3 of User Story 5, using nested objects so the keys go through escape decoding (see plan.md Known Limitation)
- [ ] T037 [US5] Add a test in src/formatting/tests.rs that the existing Kubernetes-style key `k:{"type":"Ready"}` displays as `'k:{"type":"Ready"}'`

**Checkpoint**: All quote styles covered.

---

## Phase 10: Polish & Cross-Cutting Concerns

- [ ] T038 Add a test in src/formatting/tests.rs that a hide filter (`-h "user name"`) still matches the original key (FR-014)
- [ ] T039 Run the quickstart.md amendment scenarios with the built binary and confirm the outputs
- [ ] T040 Run `just uncovered` and add tests for any uncovered changed lines
- [ ] T041 Run the formatting benchmarks in benches/bench before and after the change and confirm no regression over 5% (FR-016, SC-007)
- [ ] T042 Format with `just fmt-rust`, then run `just ci`

---

## Amendment Dependencies & Execution Order

- **Phase 5** blocks all later phases.
- **US2 (Phase 6)** and **US3 (Phase 7)** depend only on Phase 5. US3 verifies US2 caused no regressions, so run it right after US2.
- **US4 (Phase 8)** depends on Phase 5. T034 touches `KeyPrefix::push()` and its call in `FieldFormatter::begin()`, so do it after T029.
- **US5 (Phase 9)** depends on Phases 6 and 8 (nested keys go through both call sites).
- **Phase 10** runs last.

### Parallel Opportunities

All amendment tasks touch the same two files, so code tasks are sequential. Test-writing tasks T028, T031, T033 and T036 can be drafted in parallel once T026 exists.

## Amendment Implementation Strategy

1. Phase 5 (key writer), then Phase 6 (US2): the issue repro is fixed for top-level keys. This is the MVP.
2. Phase 7 (US3): confirm nothing else changed.
3. Phase 8 (US4) and Phase 9 (US5): nested paths and quote styles.
4. Phase 10: coverage, benchmarks, CI.

## Amendment Summary

- **Total**: 18 tasks (T025-T042)
- Foundational: 3 · US2: 3 · US3: 2 · US4: 3 · US5: 2 · Polish: 5
