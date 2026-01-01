# Feature Specification: Logfmt Input Format Support

**Feature Name:** Logfmt Input Format Support  
**Feature ID:** 005-logfmt-input-format  
**Status:** Existing Implementation (Documented)  
**Last Updated:** 2025-11-02  

## Clarifications

### Session 2025-11-02

- Q: When a logfmt record has duplicate keys, which value should be preserved? → A: Duplicate keys are NOT collapsed. Each key/value pair is treated as an independent field in the output, displayed in the same order they appear in the input. When filters apply to a key that appears multiple times, OR logic is used: if ANY of the key/value pairs with that key match the filter, the entire message passes the filter.
- Q: Does type inference in logfmt affect filtering operations? → A: No. Logfmt uses the same two-tier filtering approach as JSON: (1) Field filtering (`-f`) uses string-based matching only; (2) Query filtering (`-q`) with numeric operators requires the field value to successfully parse as a number. Type inference is used only for output formatting/display, not for filtering comparisons.
- Q: When logfmt parsing fails in auto mode, what should happen? → A: Same as JSON: try remaining supported input formats sequentially. If none match, process as raw line: pass through unfiltered in concatenation mode, or discard if filters are applied or sorting/following mode is enabled.
- Q: How should empty lines and records with no key-value pairs be handled? → A: Empty lines (blank or whitespace-only) are treated as raw content and follow the raw content fallback strategy: pass through unfiltered in concatenation mode, discard with filters or in sorting/following mode.

## Overview
</parameter>

The logfmt input format support feature enables `hl` to parse and process structured logs in logfmt format (key=value pairs). Logfmt logs are automatically detected and parsed into structured records, extracting fields that downstream features can filter, format, and display.

This feature works with line-delimited logfmt records where each line contains space-separated key=value pairs, and is transparent to all downstream features (filtering, formatting, sorting, etc.).

## User Stories & Acceptance Criteria

### US-1: Parse Line-Delimited Logfmt Logs
**As a** systems engineer  
**I want to** process logs in logfmt format (key=value pairs)  
**So that** I can analyze structured log output from tools and applications that use logfmt

**Acceptance Criteria:**
- Given a log file with one logfmt record per line (e.g., `level=info msg="User login" user_id=42`)
- When I run `hl logs.log`
- Then each line is parsed as a separate logfmt record
- And key-value pairs are extracted as fields
- And parsing errors on a single line do not prevent processing of subsequent lines

### US-2: Extract Key-Value Pairs
**As a** a user processing logfmt logs  
**I want to** have key-value pairs extracted as individual fields  
**So that** I can filter and display them consistently

**Acceptance Criteria:**
- Given logfmt records with various key-value pairs
- When records are processed
- Then each key-value pair is extracted as a named field
- And field values are preserved as strings (the underlying logfmt representation)
- And unquoted values are inspected for type inference to enable better output formatting (e.g., numeric color highlighting)
- And quoted values are treated as literal strings without type inference
- And multiple fields in a single record are all extracted

### US-3: Automatic Format Detection
**As a** a user processing mixed log formats  
**I want to** process logfmt logs without explicitly specifying the format  
**So that** logfmt is automatically detected based on content

**Acceptance Criteria:**
- Given a log file with logfmt content (first character is not `{`)
- When I run `hl logfile.log` without the `--input-format` flag
- Then logfmt format is automatically detected
- And the file is parsed as logfmt without explicit configuration

### US-4: Explicit Logfmt Format Specification
**As a** a user with ambiguous log content  
**I want to** explicitly specify logfmt format even if content might be ambiguous  
**So that** parsing is deterministic regardless of content characteristics

**Acceptance Criteria:**
- Given a log file
- When I run `hl --input-format logfmt logfile.log`
- Then the file is forcibly parsed as logfmt
- And non-logfmt content results in parsing errors

### US-5: Handle Quoted Values
**As a** a user with complex logfmt data  
**I want to** have quoted values and escaped characters handled correctly  
**So that** values containing spaces and special characters are preserved

**Acceptance Criteria:**
- Given logfmt records with quoted string values (e.g., `msg="error: something went wrong"`)
- When records are processed
- Then quoted values are parsed correctly
- And escaped characters within quotes are unescaped properly
- And the resulting field value contains the actual string content without quotes

### US-6: Handle Multiple Values per Key
**As a** a user with repeated keys in logfmt records  
**I want to** have clarity on how multiple values for the same key are handled  
**So that** I understand the behavior with non-standard logfmt

**Acceptance Criteria:**
- Given logfmt records with the same key appearing multiple times
- When records are processed
- Then the behavior is deterministic (typically first or last value is used)
- And the record is still parsed without errors

### US-7: Infer and Preserve Value Types for Better Formatting
**As a** a DevOps engineer  
**I want to** have unquoted logfmt values inferred as their semantic types (numbers, booleans)  
**So that** the output can apply appropriate formatting and color highlighting

