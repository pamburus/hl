# Feature Specification: Structured Logs Processing

**Feature Name:** Structured Logs Processing
**Feature ID:** 003-structured-logs
**Status:** Existing Implementation (Documented)
**Last Updated:** 2025-11-02

## Clarifications

### Session 2025-11-02

- Q: How should missing/null predefined fields be represented in the output record? → A: Missing predefined fields are omitted from the record entirely. However, the configuration file (`fields.predefined.time.show` and `fields.predefined.level.show`) controls rendering behavior for specific fields in output. Some fields like timestamp and level may be rendered with special placeholders when missing, as configured in the configuration file.
- Q: What happens when a timestamp field value doesn't match any recognized format? → A: Unknown timestamp formats are treated as string literals and output as-is, but truncated to match the output width specified by `--time-format` option. In sorting or following modes that require timestamp-based operations, records with unrecognized timestamps are discarded from those operations. Note: `--time-format` affects output rendering and `--since`/`--until` filtering, NOT input timestamp parsing.
- Q: How are caller field components (name, file, line) parsed and displayed? → A: All caller components are parsed. When displaying, they are output in order: caller name, then `punctuation.caller-name-file-separator`, then file, then colon, then line. The entire block is preceded by `punctuation.source-location-separator`. If any component is missing, only non-empty components are displayed with appropriate separators.
- Q: How are unknown/unrecognized level values handled? → A: Behavior depends on `fields.predefined.level.show` configuration: (1) If set to "never" or "auto", unknown levels are not treated as predefined fields; they appear as regular custom fields. (2) If set to "always", unknown level values are replaced with "(?)" placeholder in output. (3) In filtering: if level-based filtering is applied and the level cannot be recognized, the record is discarded.

## Overview

The structured logs processing feature provides a unified mechanism for identifying and extracting predefined fields (level, message, timestamp, logger, caller) from any parsed input format. After input parsing (JSON, logfmt, or other formats), records are processed by the structured logs engine to detect and map field names according to configurable patterns, making this metadata available to downstream features.

This feature operates consistently across all input formats and is transparent to downstream features like filtering, sorting, and formatting.

## User Stories & Acceptance Criteria

### US-1: Extract Log Level Field
**As a** DevOps engineer
**I want to** have the log level field automatically identified and extracted from structured logs
**So that** I can filter by severity level and apply level-specific formatting

**Acceptance Criteria:**
- Given parsed records with a field named according to level patterns (configured in `fields.predefined.level.names`)
- When the record is processed
- Then the level field is extracted and mapped to a canonical `level` representation
- And the extracted level can be used by downstream features for filtering and formatting
- And if multiple fields match level patterns, the first configured match is used

### US-2: Extract Message Field
**As a** application developer
**I want to** have the message/log text field automatically identified
**So that** it can be displayed and filtered consistently

**Acceptance Criteria:**
- Given parsed records with a field name matching message patterns (configured in `fields.predefined.message.names`)
- When the record is processed
- Then the message field is extracted and mapped to canonical form
- And the message is available to downstream features for display and querying
- And if no configured message field is found, no message is extracted

### US-3: Extract Timestamp Field
**As a** a system administrator
**I want to** have the timestamp field automatically identified and parsed
**So that** time-based filtering, sorting, and display work correctly

**Acceptance Criteria:**
- Given parsed records with a field named according to timestamp patterns (configured in `fields.predefined.time.names`)
- When the record is processed
- Then the timestamp field is extracted and parsed
- And various timestamp formats (RFC-3339, Unix timestamps, etc.) are recognized and normalized
- And the normalized timestamp is used by downstream features for filtering, sorting, and display
- And if multiple timestamp fields exist, the first configured match is used

### US-4: Extract Logger Field
**As a** a developer debugging applications
**I want to** have the logger name field automatically identified
**So that** I can filter logs by source component

**Acceptance Criteria:**
- Given parsed records with a field named according to logger patterns (configured in `fields.predefined.logger.names`)
- When the record is processed
- Then the logger field is extracted and mapped to canonical form
- And the logger name is available to downstream features for display and filtering

### US-5: Extract Caller Information
**As a** a developer
**I want to** have the caller/source location field automatically identified
**So that** I can trace where log messages originated

**Acceptance Criteria:**
- Given parsed records with a field named according to caller patterns (configured in `fields.predefined.caller.names`, `fields.predefined.caller-file.names`, `fields.predefined.caller-line.names`)
- When the record is processed
- Then the caller information is extracted and parsed into components (name, file, line)
- And caller components are available separately to downstream features
- And when displayed, components are rendered in order: caller name, separator, file, colon, line
- And missing components are gracefully omitted from display while maintaining separator consistency

