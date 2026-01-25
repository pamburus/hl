# File Restructuring Proposal

This document proposes changes to the documentation file structure to better support the single-source-of-truth principle.

## Current Structure

```
docs/src/
├── SUMMARY.md
├── intro.md
├── installation.md
├── quick-start.md
│
├── features/                    # 23 files - Too many, some overlap
│   ├── filtering.md            # Overview that duplicates child pages
│   ├── filtering-fields.md
│   ├── filtering-level.md
│   ├── filtering-queries.md
│   ├── filtering-time.md
│   ├── formatting.md           # Overview that duplicates child pages
│   ├── field-expansion.md
│   ├── field-visibility.md
│   ├── time-display.md
│   ├── time-handling-overview.md  # Overlaps with time-display.md
│   ├── timestamps.md           # Overlaps with time-handling-overview.md
│   ├── sorting.md
│   ├── sorting-chrono.md       # Overlaps with sorting.md
│   ├── input.md                # Overview
│   ├── input-formats.md
│   ├── prefixes.md
│   ├── compressed.md
│   ├── multiple-files.md
│   ├── streaming.md
│   ├── follow-mode.md
│   ├── pager.md
│   ├── raw-output.md
│   └── viewing-logs.md
│
├── customization/               # 7 files - Contains feature explanations
│   ├── config-files.md         # Duplicates feature details
│   ├── environment.md          # Duplicates feature details
│   ├── themes.md
│   ├── themes-selecting.md
│   ├── themes-stock.md
│   ├── themes-custom.md
│   └── themes-overlays.md
│
├── examples/                    # 7 files - Re-explains features
│   ├── basic.md
│   ├── filtering.md
│   ├── time-filtering.md
│   ├── field-management.md
│   ├── multiple-logs.md
│   ├── live-monitoring.md
│   └── queries.md
│
├── reference/                   # 4 files - Duplicates features
│   ├── options.md              # Full explanations, not quick reference
│   ├── query-syntax.md
│   ├── time-format.md          # Duplicates time-display.md
│   └── performance.md
│
└── help/                        # 2 files
    ├── faq.md
    └── troubleshooting.md
```

## Issues with Current Structure

### 1. Overlapping Feature Files

Several feature files cover the same or overlapping topics:

| Files | Overlap |
|-------|---------|
| `time-display.md`, `time-handling-overview.md`, `timestamps.md` | Time concepts |
| `sorting.md`, `sorting-chrono.md` | Sorting functionality |
| `filtering.md`, `filtering-*.md` | Filtering (overview + details) |
| `formatting.md`, `field-*.md`, `time-display.md` | Output formatting |
| `input.md`, `input-formats.md`, `prefixes.md` | Input handling |

### 2. Overview Files That Duplicate Details

`filtering.md` and `formatting.md` are meant to be overviews but contain the same detailed content as their child pages.

### 3. Reference Files That Explain

`reference/options.md` and `reference/time-format.md` contain full explanations instead of quick lookup tables.

---

## Proposed Structure

### Option A: Minimal Changes (Recommended First)

Keep the current file structure but change content strategy:

```
docs/src/
├── features/                    # AUTHORITATIVE source for all features
│   └── [keep all files]        # Each file is THE source for its topic
│
├── customization/               # HOW to configure, not WHAT features do
│   ├── config-files.md         # Syntax + links to features
│   └── environment.md          # Variables + links to features
│
├── reference/                   # QUICK LOOKUP only
│   └── options.md              # Table format + links to features
│
└── examples/                    # SCENARIOS, not tutorials
    └── [keep all files]        # Goal-oriented, link to features
```

**Pros:**
- No file reorganization needed
- Can be done incrementally
- Existing links continue to work

**Cons:**
- Some awkward file names remain
- Overview files (`filtering.md`, `formatting.md`) purpose unclear

---

### Option B: Consolidate Time Documentation

Merge overlapping time-related files:

**Before:**
```
features/
├── time-display.md
├── time-handling-overview.md
├── timestamps.md
```

**After:**
```
features/
├── time-display.md              # Output formatting (keep)
└── timestamps.md                # Input parsing + merge overview content
```

- [ ] Merge `time-handling-overview.md` content into `timestamps.md` (input parsing) and `time-display.md` (output formatting)
- [ ] Delete `time-handling-overview.md`
- [ ] Update all links

---

### Option C: Consolidate Sorting Documentation

