# Feature Specification: Compressed Input Support

**Feature Name:** Compressed Input Support
**Feature ID:** 002-compressed-input
**Status:** Existing Implementation (Documented)
**Last Updated:** 2025-11-02

## Clarifications

### Session 2025-11-02

- Q: When decompression fails for a file, should hl halt the entire operation or continue processing remaining files? → A: Halt on first decompression failure with exit code 1. This is consistent with file validation strategy and prevents partial or corrupted output.
- Q: What is an acceptable time to process large (1-10 GB) compressed log files? → A: No specific absolute time targets. Decompression overhead should not exceed 50% compared to processing uncompressed files. Performance is determined by the underlying compression algorithm and hardware capabilities.
- Q: Should decompression buffer size be the same as input processing buffer (256 KiB from concatenation)? → A: Yes, unified buffer size across all features. Default is 256 KiB, but buffer size is configurable. Single configuration point for consistent memory behavior across the system.
- Q: For unrecognized (non-JSON, non-logfmt, uncompressed) files in sorting/following modes, should they error or skip gracefully? → A: Unrecognized formats are discarded in sorting and following modes. In concatenation mode, they pass through transparently. This preserves the concatenation feature's pass-through philosophy while preventing errors in structured-data modes.

## Overview
</parameter>

The compressed input support feature enables `hl` to transparently handle log files in compressed formats. When an input file is detected to be compressed, it is automatically decompressed before processing, allowing users to work with archived logs without manual decompression steps.

This feature works with any input source that supports compression detection and is transparent to all downstream features (filtering, formatting, sorting, etc.).

## User Stories & Acceptance Criteria

### US-1: Auto-Detection of Gzip Compression
**As a** system administrator
**I want to** process gzipped log files without explicitly decompressing them
**So that** I can analyze archived logs directly

**Acceptance Criteria:**
- Given a gzip-compressed log file (e.g., `app.log.gz`)
- When I run `hl app.log.gz`
- Then the file is automatically decompressed and its contents are processed
- And the user sees no difference in output compared to uncompressed input
- And no temporary files are created during decompression

### US-2: Auto-Detection of Bzip2 Compression
**As a** system administrator
**I want to** process bzip2-compressed log files without explicitly decompressing them
**So that** I can work with older archived logs

**Acceptance Criteria:**
- Given a bzip2-compressed log file (e.g., `app.log.bz2`)
- When I run `hl app.log.bz2`
- Then the file is automatically decompressed and its contents are processed
- And the user sees no difference in output compared to uncompressed input

### US-3: Auto-Detection of Xz Compression
**As a** system administrator
**I want to** process xz-compressed log files without explicitly decompressing them
**So that** I can analyze highly compressed archives

**Acceptance Criteria:**
- Given an xz-compressed log file (e.g., `app.log.xz`)
- When I run `hl app.log.xz`
- Then the file is automatically decompressed and its contents are processed
- And the user sees no difference in output compared to uncompressed input

### US-4: Auto-Detection of Zstandard Compression
**As a** system administrator
**I want to** process zstandard-compressed log files without explicitly decompressing them
**So that** I can work with modern high-speed compressed archives

**Acceptance Criteria:**
- Given a zstandard-compressed log file (e.g., `app.log.zst`)
- When I run `hl app.log.zst`
- Then the file is automatically decompressed and its contents are processed
- And the user sees no difference in output compared to uncompressed input

### US-5: Automatic Format Detection
**As a** a user
**I want to** not worry about file extensions when working with compressed logs
**So that** compression format is detected based on content, not filename

**Acceptance Criteria:**
- Given compressed log files with unusual or missing extensions (e.g., `log.backup`, `archive.1`)
- When I run `hl log.backup archive.1` where files are actually gzip or bzip2 compressed
- Then the compression format is correctly detected from file content
- And the files are decompressed correctly regardless of extension

### US-6: Mixed Compressed and Uncompressed Input
**As a** system administrator
**I want to** process a mix of compressed and uncompressed log files in a single command
**So that** I don't need separate commands for different file types

**Acceptance Criteria:**
- Given a command with multiple files: some compressed (`.gz`, `.bz2`, `.xz`, `.zst`) and some uncompressed
- When I run `hl app.log app.log.gz archive.log.bz2 recent.log.zst`
- Then all files are processed seamlessly in order, with each decompressed if needed
- And the output is indistinguishable from running separate commands on each file

### US-7: Preserved Decompression Order
**As a** a user analyzing mixed archives
**I want to** have the decompression maintain the order of files specified
**So that** output order matches my command line argument order

**Acceptance Criteria:**
- Given multiple compressed files in a specific order
- When I run `hl first.log.gz second.log.gz third.log.gz`
- Then the output contains content in the order: first, then second, then third
- And no re-ordering or parallel decompression changes this sequence

### US-8: Transparent Handling of Unrecognized Formats
**As a** a user with atypical log files  
**I want to** have unknown formats passed through without special handling  
**So that** I can process non-standard log files if other features support them

**Acceptance Criteria:**
- Given a file that is not in a recognized compression format
- When I run `hl unknown.log` (in concatenation mode)
- Then the file is treated as uncompressed and passed through transparently
- And the raw content is included in the output
- And if other features (sorting, following) are enabled, unrecognized formats may be ignored or cause errors depending on those features

### US-9: Streaming Decompression
**As a** a user with very large compressed logs
**I want to** process huge compressed files without loading them entirely into memory
**So that** I don't run out of RAM on large archives

**Acceptance Criteria:**
- Given a multi-gigabyte compressed log file
- When I run `hl large-archive.log.gz`
- Then the file is decompressed in a streaming fashion
- And memory usage stays bounded regardless of the decompressed size
- And processing completes in reasonable time

