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

## Technology Stack & Standards

**Language**: Rust (enforced for reliability and performance)
**Build System**: Cargo
**Testing**: cargo test, criterion for benchmarks, proptest for property-based tests
**Output Format**: Primary output designed for human readability using ANSI colors and styles. Additional formats (JSON, etc.) may be added as needed but are not required.
**Compatibility**: SEMVER for versioning; breaking changes require major version bump and migration guide
**Documentation**: README, man pages, inline examples, architecture decision records for major changes

## Development Workflow & Quality Gates

**Code Review**: All PRs require at least one review. Performance-sensitive changes require benchmark verification.
**Testing Gates**: All tests must pass (`cargo test`). Benchmarks must not regress (`cargo bench`). Clippy warnings must be resolved.
**Documentation**: Features documented before or concurrent with implementation. Breaking changes documented in CHANGELOG.
**Performance**: New features benchmarked. Regressions >5% must be justified and documented.
**Backwards Compatibility**: Maintained across minor versions. Deprecation period required before removing features.

## Governance

This constitution supersedes all other practices and informal conventions. All PRs and code reviews must verify compliance with these principles. Complexity introduced must be justified against the principles and documented.

**Constitution Authority**: These principles are non-negotiable. Exceptions require explicit discussion and documentation in PR comments.

**Amendments**: Changes to this constitution require:
1. Clear justification
2. Community/maintainer discussion
3. Documentation of rationale
4. Migration plan for any breaking changes

**Version**: 1.0.0 | **Ratified**: 2025-11-02 | **Last Amended**: 2025-11-02
