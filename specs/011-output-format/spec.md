# Feature Specification: Machine-Readable Output Formats

**Feature Branch**: `feature/output-format`

**Created**: 2026-10-20

**Status**: Draft

**Input**: User description: "Machine-readable output formats for hl (GitHub issue #1481). Add `--output-format=json|logfmt` (plus the default human-readable mode) so hl re-emits parsed, filtered, sorted records as structured output (NDJSON for json, standard key=value for logfmt). Default behavior unchanged; opt-in. Works with sorting (-s), following (-F), filtering (-l, -q, -h/fields), --output-delimiter, multiple inputs. Schema is normalized: fixed predefined keys (timestamp, level, message, logger, caller) with canonical level values, then remaining fields in original order; must not depend on the internal model refactoring (faithful passthrough of original keys/order is out of scope, deferred until the v2 model lands). logfmt-sourced values have types inferred (numbers, booleans, null) consistent with how hl detects them, so output stays stable across future model changes. Machine-readable modes: no ANSI colors, no pager, nothing but records on stdout. Unparsed lines and non-JSON prefixes are dropped from structured output; each dropped line is logged via internal logging (HL_DEBUG_LOG) at debug level with source, line number and truncated content, plus a single info-level summary with counts per source at the end. Must not degrade performance of the default human-readable mode."

## Clarifications

### Session 2026-10-20

- Q: How should the `timestamp` value be represented in structured output? → A: Normalize to RFC 3339 in UTC, keeping the precision of the source. If a timestamp cannot be parsed, its original value is emitted unchanged.
- Q: Should nested objects in JSON mode follow `--flatten`, whose current default is `always`? → A: Add an `auto` variant to `--flatten` and make it the default everywhere, including the default configuration file. `auto` resolves to `always` in the default (`pretty`) mode and in `logfmt` mode, and to `never` in `json` mode. An explicit `always` flattens nested objects into dotted keys even in JSON. An explicit `never` in logfmt mode emits nested objects as quoted compact JSON values, as arrays already are.

### Session 2026-10-21

