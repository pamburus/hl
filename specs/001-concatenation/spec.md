# Feature Specification: Multiple Log File Concatenation

**Feature Name:** Multiple Log File Concatenation
**Branch:** 001-concatenation
**Status:** Existing Implementation (Documented)
**Last Updated:** 2025-11-02

## Clarifications

### Session 2025-11-02

- Q: When a file can't be opened, should hl halt the entire operation or skip that file and continue? → A: All files are validated upfront before any processing. If any file cannot be opened, report errors immediately and exit with code 1. No partial output is produced.
- Q: Are there limits on the number of files or total size for concatenation? → A: No arbitrary hard limits. The system is constrained only by available file descriptors and system memory. Users can concatenate as many files as the system can handle.
- Q: What are the performance/latency targets for concatenation? → A: Focus on streaming latency - output begins as soon as data is available. No specific throughput targets; let underlying infrastructure determine throughput.
- Q: Should hl detect or deduplicate identical log entries across different input files? → A: No deduplication. Concatenation is a transparent pass-through operation; all entries are included as-is. Deduplication is left to downstream tools or manual post-processing.

## Overview

The concatenation feature enables users to process multiple log files in a single command, with `hl` automatically concatenating and formatting the output. This is the default operational mode when no sorting (`-s`/`--sort`) or following (`-F`/`--follow`) flags are specified.

**Key Principle:** Concatenation operates as a transparent pass-through. All log entries from all input files are included in the output exactly as they appear, with no filtering, deduplication, or reordering (unless explicitly requested via other flags).

## User Stories & Acceptance Criteria

### US-1: Basic Multi-File Concatenation
**As a** DevOps engineer
**I want to** process multiple log files in a single command
**So that** I can view logs from all sources in one continuous, formatted stream

**Acceptance Criteria:**
- Given multiple log files (`file1.log`, `file2.log`, `file3.log`) containing JSON logs
- When I run `hl file1.log file2.log file3.log`
- Then all logs from all three files are displayed sequentially in human-readable format
- And the output is automatically paged (unless `-P` flag is set)
- And the order of files on the command line is preserved in the output

### US-2: Wildcard Pattern Support
**As a** system administrator
**I want to** process all log files matching a pattern without listing each one
**So that** I can efficiently work with large numbers of log files

**Acceptance Criteria:**
- Given log files in current directory matching a pattern (`*.log`)
- When I run `hl *.log` or similar glob patterns
- Then the shell expands the pattern and all matching files are processed
- And files are processed in the order determined by the shell expansion

### US-3: Stdin as Input
**As a** DevOps engineer
**I want to** use stdin as input alongside file arguments
**So that** I can pipe logs from other tools (like `tail -f`, `kubectl logs`, etc.)

**Acceptance Criteria:**
- Given logs piped from another tool
- When I run `hl -` (dash indicates stdin)
- Then logs from stdin are processed with the same formatting as file inputs
- And if no files are specified and stdin is not a terminal, stdin is used by default
- And if no files are specified and stdin IS a terminal, help is printed

### US-4: Mixed File & stdin Input
**As a** DevOps engineer
**I want to** concatenate file-based logs with piped logs
**So that** I can combine logs from different sources (files and pipes)

**Acceptance Criteria:**
- Given log files and piped input
- When I run `hl file.log - another.log` (where `-` represents stdin)
- Then files are processed in the order specified, with stdin processed when encountered
- And the output is a continuous concatenation of all sources

### US-5: Compressed File Support
**As a** a system operator
**I want to** process compressed log files (gzip, bzip2, xz, zstd) alongside uncompressed files
**So that** I don't need to manually decompress logs before viewing

**Acceptance Criteria:**
- Given compressed log files (`.log.gz`, `.log.bz2`, `.log.xz`, `.log.zst`)
- When I run `hl *.log *.log.gz *.log.bz2 *.log.xz *.log.zst`
- Then all files (compressed and uncompressed) are concatenated in the specified order
- And compression format is auto-detected from file content, not just file extension
- And decompression is handled transparently

### US-6: Input File Tracking Display
**As a** a user analyzing logs
**I want to** see which file each log line came from
**So that** I can trace logs back to their source when needed

**Acceptance Criteria:**
- Given multiple input files
- When I run `hl file1.log file2.log file3.log`
- Then metadata about which input each log line came from is available to the output formatting system
- And input badges are generated for each source (file name or stdin identifier)
- And this metadata can be displayed by the output formatting feature when configured to do so

### US-7: Empty Input Handling
**As a** a user running hl interactively
**I want to** get clear feedback when no input is provided
**So that** I understand what went wrong

**Acceptance Criteria:**
- Given no file arguments and stdin connected to a terminal
- When I run `hl` with no arguments
- Then help message is printed to stdout
- And no error is logged
- And exit code is 0 (normal exit)

## Technical Specifications

### Input Processing Pipeline

**Concatenation is the primary mode of operation.** When no sorting or following flags are specified, all input sources are read sequentially and their contents are output in order.

