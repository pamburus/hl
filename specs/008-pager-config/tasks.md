# Tasks: Pager Configuration System

**Input**: Design documents from `/specs/008-pager-config/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, quickstart.md

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

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and module structure

- [ ] T001 Add `which` crate dependency to Cargo.toml
- [ ] T002 Create pager module structure: `src/pager/mod.rs` with submodule declarations
- [ ] T003 [P] Create `src/pager/config.rs` with `PagerConfig`, `PagerProfile`, `PagerRoleConfig`, `PagerRole` structs (no Default impls)
- [ ] T004 [P] Add `pager` and `pagers` fields to `Settings` struct in `src/settings.rs`
- [ ] T005 Add `pub mod pager;` to `src/lib.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T006 Create `src/pager/selection.rs` with `PagerSelector` struct skeleton and `PagerSpec`, `SelectedPager` enums
- [ ] T007 Implement `PagerProfile::is_valid()` and `PagerProfile::executable()` methods in `src/pager/config.rs`
- [ ] T008 Implement `PagerProfile::build_command(role)` method in `src/pager/config.rs`
- [ ] T009 Implement `is_available()` method in `src/pager/selection.rs` using `which` crate for PATH lookup
- [ ] T010 [P] Create test asset directory `src/testing/assets/pagers/`
- [ ] T011 [P] Create basic test config `src/testing/assets/pagers/basic.toml` with single profile

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Configure Preferred Pager (Priority: P1) 🎯 MVP

**Goal**: Users can configure a pager profile in config and have it work for viewing logs

**Independent Test**: Create config with `pager = "less"` and `[pagers.less]`, run `hl logfile.log`, verify less is used

### Tests for User Story 1

- [ ] T012 [P] [US1] Create unit test for `PagerConfig` deserialization (single profile) in `src/pager/tests.rs`
- [ ] T013 [P] [US1] Create unit test for `PagerProfile` deserialization in `src/pager/tests.rs`
- [ ] T014 [P] [US1] Create test config `src/testing/assets/pagers/single-profile.toml`

### Implementation for User Story 1

- [ ] T015 [US1] Implement `select_for_view()` method in `src/pager/selection.rs` (single profile case)
- [ ] T016 [US1] Refactor `Pager::new()` to `Pager::from_selection(SelectedPager)` in `src/output.rs`
- [ ] T017 [US1] Implement environment variable application from `PagerProfile.env` in `src/output.rs`
- [ ] T018 [US1] Wire up `PagerSelector` in `src/main.rs` for view mode
- [ ] T019 [US1] Add debug logging for pager selection when `HL_DEBUG_LOG` is set in `src/pager/selection.rs`

**Checkpoint**: User Story 1 complete - single pager profile works in view mode

---

## Phase 4: User Story 2 - Priority-Based Pager Fallback (Priority: P1)

**Goal**: Users can specify multiple pager profiles, system tries each until one is available

**Independent Test**: Configure `pager = ["fzf", "less"]` on system without fzf, verify less is used

### Tests for User Story 2

- [ ] T020 [P] [US2] Create unit test for `PagerConfig` deserialization (priority list) in `src/pager/tests.rs`
- [ ] T021 [P] [US2] Create unit test for priority-based selection with unavailable first choice in `src/pager/tests.rs`
- [ ] T022 [P] [US2] Create test config `src/testing/assets/pagers/priority-list.toml`

### Implementation for User Story 2

- [ ] T023 [US2] Implement `PagerConfig::profiles()` iterator method in `src/pager/config.rs`
- [ ] T024 [US2] Update `select_for_view()` to iterate through priority list in `src/pager/selection.rs`
- [ ] T025 [US2] Handle fallback to stdout when no pager available in `src/pager/selection.rs`
- [ ] T026 [US2] Add debug logging for each profile tried and reason skipped in `src/pager/selection.rs`

**Checkpoint**: User Story 2 complete - priority-based fallback works

---

## Phase 5: User Story 3 - Role-Specific Pager Arguments (Priority: P2)

**Goal**: Different arguments for view mode vs follow mode, follow mode requires explicit opt-in