**Acceptance Criteria:**
- Given logfmt records with unquoted values (e.g., `status=200 error=false duration=1.5`)
- When records are processed
- Then unquoted numeric values are inferred as numbers
- And unquoted boolean values (`true`, `false`) are inferred as booleans
- And this type inference is used for output formatting (e.g., numeric color, boolean highlighting)
- And explicitly quoted values (e.g., `msg="200 requests"`) are always treated as strings regardless of content
- And filtering and comparisons still work on the string values for consistency

### US-8: Streaming Logfmt Parsing
**As a** a user with very large logfmt log files  
**I want to** process huge logfmt files without loading them entirely into memory  
**So that** memory usage stays bounded regardless of file size

**Acceptance Criteria:**
- Given a multi-gigabyte logfmt log file
- When I run `hl huge-log.log`
- Then the file is parsed in a streaming fashion, one record at a time
- And memory usage stays bounded
- And processing completes in reasonable time

## Technical Specifications

### Input Format

**Logfmt Format:**
- Each line is a complete logfmt record
- Lines are separated by newline (LF, CR, or CRLF depending on configuration)
- Each record consists of space-separated key=value pairs
- Key names are unquoted identifiers
- Values can be unquoted strings, quoted strings, numbers, or booleans
- Each record is processed independently

**Key-Value Pairs:**
- Keys are alphanumeric identifiers (and underscores)
- Values follow the key after an `=` character
- In the logfmt standard, all values are semantically strings
- Unquoted values are terminated by whitespace or end of line
- Quoted values are enclosed in double quotes and can contain spaces and escaped characters
- **Duplicate Keys:** Duplicate keys are NOT collapsed. Each key/value pair is treated as an independent field in the output, displayed in the same order they appear in the input. When filters apply to a key with duplicates, OR logic is used: if ANY of the key/value pairs with that key match the filter condition, the entire message passes the filter.
- **Type Inference:** Unquoted values are inspected and inferred as numbers, booleans, null, or strings for output formatting purposes
  - Unquoted numeric values (e.g., `123`, `1.5`, `-42`) are inferred as numbers
  - Unquoted boolean values (`true`, `false`) are inferred as booleans
  - Unquoted `null` is inferred as null and displayed as null representation
  - All other unquoted values are treated as strings
- **Quoted Values:** Values enclosed in double quotes are always treated as literal strings, regardless of content
  - Example: `msg="200 requests"` is a string, even though it contains numbers
- For filtering and field comparisons, all values are compared as strings (the underlying representation)

### Format Detection

**Detection Order (in Auto Mode):**
1. If the line starts with `{`, parse as JSON (handled by JSON input format)
2. Otherwise, try logfmt parsing
3. If logfmt parsing fails, try remaining supported input formats
4. If no format matches, process as raw line:
   - In concatenation mode (no filters, no sorting/following): pass through unfiltered
   - If any filters are applied: discard the line
   - In sorting or following mode: discard the line

### Processing Pipeline

**Data Path:**
```
Line → Format Detection → Logfmt Parsing → Record Fields → Downstream Features
```

**Key Properties:**
- Each line is independent
- Parsing failures on one line do not affect subsequent lines
- Key-value extraction happens during parsing
- All downstream features receive structured records with extracted fields

### Error Handling

**Parsing Failures:**
- If a line cannot be parsed as logfmt, the line is attempted with remaining supported formats
- If all formats fail, the line is treated as raw content per the fallback strategy above
- Parsing errors typically include line number and error details
- Raw lines are handled based on mode: pass through in concatenation, discarded with filters or in structured modes

**Empty Records and Whitespace-Only Lines:**
- Empty lines (completely blank or containing only whitespace) are treated as raw content
- They follow the same fallback strategy as unparseable lines: pass through unfiltered in concatenation mode, discard if filters are applied or in sorting/following mode
- This ensures no data loss in concatenation mode while maintaining consistent filtering behavior

**Type Inference:**
- Logfmt format itself defines all values as strings (no type system)
- Type inference happens during parsing for **display formatting only** (not for filtering/comparison)
- Unquoted values are inspected to determine inferred type:
  - Values matching numeric patterns (integers, floats, including negative) → inferred as numbers
  - Values `true` or `false` → inferred as booleans
  - Value `null` → inferred as null
  - All other values → remain as strings
- Quoted values are **never** type-inferred; they remain as literal strings regardless of content
- Purpose of type inference: enable appropriate color highlighting and formatting in output (e.g., numeric values in blue, booleans in bold, null in special formatting)

