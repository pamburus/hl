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
