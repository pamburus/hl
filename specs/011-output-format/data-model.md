# Data Model: Machine-Readable Output Formats

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Research**: [research.md](./research.md)

The feature adds no persistent data. The entities below are configuration values, per-run state and the per-record output shape.

## OutputFormat

The output format chosen for a run.

| Value | Meaning |
|---|---|
| `pretty` | Default human-readable output, unchanged (FR-002) |
| `json` | One compact JSON object per record (FR-003) |
| `logfmt` | One line of `key=value` pairs per record (FR-004) |

- **Sources**: `--output-format`, then `HL_OUTPUT_FORMAT`, then the default `pretty`. There is no configuration setting (FR-001).
- **Effective format**: `pretty` whenever `--raw` is in effect (FR-016).
- **Structured**: `json` and `logfmt` are called structured formats. They imply no colors, no badges, no sync indicators and no unparsed entries on the output (FR-017). Paging is not affected.

## FlattenMode

| Value | Effective in `pretty` | Effective in `json` | Effective in `logfmt` |
|---|---|---|---|
| `auto` (new default) | `always` | `never` | `always` |
| `always` | `always` | `always` | `always` |
| `never` | `never` | `never` | `never` |

- **Sources**: `--flatten`, `HL_FLATTEN`, `formatting.flatten` (FR-024, FR-025).
- **Resolution**: once at startup, to a boolean.

## PredefinedFieldOutputKeys

A per-run table mapping each predefined field to its key in structured output (FR-026).

| Field | Setting | Default key |
|---|---|---|
| time | `fields.predefined.time.output-key` | `timestamp` |
| level | `fields.predefined.level.output-key` | `level` |
| logger | `fields.predefined.logger.output-key` | `logger` |
| message | `fields.predefined.message.output-key` | `message` |
| caller | `fields.predefined.caller.output-key` | `caller` |
| caller-file | `fields.predefined.caller-file.output-key` | `caller-file` |
| caller-line | `fields.predefined.caller-line.output-key` | `caller-line` |
| input | `fields.predefined.input.output-key` | `input` |

**Validation rules**, applied at configuration load:
- An `output-key` that is set MUST be non-empty.
- Effective keys MUST be pairwise distinct. The two caller parts count only when caller splitting is active (see CallerMode).
- A violation is a configuration error naming the setting, e.g. ``fields.predefined.logger.output-key: duplicates the output key of fields.predefined.message (`msg`)``.

**Derived values**:
- **Path segments**: each key is split at dots. A key with more than one segment is a *dotted output key* (FR-027a).
- **Output-key tree**: built when the effective flatten mode is `never` and at least one emitted key is dotted; empty otherwise (research R2).

## CallerMode

| Value | Condition | Emitted keys |
|---|---|---|
| Combined | neither `caller-file` nor `caller-line` has `output-key` set | `caller` as one string (FR-008) |
| Split | either has `output-key` set | `caller` (name), `caller-file` (string), `caller-line` (number or string) (FR-008a) |

## InputFieldLayout

Derived from the requested input-info layouts, which are the configured set minus `auto` (FR-029).

| Requested layouts | Field value | Single input with `none` also requested |
|---|---|---|
| none of `minimal`, `compact`, `full` | no field | n/a |
| most detailed is `minimal` | input number (JSON number), from 0 | field omitted |
| most detailed is `compact` | compact name (string), as in badges | field omitted |
| most detailed is `full` | full name (string), `<stdin>` for standard input | field omitted |

The value is pre-rendered once per input, in the encoding of the target format.

## NormalizedRecord

The structured representation emitted for one source record. It is not materialized as a whole; the formatter streams it (research R2). Logically, it is an ordered list of unique `(key, value)` entries:

1. **Predefined section**, in this order, each omitted when absent or hidden (FR-005, FR-014):
   1. time: RFC 3339 UTC string, or the original value with its source type (FR-007, FR-005a);
   2. level: canonical level name (FR-006);
   3. logger: string;
   4. message: source type kept (FR-005a);
   5. caller: one string (Combined), or name, file and line (Split).
2. **Input entry**, when InputFieldLayout yields one (FR-029).
3. **Regular section**: the remaining fields in source order, after:
   - excluding fields consumed as predefined, including nested ones (FR-027b);
   - excluding fields ignored by `fields.ignore` (existing parser behavior);
   - applying hide rules and `--hide-empty-fields` (FR-014);
   - resolving repeated keys: first position, last value (FR-009a);
   - dropping collisions with emitted predefined or input keys (FR-027).

**Value types**: string, number, boolean, null, object, array.

**Invariant**: at every nesting level, and over full dotted paths when flattening, every key is unique.

**Dotted output keys** (FR-027a):
- With effective flatten `always`, the entry is emitted under the dotted key.
- With `never`, it is placed in a nested object along its path. A regular object field at the same first segment is merged into that object and moved to the predefined entry's position: predefined entries first, then the object's other entries in source order. A regular field at the same path that is not an object is a collision (FR-027).

## Value encoding rules

| Source value | `json` output | `logfmt` output |
|---|---|---|
| JSON string, no backslash | copied verbatim | decoded; bare if safe (see below), else quoted |
| JSON string with escapes | decoded and re-encoded; invalid sequences become U+FFFD | decoded; bare if safe, else quoted |
| logfmt unquoted text | typed by `RawValue::auto`: number, boolean, null or string | same type; strings bare if safe |
| number, valid JSON syntax | verbatim | verbatim |
| number, invalid JSON syntax (logfmt only) | rewritten per FR-010a | verbatim |
| boolean, null | `true`, `false`, `null` | `true`, `false`, `null` |
| object | compact nested object, or flattened under `always` | flattened under `always`, else quoted compact JSON |
| array | compact array, elements unfiltered | quoted compact JSON |

**Bare logfmt string**: the value passes `is_bare_value` from `serde-logfmt`, doesn't start with `"`, and `RawValue::auto` types it as a string (FR-004).

**Bare logfmt key**: the key passes `is_bare_key` from `serde-logfmt` (FR-004a). Keys are otherwise quoted and never altered.

## DroppedLine

| Attribute | Description |
|---|---|
| source | input name: path as given on the command line, or `<stdin>` |
| range | byte range of the dropped content in the input's decoded data, counted from where hl started reading the input (research R9) |
| kind | `entry` (a whole entry with no record), `remainder` (text after the records in an entry), `prefix` (text before a record) or `fragment` (a part of an entry too long to be read as a record) |
| preview | at most 100 characters, cut at a character boundary, with control characters escaped |

Text that is empty or contains only whitespace is not dropped text.

## DropStats

Per-run state: one counter per input, incremented for each DroppedLine. At exit, one info-level summary entry is written for every input with a non-zero counter (FR-020). The summary is written once, on normal completion, on SIGINT or SIGTERM, and on a closed output (research R10).

## State transitions

There are none. Each record is formatted independently, and DropStats only grows until it is reported at exit.