### US-6: Configurable Field Recognition
**As a** a DevOps engineer with custom logging standards
**I want to** customize which field names are recognized as predefined fields
**So that** I can adapt hl to my organization's logging conventions

**Acceptance Criteria:**
- Given a configuration file with custom field name patterns in `fields.predefined`
- When logs are processed
- Then the custom field names are recognized and mapped according to the configuration
- And all input formats use the same configuration for consistent field extraction
- And configuration can be changed without modifying code

### US-7: Format-Independent Field Extraction
**As a** a user processing mixed log formats
**I want to** have field extraction work the same way regardless of input format
**So that** I get consistent behavior with JSON, logfmt, or other formats

**Acceptance Criteria:**
- Given records parsed from different formats (JSON, logfmt)
- When they are processed through the structured logs engine
- Then predefined fields are extracted and mapped identically
- And downstream features receive records with the same extracted fields regardless of source format
- And configuration applies uniformly across all formats

### US-8: Preserve Unmapped Fields
**As a** a user
**I want to** have custom fields that don't match predefined patterns preserved
**So that** I can access all data even if it's not mapped to standard fields

**Acceptance Criteria:**
- Given parsed records with fields that don't match any predefined patterns
- When the record is processed
- Then these custom fields are preserved and passed through
- And they remain accessible to downstream features
- And they can be displayed or filtered if downstream features support it

### US-9: Handle Missing Predefined Fields
**As a** a user with incomplete log records
**I want to** have graceful handling when predefined fields are not present
**So that** processing continues without errors

**Acceptance Criteria:**
- Given parsed records that don't contain any of the configured predefined field names
- When the record is processed
- Then the record is processed successfully with those fields omitted from the output record
- And downstream features treat missing fields as not available rather than erroring
- And the record can still be displayed and processed based on custom fields
- And certain fields (timestamp, level) may have special placeholder display behavior controlled by configuration (`fields.predefined.time.show`, `fields.predefined.level.show`)

## Technical Specifications

### Processing Pipeline

**Data Path:**
```
Parsed Record (from any format) → Predefined Field Detection → Field Extraction & Mapping → Enhanced Record → Downstream Features
```

**Key Components:**
1. **Input:** Parsed record from any input format (JSON, logfmt, etc.) containing arbitrary fields
2. **Detection:** Field names are matched against configured predefined patterns
3. **Extraction:** Matching fields are extracted and mapped to canonical forms
4. **Output:** Enhanced record with both custom fields and mapped predefined fields
5. **Downstream:** All features receive records with consistent predefined field structure

### Predefined Fields

**Standard Predefined Fields:**
- **level** — Log severity level (error, warning, info, debug, trace, etc.)
- **message** — Log message or text content
- **timestamp** — Timestamp of the log event
- **logger** — Source logger or component name
- **caller** — Source code location (file/function/line)

### Field Recognition Configuration

**Location:** `fields.predefined` section in `config.yaml`

**Configuration Structure:**
- Each predefined field has a `names` array listing recognized field name patterns
- Recognition is case-sensitive (configuration specifies exact names to match)
- The first matching name in the configuration order is used

**Example Configuration:**
```yaml
fields:
  predefined:
    level:
      names: ["level", "LEVEL", "Level"]
    message:
      names: ["msg", "message", "MESSAGE", "Message"]
    time:
      names: ["ts", "timestamp", "time"]
    logger:
      names: ["logger", "LOGGER", "Logger"]
    caller:
      names: ["caller", "CALLER", "Caller"]
```

### Field Extraction Behavior

**Recognition Process:**
1. For each predefined field type (level, message, timestamp, etc.)
2. Check if any field in the record matches the configured names (in order)
3. If match found, extract and map to canonical form
4. If no match found, that predefined field is omitted from the output record
5. For timestamp and level fields, the configuration (`show` property) controls whether missing fields are displayed with special placeholders during rendering

**Multiple Matches:**
- If a record contains multiple fields that could map to the same predefined field, the first configured match is used
- Other similarly-named fields are preserved as custom fields

**Type Conversion:**
- Field values are converted to appropriate types for the predefined field
- Timestamps are parsed into normalized form
- Level values are normalized to canonical level names if applicable
- Caller information is parsed into components (name, file, line) when possible
- String values remain as-is if type conversion is not applicable
- Missing predefined fields are not present in the record; they are not represented as null, empty, or default values

### Caller Field Parsing

**Caller Component Extraction:**
- When a caller field is matched, it is parsed to extract components:
  - **Caller name** (function/method name)
  - **Caller file** (source file path)
  - **Caller line** (line number)
- Additionally, `caller-file` and `caller-line` fields can be matched from separate input fields
- All components are optional; records may have any subset of caller information

