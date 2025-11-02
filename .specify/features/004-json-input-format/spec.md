# Feature Specification: JSON Input Format Support

**Feature Name:** JSON Input Format Support  
**Feature ID:** 004-json-input-format
**Status:** Existing Implementation (Documented)
**Last Updated:** 2025-11-02

## Clarifications

### Session 2025-11-02

- Q: When a JSON object contains duplicate field names, which value should be preserved? → A: Duplicate keys are NOT collapsed. Each key/value pair is treated as an independent field in the output, displayed in the same order they appear in the input. When filters apply to a key that appears multiple times, OR logic is used: if ANY of the key/value pairs with that key match the filter, the entire message passes the filter.
- Q: When JSON parsing fails in auto-detect mode, what should happen? → A: Try all remaining supported input formats sequentially (e.g., logfmt). If none match, process the line as raw content: pass through unfiltered in concatenation mode, or discard it if any filters are applied or if sorting/following mode is enabled.
- Q: How and when is prefix extraction performed relative to format detection? → A: Prefix extraction occurs simultaneously with JSON parsing (not before). When `--allow-prefix` is enabled: search for first `{` character; if remaining content parses as valid JSON, everything before `{` is the prefix. Prefix cannot contain `{`. If JSON parsing with prefix fails, try remaining formats as-is (without prefix extraction). Raw lines are processed per fallback strategy.
- Q: How do type mismatches affect field filtering and comparisons? → A: Field filtering (`-f` option) uses string-based matching (exact, substring, wildcard, regex). Query filtering (`-q`) with numeric operators (=, !=, >, <, >=, <=) requires type coercion: the field value is parsed as a number; if parsing fails, the record does not pass the filter (is discarded).

## Overview

The JSON input format support feature enables `hl` to parse and process structured logs in JSON format. JSON logs are automatically detected and parsed into structured records, extracting fields, values, and metadata that downstream features can filter, format, and display.

This feature works with line-delimited JSON (NDJSON/JSONL) where each line is a separate JSON object, and is transparent to all downstream features (filtering, formatting, sorting, etc.).

## User Stories & Acceptance Criteria

### US-1: Parse Line-Delimited JSON Logs
**As a** application developer
**I want to** process logs in line-delimited JSON format (NDJSON)
**So that** I can analyze structured log output from JSON-logging frameworks

**Acceptance Criteria:**
- Given a log file with one JSON object per line (e.g., `{"timestamp":"2025-01-01T10:00:00Z","level":"info","message":"User logged in","userId":"42"}`)
- When I run `hl logs.jsonl`
- Then each line is parsed as a separate JSON record
- And nested fields and arrays are accessible to downstream features
- And parsing errors on a single line do not prevent processing of subsequent lines

### US-2: Extract Nested JSON Fields
**As a** DevOps engineer
**I want to** access deeply nested fields within JSON log objects
**So that** I can filter and display data from hierarchical log structures

**Acceptance Criteria:**
- Given JSON logs with nested objects (e.g., `{"request":{"method":"GET","path":"/api/users"}}`)
- When downstream features filter or display the logs
- Then nested fields are accessible using dot notation (e.g., `request.method`)
- And array indices are supported (e.g., `items[0].name`)

### US-3: Automatic Format Detection
**As a** a user processing mixed log formats
**I want to** process JSON logs without explicitly specifying the format
**So that** JSON is automatically detected based on content

**Acceptance Criteria:**
- Given a log file that starts with JSON content (first field is `{`)
- When I run `hl logfile.log` without the `--input-format` flag
- Then JSON format is automatically detected
- And the file is parsed as JSON without explicit configuration

### US-4: Explicit JSON Format Specification
**As a** a user with ambiguous log content
**I want to** explicitly specify JSON format even if content might be ambiguous
**So that** parsing is deterministic regardless of content characteristics

**Acceptance Criteria:**
- Given a log file
- When I run `hl --input-format json logfile.log`
- Then the file is forcibly parsed as JSON
- And non-JSON content results in parsing errors

### US-5: Extraction of Predefined Fields (via Structured Logs Feature)
**As a** a user with structured logs
**I want to** have standard log fields (level, message, timestamp, etc.) automatically identified from JSON
**So that** downstream features can process this metadata consistently

