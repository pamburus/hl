# Research: Machine-Readable Output Formats

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Date**: 2026-10-21

Each decision lists what was chosen, why, and what else was considered. References like `src/app.rs` describe the code as of commit `1dfc8536`.

## R1. Where structured output plugs into the pipeline

**Decision**: Add a `StructuredRecordFormatter` that implements the existing `RecordWithSourceFormatter` trait and is selected in `App::new_formatter` next to `RawRecordFormatter` and `RecordFormatter`. It is generic over an `Encoder` (JSON or logfmt), so each format is monomorphized and the per-record path has no dynamic dispatch below the formatter boundary.

**Rationale**: `App` already holds a `DynRecordWithSourceFormatter` and every mode (`cat`, `sort`, `follow`) calls `format_record` on it through `SegmentProcessor`. Plugging in at this boundary reuses parsing, filtering, sorting, following, `--tail` and multiple inputs unchanged, which is what FR-015 and SC-002 require. Nothing on the `pretty` path changes, which protects FR-002 and SC-004.

**Alternatives considered**:
- A separate pipeline for structured output: duplicates the threading and merge logic of three modes, and the record sets could drift apart.
- Serializing via `serde::Serialize` on a normalized struct: requires building an owned intermediate per record and gives no control over key order, deduplication or logfmt quoting.

## R2. Normalized record: streaming with a precomputed output-key tree

**Decision**: The formatter streams key/value pairs straight into the output buffer in the order of FR-005, FR-029 and FR-009.

Dotted output keys (FR-027a) are handled by an output-key tree built once at startup from the configuration. It applies only when the effective flatten mode is `never` and at least one emitted output key contains a dot, which with the bundled configurations means the ECS preset in JSON mode. Each tree node is a group, such as `log` or `log.origin`, that lists the predefined entries it receives in FR-005 order. For each record:
- a group is written once, at the position of its first predefined entry;
- before a group's `{` is written, one scan of the record's fields at the same level looks for a regular object with the group's key;
- the group's predefined entries are written first, then the matching regular object's other entries in source order, recursing for nested groups such as `log.origin`;
- the matched regular field is marked as consumed, so it isn't written again among the regular fields.

**Rationale**: The tree's shape depends only on configuration, so per record the extra cost is one scan of a level's fields per group, plus the merge when a matching object exists. Nothing is allocated per record, so this path stays close to the plain streaming path and SC-005 still applies to it. With default settings and the k8s preset, no output key has a dot and the tree is empty.

**Alternatives considered**: Building a tree of the whole record per record: simpler, but it allocates on every record and makes SC-005 unreachable for large records.

## R3. Key uniqueness (FR-009a, FR-027)

**Decision**:
- Use a per-record key set of `(hash, &str)` entries with inline capacity: `heapopt::Vec`, which hl already uses for record fields, and `wyhash`, which is already a dependency. Strings are compared only when hashes match.
- The emitted predefined output keys are inserted first. A regular field whose key is already present is dropped and reported (FR-027).
- Repeated source keys (FR-009a) are found in a pre-pass that hashes the keys at one nesting level. The common case without repeats sets a flag and streams normally. Only when repeats exist is a "last occurrence index" computed, so each key is emitted at its first position with its last value.
- The same pre-pass runs for every object that is re-encoded (one nesting level at a time). With `--flatten=always`, it runs on full dotted paths.
- With `unique-keys = false` for the output format, the pre-pass is skipped and every level is written in source order. The collision check of FR-027 still runs, as it only compares keys with the few emitted output keys.

- **Memory**: formatting allocates nothing per record once warmed up. The pre-pass works on field indices, and its stacks, the flattened key and the leaves of dotted-key subtrees live in a per-thread scratch that each record reuses, since the formatter is shared by the worker threads. Under `--flatten=always`, a level with a dotted key resolves its whole subtree together, walking it again instead of storing values. In logfmt, an object or array value is written as compact JSON and then quoted in place in the output buffer, so it needs no scratch either.

