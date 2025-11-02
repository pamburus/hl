# Feature Specification: Input Message Delimiter

**Feature Name:** Input Message Delimiter  
**Feature ID:** 006-input-message-delimiter  
**Status:** Existing Implementation (Documented)  
**Last Updated:** 2025-11-02  

## Clarifications

### Session 2025-11-02

- Q: When auto-detecting with mixed line endings (LF, CR, CRLF), which should be the delimiter? → A: LF is the canonical delimiter in auto-detection mode. When scanning input, LF is treated as the primary line ending. If CRLF is encountered, it is processed as LF (the CR is treated as part of the previous message/content and ignored as a delimiter). This ensures deterministic behavior and handles most real-world mixed line-ending cases.
- Q: How should empty messages (consecutive delimiters) be handled? → A: Empty messages are passed through to the downstream parsing layer as valid (but empty) records. The parsing layer then decides the fate: in concatenation mode, empty records pass through unaltered; in filtering/sorting mode, empty records are discarded during parsing/filtering.
- Q: When CRLF is the delimiter, how should orphaned CR or LF characters be handled? → A: Orphaned CR or LF characters are treated as regular message content, not delimiters. Only the complete CRLF sequence (`\r\n`) triggers a message split. This ensures strict CRLF matching and deterministic behavior when input contains mixed or partial line endings.
- Q: When a specified delimiter is not found in the input, should this be an error or silent operation? → A: Silent operation. If the specified delimiter is not found, the entire input is treated as a single message. This is normal behavior, not an error condition. No warning is issued. This provides flexibility and aligns with Unix philosophy where absence of a delimiter is a valid input state.

## Overview

The input message delimiter feature enables users to customize how input streams are split into individual log messages. By default, messages are delimited by newline characters, but users can specify alternative delimiters (NUL, CR, LF, CRLF, or custom strings) to handle different log formats and data sources.

This feature operates at the input layer before any parsing, allowing flexibility in how different log sources are segmented into individual records.

## User Stories & Acceptance Criteria

### US-1: Default Newline Delimiter
**As a** a standard user  
**I want to** have log messages split by newlines automatically  
**So that** I don't need to configure anything for typical line-delimited logs

**Acceptance Criteria:**
- Given a log input with lines separated by newline characters (LF, CR, or CRLF)
- When I run `hl logfile.log` without specifying a delimiter
- Then messages are automatically detected and split on newline boundaries
- And the detection handles LF, CR, and CRLF line endings transparently
- And no configuration is needed

### US-2: Custom Delimiter Specification
**As a** a system engineer  
**I want to** specify a custom delimiter for log messages  
**So that** I can process logs using non-standard delimiters

**Acceptance Criteria:**
- Given logs delimited by a character or string other than newline
- When I run `hl --delimiter '<delimiter>' logfile.log`
- Then messages are split using the specified delimiter
- And the specified delimiter is used consistently throughout processing
- And configuration can be changed without modifying code

### US-3: NUL Delimiter
**As a** a user with NUL-delimited input  
**I want to** process messages delimited by NUL bytes  
**So that** I can handle logs from tools that output NUL-separated records

**Acceptance Criteria:**
- Given input where messages are separated by NUL bytes
- When I run `hl --delimiter NUL logfile.log`
- Then messages are split on NUL bytes
- And the input is processed correctly

### US-4: Carriage Return Delimiter
**As a** a user with CR-delimited input  
**I want to** process messages delimited by carriage return  
**So that** I can handle logs using CR-only line endings

**Acceptance Criteria:**
- Given input where messages are separated by CR (carriage return) characters
- When I run `hl --delimiter CR logfile.log`
- Then messages are split on CR boundaries
- And the input is processed correctly

### US-5: LF Delimiter
**As a** a user  
**I want to** explicitly specify LF as the delimiter  
**So that** I have control over which newline format is used

**Acceptance Criteria:**
- Given input where messages are separated by LF characters
- When I run `hl --delimiter LF logfile.log`
- Then messages are split on LF boundaries
- And only LF is used as the delimiter (not CR or CRLF combinations)

### US-6: CRLF Delimiter
**As a** a user with Windows-style line endings  
**I want to** explicitly specify CRLF as the delimiter  
**So that** messages are split on Windows line endings (CR+LF pairs)

**Acceptance Criteria:**
- Given input where messages are separated by CRLF (CR+LF) sequences
- When I run `hl --delimiter CRLF logfile.log`
- Then messages are split on CRLF boundaries
- And both CR and LF must be present together for a split to occur

### US-7: Custom String Delimiter
**As a** a user with arbitrarily delimited input  
**I want to** specify a custom multi-character string as a delimiter  
**So that** I can handle logs with unusual delimiters

**Acceptance Criteria:**
- Given logs where messages are separated by a custom string (e.g., `|||`, `--END--`, `:::`)
- When I run `hl --delimiter '|||' logfile.log`
- Then messages are split using the exact string as a delimiter
- And the delimiter is applied consistently throughout the input

### US-8: Environment Variable Configuration
**As a** a system administrator  
**I want to** set the delimiter via environment variable  
**So that** I can configure it systemwide or for specific commands

**Acceptance Criteria:**
- Given an environment variable `HL_DELIMITER` set to a delimiter value
- When I run `hl logfile.log`
- Then the delimiter from the environment variable is used
- And CLI flag overrides the environment variable if specified

## Technical Specifications

### Delimiter Types