**Acceptance Criteria:**
- Given JSON logs with fields that match configured predefined field patterns
- When parsed records are processed
- Then predefined fields are identified and extracted by the Structured Logs feature
- And downstream features have access to these extracted fields
- And the specific field names recognized depend on configuration (separate feature responsibility)
- Note: See the Structured Logs feature specification for details on field recognition and extraction

### US-6: Non-JSON Prefix Handling
**As a** system administrator
**I want to** process logs with prefixes before JSON content
**So that** I can analyze logs from tools that add timestamps, hostnames, or other metadata before JSON

**Acceptance Criteria:**
- Given logs with a prefix before JSON (e.g., `[2025-01-01 10:00:00] {"level":"info","message":"test"}` or `hostname | {"key":"value"}`)
- When I run `hl --allow-prefix logfile.log`
- Then the prefix is recognized and associated with the JSON record
- And the prefix is preserved and prepended to the output when the message is formatted
- And JSON parsing of the record works correctly despite the prefix
- And if `--allow-prefix` is not set, the line with prefix is not considered valid JSON and will be processed as a different format or as raw content

### US-7: Timestamp Field Recognition (via Structured Logs Feature)
**As a** a user with timestamp fields in logs
**I want to** have timestamp fields automatically identified in JSON records
**So that** they can be extracted and used for time-based filtering and sorting

**Acceptance Criteria:**
- Given JSON logs with a timestamp field named according to configured patterns
- When records are processed through the Structured Logs feature
- Then the timestamp field is identified and extracted
- And time-based filtering and sorting work on the extracted timestamp
- Note: Timestamp format parsing and conversion is handled by the Structured Logs feature

### US-8: Field Type Preservation
**As a** a data analyst
**I want to** have JSON field types (strings, numbers, booleans, nulls) preserved during processing
**So that** comparisons and filtering can distinguish types correctly

**Acceptance Criteria:**
- Given JSON logs with different field types (e.g., `{"status": 200, "error": null, "active": true, "duration": 1.5}`)
- When filtering or comparing fields
- Then numeric fields are compared numerically (not as strings)
- And boolean and null values are handled according to their types

### US-9: Streaming JSON Parsing
**As a** a user with very large JSON log files
**I want to** process huge JSON log files without loading them entirely into memory
**So that** memory usage stays bounded regardless of file size

**Acceptance Criteria:**
- Given a multi-gigabyte JSON log file (NDJSON format)
- When I run `hl huge-log.jsonl`
- Then the file is parsed in a streaming fashion, one record at a time
- And memory usage stays bounded
- And processing completes in reasonable time

## Technical Specifications

### Input Format

**NDJSON (Newline-Delimited JSON):**
- Each line is a complete JSON object
- Lines are separated by newline (LF, CR, or CRLF depending on configuration)
- Each record is processed independently

**JSON Structure:**
- Top-level records must be JSON objects (not arrays or primitives)
- Nested objects and arrays are fully supported
- Fields can contain any valid JSON value (string, number, boolean, null, object, array)
- Duplicate field names are NOT collapsed: each key/value pair is treated as an independent field in the output, displayed in the order they appear in the input. When filters apply to a key with duplicates, OR logic is used: if ANY of the key/value pairs with that key match the filter condition, the entire message passes the filter.

### Standard Field Recognition

Standard field extraction (level, message, timestamp, logger, caller) is handled by the separate Structured Logs Processing feature. JSON records are converted to a common internal representation and then processed by that feature to identify and extract predefined fields according to configuration.

This design ensures consistent field extraction across all input formats (JSON, logfmt, etc.) using the same configuration and logic.

### Timestamp Detection

Timestamp field detection and parsing is handled by the Structured Logs Processing feature after JSON parsing. Once a timestamp field is identified in the JSON record, the Structured Logs feature handles parsing and normalization.

### Prefix Handling

**When `--allow-prefix` is Enabled (JSON Format Only):**
1. Each line is scanned for the first `{` character
2. If the remaining content (from `{` onwards) can be parsed as a valid JSON object, the bytes before `{` are extracted as the prefix
3. The prefix is stored with the JSON record
4. The JSON portion (from `{` onwards) is parsed for field extraction
5. When the record is formatted for output, the prefix is prepended to the formatted message
6. If the remaining content cannot be parsed as valid JSON, no prefix extraction occurs and the full line is attempted with other formats