**Independent Test**: Configure `view.args` and `follow.args` differently, verify correct args used in each mode

### Tests for User Story 3

- [ ] T027 [P] [US3] Create unit test for `PagerRoleConfig` deserialization in `src/pager/tests.rs`
- [ ] T028 [P] [US3] Create unit test for `follow.enabled` defaulting to false in `src/pager/tests.rs`
- [ ] T029 [P] [US3] Create unit test for `build_command()` with role-specific args in `src/pager/tests.rs`
- [ ] T030 [P] [US3] Create test config `src/testing/assets/pagers/follow-mode.toml`

### Implementation for User Story 3

- [ ] T031 [US3] Implement `select_for_follow()` method in `src/pager/selection.rs`
- [ ] T032 [US3] Add `follow.enabled` check - return `SelectedPager::None` if not enabled in `src/pager/selection.rs`
- [ ] T033 [US3] Wire up role selection based on `--follow` flag in `src/main.rs`
- [ ] T034 [US3] Ensure `--paging=always` does not override `follow.enabled = false` in `src/main.rs`

**Checkpoint**: User Story 3 complete - role-specific arguments work, follow mode opt-in works

---

## Phase 6: User Story 4 - Environment Variable Override (Priority: P2)

**Goal**: `HL_PAGER` overrides config, supports both profile names and command strings

**Independent Test**: Set `HL_PAGER=less`, verify it overrides config `pager = "fzf"`

### Tests for User Story 4

- [ ] T035 [P] [US4] Create unit test for `HL_PAGER` as profile name in `src/pager/tests.rs`
- [ ] T036 [P] [US4] Create unit test for `HL_PAGER` as command string in `src/pager/tests.rs`
- [ ] T037 [P] [US4] Create unit test for `HL_PAGER=""` disabling pager in `src/pager/tests.rs`

### Implementation for User Story 4

- [ ] T038 [US4] Implement `resolve_env_pager()` function to parse `HL_PAGER` in `src/pager/selection.rs`
- [ ] T039 [US4] Add profile name vs command string detection logic in `src/pager/selection.rs`
- [ ] T040 [US4] Implement special `less` handling for command strings (auto `-R`, `LESSCHARSET`) in `src/pager/selection.rs`
- [ ] T041 [US4] Update `select_for_view()` to check `HL_PAGER` first in `src/pager/selection.rs`
- [ ] T042 [US4] Handle `HL_PAGER=""` as explicit disable in `src/pager/selection.rs`

**Checkpoint**: User Story 4 complete - HL_PAGER override works

---

## Phase 7: User Story 5 - Follow Mode Pager Override (Priority: P2)

**Goal**: `HL_FOLLOW_PAGER` allows separate override for follow mode, can override `HL_PAGER=""`

**Independent Test**: Set `HL_FOLLOW_PAGER=less`, verify it's used only in follow mode

### Tests for User Story 5

- [ ] T043 [P] [US5] Create unit test for `HL_FOLLOW_PAGER` taking precedence in follow mode in `src/pager/tests.rs`
- [ ] T044 [P] [US5] Create unit test for `HL_FOLLOW_PAGER` ignored in view mode in `src/pager/tests.rs`
- [ ] T045 [P] [US5] Create unit test for `HL_FOLLOW_PAGER` overriding `HL_PAGER=""` in `src/pager/tests.rs`

### Implementation for User Story 5

- [ ] T046 [US5] Implement `resolve_env_follow_pager()` function in `src/pager/selection.rs`
- [ ] T047 [US5] Update `select_for_follow()` to check `HL_FOLLOW_PAGER` before `HL_PAGER` in `src/pager/selection.rs`
- [ ] T048 [US5] Ensure `HL_FOLLOW_PAGER` can override `HL_PAGER=""` in `src/pager/selection.rs`

**Checkpoint**: User Story 5 complete - HL_FOLLOW_PAGER override works

---

## Phase 8: User Story 6 - Backward Compatibility with PAGER (Priority: P3)