- Q: What should determine the key name each predefined field gets in structured output? → A: Fixed hl-defined defaults (`timestamp`, `level`, `logger`, `message`, `caller`), each overridable by a new per-field configuration setting `output-key` under `fields.predefined.<field>`, e.g. `fields.predefined.time.output-key = "@timestamp"`. The bundled ECS and k8s presets set it to their ecosystem's names.
- Q: When a regular field has the same key as one of the predefined fields' output keys, what should structured output do with that regular field? → A: The predefined field keeps the key. The regular field is dropped and reported in the debug-level diagnostic log.
- Q: When an output key contains dots, such as `log.level`, should JSON output nest it or use it as a literal key? → A: Treat it as a path. When the effective flatten mode is `never`, the value is nested and merged into an existing object of the same name, replacing that object's own copy of the field. Under `always`, it is emitted as the dotted key.
- Q: How should the caller be emitted when a configuration needs its parts in separate fields, as ECS does? → A: `caller-file` and `caller-line` also accept `output-key`. When either is set, the caller is emitted as separate parts: the name under the `caller` output key, the file as a string, and the line as a number. Otherwise the combined string is used. ECS maps the parts to `log.origin.function`, `log.origin.file.name` and `log.origin.file.line`.
- Q: When several inputs are processed together, should each emitted record say which input it came from? → A: Opt-in. There is no input field by default. When `--input-info` explicitly requests a layout, records get an `input` field, whose key is configurable via `output-key`.
- Q: What should the default human-readable output format be called? → A: `pretty`.
- Q: Should `--raw` take precedence over a structured `--output-format`? → A: Yes, `--raw` wins, as FR-016 states.
- Q: Should the output format be settable in the configuration file? → A: No. It is set only by the command-line option and the environment variable in this iteration.
- Q: When a source record contains the same key more than once, what should structured output do with the repeats? → A: Keep the last occurrence's value at the position of the first occurrence, and drop the others with a debug-level diagnostic entry.
- Q: Which spelling should structured output use for level values, and should a configuration be able to change it? → A: Always hl's canonical names (`error`, `warning`, `info`, `debug`, `trace`), with no configuration in this iteration.
- Q: When a logfmt value counts as a number by hl's rules but is not valid JSON number syntax (e.g. `+5`, `.5`, `5.`, `007`), how should `json` mode emit it? → A: Rewrite it to valid JSON number syntax with the same value (`+5` → `5`, `.5` → `0.5`, `5.` → `5`, `007` → `7`). Values that are already valid JSON numbers are emitted exactly as written.
- Q: When a source timestamp has no time zone or offset, what should structured output emit for it? → A: Interpret it as UTC, as hl already does for filtering and sorting, e.g. `2026-07-06 10:00:00.123` → `2026-07-06T10:00:00.123Z`.
- Q: When a field key cannot be written as a bare logfmt key (e.g. it contains a space, `=` or `"`, or is empty), what should `logfmt` mode do with it? → A: Quote it the same way as a value, as Go's `log/slog` does (`"user name"=x`). hl's logfmt parser is fixed to read quoted keys before this feature lands. Its set of characters allowed in bare keys stays as strict as it is, and any key with other characters is written quoted.
- Q: Should SC-005 (structured output at least as fast as `pretty`) block merging, or be a target? → A: A target. Benchmark results are reported in the PR, and a shortfall is allowed when explained, but the common case without collisions or dotted output keys must stay close to `pretty`.
- Q: When a predefined field's source value is not a string, what type should structured output give it? → A: Keep the source type for every predefined field, e.g. an object message stays an object, and an unparsable numeric timestamp stays a number.
- Q: In `logfmt` mode, should a string value that would otherwise read back as another type be quoted? → A: Yes. Any string that would read back as a number, boolean or null is quoted (`"42"`, `"true"`, `"null"`), and an empty string is written as `""`.
- Q: When a timestamp's source unit implies a precision but the value has no non-zero fraction, should the output keep the unit's digits? → A: Yes. The source's digit count is kept: Unix seconds give no fraction, milliseconds 3 digits, microseconds 6, nanoseconds 9, fractional Unix numbers and textual timestamps keep the fraction digits they have, including trailing zeros.
- Q: With `--flatten=always`, what should happen to objects inside arrays? → A: Flattening stops at arrays. Array elements, including objects inside them, are emitted unchanged as nested JSON, and hide rules do not reach into arrays, as in `pretty`.
- Q: Should `-h` hide predefined fields in structured output, although it does not in `pretty`? → A: Yes. Hide rules match predefined fields by their output keys, e.g. `-h caller` drops the caller, and `--hide-empty-fields` drops empty predefined values.
- Q: Should empty or whitespace-only input lines be reported as dropped lines? → A: No. They are dropped without a diagnostic entry and are not counted in the summary.

### Session 2026-10-22

- Q: How should diagnostic entries locate a dropped line or a record in its input? → A: By the byte range of its content in the input. Line and column numbers are not tracked: the input delimiter may not be a newline, and the entries are only for debugging.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Emit filtered records as JSON for further processing (Priority: P1)

A user investigating an incident has large JSON or logfmt log files. They use hl to filter (by level, time range, query) and optionally sort records, and want the result as one JSON object per line so they can pipe it into `jq` or another JSON-aware tool to extract or aggregate fields.

**Why this priority**: This is the core value of the feature: hl's parsing, filtering and sorting become usable as a preprocessing stage in pipelines. It covers the main use cases in issue #1481 on its own.

**Independent Test**: Run hl with `--output-format=json` over a sample file, apply a level filter, and pipe the output into a JSON parser; every output line parses as a JSON object and contains only the records matching the filter.

**Acceptance Scenarios**:

1. **Given** a JSON log file with records of mixed levels, **When** the user runs `hl --output-format=json -l error app.log`, **Then** stdout contains exactly one JSON object per matching record, one per line, and nothing else.
2. **Given** a logfmt log file, **When** the user runs `hl --output-format=json app.log`, **Then** each record is emitted as a JSON object whose numeric, boolean and null values are JSON numbers, booleans and null rather than strings.
3. **Given** several log files with interleaved timestamps, **When** the user runs `hl -s --output-format=json a.log b.log`, **Then** the records are emitted in chronological order as JSON lines.
4. **Given** a record with predefined fields under source-specific names (e.g. `ts`, `lvl`, `msg`), **When** it is emitted as JSON, **Then** those values appear under the normalized keys `timestamp`, `level` and `message`, with the level expressed as one of hl's canonical level names.
5. **Given** the user hides a field with `-h secret`, **When** records are emitted as JSON, **Then** the `secret` field is absent from every output object.

---