**Caller Display Format:**
- When displaying caller information, components are rendered in sequence:
  1. Caller name (if present)
  2. `punctuation.caller-name-file-separator` (if name and file/line are both present)
  3. Caller file (if present)
  4. `:` (colon separator, if both file and line are present)
  5. Caller line (if present)
- The entire caller block is preceded by `punctuation.source-location-separator`
- If any component is missing, only non-empty components are displayed with appropriate separators

### Timestamp Parsing

**Supported Formats:**
- RFC-3339 with timezone (e.g., `2025-01-01T10:00:00Z`, `2025-01-01T10:00:00+01:00`)
- Unix timestamps (seconds, milliseconds, microseconds, nanoseconds)
- ISO 8601 date-time format without timezone (e.g., `2025-01-01 10:00:00`, `2025-01-01 10:00:00.123`)

**Behavior:**
- Timestamps are automatically detected and converted to a normalized representation
- Timezone information in RFC-3339 is preserved
- Naive datetime (without timezone) assumes UTC

**Unknown Format Handling:**
- If a timestamp field value doesn't match any recognized format, it is treated as a string literal
- Unknown timestamp values are output as-is but truncated to the width of the output time format (as determined by `--time-format` in the output rendering, not input parsing)
- In concatenation mode: unrecognized timestamps are displayed as string literals with truncation to output width
- In sorting mode: records with unrecognized timestamps are discarded from sorting and excluded from output
- In following mode: records with unrecognized timestamps are discarded from following operations and excluded from output
- This behavior ensures consistent time-based operations while preserving data in concatenation mode

**Unix Timestamp Unit Detection:**
When a numeric timestamp is detected, the unit (seconds, milliseconds, microseconds, nanoseconds) can be:
- Auto-detected based on the magnitude of the number
- Explicitly specified via `--unix-timestamp-unit` option or `HL_UNIX_TIMESTAMP_UNIT` environment variable

### Level Normalization

**Standard Levels:**
- `trace`
- `debug`
- `info`
- `warning`
- `error`

**Normalization:**
- Input level values are mapped to standard levels based on configuration
- Multiple aliases for the same level (e.g., "warn" → "warning", "err" → "error") are handled per configuration
- Level variants are configured in `fields.predefined.level.variants` with field names and mapped values

**Unknown Level Handling:**
- Behavior depends on the `fields.predefined.level.show` configuration setting:

  | Configuration | Behavior |
  |---------------|----------|
  | `show: "never"` | Unknown level values are not treated as predefined fields; displayed as regular custom fields |
  | `show: "auto"` | Unknown level values are not treated as predefined fields; displayed as regular custom fields |
  | `show: "always"` | Unknown level values are replaced with "(?)" placeholder in output |

- In level-based filtering (e.g., `--level error`): records with unrecognized level values are discarded
- This ensures filtering operates only on levels that can be reliably classified

## Configuration & CLI

**CLI Flags:**
- `--unix-timestamp-unit <UNIT>` — Specify Unix timestamp unit for ambiguous numeric timestamps (auto/s/ms/us/ns)

**Environment Variables:**
- `HL_UNIX_TIMESTAMP_UNIT=ms` — Set Unix timestamp unit

**Configuration File** (`config.yaml`):
```yaml
fields:
  predefined:
    time:
      names: [field names to recognize as timestamp]
      show: [placeholder display strategy when timestamp is missing]
    level:
      names: [field names to recognize as level]
      show: [placeholder display strategy when level is missing]
    message:
      names: [field names to recognize as message]
    logger:
      names: [field names to recognize as logger]
    caller:
      names: [field names to recognize as caller]
  ignore: [wildcard patterns for fields to ignore]
  hide: [exact field names to hide]
```

**Field Rendering Behavior:**
- Most predefined fields: Missing fields are omitted from records and not rendered
- Timestamp field: Missing behavior controlled by `fields.predefined.time.show` configuration
- Level field: Missing behavior controlled by `fields.predefined.level.show` configuration; unknown levels also subject to this setting
- Caller field: Components (name, file, line) are displayed with appropriate separators from punctuation config
- These configuration properties specify how missing and unknown values are displayed (e.g., with placeholders or special indicators)

**Timestamp Output Format Consistency:**
- All timestamp values in output (both parsed and unrecognized string literals) are constrained to the width of the output time format
- The `--time-format` option determines:
  - The format of parsed timestamps in the output
  - The display width for unrecognized timestamp string literals
  - The timestamp format for `--since` and `--until` filtering options
- `--time-format` does NOT affect how input timestamps are parsed; input parsing recognizes all supported formats regardless of this option
- Unrecognized timestamp strings are truncated to fit the output width, ensuring consistent output alignment