## Technical Specifications

### Supported Compression Formats

**Automatically Supported:**
- **gzip** (.gz) — Deflate-based compression; widely supported
- **bzip2** (.bz2) — Burrows-Wheeler transform compression; older standard
- **xz** (.xz) — LZMA2-based compression; efficient compression ratio
- **zstandard** (.zst) — Modern real-time compression algorithm; high speed
- **Uncompressed** — Files are read as-is

### Detection Mechanism

**File Content-Based Detection:**
- Compression format is detected from file magic bytes (file header signatures), not file extensions
- This allows processing of files with non-standard or missing extensions
- Unknown formats are treated as uncompressed and passed through as-is

**Process:**
1. Open file and read initial bytes
2. Match against known compression format signatures
3. If match found, apply appropriate decompression
4. If no match, process as uncompressed
5. If decompression fails, report error

### Processing Pipeline

**Decompression is transparent:** The file is decompressed on-the-fly as it is read, not loaded entirely into memory first.

**Data Path:**
```
Compressed File → Content Detection → Decompression (if needed) → Segment Parsing → Downstream Features
```

**Applicable to:**
- File inputs (including stdin piped from `cat` or similar)
- Mixed input sources (some compressed, some not)
- All downstream features (concatenation, filtering, formatting, sorting, following)

### Error Handling

**Decompression Failures:**
- If a file is detected as compressed but decompression fails, an error is reported immediately
- Corrupted compressed files result in an IO error with descriptive message
- Processing halts with exit code 1; no partial output is produced
- All files are considered together; one failed decompression stops the entire operation

**Buffer Management:**
- Decompression uses a unified buffer shared with input processing (default: 256 KiB)
- Buffer size is configurable; users can adjust via configuration file or environment variable
- Larger buffers improve throughput for highly compressed data; smaller buffers reduce memory overhead
- Default 256 KiB provides reasonable balance for typical log processing workloads

**Unknown/Invalid Formats:**
- Files that are not recognized as any supported compression format are treated as uncompressed
- In concatenation mode, they are passed through transparently to the output
- In sorting and following modes, unrecognized/unstructured formats are discarded from processing
- This allows mixed input sets where some files may not be structured logs; only processable files contribute to output

## Configuration & CLI

**No explicit compression options are required.** Compression handling is automatic.

**Processing Order:**
Compression detection and decompression occurs before any other processing. All downstream features (filtering, formatting, input format detection, prefix handling, etc.) operate on decompressed content.

## Testing Requirements

### Unit Tests
- Gzip file: correctly identified and decompressed
- Bzip2 file: correctly identified and decompressed
- Xz file: correctly identified and decompressed
- Zstandard file: correctly identified and decompressed
- Uncompressed file: passed through unchanged
- Mixed compressed/uncompressed: processed in order without error
- File with unusual extension but compressed content: detected and decompressed
- Compressed file with wrong extension: detected and decompressed correctly
- Corrupted compressed file: produces appropriate error
- Empty compressed file: handled gracefully

### Integration Tests
- CLI: `hl file.log.gz` decompresses and processes correctly
- CLI: `hl *.log *.log.gz *.log.bz2 *.log.xz *.log.zst` processes all types
- Order: `hl first.gz second.log third.bz2` maintains specified order
- Large file: Multi-gigabyte compressed file processes without memory issues
- Piped input: `gzip -dc archive.log.gz | hl` works correctly
- Mix of stdin and files: `hl file1.log - file2.log.gz` with stdin as compressed data

### Edge Cases
- Empty compressed file
- Partially corrupted compressed file (header OK, body corrupted)
- File with multiple compression layers (if applicable)
- File with compression signature but not actually compressed
- Compressed file with binary content (not structured logs)
- Very small compressed file (< 10 bytes)

## Interactions with Other Features

This feature provides transparent decompression capability consumed by:

- **Concatenation** — Compressed files can be mixed with uncompressed files in multi-file concatenation
- **Filtering** — Filters are applied to decompressed content
- **Sorting** — Sorting works on decompressed logs
- **Following** — Can follow changes to compressed files as they grow
- **All Output Features** — Human-readable formatting, themes, field visibility, etc. work on decompressed content

For details on how each feature integrates, see their respective specifications.

## Performance Characteristics

**Streaming Decompression:**
- Decompression happens on-the-fly during reading
- Memory usage is bounded by buffer size and max message size, not file size
**Processing speed depends on decompression algorithm, CPU capability, and buffer size**

**Typical Performance (on modern hardware):**
- gzip: ~100-500 MB/s
- bzip2: ~10-50 MB/s
- xz: ~10-100 MB/s
- zstandard: ~500+ MB/s

**Performance Target:**
- Decompression overhead should not exceed 50% compared to processing equivalent uncompressed content
- This means for a 1 GB compressed file, total processing time should be at most 1.5× the time to process its uncompressed equivalent

**Actual performance varies based on:**
- CPU capabilities and available cores
- Compression level used when file was compressed
- Input file composition and structure

## Future Enhancements (Out of Scope)

- Automatic decompression with external tools (e.g., calling `gunzip` command)
- Nested/layered compression (e.g., gzip-compressed tar file)
- Custom compression format support
- Compression of output files (separate feature)
- Lenient decompression error handling (e.g., skip corrupted compressed files and continue with others)

## Notes

- Compressed file handling is transparent to the user; no special flags or options are required
- Files are always read sequentially in the order specified, regardless of compression
- This feature is independent of other features and provides a foundational capability for input handling
