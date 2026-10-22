---

description: "Task list for machine-readable output formats"
---

# Tasks: Machine-Readable Output Formats

**Input**: Design documents from `specs/011-output-format/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Included. The constitution (principle V) makes test-first development mandatory: write each phase's tests first and confirm they fail before implementing.

**Organization**: Tasks are grouped by user story. Configurable output keys, presets and the input field (FR-026 to FR-029) extend User Story 1's schema; they get their own phase after User Story 4, so the MVP stays small.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on unfinished tasks)
- **[Story]**: The user story the task belongs to (US1 to US4)
- Paths are relative to the repository root

## Conventions for every task

- Unit tests live in a `tests.rs` file next to the module, declared with `#[cfg(test)] mod tests;`, as in the rest of `src/`.
- Test data longer than a couple of lines goes in `src/testing/assets/` (constitution VII).
- Format with `just fmt-rust`, not plain `cargo fmt`.
- Run `just ci` before every commit. Commit at each checkpoint at the latest.
- No version bump on this branch; the PR merging it into `develop` sets it, and its commit ends with `Closes: #1481`.

---

## Phase 0: Prerequisite refactoring (separate PR to `develop`)

**Purpose**: Behavior-neutral groundwork that later phases build on: per-record origin and per-input descriptors for the formatter, and the badge name computation split from its rendering (research R8, R9). It lands on `develop` before this feature, like the quoted-key parser fix (#1607).

**Branch**: `refactor/record-origin` from `develop`, in its own worktree. The PR bumps the version with `just bump` (no SemVer impact, but it changes the built program), its commit carries `Refs: #1481` since it resolves no issue of its own, and `feature/output-format` is rebased onto `develop` after it merges.

**Independent Test**: `pretty` and `--raw` output stay byte-identical: all existing tests pass unchanged, and `parse-and-format` in `benches/bench/ws/hl/combined.rs` doesn't regress.

### Tests (write first, confirm they fail)

- [x] T000a In src/app/tests.rs, with a capturing `RecordWithSourceFormatter`, check for `cat`, `sort` and `follow` over two inputs that each formatted record's `origin.input` refers to the descriptor of the input it came from (index and name, `<stdin>` for standard input), and that `origin.range` is the record's byte range in that input, also across segments and index blocks.
- [x] T000b [P] In src/app/tests.rs, test the new input name computation directly: full names and compact names (with clipping and the common-part marker) for the inputs already used by the `prepare_follow_badges` and `input_badges` tests. The existing badge tests stay unchanged and keep passing.

### Implementation

- [x] T000c In src/input.rs, add `pub struct InputDescriptor { pub index: usize, pub name: String }`, with `From<(usize, String)>` and `From<(usize, &InputReference)>`. In src/model.rs, add `#[derive(Clone, Debug)] pub struct Origin<'r> { pub input: &'r InputDescriptor, pub range: Range<u64> }` with `new` and `slice`, and a required `pub origin: Origin<'r>` field on `RecordWithSource`, taken by its constructors.
- [x] T000d In src/app.rs, add the origin of the processed data (`Origin`) as a parameter of `SegmentProcess::process`, and build the descriptors once per run in `cat`, `sort` and `follow`, before any record is processed. Readers send each segment with its byte offset in the input: a running sum of segment sizes in `cat`, kept across scans by `BlockScanner` in `follow`, starting at the position after `--tail` and reset when a file is replaced, and the block offset plus the entry offset in `sort`. Offsets are in the decompressed data. `SegmentProcessor` sets each record's origin to its slice of the data's origin. Update every call site in src/app.rs, src/app/tests.rs and benches/bench/ws/hl/combined.rs.
- [x] T000e In src/app.rs, split `App::input_badges` into a name computation (full names, and compact names with the existing clipping rules and resolved punctuation) and the themed, padded badge rendering, which uses the computed names. Badge output is unchanged.
- [x] T000f Run `just ci`. Compare `cargo bench --bench bench -- combined` with `develop`. Bump with `just bump` after rebasing onto `develop`, and open the PR with a `refactor(app): ...` title and the benchmark comparison.

**Checkpoint**: The PR is merged into `develop`, and `feature/output-format` is rebased onto it.

---

## Phase 1: Setup

**Purpose**: Dependencies, module skeleton and test fixtures

- [ ] T001 Add `proptest` to `[workspace.dependencies]` and as a dev-dependency of the `hl` package in Cargo.toml (research R17)
- [ ] T002 [P] Create the module skeleton:
  - `src/formatting/structured.rs`, declaring `mod json; mod logfmt; mod keyset;` and `#[cfg(test)] mod tests; #[cfg(test)] mod proptests;`;
  - empty `src/formatting/structured/json.rs`, `logfmt.rs` and `keyset.rs`, each with its own `#[cfg(test)] mod tests;`;
  - `pub mod structured;` in src/formatting.rs.
- [ ] T003 [P] Create the fixtures in `src/testing/assets/fixtures/output-format/`:
  - `mixed.log`: JSON records of mixed levels, with nested objects, arrays, a `secret` field, a multi-line pretty-printed record, a plain-text stack trace line, an empty line and a whitespace-only line;
  - `types.logfmt`: unquoted numbers including `+5`, `.5`, `5.`, `007` and `5.e3`, booleans, `null`, quoted `"42"`, `"true"` and `""`, the quoted keys `"user name"` and `"items[0]"`, and repeated keys `tag=a tag=b`;
  - `timestamps.log`: one record per FR-007 example (Unix s, ms, µs and ns; fractional seconds and milliseconds; a textual timestamp with trailing zeros; zone-less; a non-UTC offset; unparsable string; unparsable number);
  - `ecs.log` and `k8s.log`: records in each preset's format.

---

## Phase 2: Foundational

**Purpose**: Options, flatten mode and structured-mode plumbing that every story needs

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

### Tests (write first, confirm they fail)

- [ ] T004 [P] Settings tests in src/settings/tests.rs:
  - `formatting.flatten` accepts `"auto"`, `"never"` and `"always"`;
  - the embedded default configuration resolves `formatting.flatten` to `auto`.
- [ ] T005 [P] CLI tests in src/cli/tests.rs:
  - `--output-format` accepts `pretty`, `json` and `logfmt`, defaults to `pretty`, and rejects other values;
  - `HL_OUTPUT_FORMAT` is used when the option is absent, and the option wins over it;
  - a repeated option overrides the earlier one;
  - `--flatten` accepts `auto` and defaults to `auto`.

### Implementation

- [ ] T006 Add `Auto` to `settings::FlattenOption` in src/settings.rs. Set `flatten = "auto"` in etc/defaults/config.toml, with its comment listing `auto`, `never` and `always`. Change the `formatting.flatten` enum in schema/json/config.schema.json to `["auto", "never", "always"]`.
- [ ] T007 In src/cli.rs:
  - add `Auto` to `cli::FlattenOption`, map it in both `From` impls, and make the `--flatten` default fall back to `Auto`;
  - add `#[derive(ValueEnum)] pub enum OutputFormat { Pretty, Json, Logfmt }` and an `output_format` option with `long`, `env = "HL_OUTPUT_FORMAT"`, `default_value = "pretty"`, `overrides_with = "output_format"`, `value_name = "FORMAT"` and `help_heading = heading::OUTPUT`, with the help text from contracts/cli.md.
- [ ] T008 In src/app.rs:
  - add `pub enum OutputFormat { Pretty, Json, Logfmt }` and `pub output_format: OutputFormat` to `Options`, with `is_structured()`;
  - set `output_format: OutputFormat::Pretty` in `options()` in src/app/tests.rs and in every other `Options` literal (`grep -rn "hl::Options {" src benches`).
- [ ] T009 In src/main.rs:
  - compute the effective output format: `Pretty` whenever `opt.raw` is set (FR-016);
  - resolve `opt.flatten`: `Auto` gives `true` for `Pretty` and `Logfmt` and `false` for `Json`;
  - when the format is structured: use `Theme::none()` whatever the color options say, never start a pager, use `opt.output_delimiter`, and skip the terminal-width input-info resolution (FR-017);
  - pass `output_format` into `hl::Options`.
- [ ] T010 In src/app.rs, when `options.output_format.is_structured()`:
  - `input_badges` returns `None`;
  - `prepare_follow_badges` yields a `SyncIndicator` with width 0 and empty strings;
  - `new_segment_processor` sets `allow_unparsed_data: false` (FR-017, FR-018).
- [ ] T011 Run `cargo build`, then `just ci`, since the schema changed (constitution V). Verify that `pretty` output is unchanged: the existing tests in src/app/tests.rs and src/formatting/tests.rs pass without edits (FR-002).

**Checkpoint**: `--output-format` parses and structured mode suppresses colors, pager, badges and unparsed lines, but still formats with the `pretty` formatter until US1 wires in the new one.

---

## Phase 3: User Story 1, JSON output for pipelines (Priority: P1) 🎯 MVP

**Goal**: `--output-format=json` emits one compact, normalized JSON object per selected record, with default output keys.

**Independent Test**: `hl --output-format=json -l warning src/testing/assets/fixtures/output-format/mixed.log | jq -c .` parses every line, and outputs only the matching records with `timestamp`, `level`, `message` first (quickstart scenarios 1, 2 and 5).

### Tests for User Story 1 (write first, confirm they fail)

- [ ] T012 [P] [US1] Tests for `Timestamp::format_rfc3339_utc` in src/timestamp/tests.rs, one case per FR-007 rule:
  - `2026-07-06T10:00:00.123Z` is copied verbatim;
  - `2026-07-06T12:00:00.5+02:00` gives `2026-07-06T10:00:00.5Z`;
  - Unix `1700000000` gives `2023-11-14T22:13:20Z`, `1700000000000` gives `.000Z`, the microsecond value gives 6 digits and the nanosecond value 9;
  - `1700000000.25` gives `.25`, and milliseconds `1700000000000.5` give `.0005`;
  - `10:00:00.10` keeps `.10`, and a source without a fraction gets none;
  - `2026-07-06 10:00:00.123` is interpreted as UTC;
  - an explicit `UnixTimestampUnit` is honored;
  - a year outside 0000 to 9999, and an unparsable value, return `false` and write nothing.
- [ ] T013 [P] [US1] Tests for the key set in src/formatting/structured/keyset/tests.rs: insert reports new versus present; keys with equal hashes but different text are distinct; more than the inline capacity spills correctly.
- [ ] T014 [P] [US1] Tests for the JSON encoder in src/formatting/structured/json/tests.rs:
  - a JSON string without a backslash is copied verbatim;
  - `\/` and `A` are normalized;
  - a lone surrogate `\ud800` becomes U+FFFD;
  - non-ASCII text is written as UTF-8;
  - a raw string with `"`, `\` and control characters is escaped;
  - valid JSON numbers are copied verbatim;
  - logfmt numbers are rewritten as in FR-010a: `+5` → `5`, `.5` → `0.5`, `5.` → `5`, `007` → `7`, `-007.50` → `-7.50`, `5.e3` → `5e3`, `+.5e-3` → `0.5e-3`;
  - output is compact, with no whitespace.
- [ ] T015 [P] [US1] Normalizer tests with the JSON encoder in src/formatting/structured/tests.rs, using default output keys:
  - **order and presence**: predefined order time, level, logger, message, caller, then regular fields in source order (FR-005, FR-009); a missing predefined key is omitted;
  - **predefined values**: `WARN` gives `"warning"` (FR-006); an unrecognized level stays among the regular fields under its source key; an object message stays an object and an unparsable numeric timestamp stays a number (FR-005a);
  - **caller**: combined forms `name`, `file:line`, `file` and `name @ file:line` (FR-008);
  - **repeats**: `tag=a tag=b` gives one `tag` at the first position with value `b`, at the top level and inside a nested object (FR-009a);
  - **collisions**: a raw `message` field next to `msg` is dropped (FR-027), and nested predefined source fields such as `span.name` are not repeated (FR-027b);
  - **flattening**: `never` nests, `always` gives dotted keys, flattening stops at arrays (`"items":[{"id":1,"tags":{"a":1}}]`), and `{"a":{"b":1}}` with a literal `"a.b"` under `always` is a repeat (FR-009a, FR-012);
  - **hiding**: `-h secret` hides a regular field, hiding a nested key leaves its siblings, `!` reveals, `-h caller` and `-h message` hide predefined fields by output key, `--hide-empty-fields` drops an empty message and empty objects, and hide rules don't reach into arrays (FR-014);
  - **compaction**: a multi-line pretty-printed input record is emitted on one line.
- [ ] T016 [P] [US1] Property test in src/formatting/structured/proptests.rs: for generated JSON and logfmt records, every JSON output line parses with `serde_json` as an object with unique keys (SC-001).
- [ ] T017 [P] [US1] App-level tests in src/app/tests.rs with `output_format: OutputFormat::Json`:
  - `cat` over mixed.log with a level filter outputs only the matching records, with no unparsed lines;
  - `sort` over two inputs is chronological;
  - the record counts equal `pretty`'s for the same options in `cat` and `sort` (SC-002);
  - `raw: true` with JSON gives today's raw output (FR-016);
  - the `"\0"` output delimiter separates records;
  - a prefix with `allow_prefix` is not emitted (FR-018).

### Implementation for User Story 1

- [ ] T018 [P] [US1] Implement `Timestamp::format_rfc3339_utc(&self, buf: &mut Vec<u8>) -> bool` in src/timestamp.rs (research R6):
  - derive the precision from the source text, with the unit resolved as in `reparse` (an explicit unit, else `UnixTimestampUnit::guess`);
  - copy an RFC 3339 `Z` source verbatim;
  - otherwise write `%Y-%m-%dT%H:%M:%S`, `.` and N digits of the zero-padded nanoseconds when N > 0, then `Z`;
  - return `false` for unparsable values and for years outside 0000 to 9999.
- [ ] T019 [P] [US1] Implement `KeySet<'a>` in src/formatting/structured/keyset.rs: `heapopt::Vec<(u64, &'a str), 32>` hashed with `wyhash`, with `insert(&mut self, key: &'a str) -> bool` and `clear()`, comparing strings only on hash equality (research R3).
- [ ] T020 [P] [US1] In src/formatting/structured.rs, define the `Encoder` trait: `begin_record`, `end_record`, `key`, `string(EncodedString)`, `number(&str, NumberSource)`, `boolean`, `null`, `begin_object`, `end_object`, `begin_array`, `end_array`, and `flattens()`. Implement `JsonEncoder` in src/formatting/structured/json.rs:
  - strings verbatim when they have no backslash, otherwise decoded and re-encoded through the `encstr` JSON appender;
  - the FR-010a number rewrite for logfmt-sourced numbers only;
  - commas and colons with no whitespace.
- [ ] T021 [US1] Implement `StructuredRecordFormatter<E: Encoder>` and its builder in src/formatting/structured.rs (research R2 to R4, R7):
  - **settings**: output keys, defaulting to `timestamp`, `level`, `logger`, `message` and `caller`; the flatten flag; the user hide filter (`Arc<IncludeExcludeKeyFilter>`); the predefined filter built like `App::build_predefined_filter`; `hide_empty_fields`;
  - **predefined section**: time via `format_rfc3339_utc`, falling back to the raw value with its source type; level from `Level::as_ref()`; logger; message with its source type; the combined caller; all subject to the hide filter matched on output keys;
  - **regular section**: the repeat pre-pass with `KeySet`, the collision check against emitted keys, `RawValue::auto` typing for `EncodedString::Raw`, and recursive object re-encoding with per-level repeat handling, hide filters and the predefined filter, flattened under `always`;
  - arrays re-encoded element by element with no filters;
  - repeats and collisions dropped silently for now (US4 adds the debug entries);
  - implements `RecordWithSourceFormatter`, writing nothing for the prefix range.
- [ ] T022 [US1] In `App::new_formatter` in src/app.rs, build `StructuredRecordFormatter<JsonEncoder>` from `Options` (flatten, `fields.filter`, the predefined filter, `hide_empty_fields`) when the effective output format is `Json`. `--raw` keeps `RawRecordFormatter`.

**Checkpoint**: User Story 1 works end to end with default settings. Run quickstart scenarios 1, 2 and 5, and commit.

---

## Phase 4: User Story 2, convert between JSON and logfmt (Priority: P2)

**Goal**: `--output-format=logfmt` emits records that hl's and standard logfmt parsers read back with the same keys, values and types.

**Independent Test**: `hl --output-format=json src/testing/assets/fixtures/output-format/types.logfmt | hl --output-format=logfmt` preserves keys, values and types (quickstart scenarios 3 and 4).

### Tests for User Story 2 (write first, confirm they fail)

- [ ] T023 [P] [US2] Predicate tests in crates/serde-logfmt/src/logfmt/de/tests.rs:
  - `is_bare_key`: true for `a`, `a.b`, `café`, `a-b_c`; false for `""`, `user name`, `items[0]`, `{a`, `a,b`, `'a'`, `a"b`, `a=b`;
  - `is_bare_value`: false for `""`, values with a space, `=`, `"` or a control character;
  - property: for generated keys, `is_bare_key(k)` holds exactly when `from_str::<HashMap<String,String>>(&format!("{k}=1"))` succeeds with key `k`.
- [ ] T024 [P] [US2] Tests for the logfmt encoder in src/formatting/structured/logfmt/tests.rs:
  - bare `message=started`, quoted `"slow request"`;
  - quoting for `=`, `"` and control characters, with JSON escapes and non-ASCII kept as UTF-8;
  - `"42"`, `"-1.5"`, `"true"`, `"null"` and a string starting with `"` are quoted (FR-004);
  - an empty string is `""`;
  - keys: `"user name"=x`, `"items[0]"=x`, `""=x`, and bare `a.b=x` (FR-004a);
  - numbers verbatim, including `+5` (FR-010a);
  - objects under `never` are quoted compact JSON, `http="{\"status\":200}"`, and under `always` are dotted, `http.status=200`;
  - arrays are quoted compact JSON (FR-013).
- [ ] T025 [P] [US2] Round-trip property tests in src/formatting/structured/proptests.rs, through hl's own parsers and formatters:
  - logfmt → JSON → logfmt preserves keys, values and types;
  - JSON → logfmt → JSON preserves them as well;
  - every logfmt output line parses with `serde_logfmt::logfmt::from_str` (SC-001, SC-003).
- [ ] T026 [P] [US2] App-level tests in src/app/tests.rs for `OutputFormat::Logfmt`: User Story 2 acceptance scenarios 1 to 4, including JSON's default nesting versus `--flatten=always` dotted keys.

### Implementation for User Story 2

- [ ] T027 [P] [US2] In crates/serde-logfmt/src/logfmt/de.rs, add `pub fn is_bare_key(s: &str) -> bool` (non-empty, every byte `KeyCh::ALLOWED` or `KeyCh::UNICODE` in `KEY`) and `pub fn is_bare_value(s: &str) -> bool` (non-empty, no byte `<= b' '`, no `"`, no `=`). Re-export both from crates/serde-logfmt/src/logfmt/mod.rs (research R5).
- [ ] T028 [US2] Implement `LogfmtEncoder` in src/formatting/structured/logfmt.rs:
  - `flattens()` follows the flatten flag;
  - keys bare when `is_bare_key` holds, else quoted JSON strings;
  - strings decoded, bare when `is_bare_value` holds, they don't start with `"` and `RawValue::auto` types them as a string, else quoted;
  - numbers, booleans and null verbatim;
  - nested objects under `never`, and arrays, rendered with a `JsonEncoder` into a scratch buffer, then written as a quoted string;
  - pairs separated by single spaces.
- [ ] T029 [US2] In `App::new_formatter` in src/app.rs, build `StructuredRecordFormatter<LogfmtEncoder>` when the effective output format is `Logfmt`.

**Checkpoint**: Both formats work with default settings. Run quickstart scenarios 3 and 4, and commit.

---

## Phase 5: User Story 3, follow with structured output (Priority: P3)

**Goal**: `hl -F --output-format=json` streams each new matching record promptly as a complete line, with no decoration.

**Independent Test**: Quickstart scenario 7.

### Tests for User Story 3 (write first, confirm they fail)

- [ ] T030 [US3] App-level follow tests in src/app/tests.rs, modeled on the existing follow tests, with `OutputFormat::Json` and two inputs:
  - records from both inputs appear as complete JSON lines, with no sync indicator or badge bytes;
  - with `"\0"` as the output delimiter, the output is flushed after each batch (use a `Write` mock that counts `flush` calls).

### Implementation for User Story 3

- [ ] T031 [US3] In `merge_segments` in src/app.rs, call `output.flush()` after each pass that wrote at least one record (research R14). Check that the zero-width sync indicator from T010 writes no bytes in structured mode.

**Checkpoint**: Run quickstart scenario 7, and commit.

---

## Phase 6: User Story 4, diagnose dropped lines (Priority: P4)

**Goal**: Dropped lines, repeats and collisions are reported on stderr through `HL_DEBUG_LOG`, with per-source summaries at exit, while stdout stays clean.

**Independent Test**: Quickstart scenario 6.

### Tests for User Story 4 (write first, confirm they fail)

- [ ] T032 [P] [US4] Tests in src/diagnostics/tests.rs, using a capturing test sink rather than the global logger:
  - `DropStats` counts per input;
  - the preview keeps at most 100 characters, cuts at a character boundary and escapes control characters;
  - the summary lists only inputs with a non-zero count;
  - the exit hook writes once even when called twice.
- [ ] T033 [P] [US4] App-level tests in src/app/tests.rs with a capturing sink, for `cat`, `sort` and `follow` over mixed.log in JSON mode:
  - counts and byte ranges of dropped lines, remainders and prefixes are correct;
  - empty and whitespace-only lines are neither counted nor reported;
  - in `sort`, dropped lines in blocks without timestamps, and in blocks excluded by a time filter, are still counted;
  - the repeat and collision debug entries name the source, the record's byte range and the key (FR-009a, FR-027);
  - in `pretty` mode the sink receives nothing.

### Implementation for User Story 4

- [ ] T034 [US4] Create src/diagnostics.rs and declare it in src/lib.rs:
  - `DropKind { Unparsed, Remainder, Prefix }`;
  - `DropStats` with `Arc<[AtomicU64]>` and source names;
  - a `DiagnosticSink` trait, whose default implementation writes through `log::debug!` and `log::info!`;
  - `report_dropped(source, range, kind, chunk)` with the preview rules;
  - `report_repeat` and `report_collision`;
  - the summary writer;
  - a once-only exit hook (`std::sync::Once` plus a registered `Arc<DropStats>`).
- [ ] T035 [US4] In src/app.rs:
  - add `fn observe_dropped(&mut self, _chunk: &[u8], _kind: DropKind) {}` with a default body to `RecordObserver`;
  - in `SegmentProcessor::process`, call it for every unparsed chunk, remainder and prefix when `show_unparsed()` is false, skipping chunks that are empty or contain only whitespace;
  - pass the dropped content's origin, sliced from the data's origin from T000d, to `observe_dropped`.
- [ ] T036 [US4] In `cat` in src/app.rs, wire a drop observer that updates `DropStats` and reports entries.
- [ ] T037 [US4] In `follow` in src/app.rs, wire the drop observer in `process_segments`.
- [ ] T038 [US4] In `sort` in src/app.rs (research R9):
  - in structured mode, send blocks that the pusher would skip but whose `stat.entries_invalid > 0` to workers in a diagnostics-only mode, which reports drops and emits no items.
- [ ] T039 [US4] Call the exit hook:
  - at the end of `App::run` in src/app.rs;
  - in `shutdown` in src/signal_common.rs, before `exit`, which covers SIGINT, SIGTERM and the Windows console control handler;
  - before `process::exit` in `ExitOnWriteError::inspect` in src/main.rs (research R10).
- [ ] T040 [US4] In src/formatting/structured.rs, replace the silent drops from T021 with `diagnostics::report_repeat` and `report_collision` calls, guarded by `log::log_enabled!(log::Level::Debug)`. They take the source name from `rec.origin.input` and the byte range from `rec.origin.range`.

**Checkpoint**: Run quickstart scenario 6, and commit.

---

## Phase 7: User Story 1 (continued), configurable output keys, presets and input field

**Goal**: FR-026 to FR-029: per-field `output-key`, the split caller, dotted keys with merging, the ECS and k8s presets, and the opt-in input field.

**Independent Test**: Quickstart scenario 8. Also `hl --input-info=minimal --output-format=json a.log b.log` adds `"input":0` or `"input":1` after the predefined fields.

### Tests (write first, confirm they fail)

- [ ] T041 [P] [US1] Settings tests in src/settings/tests.rs, with configuration files in src/testing/assets/configs/:
  - `output-key` loads for all seven predefined fields and for `fields.predefined.input`;
  - an empty value fails with an error naming `fields.predefined.<field>.output-key`;
  - two fields with the same effective key fail with an error naming both;
  - `caller-file` and `caller-line` defaults only take part in the duplicate check when splitting is active;
  - the embedded ECS and k8s presets load with the FR-028 values.
- [ ] T042 [P] [US1] Normalizer tests in src/formatting/structured/tests.rs:
  - custom output keys replace the defaults;
  - split caller: name, file string, and line as a number or a string (FR-008a);
  - dotted keys under `always` are written dotted, and under `never` are nested (FR-027a);
  - merging into a regular `log` object: the object moves to the first predefined position, with predefined entries first and its remaining entries after;
  - a nested group `log.origin` merges with a regular `log.origin` object;
  - a non-object regular `log` field is a collision;
  - ECS `fields.hide = ["log"]` still emits the predefined entries in `log`, and only those (FR-027b);
  - the input field for `minimal` (a number), `compact` and `full` (strings), and `none` with a single input (omitted);
  - a custom input key, and its collision with a regular field (FR-029).

### Implementation

- [ ] T043 [US1] In src/settings.rs and src/settings/error.rs:
  - add `#[serde(default, rename = "output-key")] pub output_key: Option<String>` to `Field` and `LevelField`;
  - add `InputField { output_key }` and `PredefinedFields.input` with `#[serde(default)]`;
  - add `Settings::validate()`, called from `Settings::load`, enforcing "MUST be non-empty" and "pairwise distinct" over the effective keys, with errors that name the setting path.
- [ ] T044 [P] [US1] In schema/json/config.schema.json, add `output-key` (`type: string`, `minLength: 1`) to every predefined field definition, and add `fields.predefined.input` with only `output-key`. Then run `cargo build` and `just ci`.
- [ ] T045 [P] [US1] Set the FR-028 output keys:
  - etc/defaults/config-k8s.toml: time `ts`, message `msg`, caller `caller`; no level key;
  - etc/defaults/config-ecs.toml: `@timestamp`, `log.level`, `log.logger`, `message`, `log.origin.function`, `log.origin.file.name`, `log.origin.file.line`;
  - etc/defaults/config.toml: a commented `output-key` example for each predefined field.
- [ ] T046 [US1] In src/formatting/structured.rs (research R2):
  - read output keys and caller mode from `PredefinedFields`;
  - build the output-key tree at construction, only for `never` with dotted keys;
  - write each group once, at its first predefined position, merging one matching regular object per level found by a scan of that level's fields, and marking it consumed;
  - for groups hidden by user filters, emit only the predefined entries;
  - add the split caller.
- [ ] T047 [US1] In src/app.rs (research R8):
  - compute the requested layouts as the configured set minus `auto`;
  - add the pre-rendered input field value (the number, or the compact or full name) to `InputDescriptor` from T000c, using the names computed by T000e, so it reaches the formatter through `RecordWithSource::origin`;
  - in src/formatting/structured.rs, add the field after the predefined section, with collision handling.
- [ ] T048 [US1] In src/main.rs, pass the requested input-info layouts unresolved to `App` for structured formats, and keep today's resolution for `pretty`.

**Checkpoint**: Run quickstart scenario 8, and commit.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [ ] T049 [P] In benches/bench/ws/hl/combined.rs, add `format-json` and `format-logfmt` cases next to `parse-and-format` for the existing samples. Add a `structured` case using the ECS preset with a nested sample in benches/bench/samples/ (research R16).
- [ ] T050 [P] In README.md, regenerate the options listing from `hl --help`, which now shows `--output-format` and `--flatten` with `auto`. Add a short "Machine-readable output" section with the jq and conversion examples from quickstart.md.
- [ ] T051 [P] Review the help text in src/cli.rs for `--output-format` and `--flatten`, and the comments in etc/defaults/config.toml, against contracts/cli.md.
- [ ] T052 Run every quickstart.md scenario against `target/debug/hl` and fix any deviation from the expected results.
- [ ] T053 Run `just uncovered` and add tests for uncovered changed lines. Document any that need environmental interaction, such as signal and console handlers (constitution V, SC-007).
- [ ] T054 Run `cargo bench --bench bench -- combined` on `develop` and on this branch. Record `pretty` before and after (SC-004: at most 5% regression) and structured against `pretty` (SC-005, a target) for the PR description.
- [ ] T055 Run `just ci` for the final gate.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Prerequisite refactoring (Phase 0)**: lands on `develop` first, as its own PR; every later phase builds on it after the rebase.
- **Setup (Phase 1)**: depends on Phase 0 being merged and rebased in.
- **Foundational (Phase 2)**: depends on Setup; blocks every user story.
- **US1 (Phase 3)**: depends on Foundational.
- **US2 (Phase 4)**: depends on US1's `Encoder` trait, `StructuredRecordFormatter` and `JsonEncoder` (T020, T021); `LogfmtEncoder` reuses `JsonEncoder` for nested values.
- **US3 (Phase 5)**: depends on US1 (T022); independent of US2 and US4.
- **US4 (Phase 6)**: depends on Foundational. T040 depends on T021. The rest is independent of US2 and US3.
- **US1 continued (Phase 7)**: depends on US1. T047 depends on T000c, the input descriptor.
- **Polish (Phase 8)**: depends on every phase.

### Within Each Phase

- Tests first, and they must fail before implementation starts.
- In US1, T018 to T020 come before T021, and T021 before T022.
- In US2, T027 comes before T028, and T028 before T029.
- In US4, T034 comes before T035, which comes before T036 to T040.

### Parallel Opportunities

- **Setup**: T002 and T003.
- **Foundational**: tests T004 and T005.
- **US1**: tests T012 to T017; implementation T018, T019 and T020, which touch different files.
- **US2**: tests T023 to T026; T027 alongside T024 to T026.
- **US4**: tests T032 and T033.
- **Phase 7**: tests T041 and T042; T044 and T045 alongside T043.
- **Across stories**: once US1 is complete, US3 and US4 can proceed in parallel with US2.
- **Polish**: T049, T050 and T051.

---

## Parallel Example: User Story 1

```text
# Tests, all in different files:
Task: "T012 Timestamp precision tests in src/timestamp/tests.rs"
Task: "T013 Key set tests in src/formatting/structured/keyset/tests.rs"
Task: "T014 JSON encoder tests in src/formatting/structured/json/tests.rs"
Task: "T015 Normalizer tests in src/formatting/structured/tests.rs"
Task: "T016 JSON validity property test in src/formatting/structured/proptests.rs"
Task: "T017 App-level JSON tests in src/app/tests.rs"

# Then the independent building blocks:
Task: "T018 Timestamp::format_rfc3339_utc in src/timestamp.rs"
Task: "T019 KeySet in src/formatting/structured/keyset.rs"
Task: "T020 Encoder trait and JsonEncoder in src/formatting/structured.rs and json.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Phase 0 as a separate PR to `develop`, then Phase 1 (Setup) and Phase 2 (Foundational).
2. Phase 3 (US1): JSON output with default output keys.
3. **Stop and validate** with quickstart scenarios 1, 2 and 5. This alone covers the main use cases of #1481.

### Incremental Delivery

1. Setup and Foundational, then US1 (MVP).
2. US2: logfmt and conversion.
3. US3: follow mode.
4. US4: diagnostics.
5. US1 continued: configurable keys, presets and the input field.
6. Polish, then the PR to `develop` with the version bump, the benchmark results and a `Closes: #1481` footer.

Each step leaves `pretty` output byte-identical and `just ci` green.

---

## Notes

- [P] tasks touch different files and depend on no unfinished task.
- Commit after each task or logical group, using the conventional commit format from the constitution: `test(...)`, `feat(output): ...`, and so on, with `Refs: #1481`.
- Stop at any checkpoint to validate a story on its own.