**Goal**: Standard `PAGER` env var works when `HL_PAGER` not set and no config

**Independent Test**: Unset `HL_PAGER`, remove config, set `PAGER=most`, verify most is used

### Tests for User Story 6

- [ ] T049 [P] [US6] Create unit test for `PAGER` fallback when no `HL_PAGER` or config in `src/pager/tests.rs`
- [ ] T050 [P] [US6] Create unit test for `PAGER` being lower precedence than config in `src/pager/tests.rs`

### Implementation for User Story 6

- [ ] T051 [US6] Add `PAGER` env var check as final fallback in `select_for_view()` in `src/pager/selection.rs`
- [ ] T052 [US6] Apply special `less` handling for `PAGER` command strings in `src/pager/selection.rs`

**Checkpoint**: User Story 6 complete - PAGER backward compatibility works

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Cleanup, documentation, and final validation

- [ ] T053 [P] Remove old `Pager::new()` implementation after migration complete in `src/output.rs`
- [ ] T054 [P] Update README.md with new pager configuration documentation
- [ ] T055 [P] Add CHANGELOG entry for pager configuration feature and `HL_PAGER=""` behavior change
- [ ] T056 Run `cargo clippy --workspace --all-targets --all-features` and fix warnings
- [ ] T057 Run `cargo test` and verify all tests pass
- [ ] T058 Manual validation: test with fzf, less, and missing pager scenarios

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-8)**: All depend on Foundational phase completion
  - US1 and US2 are both P1 and can proceed in parallel
  - US3, US4, US5 are P2 and can proceed in parallel after US1/US2
  - US6 is P3 and can proceed after foundational
- **Polish (Phase 9)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Foundation only - No dependencies on other stories
- **User Story 2 (P1)**: Foundation only - Can run parallel with US1
- **User Story 3 (P2)**: Builds on US1/US2 selection infrastructure
- **User Story 4 (P2)**: Builds on US1/US2 selection infrastructure
- **User Story 5 (P2)**: Depends on US4 (HL_PAGER handling)
- **User Story 6 (P3)**: Depends on US4 (env var precedence pattern)

### Within Each User Story

- Tests written first (TDD)
- Config structs/methods before selection logic
- Selection logic before main.rs integration
- Story complete before moving to next priority

### Parallel Opportunities

- T003, T004 can run in parallel (different files)
- T010, T011 can run in parallel (test assets)
- All test tasks within a user story marked [P] can run in parallel
- US1 and US2 can run in parallel after foundational phase
- US3, US4 can run in parallel after US1/US2
- All Polish tasks marked [P] can run in parallel

---

## Parallel Example: User Story 1

```bash
# Launch all tests for User Story 1 together:
Task: "Create unit test for PagerConfig deserialization in src/pager/tests.rs"
Task: "Create unit test for PagerProfile deserialization in src/pager/tests.rs"
Task: "Create test config src/testing/assets/pagers/single-profile.toml"
```

---

## Implementation Strategy

### MVP First (User Stories 1 + 2)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (single profile)
4. Complete Phase 4: User Story 2 (priority fallback)
5. **STOP and VALIDATE**: Test basic pager configuration works
6. Deploy/demo if ready - this is the MVP!

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add US1 + US2 → Test independently → MVP!
3. Add US3 (role-specific args) → Follow mode support
4. Add US4 + US5 (env overrides) → Advanced usage
5. Add US6 (PAGER compat) → Full backward compatibility
6. Polish → Production ready

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 + 3
   - Developer B: User Story 2 + 4 + 5
   - Developer C: User Story 6 + Polish

---

## Summary

- **Total tasks**: 58
- **Phase 1 (Setup)**: 5 tasks
- **Phase 2 (Foundational)**: 6 tasks
- **User Story 1**: 8 tasks
- **User Story 2**: 7 tasks
- **User Story 3**: 8 tasks
- **User Story 4**: 8 tasks
- **User Story 5**: 6 tasks
- **User Story 6**: 4 tasks
- **Polish**: 6 tasks
- **MVP scope**: Phases 1-4 (US1 + US2) = 26 tasks