**Important Constraints:**
- Prefix extraction is conditional on successful JSON parsing of the remainder
- A prefix cannot contain the `{` character (it marks the start of JSON content)
- Prefix extraction only applies to JSON format; other formats are not attempted with prefix extraction

**When `--allow-prefix` is NOT Enabled:**
- Lines with non-JSON prefixes are not processed with prefix extraction
- They will be processed as-is with other formats or as raw content depending on configuration

### Format Detection

**Detection Order (in Auto Mode):**
1. If `--allow-prefix` is enabled and line contains `{`, attempt JSON parsing with prefix extraction (conditional on successful JSON parse)
2. Otherwise, if line starts with `{`, try JSON parsing without prefix
3. If JSON fails (or no `{` found), try logfmt parsing
4. If logfmt fails, try remaining supported input formats
5. If no format matches, process as raw line:
   - In concatenation mode (no filters, no sorting/following): pass through unfiltered
   - If any filters are applied: discard the line
   - In sorting or following mode: discard the line

**Prefix Extraction Behavior:**
- Prefix extraction is attempted only during JSON parsing when `--allow-prefix` is enabled
- If JSON parsing with prefix extraction fails, the line is NOT retried with other formats using prefix extraction
- Subsequent format attempts (logfmt, etc.) process the full line as-is without prefix extraction

### Processing Pipeline

**Data Path:**
```
Line → Prefix Extraction (optional) → Format Detection → JSON Parsing → Record Fields + Prefix → Downstream Features → Output with Prefix Prepended
```

**Key Properties:**
- Each line is independent
- Prefix (if present and `--allow-prefix` enabled) is extracted before format detection
- Parsing failures on one line do not affect subsequent lines
- Field extraction happens during JSON parsing
- Prefix is preserved through all downstream processing and prepended during output formatting
- All downstream features receive structured records with extracted fields (prefix is separate metadata)

### Error Handling

**Parsing Failures:**
- If a line cannot be parsed as JSON, an error is reported (or line is skipped depending on configuration)
- Parsing errors typically include line number and error details
- Invalid JSON may trigger fallback to logfmt parsing

**Type Handling in Filtering:**
- Field filtering (`-f` option): Comparisons are performed on string representations of field values (exact match, substring, wildcard, regex). No type coercion occurs; all values are treated as strings.
- Query filtering (`-q` option): Type-aware filtering with numeric operators (=, !=, >, <, >=, <=). When a numeric operator is used, the field value must be parseable as a number. If parsing fails, the record does not pass the filter and is discarded.
- Type preservation: JSON field types (string, number, boolean, null) are preserved in the parsed record and available to downstream features for type-aware operations.

**Unknown Formats (Raw Line Handling):**
- Lines that don't match any supported format are treated as raw content
- In concatenation mode without filters: raw lines pass through unfiltered
- In filtering mode: raw lines are discarded
- In sorting or following mode: raw lines are discarded
- This ensures predictable behavior: data is preserved in concatenation, filtered in structured modes

**Query Filtering Type Requirements:**
- Numeric comparisons (>, <, >=, <=, =, !=) require the field to be numeric. If the field cannot be parsed as a number, the record fails the filter.
- String-based field filtering has no type requirements; values are compared as strings regardless of their JSON type.

## Configuration & CLI

**CLI Flags:**
- `--input-format json` — Force JSON parsing regardless of content
- `--allow-prefix` — Enable recognition and preservation of non-JSON prefix before JSON content

**Environment Variables:**
- `HL_INPUT_FORMAT=json` — Force JSON format
- `HL_ALLOW_PREFIX=true` — Enable prefix handling

**Message Delimiter Configuration:** The `--delimiter` option is handled by the separate Input Message Delimiter feature.

**Configuration File** (`config.yaml`):
```yaml
input_format: auto    # or: json, logfmt
allow_prefix: false
```

**Timestamp Unit Configuration:** Unix timestamp unit configuration (if applicable) is handled by the Structured Logs Processing feature.

## Testing Requirements

