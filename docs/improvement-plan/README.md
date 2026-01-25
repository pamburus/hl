# Documentation Improvement Plan

This directory contains the analysis and action plan for improving the `hl` documentation book.

## Problem Statement

The current documentation suffers from significant content duplication. The same features, options, and examples are described multiple times across different files, leading to:

1. **Maintenance burden** — Updates must be made in multiple places
2. **Inconsistency risk** — Different files may have conflicting information
3. **Reader confusion** — Users encounter the same information repeatedly
4. **Bloated documentation** — The book is larger than necessary

## Documents

- [Analysis](./analysis.md) — Detailed analysis of current duplication issues
- [Principles](./principles.md) — Documentation principles to follow
- [Action Items](./action-items.md) — Specific tasks organized by priority
- [File Restructuring](./file-restructuring.md) — Proposed changes to file organization

## Quick Summary

### Current State

The documentation has **4 main sections** that overlap significantly:

| Section | Purpose | Problem |
|---------|---------|---------|
| `features/` | Detailed feature docs | Contains full option descriptions |
| `customization/` | Config & environment | Repeats all option descriptions |
| `reference/` | CLI reference | Repeats all option descriptions again |
| `examples/` | Usage examples | Repeats feature explanations |

### Target State

Each piece of information should exist in **exactly one place**:

| Section | Should Contain |
|---------|----------------|
| `features/` | **Authoritative** feature documentation with full details |
| `customization/` | How to configure (file format, env vars) with **links** to features |
| `reference/` | Quick lookup tables with **links** to features |
| `examples/` | Practical scenarios with **links** to features |

### Key Metrics

- **Files affected**: ~25 markdown files
- **Estimated duplication**: 40-60% of content
- **Priority features to deduplicate**:
  1. Expansion modes (4+ copies)
  2. Time format options (6+ copies)
  3. Field visibility (5+ copies)
  4. Filtering options (4+ copies)
  5. Paging/color options (4+ copies)

## Next Steps

1. Review this plan
2. Approve the principles
3. Execute action items by priority
4. Validate cross-references work correctly