**Caller Output Format:**
- Caller components are rendered using configuration from `punctuation` section:
  - `punctuation.source-location-separator` — Precedes the entire caller block
  - `punctuation.caller-name-file-separator` — Separates caller name from file information
  - Colon (`:`) — Always separates file from line number (hardcoded)
- Display order is: [source-location-separator] [caller-name] [caller-name-file-separator] [file] [:] [line]
- Only non-empty components are included in output

**Field Recognition:** Configuration is via `config.yaml` with the `fields.predefined` section. CLI flags for field recognition patterns are not available; all configuration is file-based.

## Testing Requirements

### Unit Tests
- Record with level field matching config: level extracted and mapped correctly
- Record with message field matching config: message extracted correctly
- Record with timestamp field matching config: timestamp parsed and converted
- Record with logger field matching config: logger extracted correctly
- Record with caller field matching config: caller extracted correctly
- Record with multiple fields matching same predefined type: first configured match used
- Record with no matching predefined fields: all predefined fields marked as absent
- Record with custom fields only: custom fields preserved, predefined fields absent
- Record with mixed predefined and custom fields: both preserved correctly
- RFC-3339 timestamps: parsed correctly (e.g., `2025-01-01T10:00:00Z`)
- Unix timestamps in seconds: parsed correctly when `--unix-timestamp-unit s` or auto-detected
- Unix timestamps in milliseconds: parsed correctly when `--unix-timestamp-unit ms` or auto-detected
- Unix timestamps in microseconds: parsed correctly with `--unix-timestamp-unit us`
- Unix timestamps in nanoseconds: parsed correctly with `--unix-timestamp-unit ns`
- ISO 8601 datetime (naive): parsed correctly (e.g., `2025-01-01 10:00:00`)
- ISO 8601 datetime with fractional seconds: parsed correctly (e.g., `2025-01-01 10:00:00.123`)
- Floating point Unix timestamps: fractional part preserved based on unit
- Level values in various formats: normalized according to configuration
- Empty predefined field names list: no fields of that type extracted
- Configuration with case-sensitive matching: field names matched exactly as configured

### Integration Tests
- JSON records: fields extracted according to configuration
- Logfmt records: fields extracted with same configuration
- Mixed JSON and logfmt: fields extracted consistently
- Filtering by extracted level field: works correctly
- Filtering by extracted message field: works correctly
- Time filtering by extracted timestamp: works correctly
- Sorting by extracted timestamp: works correctly
- Custom fields and predefined fields coexist: both available to downstream features

### Edge Cases
- Record with null predefined field value: handled gracefully
- Record with predefined field having unexpected type: conversion attempted or value preserved
- Very long message field: no truncation
- Very nested custom field alongside predefined fields: both preserved
- Duplicate predefined field names in configuration: first occurrence used
- Empty configuration (no predefined names): no fields extracted as predefined
- Field name that matches multiple configured patterns: first match used
- Record with level value that doesn't match any configured variant: handling depends on `level.show` setting
- Record with level filtering applied and unrecognized level: record is discarded from output

## Interactions with Other Features

This feature is consumed by and works with:

- **All Input Formats** (JSON, logfmt, others) — Receives parsed records and extracts predefined fields
- **Field-Based Filtering** — Filters can operate on extracted predefined fields or custom fields
- **Level-Based Filtering** — Uses extracted level field for severity filtering
- **Time-Range Filtering** — Uses extracted timestamp field for temporal filtering
- **Sorting** — Uses extracted timestamp for chronological sorting
- **Following** — Processes records for predefined field extraction as they arrive
- **Concatenation** — Extracts predefined fields from concatenated records
- **Human-Readable Formatting** — Uses extracted fields for display
- **All Output Features** — Field visibility, themes, etc. work on both extracted and custom fields

For details on each feature, see their respective specifications.

## Performance Characteristics

**Field Extraction:**
- Field name matching is performed once per record during processing
- Matching is sequential (stops at first match per field type)
- Timestamp parsing happens only for extracted timestamp fields

**Typical Performance:**
- Field extraction adds minimal overhead to record processing
- Performance is proportional to number of fields in record and number of configured patterns

**Actual performance varies based on:**
- Complexity of timestamp format parsing
- Number of fields in each record
- Number of configured predefined field patterns

## Future Enhancements (Out of Scope)

- Dynamic field type inference
- Nested field path extraction for predefined fields
- Regular expression field matching (currently exact string matching)
- Custom field normalization rules
- Predefined field validation against schemas

## Notes

- Field extraction is deterministic based on configuration
- Configuration can be modified to adapt to different logging standards
- All input formats use the same configuration and extraction logic
- Predefined fields are always extracted before downstream processing
- Custom fields that don't match predefined patterns are preserved and available
- The structured logs processing feature is a unifying layer across all input formats