### User Story 2 - Convert between JSON and logfmt (Priority: P2)

A user has logs in one structured format and needs them in the other, for example JSON logs that must be fed into a logfmt-based tool, or logfmt logs that must be stored as JSON.

**Why this priority**: Format conversion is a frequently requested capability with no good single-binary alternative. It builds on the same normalized record representation as Story 1 but adds a second serializer.

**Independent Test**: Run hl with `--output-format=logfmt` over a JSON file and parse the output with a logfmt parser; every record round-trips with the same keys and values.

**Acceptance Scenarios**:

1. **Given** a JSON log file, **When** the user runs `hl --output-format=logfmt app.log`, **Then** each record is emitted on one line as space-separated `key=value` pairs, with values quoted and escaped only when required.
2. **Given** a JSON record containing a nested object `{"http":{"status":500}}`, **When** it is emitted as logfmt, **Then** the nested field appears as a flattened dotted key, e.g. `http.status=500`.
3. **Given** a value containing spaces, quotes, `=` or newlines, **When** it is emitted as logfmt, **Then** the value is quoted and escaped so that a standard logfmt parser recovers the original value.
4. **Given** the record from scenario 2, **When** it is emitted as JSON with the default flatten mode, **Then** the object stays nested (`"http":{"status":500}`). **When** `--flatten=always` is given, **Then** it is emitted as `"http.status":500`.

---

### User Story 3 - Stream structured output while following logs (Priority: P3)

An operator follows live log files and forwards the parsed, filtered records as JSON to a log shipper or dashboard process reading from hl's stdout.

**Why this priority**: Valuable for live pipelines, but it reuses the behavior of Stories 1 and 2. The only new requirements are timely per-record emission and the absence of any decoration on stdout.

**Independent Test**: Run `hl -F --output-format=json app.log`, append records to the file, and verify that each appended matching record appears promptly on stdout as a complete JSON line, with no pager, colors or banners.

**Acceptance Scenarios**:

1. **Given** hl is following a file with `--output-format=json`, **When** a new matching record is appended, **Then** it appears on stdout as a complete JSON line within the normal follow-mode latency.
2. **Given** hl is following several files, **When** records arrive from different files, **Then** each is emitted as a standalone record with no file badges or other prefixes on stdout.

---

### User Story 4 - Diagnose dropped lines (Priority: P4)

A user notices that fewer records came out than they expected and wants to know which input lines hl could not parse and dropped from the structured output.

**Why this priority**: Structured output must be clean, so non-record content is dropped silently. Users still need a way to see what was dropped without polluting stdout.

**Independent Test**: Run hl with `--output-format=json` and `HL_DEBUG_LOG=debug` over a file mixing JSON records and plain-text lines; stdout contains only the JSON records, while the diagnostic log lists each dropped line with its source and byte range and ends with a per-source summary.

**Acceptance Scenarios**:

1. **Given** an input containing non-record lines (e.g. stack traces, banners), **When** hl runs with `--output-format=json` and no diagnostic logging enabled, **Then** stdout contains only records and nothing is written about the dropped lines.
2. **Given** the same input, **When** hl runs with `HL_DEBUG_LOG=debug`, **Then** each dropped line is reported in the diagnostic log at debug level with the source name, byte range and a truncated preview of its content.
3. **Given** the same input, **When** hl runs with `HL_DEBUG_LOG=info`, **Then** only one summary entry per source is reported at info level, giving the number of dropped lines, and no per-line entries appear.

---

### Edge Cases

