<!--
Sync Impact Report:
- Version: 1.4.2 → 1.4.3
- Modified Sections:
  - Development Workflow & Quality Gates (Version Check): Added git fetch -t requirement
- Rationale: PATCH version bump - workflow clarification to fetch latest tags
- Templates requiring updates:
  - ✅ plan-template.md: Compatible - no structural changes needed
  - ✅ spec-template.md: Compatible - no structural changes needed
  - ✅ tasks-template.md: Compatible - workflow guidance orthogonal to task structure
- Modified Requirements:
  - Step 2 of version check now includes git fetch -t to ensure latest tags
- Date: 2026-02-04
-->

# hl (High-performance Log viewer) Constitution
<!-- A high-performance log viewer and processor command-line app -->

## Core Principles

### I. Performance First
<!-- Performance is non-negotiable -->
Every design decision prioritizes speed and memory efficiency. Streaming processing handles logs of unlimited size without loading entire files into memory. Rust as the implementation language ensures predictable performance. Benchmarks track performance regressions and must pass before merging.

### II. Composability & Modularity
<!-- Features built as reusable components -->
Clear separation between core filtering engine, parsing modules, and output formatters. Each component has well-defined interfaces enabling independent testing and extension. The filtering and formatting pipeline is extensible for custom processors without modifying core code.

### III. User Experience & Intuitiveness
<!-- CLI should be intuitive and helpful -->
Sensible defaults for common workflows. Primary output designed for human readability using ANSI colors and styles. Progressive disclosure of advanced options in help text. Error messages are actionable and guide users toward solutions. No silent failures.

### IV. Reliability & Robustness
<!-- Data integrity is paramount -->
Comprehensive error handling with graceful degradation. All input validated; malformed logs processed without panicking. No data loss on edge cases. Streaming ensures memory-bounded execution regardless of input size. Clear documentation of supported log formats and limitations.

### V. Test-First Development & Quality
<!-- TDD mandatory; no exceptions for performance-critical code -->
Unit tests for algorithms and parsers. Integration tests for end-to-end CLI workflows. Property-based tests for streaming behavior. Performance benchmarks tracked and enforced. All tests must pass before merging. Coverage must not decrease: patches must maintain or improve the project's average code coverage.

**Schema Validation Requirements:**
When modifying configuration file structure (`src/settings.rs`, `etc/defaults/config.toml`) or theme configuration files, the corresponding JSON schema files (`schema/json/config.schema.json`, `schema/json/theme.schema.*.json`) MUST be updated to reflect the changes.

**Schema Build Workflow:**
After updating schema files, the following workflow MUST be followed:
1. Run `cargo build` to automatically update embedded schema references
2. Only after build completes successfully, run `just ci` for validation
3. This order is mandatory because the build process updates schema references that CI validates

This ensures:
- Configuration validation tooling remains accurate
- IDE autocomplete and validation work correctly
- Documentation stays synchronized with implementation
- Embedded schema references are current before validation

**Coverage Validation Requirements:**
After implementing any new feature, run `just uncovered` to identify changed lines lacking test coverage. If uncovered lines appear:
- MUST add tests to cover them unless they require environmental interaction (file I/O, network, OS-specific behavior)
- Document why coverage cannot be added if environmental interaction prevents testing
- Aim to maintain or improve overall project coverage percentage

This ensures new features are properly tested and maintains the high-quality bar established by existing code.

### VI. Specification & Cross-Reference Integrity
<!-- Maintain referential integrity across all documentation and code -->
**Avoid renumbering identifiers whenever possible.** Prefer adding new requirements at the end of sections or using sub-identifiers (e.g., FR-030c, FR-030d) to insert requirements without disrupting existing numbering.

When renumbering identifiers (FR/requirement IDs, user story IDs, feature numbers, etc.) is unavoidable, all cross-references MUST be identified and updated throughout the complete codebase including specs, documentation, code comments, and tests. Before renumbering any requirement or feature:

1. Search entire codebase for references to affected IDs using patterns like `FR-XXX`, `US-XXX`, feature numbers
2. Update ALL found references to reflect new identifiers
3. Verify tests still pass after updates
4. Document the ID mapping in commit message (e.g., "renamed FR-037d → FR-039d")

This ensures specifications remain the single source of truth and prevents broken references that make requirements untraceable. Use grep/search tools with patterns covering all identifier formats before any renumbering operation.

### VII. Test Data Management
<!-- Separate test logic from test data -->
Tests MUST use external data files instead of inline multiline string literals for themes, configs, and other structured data. This improves maintainability, enables proper validation tooling, and separates test logic from test data.

**Requirements:**
- Theme and config test data MUST be stored in dedicated test asset directories (e.g., `src/testing/assets/themes/`)
- Test files SHOULD use the same format as production files (YAML, TOML, JSON)
- Tests MAY embed external files at compile time using `include_str!` or similar if needed for performance
- Inline strings are acceptable ONLY for very short values (<3 lines) or when the string content itself is what's being tested (e.g., parse error messages)

**Rationale:**
- External files can be validated with proper tooling (YAML/TOML linters)
- Easier to reuse test data across multiple tests
- Better separation of concerns improves test readability
- Compile-time embedding provides same performance as inline strings

## Technology Stack & Standards