### Unit Tests
- Valid JSON object: parsed correctly, fields extracted
- JSON with nested objects: nested fields accessible
- JSON with arrays: array elements accessible and indexable
- JSON with different field types (string, number, boolean, null): types preserved
- Line with non-JSON prefix + valid JSON: prefix extracted and preserved when `--allow-prefix` enabled and JSON parses successfully
- Line with `{` but prefix extraction fails: full line attempted with logfmt and other formats without prefix extraction
- Invalid JSON with valid logfmt: parsed as logfmt
- Invalid JSON and invalid logfmt: processed as raw line per format fallback strategy
- Empty JSON object: handled gracefully
- Very large JSON object: parsed without truncation or loss of data
- Multiple JSON records: each parsed independently
- JSON record with malformed nested object: error on that record only, subsequent records processed

### Integration Tests
- CLI: `hl --input-format json file.jsonl` forces JSON parsing
- CLI: `hl --input-format auto file.jsonl` auto-detects JSON
- CLI: `hl --allow-prefix file.log` with JSON-with-prefix logs processed correctly
- CLI: `hl file.jsonl` auto-detects JSON from content
- Large file: Multi-gigabyte NDJSON file processes without memory issues
- Mixed files: `hl file1.jsonl file2.jsonl` processes all files as JSON
- Filtering: `hl -f level=error file.jsonl` works on extracted `level` field
- Time filtering: `hl --since '2025-01-01' file.jsonl` works on extracted timestamp
- Nested field filtering: `hl -f request.method=GET file.jsonl` works on nested fields

### Edge Cases
- Empty file
- File with only whitespace/newlines
- JSON object with empty nested object: `{"meta":{}}`
- JSON object with empty array: `{"items":[]}`
- JSON with very long field name
- JSON with very long field value (> 1MB)
- JSON with Unicode characters
- JSON with escaped characters in strings
- Duplicate field names in JSON: each key/value pair is preserved independently in order of appearance; filtering uses OR logic (record passes if any duplicate key matches filter)
- Line with multiple JSON objects (not one per line): fallback to next format or treated as raw
- Malformed JSON (missing closing brace, trailing comma, etc.): fallback to next format or treated as raw
- Prefix that contains `{` character: prefix extraction fails, line attempted with other formats without prefix extraction
- Line with prefix but `--allow-prefix` disabled: prefix not extracted, line processed with format detection on full line

## Interactions with Other Features

This feature provides structured record parsing consumed by:

- **Structured Logs Processing** — Receives parsed JSON records and identifies predefined fields based on configuration
- **Field-Based Filtering** — Filters operate on extracted JSON fields (custom fields and those identified by Structured Logs)
- **Sorting** — Works on records with timestamp extracted by Structured Logs feature
- **Following** — Streams JSON records as they arrive
- **Concatenation** — Concatenates JSON records before Structured Logs processing
- **Human-Readable Formatting** — Formats JSON records with predefined fields mapped by Structured Logs
- **All Output Features** — Field visibility, themes, etc. work on both custom and extracted fields

For details on how predefined field extraction works, see the Structured Logs Processing feature specification.

For details on each other feature, see their respective specifications.

## Performance Characteristics

**Streaming JSON Parsing:**
- Each line is parsed independently and immediately
- Records are not buffered entirely; memory usage is bounded
- Parsing speed depends on JSON complexity and CPU capability

**Typical Performance (on modern hardware):**
- Simple JSON (few fields): ~1+ GB/s
- Complex JSON (nested objects/arrays): ~1+ GB/s
- Large field values: scales with field size

**Actual performance varies based on:**
- JSON complexity and nesting depth
- Field sizes
- CPU capabilities

## Future Enhancements (Out of Scope)

- Streaming JSON arrays (currently requires one object per line)
- JSON Schema validation
- Field renaming/aliasing configuration
- Automatic field type inference
- JSON pretty-printing in output (depends on output formatting feature)

## Notes

- JSON parsing happens after decompression (if applicable)
- Field extraction is deterministic based on field name recognition
- All downstream features operate on extracted fields
- JSON format detection is content-based; unambiguous files can be auto-detected without explicit configuration
- Nested field access (e.g., `request.method`) is supported in downstream filtering and display features