**Rationale**: Records usually have fewer than 32 fields, so a linear scan over hashes beats a hash map and avoids allocation. Running the pre-pass per object keeps the "same nesting level" rule of FR-009a exact.

**Alternatives considered**:
- `HashSet<&str>` per record: allocates.
- Sorting the keys: changes the order, which FR-009 forbids.

## R4. Value encoding (FR-010, FR-010a, FR-011)

**Decision**:
- **Logfmt-sourced unquoted strings** (`EncodedString::Raw`) are typed with `RawValue::auto`, the same function the `pretty` formatter uses. This gives numbers, booleans and null under exactly hl's existing rules (FR-010).
- **JSON strings** (and quoted logfmt strings, which use the same encoding) are copied verbatim when they contain no backslash. Otherwise they are decoded and re-encoded with the existing `encstr` JSON appender. This normalizes escapes such as `\/` and `A` and replaces invalid sequences such as lone surrogates with U+FFFD (FR-011, edge case "Invalid UTF-8"). Non-ASCII characters are written as UTF-8.
- **JSON-sourced numbers** are always valid JSON and are copied verbatim.
- **Logfmt-sourced numbers** in JSON mode are checked with a single-pass validator. Only invalid ones are rewritten by the rules of FR-010a: strip a leading `+` and redundant leading zeros, add `0` before a leading `.`, and drop a `.` that has no fraction digits. In logfmt mode, numbers are copied verbatim.
- **Objects and arrays** are re-encoded compactly by walking the parsed `Object`/`Array`, because whitespace must be removed (multi-line input), keys deduplicated and hide filters applied. Arrays are re-encoded element by element, but no filters are applied inside them (FR-012, FR-014). Scalars inside them follow the rules above.

**Rationale**: Copying verbatim avoids decoding in the common case. Reusing `RawValue::auto` guarantees that structured output types values exactly as `pretty` displays them, which is the stability argument of the spec's input.

## R5. Logfmt output and round trip (FR-004, FR-004a, SC-003)

**Decision**: The `serde-logfmt` crate exports predicates that use the parser's own lookup tables:
- `is_bare_key(&str)`: non-empty, and every byte allowed by the `KEY` table;
- `is_bare_value(&str)`: non-empty, with no control character, space, `"` or `=`.

The logfmt encoder writes a key bare when `is_bare_key` holds, and quoted as a JSON string otherwise. A string value is written bare only when `is_bare_value` holds, it doesn't start with `"`, and `RawValue::auto` would type it as a string. Otherwise it is quoted. The empty string is written as `""`.

**Rationale**: Sharing the tables makes "hl reads its own logfmt output back" hold by construction. If the parser's bare-key set ever changes, the writer follows it automatically. This builds on the quoted-key parser fix (#1607), which is already merged.

**Alternatives considered**: A separate character table in the writer: it can drift from the parser.

## R6. Timestamp normalization (FR-007)

**Decision**: Add `Timestamp::format_rfc3339_utc(&self, buf) -> bool` to `src/timestamp.rs`:
- **Precision**: derived from the source text, not from the parsed value.
  - Integer Unix values: the resolved unit gives 0, 3, 6 or 9 digits.
  - Fractional Unix values: the unit's digits plus the source's fraction digits, clamped to 9.
  - Textual values: the number of fraction digits in the source, clamped to 9.
