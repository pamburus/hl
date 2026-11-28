<!--
Sync Impact Report:
- Version: 2.5.0 → 2.6.0
- Modified Sections:
  - V. Test-First Development & Quality, Test Placement Requirements: unit tests no longer interact with the OS or the environment, and every test that does is an integration test; unit tests do not wait for wall-clock time and mock time instead; the placement of tests of `mod.rs` and crate root modules is stated; the integration tests of each crate go to that crate.
  - V. Test-First Development & Quality, Coverage Validation Requirements: code that interacts with the environment is covered by integration tests instead of being left uncovered.
- Rationale: MINOR version bump. Requirements are added and extended; no principle is removed.
- Templates requiring updates:
  - ✅ plan-template.md: Compatible - no test placement rules
  - ✅ spec-template.md: Compatible - no test placement rules
  - ✅ tasks-template.md: Compatible - test paths are given per project
- Follow-up TODOs: move the existing unit tests that use the file system to integration tests or to in-memory file systems, and replace the sleep in the lifecycle crate's unit tests with mocked time
- Date: 2026-11-28
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
- MUST add tests to cover them: unit tests, or integration tests for code that interacts with the OS or the environment (file I/O, network, OS-specific behavior), see Test Placement Requirements below
- Document why coverage cannot be added if neither kind of test can reach the code
- Aim to maintain or improve overall project coverage percentage

This ensures new features are properly tested and maintains the high-quality bar established by existing code.

**Test Placement Requirements:**
- Tests are either unit tests or integration tests. A test that interacts with the OS or the environment in any way is an integration test. This includes the file system (also reading files of the repository at run time), environment variables, the current directory, processes, signals, terminals and the network
- Unit tests MUST NOT interact with the OS or the environment. Code under test that needs them takes them through an abstraction that the test replaces with an in-memory implementation, such as `vfs::mem::FileSystem`. Test data is embedded at compile time, for example with `include_str!`
- Unit tests MUST NOT wait for wall-clock time to pass, for example by sleeping or by waiting for a timeout or an interval to expire. Logic that depends on time takes the current time or a clock from its caller, and the test passes explicit instants, such as a base instant read once plus offsets, or a mock clock. A timeout that only guards a test against hanging, such as `recv_timeout`, is allowed, because it waits only when the test fails
- All unit tests of a module, including property-based tests, MUST be in a `tests.rs` file in the module's directory, declared at the end of the module as `#[cfg(test)] mod tests;`:
  - `a/b.rs` → `a/b/tests.rs`
  - `a/b/mod.rs` → `a/b/tests.rs`
  - `lib.rs`, or `main.rs` of a crate without `lib.rs` → `tests.rs` next to it; in a crate with both, `main.rs` has no unit tests of its own, and the logic to test goes to the library or to submodules of the binary
- Test modules with other names or in other places MUST NOT be added
- Test helpers shared by several modules belong in the `testing` module
- Integration tests are the exception: they MUST be modules of the one test binary in `tests/integration/` (`tests/integration/main.rs`) of the crate they test, one module per area, with the helpers they share in its `tests/integration/support/`:
  - the `hl` crate at the repository root → `tests/integration/`
  - a crate in `crates/some/crate/` → `crates/some/crate/tests/integration/`, never the `tests/` directory of the root crate
- Supporting code in a `tests.rs` file, such as helper functions, types, trait implementations and constants, MUST be placed at the beginning of the module, after the imports and before the first test function, never between test functions
- All `tests.rs` files and the integration tests of every crate MUST be excluded from coverage reports, so that coverage measures only the code under test

This keeps tests and their helpers easy to find, keeps unit tests fast, deterministic and independent of the machine they run on, and keeps test code from inflating coverage.

### VI. Specification & Cross-Reference Integrity
<!-- Maintain referential integrity across all documentation and code -->
**Specifications are the reference for behavior.** Every change of behavior, whether a new feature, a change to an existing one or a bug fix, MUST leave the specifications under `specs/` describing the behavior after the change, in the same pull request. Changed behavior that no spec covers yet MUST be added to one. Before a behavior is reported or treated as a bug, it MUST be checked against the specifications. The procedure for both is in [Specifications](../../docs/guidelines/specifications.md).

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
**Compatibility**: SEMVER for versioning. From 1.0 onwards, breaking changes require a major version bump and a migration guide. While on 0.x, breaking changes bump the minor version and still require a migration guide.
**Documentation**: README, man pages, inline examples, architecture decision records for major changes

## Development Workflow & Quality Gates

**Code Review**: All PRs require at least one review. Performance-sensitive changes require benchmark verification.

**Testing Gates**: All tests must pass (`cargo test`). Benchmarks must not regress (`cargo bench`). Clippy warnings must be resolved.

**Documentation**: Features documented before or concurrent with implementation. Breaking changes documented in CHANGELOG.

**Performance**: New features benchmarked. Regressions >5% must be justified and documented.

**Backwards Compatibility**: From 1.0 onwards, maintained across minor versions. While on 0.x, maintained across patch versions. Breaking minor bumps are allowed but SHOULD be avoided. Deprecation period required before removing features.

**Versioning**: Every merged state of a release line that can produce a different program has a unique version that reflects the SemVer impact of the changes since the line's last release. Changes that cannot affect the built program, such as CI workflows, keep the version. Branch roles, the release flow and the versioning procedure are in [Branches and Versioning](../../docs/guidelines/branches-and-versioning.md).

**Commits**: Every commit passes `just ci`. Commit messages follow Conventional Commits and describe user-visible behavior, because they generate the release notes. Details are in [Commits](../../docs/guidelines/commits.md).

## Governance

This constitution supersedes all other practices and informal conventions. All PRs and code reviews must verify compliance with these principles. Complexity introduced must be justified against the principles and documented.

**Constitution Authority**: These principles are non-negotiable. Exceptions require explicit discussion and documentation in PR comments.

**Amendments**: Changes to this constitution require:
1. Clear justification
2. Community/maintainer discussion
3. Documentation of rationale
4. Migration plan for any breaking changes

**Version**: 2.6.0 | **Ratified**: 2025-11-02 | **Last Amended**: 2026-11-28
