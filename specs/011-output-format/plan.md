# Implementation Plan: Machine-Readable Output Formats

**Branch**: `feature/output-format` | **Date**: 2026-10-21 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/011-output-format/spec.md`

## Summary

Add `--output-format=pretty|json|logfmt`, also settable as `HL_OUTPUT_FORMAT`, so that hl emits parsed, filtered, sorted or followed records as NDJSON or logfmt with a normalized schema:
- predefined fields first, under configurable output keys, with normalized values;
- then an optional input field;
- then the remaining fields in source order, with every key unique.

**Technical approach**:
- **Formatter**: a new `StructuredRecordFormatter`, generic over a JSON or logfmt encoder, plugs into the existing `RecordWithSourceFormatter` boundary. Every mode (`cat`, `sort`, `follow`) and every record selection option is therefore reused unchanged.
- **Normalization**: records are streamed straight into the output buffer. Dotted output keys that must be nested and merged (the ECS preset in JSON mode) follow an output-key tree built once at startup, so no per-record tree is needed.
- **Dropped lines**: unparsed lines and prefixes are reported through a new no-op-by-default `RecordObserver` hook. They are counted per input, logged at debug level with line numbers only when debug logging is enabled, and summarized at info level on exit.
- **Configuration**: `--flatten` gains an `auto` default that resolves per output format. Predefined fields gain an `output-key` setting, which the ECS and k8s presets set.

Details and rationale are in [research.md](./research.md).

## Technical Context

**Language/Version**: Rust 1.95 (`rust-toolchain.toml`), edition 2024

**Primary Dependencies**:
- existing: `clap` (CLI), `serde`, `config` and `serde_json` (settings and JSON parsing), `serde-logfmt` (workspace crate: logfmt parsing, plus new bare-key and bare-value predicates), `encstr` (workspace crate: string decoding and JSON encoding), `chrono` (time conversion), `wyhash` (key hashing), `heapopt` (inline vectors), `log` and `env_logger` (diagnostics), `memchr` (line counting);
- new: `proptest` as a dev-dependency only, the property testing tool the constitution names; no new runtime crates.

**Storage**: N/A. The on-disk sort index is unchanged (research R9).

**Testing**: `cargo test` (unit and app-level), `proptest` (round trip and validity properties), `criterion` benchmarks in `benches/`, `just uncovered`, and `just ci`.

**Target Platform**: Linux, macOS and Windows, as today.

**Project Type**: Single CLI application with workspace crates.

**Performance Goals**:
- SC-004: `pretty` throughput doesn't regress by more than 5%.
- SC-005, a target: structured throughput is at least that of `pretty`, and at most 5% slower for records without collisions, repeats or dotted output keys.

**Constraints**:
- Streaming, with memory bounded by the existing buffer and message size limits.
- No allocation per record in the fast path.
- Byte-identical `pretty` output (FR-002).
- Only records on stdout in structured modes (FR-017).

**Scale/Scope**:
- Inputs: multi-gigabyte files, many inputs, and records up to `--max-message-size` (64 MiB by default).
- Code: about 8 source files modified, 4 to 6 added, plus configuration, schema, presets and docs.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|---|---|---|
| I. Performance First | ✅ Pass | Streaming encoders with no per-record allocation in the fast path. The `pretty` path gains only a no-op observer call (R9). Benchmarks are added for both formats (R16); SC-004 is a gate and SC-005 a reported target. |
| II. Composability & Modularity | ✅ Pass | Structured output is a new formatter behind the existing trait. Encoders are separate modules, and the bare-key and bare-value predicates live in `serde-logfmt` next to the parser (R5). |
| III. User Experience | ✅ Pass | Opt-in option with env support, and default output unchanged. Dropped lines aren't silent: they are reported through `HL_DEBUG_LOG` (FR-019, FR-020). Configuration errors name the offending setting (FR-026). |
| IV. Reliability & Robustness | ✅ Pass | Invalid escapes and UTF-8 are replaced, never emitted raw. Unparsable timestamps fall back to the original value. Malformed lines are dropped and counted, never fatal. |
| V. Test-First & Quality | ✅ Pass | Unit, app-level and property tests per R17. Schema update followed by `cargo build` and then `just ci`. `just uncovered` after implementation. Coverage must not decrease (SC-007). |
| VI. Specification Integrity | ✅ Pass | No FR or SC renumbering. Spec 005 is unaffected. FR-014 was reworded in place to record the hiding decision (R15). |
| VII. Test Data Management | ✅ Pass | Fixture logs, expected outputs and preset configs live in `src/testing/assets/` (R17). |
| Versioning of PRs | ✅ Pass | Commits on `feature/output-format` keep the version. The PR merging the feature into `develop` sets the next minor pre-release (`0.37.0-alpha.1`, or as computed then), and its commit ends with `Closes: #1481`. The `--flatten` default change keeps `pretty` output identical. Users whose configuration explicitly sets `flatten = "always"` get flattened JSON, which goes in the release notes. |
| Backwards compatibility | ✅ Pass | No option is removed or changes meaning. `auto` is additive, and existing explicit values behave as before. |