- **Unit**: resolved with the same rule as parsing: an explicit `--unix-timestamp-unit`, otherwise `UnixTimestampUnit::guess`.
- **Fast path**: a source that is already `YYYY-MM-DDTHH:MM:SS[.fff]Z` is copied verbatim.
- **Conversion**: other values are converted to UTC and written as `%Y-%m-%dT%H:%M:%S`, then `.` and the first N digits of the zero-padded nanoseconds when N > 0, then `Z`. Writing goes through a small digit writer, not `chrono`'s formatter, because `chrono` only offers 0, 3, 6 or 9 digits.
- **Zone-less values**: interpreted as UTC, which `reparse` already does.
- **Failure**: if parsing fails, or the year is outside 0000 to 9999 (which RFC 3339 can't represent), the function returns `false` and the caller emits the original value with its source type (FR-005a).

**Rationale**: The precision rules are about the source's format, so they must look at the source text. The verbatim fast path covers the most common modern log timestamp at zero cost.

## R7. Level, logger and caller values (FR-006, FR-008, FR-008a)

**Decision**:
- **Level**: written from `Level`'s lowercase `AsRef<str>` (`error`, `warning`, `info`, `debug`, `trace`). An unrecognized level leaves `record.level` unset and the source field stays among the regular fields, which the parser already does (edge case "Unrecognized level"). A variant's fallback `level` (as in the k8s preset's `err`) still counts as recognized.
- **Logger**: written as a string.
- **Combined caller**: written as `name`, `file:line`, `file`, or `name @ file:line`, with fixed punctuation that is independent of the theme or the `punctuation` configuration.
- **Split caller**: under FR-008a, the line is written as a number when `looks_like_number` holds and as a string otherwise.

**Rationale**: Fixed punctuation keeps the output schema stable across user configurations.

## R8. Input field (FR-029)

**Decision**: Split `App::input_badges` into two steps:
1. computing plain names (full names, and compact names using the same clipping rules and resolved punctuation);
2. themed, padded badge rendering, used only by `pretty`.

The structured formatter receives, per record, the origin of the record in `RecordWithSource`: a reference to the input's descriptor and the byte range of the record's source in the input. The descriptor is built once per run and holds the input's name and the pre-rendered input field value (the number, or the compact or full name). The formatter itself is built before the inputs are known, so it holds no per-input data. The same origin gives repeat and collision diagnostics their source name and byte range (FR-009a, FR-027), and the `pretty` formatter ignores it.

**Requested layouts**: computed from `--input-info` / `input-info` minus `auto`. The terminal-width logic in `main.rs` applies only to `pretty`.

**Rationale**: Reusing the clipping logic keeps compact names identical to the badges, as FR-029 requires, without duplicating it.

## R9. Dropped-line accounting (FR-018 to FR-021)

**Decision**:
- **Reporting hook**: `RecordObserver` gets a second method, `observe_dropped(&mut self, chunk: &[u8], kind: DropKind)`, with an empty default implementation. `SegmentProcessor` calls it for every non-empty unparsed chunk, remainder after records, and prefix, but only when `show_unparsed()` is false.
- **Where it applies**: in structured modes, `allow_unparsed_data` is always false. In `pretty`, the observers keep the empty default, so the `pretty` path compiles to the same code.
- **Counting**: the drop observer increments a per-input counter (`Arc<[AtomicU64]>`).
- **Per-line entries**: only when `log::log_enabled!(log::Level::Debug)` holds, the observer also writes a debug entry with the source name, byte range and a preview of at most 100 characters. The preview is cut at a character boundary and its control characters are escaped.
- **Empty lines**: empty and whitespace-only lines are not dropped lines. They carry no content and are skipped silently, as `pretty` does with filters active.
- **What is reported**: dropped content is reported line by line, without the spaces around each line, so a multi-line chunk gives one entry per line. A prefix is reported only for records that are written: the prefix of a record excluded by a filter goes with its record. Text after the first record of a sort index entry isn't a remainder, since sort reads one record per entry. The summary counts pieces of unparsed text rather than lines, since a prefix or a remainder is only a part of a line.
- **Oversized lines**: a line longer than `--max-message-size` is passed on unparsed in `pretty`. In structured output it is dropped and reported like any other unparsed line, and the records after it are parsed as usual since [#1642](https://github.com/pamburus/hl/pull/1642).

**Byte ranges** locate dropped content and records in their input, measured in the input's decoded data from where hl started reading it (after `--tail` positioning in follow mode):
- **cat and follow**: the reader sends each segment with its base offset, a running sum of the segment sizes. In follow mode, `BlockScanner` keeps it across scans and resets it when the file is replaced. This is in place since the record origin refactoring (#1630), which tracks the range of every record. `SegmentProcessor` adds the chunk's position within the segment and the record's offsets within the chunk.
- **sort**: each entry already knows its position, the block offset plus the entry offset, so no extra pass is needed. Only the blocks that sort reads are reported. Blocks that the index tells it to skip (no valid entries, no timestamps, or excluded by level/time filters) are not read for diagnostics: reading them would cost the index its purpose for filtered queries, so their dropped lines are neither reported nor counted.

**Summary**: on completion, `App::run` writes one info-level entry per input with a non-zero count (FR-020).

**Rationale**: The common case has no debug logging, so the only added cost is an atomic increment per dropped line. Byte ranges cost a few additions per segment and per record, and need no scan of the data. The observer hook already exists for timestamp indexing, so the extension follows its pattern.

**Alternatives considered**:
- Line and column numbers: they need a scan for delimiters, which don't have to be newlines (`nul` input has no lines at all), and in sort mode either a pre-pass over the input or per-block line counts in the on-disk index, which requires a schema change and invalidates every user's index cache. Nobody reads these entries except while debugging, where a byte range is enough.
- A start offset only: a range also shows where the content ends, without reading the preview.

## R10. Summaries at exit in follow mode (FR-020)

**Decision**: Register a process-wide exit hook that writes the summaries. Call it at the end of `App::run`. `signal_common::shutdown`, the single exit point for handled signals since #1616, also calls it before exiting; this covers SIGINT and SIGTERM on Unix and the console control handler on Windows (#1614). `ExitOnWriteError` calls it just before its exit on a closed output. The hook runs at most once (`std::sync::Once`). Termination by SIGKILL can't be covered.

**Rationale**: In follow mode the normal exit path is Ctrl-C or SIGTERM, which exit from the signal handler thread without unwinding `App::run`.

## R11. Flatten mode `auto` (FR-024, FR-025)

**Decision**:
- **Enums**: add `Auto` to `settings::FlattenOption` and `cli::FlattenOption`.
- **Defaults**: the default configuration sets `flatten = "auto"`. The CLI default comes from the configuration, falling back to `Auto` when it is absent.
- **Schema**: the configuration schema enum becomes `["auto", "never", "always"]`.
- **Resolution**: `main.rs` resolves the option once, after the output format is known: `auto` gives `always` for `pretty` and `logfmt`, and `never` for `json`. `Options.flatten` stays a resolved `bool`.

**Rationale**: Resolving at startup keeps the formatters unaware of `auto`, and `pretty` keeps receiving `true` by default (FR-002).

## R12. Output key configuration (FR-026, FR-028)

**Decision**:
- **New settings**: add `output_key: Option<String>` (serialized as `output-key`) to `settings::Field` and `settings::LevelField`. Add a new `settings::InputField { output_key }` under `fields.predefined.input`, with all keys optional.
- **Validation**: a new `Settings::validate()` step in `Settings::load` rejects an empty `output-key` and duplicate effective output keys. The duplicate check runs over the effective set: the defaults (`timestamp`, `level`, `logger`, `message`, `caller`, `caller-file`, `caller-line`, `input`) overridden by configured values. The error names the setting path, e.g. `fields.predefined.logger.output-key`.
- **Presets**: the ECS and k8s preset files set the values listed in FR-028.
- **Schema**: `schema/json/config.schema.json` describes `output-key` for each field and the new `input` field. After the schema edit, run `cargo build` and then `just ci` (constitution V).

**Rationale**: Per-field settings sit next to the `names` they map from, which is where users already configure these fields.

## R13. Command line and environment (FR-001, FR-016, FR-017)

**Decision**:
- **New option**: `cli::OutputFormat { Pretty, Json, Logfmt }` with `--output-format <FORMAT>`, `env = "HL_OUTPUT_FORMAT"`, `default_value = "pretty"` and `overrides_with = "output_format"`, under the Output help heading. No configuration setting.
- **In `main.rs`, structured output with `--raw` absent**:
  - the theme is `Theme::none()`, whatever `--color` and `-c` say;
  - no pager starts, whatever `--paging` says;
  - the output delimiter comes from `--output-delimiter`;
  - input-info terminal resolution is skipped.
- **`--raw`**: when it is in effect, the output format is treated as `pretty`. `RawRecordFormatter` is used and today's raw behavior is kept, including pager and colors.

**Rationale**: Deciding at the composition root keeps `App` free of terminal concerns and makes FR-017 hold for every output path.

## R14. Follow mode emission (FR-022)

**Decision**: Nothing specific to structured output is needed. Follow mode already flushes the output after each pass that wrote records, before it waits for more input. That fix landed separately for every output format ([#1635](https://github.com/pamburus/hl/issues/1635)), because the line-buffered stdout doesn't flush on the `nul` delimiter. Structured output only needs to leave out the sync indicator and input badges (FR-017), which the zero-width sync indicator from T010 does. A test pins down that follow output in `json` and `logfmt` holds nothing but the records.

## R15. Hide options and predefined fields (FR-014)

**Context**: In `pretty`, `-h` never hides predefined fields: `-h msg`, `-h level`, `-h ts` and `-h caller` all leave them visible. Hide rules apply only to regular fields and nested objects, and predefined fields are controlled by `fields.predefined.<field>.show`.

**Decision** (confirmed, recorded in FR-014): Apply hide rules to predefined fields in structured output, matched against their output keys (e.g. `-h caller` drops the caller, `-h message` drops the message), and apply `--hide-empty-fields` to empty predefined values. This follows the literal wording of FR-014, "including for predefined fields", and is useful for pipelines.

**Alternative considered**: Exact `pretty` parity, where predefined fields are never hidden by `-h`. Rejected, because structured output is mostly consumed by pipelines, where dropping a predefined field is useful.

## R16. Benchmarks and performance gates (SC-004, SC-005)

**Decision**:
- Extend `benches/bench/ws/hl/combined.rs` with `format-json` and `format-logfmt` cases over the existing `elk01` and `int01` JSON and logfmt samples, next to `parse-and-format` (`pretty`) and `parse-only`.
- Add a `structured` group with a sample that has nested objects and a dotted output key, to track the output-key tree path separately.
- The PR reports the results for `pretty` before and after (SC-004, must not regress more than 5%) and structured against `pretty` (SC-005, a target).

## R17. Testing strategy (constitution V and VII)

**Decision**:
- **Unit tests** next to each new module:
  - encoders: quoting, escaping, number rewrites, bare-key and bare-value decisions;
  - the normalizer: order, collisions, repeats, flattening, merging, hiding;
  - timestamp precision: every rule and example in FR-007;
  - settings validation.
- **App-level tests** in `src/app/tests.rs` for `cat`, `sort` and `follow` with `json` and `logfmt`: record parity with `pretty` (SC-002), `--raw` precedence, delimiters, and dropped-line counts.
- **Property tests**: `proptest`, added as a dev-dependency, for round trips (logfmt → JSON → logfmt preserves keys, values and types, SC-003), and "every output line parses" (SC-001) using `serde_json` and hl's own logfmt parser.
- **Fixtures**: input logs and expected outputs in `src/testing/assets/fixtures/output-format/`, with configuration files for presets under `src/testing/assets/configs/`.
- **Coverage**: `just uncovered` after implementation (SC-007).
