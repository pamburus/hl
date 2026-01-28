# Tasks: Pager Configuration System

**Input**: Design documents from `/specs/008-pager-config/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, quickstart.md
**Updated**: 2025-01-27

**Tests**: Tests are included as per constitution (Test-First Development & Quality).

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/` at repository root
- Test assets in `src/testing/assets/`

---

## Phase 1: Setup (Shared Infrastructure) ✅

**Purpose**: Project initialization and module structure

- [x] T001 Add `which` crate dependency to Cargo.toml
- [x] T002 Create pager module structure: `src/pager/mod.rs` with submodule declarations
- [x] T003 [P] Create `src/pager/config.rs` with `PagerConfig`, `PagerProfile`, `PagerRoleConfig`, `PagerRole` structs (no Default impls)
- [x] T004 [P] Add `pager` and `pagers` fields to `Settings` struct in `src/settings.rs`
- [x] T005 Add `pub mod pager;` to `src/lib.rs`

---

## Phase 2: Foundational (Blocking Prerequisites) ✅

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

- [x] T006 Create `src/pager/selection.rs` with `PagerSelector` struct skeleton and `PagerOverride`, `SelectedPager` enums
- [x] T007 Implement `PagerProfile::is_valid()` and `PagerProfile::executable()` methods in `src/pager/config.rs`
- [x] T008 Implement `PagerProfile::build_command(role)` method in `src/pager/config.rs`
- [x] T009 Implement `is_available()` method in `src/pager/selection.rs` using `which` crate for PATH lookup
- [x] T010 [P] Create test asset directory `src/testing/assets/pagers/`
- [x] T011 [P] Create basic test config `src/testing/assets/pagers/basic.toml` with single profile
- [x] T006a Implement `EnvProvider` and `ExeChecker` traits for dependency injection in tests

**Checkpoint**: Foundation ready ✅

---

## Phase 3: User Story 1 - Configure Preferred Pager (Priority: P1) 🎯 MVP ✅

**Goal**: Users can configure a pager profile in config and have it work for viewing logs

**Independent Test**: Create config with `pager = "less"` and `[pagers.less]`, run `hl logfile.log`, verify less is used

### Tests for User Story 1 ✅

- [x] T012 [P] [US1] Create unit test for `PagerConfig` deserialization (single profile) in `src/pager/tests.rs`
- [x] T013 [P] [US1] Create unit test for `PagerProfile` deserialization in `src/pager/tests.rs`
- [x] T014 [P] [US1] Create test config `src/testing/assets/pagers/single-profile.toml`

### Implementation for User Story 1

- [x] T015 [US1] Implement `select_for_view()` method in `src/pager/selection.rs` (single profile case)
- [ ] T016 [US1] Refactor `Pager::new()` to `Pager::from_selection(SelectedPager)` in `src/output.rs`
- [ ] T017 [US1] Implement environment variable application from `PagerProfile.env` in `src/output.rs`
- [ ] T018 [US1] Wire up `PagerSelector` in `src/main.rs` for view mode
- [x] T019 [US1] Add debug logging for pager selection in `src/pager/selection.rs`

**Checkpoint**: User Story 1 core logic complete, integration pending

---

## Phase 4: User Story 2 - Priority-Based Pager Fallback (Priority: P1) ✅

**Goal**: Users can specify multiple pager profiles, system tries each until one is available

**Independent Test**: Configure `pager = ["fzf", "less"]` on system without fzf, verify less is used

### Tests for User Story 2 ✅

- [x] T020 [P] [US2] Create unit test for `PagerConfig` deserialization (priority list) in `src/pager/tests.rs`
- [x] T021 [P] [US2] Create unit test for priority-based selection with unavailable first choice in `src/pager/tests.rs`
- [x] T022 [P] [US2] Create test config `src/testing/assets/pagers/priority-list.toml`

### Implementation for User Story 2 ✅

- [x] T023 [US2] Implement `PagerConfig::profiles()` iterator method in `src/pager/config.rs`
- [x] T024 [US2] Update `select_for_view()` to iterate through priority list in `src/pager/selection.rs`
- [x] T025 [US2] Handle fallback to stdout when no pager available in `src/pager/selection.rs`
- [x] T026 [US2] Add debug logging for each profile tried and reason skipped in `src/pager/selection.rs`