- **No parsable records**: the input has no parsable records at all. Stdout is empty, the exit status is the same as for an equivalent run in the default mode, and the summary reports the dropped lines.
- **Empty or whitespace-only lines**: they are dropped without a diagnostic entry and are not counted as dropped lines.
- **Missing predefined fields**: a record lacks some of them, e.g. it has no timestamp or level. The missing keys are omitted from the output; they are not emitted as null or empty values.
- **Unrecognized level**: a record has a level value that hl does not recognize. The level key is omitted from the normalized position, and the original field is emitted among the regular fields under its original key with its original value.
- **Regular field with the same key as a predefined output key**: e.g. a raw `message` field when the message was taken from `msg`, or a raw `timestamp` field when the time was taken from `ts`. The predefined value keeps the key, and the regular field is dropped and reported in the diagnostic log (FR-027).
- **Repeated key in the source record**: e.g. logfmt `tag=a tag=b`. The output has one `tag` key at the position of the first occurrence, holding the last value (`b`). The other occurrences are dropped and reported in the diagnostic log (FR-009a).
- **Prefix before the record**: a record is preceded by non-JSON text (with `--allow-prefix`). The prefix is dropped from the output and counted and logged like an unparsed line; the record itself is still emitted.
- **Multi-line pretty-printed JSON input**: the record is emitted as a single compact line.
- **`--raw` together with a structured output format**: see FR-016.
- **Output written to a file with `-o`**: behavior is identical to stdout, with no colors and no pager.
- **NUL delimiter**: with `--output-delimiter=nul`, records are separated by NUL instead of newlines.
- **Very large records or fields**: records up to the existing maximum message size are emitted in full, never truncated.
- **Invalid UTF-8 or escape sequences in source values**: the output is still valid JSON or logfmt. Invalid sequences are replaced, never emitted raw.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST provide an output format option `--output-format` accepting `pretty` (default, the current human-readable output), `json` and `logfmt`. It MUST also be settable through the `HL_OUTPUT_FORMAT` environment variable, with the command-line option taking precedence. There is no configuration file setting for it, matching `--input-format`.
- **FR-002**: With the default output format, the system MUST behave exactly as it does today, byte for byte, for all existing options.
- **FR-003**: In `json` mode, the system MUST emit each record as a single-line JSON object, one record per output entry, separated by the configured output delimiter (newline by default, i.e. NDJSON).
- **FR-004**: In `logfmt` mode, the system MUST emit each record as a single line of space-separated `key=value` pairs, quoting and escaping values only when required for a logfmt parser to recover them and their types:
  - A string value MUST be quoted when it contains a space, a control character, `=` or `"`, or is not valid as a bare value for hl's logfmt parser.
  - A string value MUST also be quoted when, written bare, it would be read back as a number, boolean or null under FR-010, e.g. `"42"`, `"true"`, `"null"`. An empty string MUST be written as `""`.
  - Quoted keys and values MUST be JSON-encoded strings, the notation hl's logfmt parser already decodes for quoted values. Non-ASCII characters are written as UTF-8, not as `\u` escapes.
- **FR-004a**: In `logfmt` mode, keys MUST follow the same notation as string values. A key is written bare when it is non-empty and every character in it is allowed in a bare key by hl's logfmt parser. Otherwise it MUST be written as a quoted, escaped string, e.g. `"user name"=x`, `"items[0]"=x` or `""=x`. This guarantees that hl reads its own logfmt output back. Keys MUST NOT be altered or dropped to avoid quoting.
- **FR-005**: Each emitted record MUST place the recognized predefined fields first, in this fixed order: time, level, logger, message, caller. Each MUST appear under its output key (FR-026), which defaults to `timestamp`, `level`, `logger`, `message` and `caller` respectively. A key MUST be omitted when the record has no value for it.
- **FR-005a**: Predefined field values MUST keep their source type, except where FR-006, FR-007 and FR-008 define a normalized value. For example, a message that is an object or a number is emitted as an object or a number, and an unparsable timestamp that is a number stays a number. In `logfmt` mode, objects and arrays follow FR-013.
- **FR-006**: The level value MUST be one of hl's canonical level names (`error`, `warning`, `info`, `debug`, `trace`), regardless of how the level was spelled in the source. The spelling is fixed and not configurable in this iteration; e.g. a source `WARN` is emitted as `warning`.
- **FR-007**: The time value MUST be a string in RFC 3339 format in UTC, e.g. `2026-07-06T10:00:00.123Z`.
  - Its number of fraction digits MUST match the precision of the source format, including trailing zeros, even when the fraction is zero:
    - Unix timestamps in seconds: no fraction; in milliseconds: 3 digits; in microseconds: 6; in nanoseconds: 9, e.g. `1700000000000` → `2023-11-14T22:13:20.000Z`.
    - Fractional Unix numbers: the unit's digits plus the fraction digits in the source, at most 9, e.g. seconds `1700000000.25` → `.25`, milliseconds `1700000000000.5` → `.0005`.
    - Textual timestamps: the fraction digits present in the source, e.g. `10:00:00.10` → `10:00:00.10`. A source without a fraction gives none.
  - This applies regardless of the source representation: textual date-times, Unix timestamps in any supported unit, and so on.
  - A timestamp with no time zone or offset MUST be interpreted as UTC, the same way hl interprets it for time-range filtering and sorting, e.g. `2026-07-06 10:00:00.123` → `2026-07-06T10:00:00.123Z`.
  - If the timestamp cannot be parsed, its original value MUST be emitted unchanged.
  - The output MUST NOT depend on `--time-format`, `--time-zone` or `--local`.