**Predefined Delimiters:**
- **NUL** — NUL byte (`\0`), useful for data from tools that output NUL-separated records
- **CR** — Carriage return (`\r`), single CR character
- **LF** — Line feed (`\n`), single LF character (Unix/Linux standard)
- **CRLF** — Carriage return + line feed (`\r\n`), Windows standard. Only the complete CRLF sequence triggers a message split; orphaned CR or LF characters are treated as regular message content.

**Custom Delimiters:**
- Any custom string can be specified as a delimiter
- The delimiter is matched literally in the input stream
- Multi-byte delimiters are supported

**Auto-Detection (Default):**
- If no delimiter is specified, the input is scanned for newline patterns
- In auto-detection mode, LF is the canonical line ending delimiter
- CRLF sequences are handled by treating LF as the delimiter and ignoring the preceding CR
- This ensures deterministic, predictable behavior when input contains mixed line endings
- The handling is transparent to the user; all common newline formats (LF, CR, CRLF) are processed correctly

### Processing Pipeline

**Data Path:**
```
Input Stream → Delimiter-Based Message Splitting → Individual Messages → Parsing → Downstream Features
```

**Key Properties:**
- Delimiter splitting happens at the lowest level, before any parsing
- Messages are identified by the boundaries defined by the delimiter
- All subsequent processing (format detection, parsing, field extraction) operates on the delimited messages

### Error Handling

**Delimiter Specification:**
- If an invalid delimiter is specified, an error is reported

**Delimiter Not Found:**
- If a specified delimiter is not found in the input, the entire input is treated as a single message
- This is normal operation, not an error condition
- No warning or error message is issued
- This provides flexibility for inputs that may or may not contain the specified delimiter

**Empty Messages:**
- When consecutive delimiters occur (producing empty message content), the empty message is passed through to the downstream parsing layer
- The parsing layer decides the outcome:
  - In concatenation mode: empty records pass through unaltered as blank lines
  - In filtering/sorting/following mode: empty records are discarded during parsing or filtering
- This behavior is consistent with the raw content fallback strategy used by input format features

**Parsing of Delimited Messages:**
- Once messages are delimited, each message is passed to the downstream parsing layer
- Parsing errors may result if a message is malformed or too large for the input format parser

## Configuration & CLI

**CLI Flags:**
- `--delimiter <DELIM>` — Specify message delimiter (NUL, CR, LF, CRLF, or custom string)

**Environment Variables:**
- `HL_DELIMITER=<delim>` — Set message delimiter

**Configuration File** (`config.yaml`):
Currently, delimiter configuration is not supported in config files; only CLI and environment variables are available.

**Supported Values:**
- `NUL` — NUL byte delimiter
- `CR` — Carriage return delimiter
- `LF` — Line feed delimiter
- `CRLF` — Carriage return + line feed delimiter
- Any other string — Custom delimiter (applied literally)

## Testing Requirements

### Unit Tests
- Default newline detection: LF-delimited input split correctly
- Default newline detection: CR-delimited input split correctly
- Default newline detection: CRLF-delimited input split correctly
- NUL delimiter: input split on NUL bytes
- CR delimiter: input split on CR characters
- LF delimiter: input split on LF characters
- CRLF delimiter: input split on CRLF sequences only; orphaned CR or LF characters preserved as message content
- Custom string delimiter: input split on exact string matches
- Multi-byte custom delimiter: handled correctly
- No delimiter in input: entire input treated as single message
- Empty messages between delimiters: handled gracefully
- Very long message between delimiters: no truncation

### Integration Tests
- CLI: `hl --delimiter NUL file.log` splits on NUL bytes
- CLI: `hl --delimiter '|||' file.log` splits on custom string
- Environment variable: `HL_DELIMITER=CR hl file.log` uses CR delimiter
- CLI override: `--delimiter` flag overrides environment variable
- Mixed delimiters in input: delimiter consistency

### Edge Cases
- Empty input
- Input with only delimiters (no messages)
- Delimiter at start or end of input
- Overlapping delimiter patterns
- CRLF delimiter with orphaned CR or LF: orphaned characters treated as content, not delimiters
- Very long delimiter string
- Delimiter contained within message data (test message integrity)

## Interactions with Other Features

This feature provides the message segmentation layer consumed by:

- **All Input Formats** (JSON, logfmt, others) — Receive delimited messages and parse them
- **Concatenation** — Messages are split by delimiter before concatenation
- **Compressed Input** — Delimiter-based splitting happens after decompression
- **All Downstream Features** — All features operate on messages delimited by this feature

For details on how each feature uses delimited messages, see their respective specifications.

## Performance Characteristics

**Delimiter Scanning:**
- Delimiter matching is performed once per input stream during segmentation
- Performance depends on delimiter complexity and input size
- Simple delimiters (single byte) are faster than complex multi-byte delimiters

**Typical Performance (on modern hardware):**
- Single-byte delimiter (LF, CR, NUL): ~GiB/s scanning speed
- Multi-byte delimiter: proportional to pattern complexity

**Actual performance varies based on:**
- Delimiter size and complexity
- Input stream characteristics
- Message size distribution

## Future Enhancements (Out of Scope)

- Escape sequences for delimiters containing special characters
- Delimiters based on regular expressions
- Variable-length delimiters
- Hierarchical delimiters (different delimiters at different nesting levels)

## Notes

- Delimiter specification happens at the input layer, before any format parsing
- Default behavior (auto-detect newlines) works transparently for standard line-delimited logs
- Custom delimiters are applied literally; no pattern matching or regex support
- Delimiter must be specified correctly; incorrect delimiter specification may result in malformed message boundaries
- All input formats share the same delimiter configuration
- Environment variable `HL_DELIMITER` provides a way to set delimiter globally across invocations