**Checkpoint**: User Story 2 complete ✅

---

## Phase 5: User Story 3 - Role-Specific Pager Arguments (Priority: P2) ✅

**Goal**: Different arguments for view mode vs follow mode, follow mode requires explicit opt-in

**Independent Test**: Configure `view.args` and `follow.args` differently, verify correct args used in each mode

### Tests for User Story 3 ✅

- [x] T027 [P] [US3] Create unit test for `PagerRoleConfig` deserialization in `src/pager/tests.rs`
- [x] T028 [P] [US3] Create unit test for `follow.enabled` defaulting to false in `src/pager/tests.rs`
- [x] T029 [P] [US3] Create unit test for `build_command()` with role-specific args in `src/pager/tests.rs`
- [x] T030 [P] [US3] Create test config `src/testing/assets/pagers/follow-enabled.toml`

### Implementation for User Story 3

- [x] T031 [US3] Implement `select_for_follow()` method in `src/pager/selection.rs`
- [x] T032 [US3] Add `follow.enabled` check - return `SelectedPager::None` if not enabled in `src/pager/selection.rs`
- [ ] T033 [US3] Wire up role selection based on `--follow` flag in `src/main.rs`
- [ ] T034 [US3] Ensure `--paging=always` does not override `follow.enabled = false` in `src/main.rs`

**Checkpoint**: User Story 3 core logic complete, integration pending

---

## Phase 6: User Story 4 - Environment Variable Override (Priority: P2) ✅

**Goal**: `HL_PAGER` overrides config, supports both profile names and command strings

**Independent Test**: Set `HL_PAGER=less`, verify it overrides config `pager = "fzf"`

### Tests for User Story 4 ✅

- [x] T035 [P] [US4] Create unit test for `HL_PAGER` as profile name in `src/pager/tests.rs`
- [x] T036 [P] [US4] Create unit test for `HL_PAGER` as command string in `src/pager/tests.rs`
- [x] T037 [P] [US4] Create unit test for `HL_PAGER=""` disabling pager in `src/pager/tests.rs`

### Implementation for User Story 4 ✅

- [x] T038 [US4] Implement `resolve_env_var()` function to parse `HL_PAGER` in `src/pager/selection.rs`
- [x] T039 [US4] Add profile name vs command string detection logic in `src/pager/selection.rs`
- [x] T040 [US4] Implement special `less` handling for command strings (auto `-R`, `LESSCHARSET`) in `src/pager/selection.rs`
- [x] T041 [US4] Update `select_for_view()` to check `HL_PAGER` first in `src/pager/selection.rs`
- [x] T042 [US4] Handle `HL_PAGER=""` as explicit disable in `src/pager/selection.rs`

**Checkpoint**: User Story 4 complete ✅

---

## Phase 7: User Story 5 - Follow Mode Pager Override (Priority: P2) ✅

**Goal**: `HL_FOLLOW_PAGER` allows separate override for follow mode, can override `HL_PAGER=""`

**Independent Test**: Set `HL_FOLLOW_PAGER=less`, verify it's used only in follow mode

### Tests for User Story 5 ✅

- [x] T043 [P] [US5] Create unit test for `HL_FOLLOW_PAGER` taking precedence in follow mode in `src/pager/tests.rs`
- [x] T044 [P] [US5] Create unit test for `HL_FOLLOW_PAGER` ignored in view mode in `src/pager/tests.rs`
- [x] T045 [P] [US5] Create unit test for `HL_FOLLOW_PAGER` overriding `HL_PAGER=""` in `src/pager/tests.rs`

### Implementation for User Story 5 ✅

- [x] T046 [US5] Implement `HL_FOLLOW_PAGER` check in `select_for_follow()` in `src/pager/selection.rs`
- [x] T047 [US5] Update `select_for_follow()` to check `HL_FOLLOW_PAGER` before `HL_PAGER` in `src/pager/selection.rs`
- [x] T048 [US5] Ensure `HL_FOLLOW_PAGER` can override `HL_PAGER=""` in `src/pager/selection.rs`