- **FR-008**: By default, the caller MUST be emitted as one string under the `caller` output key, combining the caller information available in the record. A caller name is emitted as is. A file and line are emitted as `file:line`, or just `file` if there is no line. When both a name and a file are present, they are emitted as `name @ file:line`.
- **FR-008a**: When `output-key` is set for `caller-file` or `caller-line` (FR-026), the caller MUST instead be emitted as separate parts, each in the predefined-field position of FR-005 and each omitted when absent:
  - the caller name under the `caller` output key;
  - the file as a string under the `caller-file` output key, defaulting to `caller-file`;
  - the line under the `caller-line` output key, defaulting to `caller-line`. The line is a number when its value is numeric, and a string otherwise.
- **FR-009**: All remaining fields MUST follow the predefined fields in the order they appear in the source record, under their original keys. Keys MUST NOT be prettified or otherwise transformed.
- **FR-009a**: When a record contains the same key more than once at the same nesting level, the output MUST contain that key once. It MUST be placed at the position of the first occurrence and hold the value of the last occurrence. The other occurrences MUST be omitted, and a debug-level entry MUST be written to the diagnostic log naming the source, the byte range of the record and the repeated key.
  - The same rule MUST apply to keys that become equal only after flattening, e.g. a nested `{"a":{"b":1}}` and a literal `"a.b"` key under `--flatten=always`.
  - FR-027 takes precedence: a regular field that collides with a predefined output key is dropped even if it is the last occurrence.
- **FR-010**: Values from logfmt sources MUST be typed using the same rules hl applies when detecting value types: unquoted numbers become numbers, `true`/`false` become booleans, and `null` becomes null. All other values, and all quoted values, become strings.
- **FR-010a**: Number values MUST keep their source text and MUST NOT be re-formatted or rounded, except where `json` mode requires a rewrite:
  - In `json` mode, a number whose source text is not valid JSON number syntax MUST be rewritten to the equivalent valid syntax with the same value. A leading `+` and redundant leading zeros are removed, a missing integer part becomes `0`, and a trailing `.` with no fraction digits is removed, e.g. `+5` → `5`, `.5` → `0.5`, `5.` → `5`, `-007.50` → `-7.50`, `5.e3` → `5e3`.
  - In `logfmt` mode, numbers MUST be emitted as written in the source.
- **FR-011**: String values MUST be emitted with their decoded content, unescaped from the source encoding and re-escaped for the target format, so the output is always valid in that format.
- **FR-012**: In `json` mode, nested objects MUST be preserved as nested JSON objects when the effective flatten mode is `never`, and flattened into dotted keys when it is `always` (see FR-024). Arrays MUST always be preserved as JSON arrays. Flattening MUST stop at arrays: array elements, including objects inside them, MUST be emitted unchanged, e.g. `"items":[{"id":1,"tags":{"a":1}}]` even under `always`. All content MUST be emitted compactly on one line.
- **FR-013**: In `logfmt` mode, nested objects MUST be flattened into dotted keys when the effective flatten mode is `always`. When it is `never`, each nested object MUST be emitted as its compact JSON representation in a quoted value. Arrays MUST always be emitted as their compact JSON representation in a quoted value.
- **FR-014**: The field visibility options (`-h`/`--hide`, including revealing with `!`) and `--hide-empty-fields` MUST apply to structured output the same way they apply to the default output. As in the default output, they apply to object fields and do not reach into array elements. Unlike in the default output, they MUST also apply to predefined fields, matched by their output keys (FR-026), e.g. `-h caller` omits the caller and `--hide-empty-fields` omits an empty message.
- **FR-015**: Record selection options MUST produce the same set of records in every output format. These are level, time-range, query and field filters, sorting (`-s`), following (`-F`), `--tail`, and multiple inputs.
- **FR-016**: When `--raw` is in effect, it MUST take precedence over a structured `--output-format`, emitting the original source entries as today. This avoids hard errors when `HL_OUTPUT_FORMAT` is set in the environment and a user passes `-r` ad hoc.
- **FR-017**: In structured modes, the system MUST NOT emit ANSI escape sequences, MUST NOT start a pager, and MUST NOT write anything to the output except records and delimiters. This applies regardless of `--color` and `--paging`. It excludes input badges, unparsed lines and prefixes. Input identification appears only as a record field (FR-029).
- **FR-018**: In structured modes, the system MUST drop input lines that do not parse as records, as well as non-record prefixes before records.
- **FR-019**: For each dropped line or prefix, the system MUST write a debug-level entry to the internal diagnostic log (controlled by `HL_DEBUG_LOG`). The entry contains the source name, the byte range of the dropped content in the input and a preview of the content truncated to a bounded length. In sort mode, this covers the content that is read: parts of the input that the index lets hl skip, such as blocks outside a time filter, are not read for diagnostics.
- **FR-020**: At the end of processing, the system MUST write one info-level entry per source that had dropped lines, giving the number of lines dropped from it. In follow mode, these summaries MUST be written when hl exits.
- **FR-021**: Diagnostic log entries MUST be written to stderr only, never to the record output, and only when diagnostic logging is enabled.
- **FR-022**: In follow mode, each matching record MUST be written and flushed as a complete entry as soon as it is emitted, so downstream readers never observe partial records.
- **FR-023**: Output written to a file via `-o`/`--output` MUST be identical to output written to stdout.
- **FR-024**: The `--flatten` option, the `HL_FLATTEN` environment variable and the `formatting.flatten` configuration setting MUST accept a new value `auto`, which MUST become the default.
  - `auto` MUST resolve to `always` in `pretty` and `logfmt` modes, and to `never` in `json` mode.
  - Explicit `always` and `never` values MUST apply in every output format.