**Language**: Rust (enforced for reliability and performance)
**Build System**: Cargo
**Testing**: cargo test, criterion for benchmarks, proptest for property-based tests
**Output Format**: Primary output designed for human readability using ANSI colors and styles. Additional formats (JSON, etc.) may be added as needed but are not required.
**Compatibility**: SEMVER for versioning; breaking changes require major version bump and migration guide
**Documentation**: README, man pages, inline examples, architecture decision records for major changes. The documentation book (`docs/src/`) MUST conform to architectural rules defined in `docs/constitution/`. These subsidiary rules govern documentation structure, cross-referencing, and single-source-of-truth principles for the user-facing book.
**Typography**: Use spaced en dash ( – ) where an em dash would otherwise be unspaced; keep spaced em dashes in contexts where dashes are already separated (e.g., option description lists).

## Development Workflow & Quality Gates

**Code Review**: All PRs require at least one review. Performance-sensitive changes require benchmark verification.

**Testing Gates**: All tests must pass (`cargo test`). Benchmarks must not regress (`cargo bench`). Clippy warnings must be resolved.

**Documentation**: Features documented before or concurrent with implementation. Breaking changes documented in CHANGELOG.

**Performance**: New features benchmarked. Regressions >5% must be justified and documented.

**Backwards Compatibility**: Maintained across minor versions. Deprecation period required before removing features.

**Version Check Before Implementation**: Before starting feature implementation, ensure version is properly bumped:

1. Run `just version` to get current version (e.g., `0.36.0-alpha.5`)
2. Run `git fetch -t && just previous-tag` to get latest tags and previous release tag (e.g., `v0.35.3`)
3. Compare the major.minor versions:
   - Extract major.minor from current version (e.g., `0.36`)
   - Extract major.minor from previous tag, removing `v` prefix (e.g., `0.35`)
4. If major.minor versions are the SAME:
   - Calculate next minor version: `major.(minor+1).0-alpha.1`
   - Run `cargo set-version <next-version>` (e.g., `cargo set-version 0.36.0-alpha.1`)
5. If major.minor versions are DIFFERENT:
   - Run `just bump` to increment alpha version (e.g., `0.36.0-alpha.5` → `0.36.0-alpha.6`)

**Rationale**: This ensures each feature branch has a unique version identifier, preventing version conflicts and enabling proper release tracking.

**Commit Workflow**: When using speckit workflow commands, commits MUST follow this discipline:

1. **Pre-commit Validation**: Before ANY commit, run `just ci` which executes:
   - `check`: Cargo check for compilation errors
   - `test`: Full test suite execution
   - `lint`: Clippy warnings resolution
   - `audit`: Security vulnerability checks
   - `fmt-check`: Code formatting verification
   - `check-schema`: JSON schema validation

   ALL reported issues MUST be resolved before proceeding with the commit.

2. **Conventional Commits**: After completing each speckit step, create a commit using conventional commit format:
   - After `/speckit.specify`: `docs(spec): add [feature-name] specification`
   - After `/speckit.clarify`: `docs(spec): clarify [aspect] in [feature-name]`
   - After `/speckit.tasks`: `docs(tasks): generate task breakdown for [feature-name]`
   - After `/speckit.implement`: `feat([scope]): [user-visible change]` or `fix([scope]): [user-visible fix]`

3. **Commit Message Content Guidelines**:
   Commit messages are used to automatically generate release notes and MUST focus on user-visible behavior:

   **DO write about**:
   - New features users can access
   - Bug fixes that affect user experience
   - Configuration options added or changed
   - Command-line flags or arguments modified
   - Output format changes
   - Performance improvements users will notice
   - Breaking changes requiring user action

   **DO NOT write about**:
   - Implementation details (functions, structs, modules)
   - Source code references (file names, line numbers)
   - Test statistics ("1464 tests passing")
   - CI validation status ("Full CI validation passed")
   - Internal refactoring unless it affects users
   - Code quality metrics

   **Exception**: Internal changes with no user-visible impact may include brief technical context, but still avoid source code specifics. Focus on "what changed" in system behavior, not "how it was implemented."

   **Examples**:
   - ✅ Good: `feat(config): add prettify-field-keys option to control key prettification`
   - ✅ Good: `fix(parser): handle timestamps with microsecond precision correctly`
   - ❌ Bad: `feat(formatting): implement prettify_field_keys in RecordFormatter struct`
   - ❌ Bad: `fix: update formatting.rs line 1211 to check prettify flag. All 1464 tests pass`

4. **Rationale**: This workflow ensures:
   - Clean, atomic commits representing logical completion points
   - All code passing quality gates before entering version control
   - Clear commit history enabling easy navigation and rollback
   - Prevention of broken commits that fail CI pipelines
   - Release notes are immediately useful to end users

## Governance

This constitution supersedes all other practices and informal conventions. All PRs and code reviews must verify compliance with these principles. Complexity introduced must be justified against the principles and documented.

**Constitution Authority**: These principles are non-negotiable. Exceptions require explicit discussion and documentation in PR comments.

**Amendments**: Changes to this constitution require:
1. Clear justification
2. Community/maintainer discussion
3. Documentation of rationale
4. Migration plan for any breaking changes

**Version**: 1.4.3 | **Ratified**: 2025-11-02 | **Last Amended**: 2026-02-04