**Process:**
1. Accumulate input sources (files, stdin, pipes) in the order specified on the command line
2. Read each input source completely before proceeding to the next
3. Pass segments from each source to downstream processing
4. Output completes only after all sources have been read

### Data Flow

**For Concatenation (default mode):**
1. Read first input source until exhausted
2. Read second input source until exhausted
3. Continue for all remaining input sources
4. Output is produced in the order read

**Key Properties:**
- Input sources are processed in CLI argument order
- Each input source is read completely before the next begins
- stdin (if specified) is processed at its position in the file list
- Metadata about which input each segment came from is available to downstream features

### Configuration Options

**CLI Flags (concatenation-specific):**
- `[FILE]...` — Positional arguments for input files (unlimited count)

**CLI Flags (general, relevant to concatenation):**
- `--max-message-size <SIZE>` — Max size per message (default: 64 MiB)
- `--delimiter <DELIM>` — Message delimiter (see Input Message Delimiter feature)

**Environment Variables:**
- `HL_MAX_MESSAGE_SIZE` — Override max message size

**Configuration File** (`config.yaml`):
```yaml
max_message_size: 64 MiB
```

### Error Handling

**File Validation:**
- All input files are validated and opened before any processing begins
- If any file cannot be opened (permission denied, file not found, etc.), an error is reported immediately
- All validation errors are reported to the user with details about which file(s) failed
- Application exits with status code 1 if any file validation fails
- No output is produced if file validation fails

**Handling of Input Read Failures During Processing:**
- Malformed input within a file does not stop processing of subsequent inputs
- Read errors during streaming are reported to stderr but do not prevent processing of remaining files

**Handling of stdin Failures:**
- Broken pipe errors indicate the consumer (e.g., pager) has closed; processing terminates gracefully
- Other IO errors are reported to the user

### Performance Characteristics

**Memory Characteristics:**
- Memory usage is bounded by buffer size + max message size
- Streaming ensures O(1) memory relative to input file size
- Default buffer (256 KiB) suitable for typical workloads

**Latency Characteristics:**
- Output begins as soon as the first data is available (streaming latency)
- No artificial buffering delays before output is produced
- Users receive responsive feedback without waiting for entire files to be read
- No specific throughput targets; performance determined by underlying infrastructure and system capabilities

**Scale Characteristics:**
- No arbitrary hard limits on the number of files that can be concatenated
- No limit on cumulative input size across all files
- System is constrained only by available file descriptors and available system memory
- Typical systems can handle hundreds or thousands of files in a single invocation
- Each open file consumes one file descriptor; system limits (often 1024 or configurable higher) apply

### Supported Input Formats

**Automatically Detected:**
- JSON (with optional non-JSON prefixes handled by a separate feature)
- logfmt (key=value pairs)
- Unix timestamps in various units

**Compression Support:**
- Compressed and uncompressed files can be mixed in a single invocation
- Compression format detection is transparent to concatenation

**Unknown Formats:**
- Concatenation mode uniquely passes unrecognized formats through transparently
- Files that are not recognized as any compression format and are not valid structured logs are still included in the output
- This allows concatenation to process mixed or non-standard content without filtering
- Note: Other modes (sorting, following) may handle unknown formats differently

## Implementation Notes

### Input Source Handling

**File Sources:**
- Resolved and opened by the input layer
- Processed in the order specified on the command line
- Auto-detection of compression is transparent

**stdin Source:**
- Specified on command line as `-`
- Defaults to stdin if no files specified and stdin is not a terminal
- Processed at the position it appears in the file list

### Delimiter Handling

Messages are delimited by a configurable character or string (default: LF). Delimiters are detected and segments are separated based on this boundary.

## Testing Requirements

### Unit Tests
- Concatenate 2 files: order preserved, all content from both files present
- Concatenate file + stdin: handled in correct order
- Stdin only (no files): recognized and processed
- No input (terminal): help is printed
- Mixed compressed/uncompressed: all decompressed correctly
- Empty files in list: handled gracefully
- Very long lines: handled without truncation (up to max message size)

### Integration Tests
- CLI: `hl file1.log file2.log` produces correct output
- CLI: `echo '...' | hl -` reads from stdin
- CLI: `hl *.log` with wildcard expansion
- Order: Files are processed in the order specified on command line
- Stdin handling: Default stdin when no files and stdin is not terminal
- Help display: No arguments with terminal shows help

### Edge Cases
- Files deleted during processing
- Permission denied on file
- Malformed input in middle of file
- Maximum message size exceeded
- Binary data in log files
- Different line endings in different files

## Future Enhancements (Out of Scope)

- Content-based file ordering (not just CLI argument order)
- Resume functionality for partially processed files
- Streaming to remote destinations (S3, HTTP, etc.)
- File rotation detection and handling

**Note:** Automatic deduplication is intentionally out of scope for concatenation mode, as the feature is designed as a transparent pass-through. Users requiring deduplication should pipe output to downstream tools or use other modes (e.g., sorting with deduplication flags if implemented).