- **FR-025**: The default configuration file MUST set `formatting.flatten` to `auto`, and the configuration schema MUST accept `auto`. With no explicit flatten setting, the default (`pretty`) output MUST be unchanged, consistent with FR-002.
- **FR-026**: The configuration MUST accept an optional `output-key` string setting for each of the predefined fields `time`, `level`, `logger`, `message`, `caller`, `caller-file` and `caller-line`, under `fields.predefined.<field>`.
  - When set, it MUST replace that field's default output key in `json` and `logfmt` modes. It MUST have no effect on the default (`pretty`) output.
  - The level field has a single `output-key` shared by all level variants.
  - Setting `output-key` for `caller-file` or `caller-line` switches the caller from the combined string to separate parts (FR-008a).
  - An empty `output-key`, or two predefined fields configured with the same output key, MUST be rejected with a configuration error that names the offending setting.
  - The configuration schema MUST describe the new setting.
- **FR-027**: When a regular field has the same key as an emitted predefined field's output key, the predefined field MUST keep the key in structured output. The regular field MUST be omitted, and a debug-level entry MUST be written to the diagnostic log naming the source, the byte range of the record and the dropped key. Every key in an emitted record is therefore unique.
- **FR-027a**: An output key containing dots MUST be treated as a path of nested keys.
  - When the effective flatten mode is `always`, the predefined value MUST be emitted under the dotted key, e.g. `log.level=info`.
  - When it is `never`, the value MUST be emitted as a nested object, e.g. `{"log":{"level":"info"}}`.
  - If the record also has a regular field that is an object at the same path prefix (e.g. a `log` object), the predefined value MUST be merged into that object rather than emitted beside it. The merged object takes the position of the predefined field (FR-005). The predefined entries come first inside it, followed by the object's remaining entries in source order.