**Checkpoint**: User Story 5 complete ✅

---

## Phase 7b: Follow Mode Pager Closure Behavior (FR-014a, FR-014b)

**Goal**: When pager is closed in follow mode, stop following and exit the application gracefully

**Independent Test**: Run `hl --follow logfile.log` with a pager, close the pager (e.g., press 'q' in less), verify application exits

### Tests for Pager Closure Behavior

- [ ] T048a [P] [US5] Create unit test for detecting pager stdin pipe closure in `src/pager/tests.rs`
- [ ] T048b [P] [US5] Create integration test for follow mode exit on pager close

### Implementation for Pager Closure Behavior

- [x] T048c [US5] Implement write error detection on pager stdin pipe in `src/app.rs`
- [x] T048d [US5] Propagate pager closure signal to stop follow mode in `src/app.rs`
- [x] T048e [US5] Ensure graceful shutdown (cleanup resources, exit code 0) on pager closure

**Checkpoint**: Pager closure behavior implementation complete ✅

---

## Phase 8: User Story 6 - Backward Compatibility with PAGER (Priority: P3) ✅

**Goal**: Standard `PAGER` env var works when `HL_PAGER` not set and no config

**Independent Test**: Unset `HL_PAGER`, remove config, set `PAGER=most`, verify most is used

### Tests for User Story 6 ✅

- [x] T049 [P] [US6] Create unit test for `PAGER` fallback when no `HL_PAGER` or config in `src/pager/tests.rs`
- [x] T050 [P] [US6] Create unit test for `PAGER` being lower precedence than config in `src/pager/tests.rs`

### Implementation for User Story 6 ✅

- [x] T051 [US6] Add `PAGER` env var check as final fallback in `select_for_view()` in `src/pager/selection.rs`
- [x] T052 [US6] Apply special `less` handling for `PAGER` command strings in `src/pager/selection.rs`

**Checkpoint**: User Story 6 complete ✅

---

## Phase 9: Integration & Polish

**Purpose**: Wire up selection to actual pager execution, cleanup, documentation

- [ ] T053 Refactor `Pager::new()` to `Pager::from_selection(SelectedPager)` in `src/output.rs`
- [ ] T054 Wire up `PagerSelector` in `src/main.rs` for view and follow modes
- [ ] T055 Remove old `Pager::new()` implementation after migration complete in `src/output.rs`
- [ ] T056 [P] Update README.md with new pager configuration documentation
- [ ] T057 [P] Add CHANGELOG entry for pager configuration feature and `HL_PAGER=""` behavior change
- [ ] T058 Run `cargo clippy --workspace --all-targets --all-features` and fix warnings
- [ ] T059 Run `cargo test` and verify all tests pass
- [ ] T060 Manual validation: test with fzf, less, and missing pager scenarios

---

## Summary

- **Total tasks**: 65
- **Completed**: 50
- **Remaining**: 15 (integration tasks + pager closure tests)

### Completed Phases
- Phase 1 (Setup): 5/5 ✅
- Phase 2 (Foundational): 7/7 ✅
- Phase 3 (US1 - Single Pager): 5/8 (core logic done, integration pending)
- Phase 4 (US2 - Priority Fallback): 7/7 ✅
- Phase 5 (US3 - Role-Specific Args): 6/8 (core logic done, integration pending)
- Phase 6 (US4 - HL_PAGER Override): 8/8 ✅
- Phase 7 (US5 - HL_FOLLOW_PAGER): 6/6 ✅
- Phase 7b (Pager Closure): 3/5 (implementation done, tests pending)
- Phase 8 (US6 - PAGER Compat): 4/4 ✅
- Phase 9 (Integration & Polish): 0/8

### Next Steps
1. Complete integration tasks (T053, T054, T055) to wire selection into output.rs and main.rs
2. Implement pager closure behavior (T048a-T048e) for follow mode graceful exit
3. Documentation updates (T056, T057)
4. Final validation (T058, T059, T060)