**Filtering Behavior (Type Inference NOT Applied):**
- Field filtering (`-f` option): All values are compared as strings (exact match, substring, wildcard, regex). Type inference does not affect filtering.
- Query filtering (`-q` option): Numeric operators (=, !=, >, <, >=, <=) require the field value to be parseable as a number. If parsing fails, the record does not pass the filter. Type inference is not used for this determination.
- Example: `unquoted_count=200` and `quoted_count="200"` both fail numeric filtering in query mode if the underlying value cannot be converted to a number (they're both strings in logfmt)
- String representation is always used for filtering and comparisons, consistent with JSON input format

## Configuration & CLI

**CLI Flags:**
- `--input-format logfmt` — Force logfmt parsing regardless of content

**Environment Variables:**
- `HL_INPUT_FORMAT=logfmt` — Force logfmt format

**Message Delimiter Configuration:** The `--delimiter` option is handled by the separate Input Message Delimiter feature.

**Configuration File** (`config.toml`):
```toml
input_format = "auto"    # or: "json", "logfmt"
```

## Testing Requirements

### Unit Tests
- Valid logfmt record: parsed correctly, fields extracted
- Logfmt with simple key=value pairs: all extracted
- Logfmt with quoted values containing spaces: parsed correctly, treated as strings
- Logfmt with escaped characters in quotes: unescaped correctly
- Logfmt with unquoted numeric values: inferred as numbers for formatting
- Logfmt with unquoted boolean values: inferred as booleans for formatting
- Logfmt with unquoted null values: inferred as null for special formatting
- Logfmt with quoted numeric values (e.g., `count="123"`): treated as strings, not inferred as numbers
- Logfmt with mixed types: all parsed, types inferred appropriately for display
- Line without `{` prefix: detected as logfmt automatically
- Invalid logfmt: produces appropriate error
- Empty logfmt record (empty line): treated as raw content per fallback strategy
- Multiple records: each parsed independently
- Logfmt record with very long value: no truncation or loss of data
- Record with duplicate key names: each key/value pair is preserved independently in order of appearance; filtering uses OR logic (record passes if any duplicate key matches filter)
- Type preservation: unquoted `true` inferred as boolean, `false` inferred as boolean, `null` inferred as null, `"null"` treated as string literal

### Integration Tests
- CLI: `hl --input-format logfmt file.log` forces logfmt parsing
- CLI: `hl file.log` auto-detects logfmt from content
- Large file: Multi-gigabyte logfmt file processes without memory issues
- Mixed files: `hl file1.log file2.log` processes all files as logfmt
- Filtering: `hl -f status=200 file.log` works on extracted fields (string comparison)
- Time filtering: works on extracted timestamp field (if present)
- Output formatting: unquoted `200` displays with numeric color, `"200"` displays as string literal
- Type consistency: filtering treats both unquoted and quoted values as strings for comparison (type inference not applied)
- Query filtering with numeric values: `unquoted_status=200` passes numeric filtering if "200" parses as a number; `quoted_status="200"` also passes if the quoted string value "200" parses as a number

### Edge Cases
- Empty file
- File with only whitespace/newlines
- Logfmt with no key-value pairs (empty records): treated as raw content
- Logfmt with keys but no values (e.g., `key=`)
- Logfmt with malformed quoted values (unclosed quote)
- Logfmt with very long field names
- Logfmt with very long field values (> 1MB)
- Logfmt with Unicode characters
- Logfmt with non-ASCII characters in values
- Record with only spaces
- Record with tab-separated values (standard is space-separated)

## Interactions with Other Features

This feature provides structured record parsing consumed by:

- **Structured Logs Processing** — Receives parsed logfmt records and identifies predefined fields based on configuration
- **Field-Based Filtering** — Filters operate on extracted logfmt fields
- **Sorting** — Works on records with timestamp extracted by Structured Logs feature
- **Following** — Streams logfmt records as they arrive
- **Concatenation** — Concatenates logfmt records before Structured Logs processing
- **Human-Readable Formatting** — Formats logfmt records with predefined fields mapped by Structured Logs
- **All Output Features** — Field visibility, themes, etc. work on extracted fields

For details on how predefined field extraction works across all formats, see the Structured Logs Processing feature specification.

For details on each other feature, see their respective specifications.

## Performance Characteristics

**Streaming Logfmt Parsing:**
- Each line is parsed independently and immediately
- Records are not buffered entirely; memory usage is bounded
- Parsing speed depends on record complexity and CPU capability

**Typical Performance (on modern hardware):**
- Simple logfmt (few fields): ~100+ MB/s
- Complex logfmt (many fields, long values): ~50-100 MB/s
- Large field values: scales with field size

**Actual performance varies based on:**
- Number of key-value pairs per record
- Field value sizes
- CPU capabilities

## Future Enhancements (Out of Scope)

- Support for tab-separated values
- Custom field delimiters (currently space-only)
- Field aliasing or renaming during parsing
- Automatic nesting based on key patterns (e.g., `foo.bar=value`)

## Notes

- Logfmt parsing happens after decompression (if applicable)
- Format detection is content-based; logfmt is assumed if line doesn't start with `{`
- All downstream features operate on extracted fields
- Logfmt format detection is transparent; users don't need explicit configuration for basic usage
- **Type Inference for Display:** Unquoted values are inspected and types are inferred (number, boolean, null, string) to enable appropriate output formatting and color highlighting
- **String Semantics:** All values remain strings semantically; type inference is used only for formatting, not for filtering or comparison
- **Quoting for String Literals:** Values explicitly wrapped in double quotes are treated as literal strings; type inference is not applied regardless of content (e.g., `"null"` is a string, not null)
- **Null Representation:** The unquoted value `null` is recognized as a null type and formatted specially in output
- Field extraction and predefined field identification is handled by the Structured Logs Processing feature