- **FR-027b**: Source fields that were recognized as predefined fields, including nested ones such as `log.level` or `span.name`, MUST NOT be repeated among the regular fields. This matches how the default output already hides them.
  - Hiding a regular field or object (e.g. the ECS preset's `fields.hide = ["log"]`) MUST NOT hide predefined fields that were read from it or are emitted into it.
  - When such a hidden object receives a merged predefined value, only the predefined entries MUST be emitted in it.
- **FR-028**: The bundled presets MUST set `output-key` so that structured output uses each ecosystem's field names:
  - k8s: time → `ts`, message → `msg`, caller → `caller`. The level keeps the default `level`, because the preset reads it from the numeric verbosity field `v`, and writing canonical level names under `v` would change that field's type and meaning.
  - ECS: time → `@timestamp`, level → `log.level`, logger → `log.logger`, message → `message`. The caller is split into parts (FR-008a): caller → `log.origin.function`, caller-file → `log.origin.file.name`, caller-line → `log.origin.file.line`. Dotted keys follow FR-027a, so with JSON's default flatten mode all `log.*` values are nested in one `log` object.
- **FR-029**: In structured modes, records MUST NOT identify their input unless `--input-info` (or the `input-info` configuration setting) explicitly requests an input layout.
  - With `auto` (the default) or `none` alone, no input field is added.
  - When the requested layouts include `minimal`, `compact` or `full`, each record MUST get an input field, placed after the predefined fields of FR-005 and before the regular fields. The most detailed requested layout determines its value:
    - `minimal`: the input's number, as a number, counting from 0 as the default output's badges do;
    - `compact`: the input's name in the compact form used by the default output's badges;
    - `full`: the input's full name as shown by the default output's badges, with standard input shown as `<stdin>`.
  - If the requested layouts also include `none`, the field MUST be omitted when there is only one input, matching how `none` suppresses badges for a single input in the default output.
  - The field's key defaults to `input`. It MUST be configurable via `fields.predefined.input.output-key`, which follows the same rules as FR-026, including the collision rule of FR-027 and the path rule of FR-027a.

### Key Entities

- **Normalized record**: the structured representation hl emits for one source record. It consists of an ordered list of key/value pairs: the predefined fields in the fixed order of FR-005 under their output keys (FR-026), then the optional input field (FR-029), followed by the remaining visible fields in source order. All keys are unique (FR-009a, FR-027). Values are typed as string, number, boolean, null, object or array.
- **Dropped line**: a contiguous piece of input that hl could not parse as a record, or a prefix preceding a record. It is identified by source and byte range, reported in the diagnostic log, and counted per source.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For any supported input, 100% of output lines in `json` mode parse as JSON objects with a standard JSON parser, and 100% of output lines in `logfmt` mode parse with a standard logfmt parser.
- **SC-002**: For the same input and record selection options, the number of records emitted is identical in `pretty`, `json` and `logfmt` modes.
- **SC-003**: Converting a logfmt file to JSON and back to logfmt yields the same keys, values and value types as the original.
- **SC-004**: Throughput of the default output mode on the project's benchmark inputs does not regress by more than 5%.
- **SC-005**: Throughput of `json` and `logfmt` output on the same benchmark inputs is targeted to be no lower than that of the default output mode. This is a target, not a merge gate: the results are reported in the PR, and a shortfall is acceptable when explained. For records without key collisions, repeated keys or dotted output keys, structured output is no more than 5% slower than the default output mode.
- **SC-006**: Every pipeline example in issue #1481 that does not depend on out-of-scope behavior works unmodified and produces the expected results.
- **SC-007**: Test coverage on the target branch does not decrease.

## Assumptions

- **Source names**: in diagnostic entries, standard input is identified as `<stdin>` and files by the names they were given on the command line.
- **Preview length**: the content preview in diagnostic entries is limited to about 100 characters, as a reasonable default.
- **Formatting-only options**: options that affect only human-readable presentation have no effect in structured modes: `--theme`, `--time-format`, `--expansion`, `--ascii` and `--raw-fields`. Time zone options are covered by FR-007. `--input-info` is covered by FR-029.
- **Existing configuration files**: a user configuration file that explicitly sets `flatten = "always"` will produce flattened JSON. This is intentional, because an explicit value is honored in every mode. The change is called out in the release notes so that users who copied the old default can switch to `auto`.
- **Prerequisite parser fix**: hl's logfmt input parser accepts quoted keys with the same escaping as quoted values, so it can read lines with such keys written by Go's `log/slog` or by this feature. This landed separately, before this feature (#1607). The default output quotes such keys too (#1609).
- **Bare key characters stay strict**: the parser keeps rejecting characters such as `{`, `[`, `(`, `,` and `'` in bare keys. This keeps logfmt easy to tell apart from JSON, plain text and other formats during detection. Because logfmt has no specification, it also avoids committing to accept arbitrary input that would then have to stay supported for backward compatibility.
- **Out of scope**: faithful passthrough of original keys, original key order for predefined fields, and source text formatting is not part of this feature. It is deferred until the internal record model rework lands, and the normalized schema is designed so that the rework does not change this feature's output.
- **Out of scope**: emitting dropped lines in-band as structured error records (e.g. an `_hl_error` field) is not part of this feature.