**Before:**
```
features/
├── sorting.md
└── sorting-chrono.md
```

**After:**
```
features/
└── sorting.md                   # Merge all sorting content
```

- [ ] Merge `sorting-chrono.md` into `sorting.md`
- [ ] Delete `sorting-chrono.md`
- [ ] Update all links

---

### Option D: Convert Overview Files to True Overviews

Change `filtering.md` and `formatting.md` to be navigational overviews:

**filtering.md (new structure):**
```markdown
# Filtering

hl provides multiple ways to filter log entries...

## Filter Types

- **[Level Filtering](./filtering-level.md)** — Filter by severity
- **[Time Filtering](./filtering-time.md)** — Filter by timestamp
- **[Field Filtering](./filtering-fields.md)** — Filter by field values
- **[Query Filtering](./filtering-queries.md)** — Complex filter expressions

## Quick Comparison

| Need | Use |
|------|-----|
| Only errors | `--level error` |
| Last hour | `--since "-1h"` |
| Specific user | `-q 'user.id = 123'` |

## Combining Filters

Filters can be combined... [brief explanation with link to examples]
```

- [ ] Rewrite `filtering.md` as navigational overview
- [ ] Rewrite `formatting.md` as navigational overview
- [ ] Move detailed content to specific feature files if not already there

---

### Option E: Restructure Reference Section

**Before:**
```
reference/
├── options.md          # Full explanations
├── query-syntax.md     # OK as-is
├── time-format.md      # Duplicates features/time-display.md
└── performance.md      # OK as-is
```

**After:**
```
reference/
├── options.md          # Quick lookup table only
├── query-syntax.md     # Keep (unique content)
└── performance.md      # Keep (unique content)
```

- [ ] Convert `options.md` to table format
- [ ] Delete or redirect `time-format.md` to `features/time-display.md`

---

## Migration Strategy

### Phase 1: Content Changes (No File Changes)

1. Update content in existing files per principles.md
2. Add cross-reference links
3. Test all links work

### Phase 2: File Consolidation (Optional)

1. Merge `time-handling-overview.md` if approved
2. Merge `sorting-chrono.md` if approved
3. Update SUMMARY.md
4. Set up redirects for old URLs

### Phase 3: Reference Cleanup

1. Convert `reference/options.md` to table format
2. Evaluate `reference/time-format.md`
3. Update navigation

---

## Files to Delete (After Migration)

These files may be candidates for deletion after content is merged elsewhere:

| File | Merge Into | Reason |
|------|------------|--------|
| `time-handling-overview.md` | Split into `timestamps.md` and `time-display.md` | Overlapping content |
| `sorting-chrono.md` | `sorting.md` | Single topic should be one file |
| `reference/time-format.md` | Keep only `features/time-display.md` | Duplicate content |

---

## SUMMARY.md Updates

After restructuring, update SUMMARY.md to reflect:

1. Clear hierarchy
2. Logical grouping
3. No duplicate entries
4. Descriptive link text

Example improved section:

```markdown
# Features

- [Filtering](./features/filtering.md)
  - [By Level](./features/filtering-level.md)
  - [By Time](./features/filtering-time.md)
  - [By Fields](./features/filtering-fields.md)
  - [By Query](./features/filtering-queries.md)

- [Output](./features/formatting.md)
  - [Field Expansion](./features/field-expansion.md)
  - [Field Visibility](./features/field-visibility.md)
  - [Time Display](./features/time-display.md)
  - [Raw Output](./features/raw-output.md)

- [Input](./features/input.md)
  - [Formats](./features/input-formats.md)
  - [Timestamps](./features/timestamps.md)
  - [Prefixes](./features/prefixes.md)
  - [Compressed Files](./features/compressed.md)

- [Sorting](./features/sorting.md)

- [Live Viewing](./features/viewing-logs.md)
  - [Follow Mode](./features/follow-mode.md)
  - [Pager](./features/pager.md)
  - [Streaming](./features/streaming.md)
```

---

## Decision Checklist

Before implementing:

- [ ] Review and approve Option A (content-only changes)
- [ ] Decide on Option B (time consolidation): Yes / No / Later
- [ ] Decide on Option C (sorting consolidation): Yes / No / Later
- [ ] Decide on Option D (overview files): Yes / No / Later
- [ ] Decide on Option E (reference restructure): Yes / No / Later
- [ ] Agree on migration phases
- [ ] Set up URL redirects if files are moved/deleted