**Post-design re-check**: unchanged, all pass. The design adds no new crate, no on-disk format change and no global mutable state beyond a once-only exit hook (R10), which is justified by Ctrl-C exiting outside `App::run`. SIGTERM handling is tracked separately (#1612).

## Project Structure

### Documentation (this feature)

```text
specs/011-output-format/
├── spec.md              # Feature specification
├── plan.md              # This file
├── research.md          # Phase 0: decisions R1–R17
├── data-model.md        # Phase 1: entities and encoding rules
├── quickstart.md        # Phase 1: end-to-end validation scenarios
├── contracts/
│   ├── cli.md           # Options, environment, configuration, schema, presets
│   └── output.md        # Record grammar, examples, diagnostics
├── checklists/
│   └── requirements.md  # Specification quality checklist
└── tasks.md             # Phase 2 (/speckit-tasks; not created here)
```

### Source Code (repository root)

```text
src/
├── cli.rs                         # MODIFY: OutputFormat enum and --output-format; FlattenOption::Auto
├── main.rs                        # MODIFY: resolve output format, flatten and input-info; structured mode disables colors and pager; exit hook on write error
├── app.rs                         # MODIFY: Options.output_format; formatter selection; per-input context for SegmentProcessor; dropped-line observer; sort diagnostics-only blocks; follow flush; exit summary
├── app/tests.rs                   # MODIFY: app-level tests per mode and format
├── formatting.rs                  # MODIFY: declare the structured submodule; share string helpers
├── formatting/
│   ├── structured.rs              # NEW: StructuredRecordFormatter, normalization (order, uniqueness, hiding, flatten, merge)
│   ├── structured/
│   │   ├── json.rs                # NEW: JSON encoder, number rewrite
│   │   ├── logfmt.rs              # NEW: logfmt encoder, bare/quoted decisions
│   │   ├── keyset.rs              # NEW: inline hashed key set
│   │   └── tests.rs               # NEW: unit and property tests
│   └── tests.rs                   # unchanged
├── timestamp.rs                   # MODIFY: format_rfc3339_utc with source-derived precision
├── timestamp/tests.rs             # MODIFY: FR-007 precision cases
├── settings.rs                    # MODIFY: output_key on Field and LevelField; InputField; FlattenOption::Auto; validate()
├── settings/error.rs              # MODIFY: output-key validation errors
├── settings/tests.rs              # MODIFY: validation and defaults tests
├── signal_unix.rs, signal_windows.rs  # MODIFY: call the exit hook before exiting on Ctrl-C
├── diagnostics.rs                 # NEW: DropStats, DroppedLine reporting, exit hook
└── testing/assets/
    ├── fixtures/output-format/    # NEW: input logs and expected json/logfmt outputs
    └── configs/                   # NEW files: preset-based test configs

crates/serde-logfmt/src/logfmt/
├── de.rs                          # MODIFY: export is_bare_key / is_bare_value using the parser tables
└── de/tests.rs                    # MODIFY: predicate tests

etc/defaults/
├── config.toml                    # MODIFY: flatten = "auto"; document output-key
├── config-ecs.toml                # MODIFY: output keys per FR-028
└── config-k8s.toml                # MODIFY: output keys per FR-028

schema/json/config.schema.json     # MODIFY: flatten enum, output-key, fields.predefined.input
benches/bench/ws/hl/combined.rs    # MODIFY: format-json and format-logfmt cases
README.md                          # MODIFY: options listing, usage section for structured output
```

**Structure Decision**: Single project with the existing module layout. The new code sits under `src/formatting/structured/`, next to the `pretty` formatter it parallels. The one cross-crate addition (bare predicates) goes in `serde-logfmt`, so the writer and the parser share one definition.

## Implementation Outline

Phases for `/speckit-tasks` to break down. Each phase ends with `just ci` passing.

0. **Prerequisite refactoring, separate PR to `develop`**: record origin and per-input descriptors passed to the formatter, an input context for the line processor, and the badge name computation split from its rendering. No behavior change.
1. **Foundations**:
   - `FlattenOption::Auto` across the CLI, settings, defaults and schema, resolving to today's behavior for `pretty`;
   - the `--output-format` and `HL_OUTPUT_FORMAT` option;
   - `output-key` settings with validation and schema.
2. **Encoders**:
   - the bare-key and bare-value predicates in `serde-logfmt`;
   - JSON and logfmt encoders, number rewrite, string re-encoding;
   - `Timestamp::format_rfc3339_utc`.
   - Unit tests first.
3. **Normalizer, fast path**: predefined section, regular fields, hiding, flattening, key set, repeats and collisions. Wired into `App::new_formatter`. Structured mode in `main.rs` (no colors, no pager). Delivers User Stories 1 and 2 for default settings.
4. **Normalizer, output-key tree**: dotted output keys, merging into regular objects, split caller, input field. Preset updates (FR-028).
5. **Diagnostics**: observer hook, DropStats, debug entries with line numbers (cat, follow, then sort), summaries and the exit hook. Delivers User Story 4.
6. **Follow and polish**: follow flush (User Story 3), README and help text, benchmarks, property tests, `just uncovered`.

## Complexity Tracking

No constitution violations to justify.
