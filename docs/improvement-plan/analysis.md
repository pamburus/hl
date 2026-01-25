# Documentation Duplication Analysis

This document analyzes the current state of content duplication in the `hl` documentation.

## Overview

The documentation contains significant redundancy where the same information is repeated across multiple files. This analysis identifies the key areas of duplication and their impact.

## Duplication Categories

### 1. Option Descriptions

The same CLI options are described in full detail in multiple locations:

| Option | Locations | Description Copies |
|--------|-----------|-------------------|
| `--expansion` | `features/field-expansion.md`, `features/formatting.md`, `customization/config-files.md`, `customization/environment.md`, `reference/options.md` | 5 |
| `--time-format` | `features/time-display.md`, `features/formatting.md`, `features/time-handling-overview.md`, `customization/config-files.md`, `customization/environment.md`, `reference/options.md`, `reference/time-format.md` | 7 |
| `--hide` | `features/field-visibility.md`, `features/formatting.md`, `examples/field-management.md`, `customization/config-files.md`, `reference/options.md`, `help/faq.md` | 6 |
| `--since/--until` | `features/filtering-time.md`, `features/filtering.md`, `features/time-handling-overview.md`, `examples/time-filtering.md`, `reference/options.md` | 5 |
| `--level` | `features/filtering-level.md`, `features/filtering.md`, `examples/filtering.md`, `reference/options.md` | 4 |
| `--flatten` | `features/field-expansion.md`, `features/field-visibility.md`, `examples/field-management.md`, `customization/environment.md`, `reference/options.md` | 5 |
| `--color` | `features/formatting.md`, `customization/environment.md`, `reference/options.md` | 3 |
| `--paging` | `features/pager.md`, `customization/environment.md`, `reference/options.md` | 3 |

### 2. Mode/Value Enumerations

Expansion modes are a prime example — the same 4 modes are listed with descriptions in:

```
- features/field-expansion.md (Overview + Expansion Modes sections)
- features/formatting.md (Field Expansion section)
- customization/config-files.md (Field Expansion section)
- customization/environment.md (HL_EXPANSION section)
- reference/options.md (-x, --expansion section)
```

Each location has its own wording for the same 4 values:
- `never`
- `inline`
- `auto`
- `always`

### 3. Time Format Specifiers

The time format specifiers (`%Y`, `%m`, `%d`, `%H`, `%M`, `%S`, etc.) are documented in:

- `features/time-display.md` — full table
- `reference/time-format.md` — full table (duplicate)
- `customization/config-files.md` — partial list with examples
- `customization/environment.md` — partial list with examples

### 4. Example Commands

Many example commands are repeated across files:

```sh
# This pattern appears in multiple files:
hl --since "1 hour ago" app.log
hl --level error app.log
hl --hide '*' --hide '!field' app.log
hl -F app.log
```

### 5. Timestamp Format Support

Information about supported timestamp formats appears in:

- `features/timestamps.md` — full documentation
- `features/sorting.md` — partial list
- `features/time-handling-overview.md` — summary

## Impact Assessment

### Maintenance Cost

When a feature changes or documentation needs correction:

| Duplication Level | Files to Update | Risk of Inconsistency |
|-------------------|-----------------|----------------------|
| High (5+ copies) | 5-7 files | Very High |
| Medium (3-4 copies) | 3-4 files | High |
| Low (2 copies) | 2 files | Moderate |

### Current Inconsistencies Found

1. **Timestamp formats in `sorting.md`** — Listed unsupported formats (`Jan 15 10:30:45`, `15/Jan/2024:10:30:45 +0000`) as supported (now fixed)

2. **Field hiding examples** — Some files showed hiding predefined fields (`timestamp`, `level`, `message`) which has no effect (now fixed)

3. **Expansion mode descriptions** — Slight wording variations across files (now standardized)

## Structural Analysis

### Current File Organization

```
docs/src/
├── features/           # 23 files - Detailed feature documentation
│   ├── filtering.md    # Overview with duplicated details
│   ├── filtering-*.md  # Specific filters (4 files)
│   ├── formatting.md   # Overview with duplicated details
│   ├── field-*.md      # Field features (2 files)
│   ├── time-*.md       # Time features (3 files)
│   └── ...
├── customization/      # 7 files - Configuration
│   ├── config-files.md # Repeats ALL option descriptions
│   ├── environment.md  # Repeats ALL option descriptions
│   └── themes*.md      # 5 files
├── reference/          # 4 files - Quick reference
│   ├── options.md      # Repeats ALL option descriptions
│   ├── time-format.md  # Duplicates time-display.md
│   └── ...
├── examples/           # 7 files - Usage examples
│   └── ...             # Each repeats relevant feature descriptions
└── help/               # 2 files
    ├── faq.md          # Repeats common option descriptions
    └── troubleshooting.md
```

### Problematic Patterns

1. **"Overview" files that contain full details**
   - `features/filtering.md` duplicates content from `filtering-*.md`
   - `features/formatting.md` duplicates content from specific feature files

2. **Reference files that explain instead of reference**
   - `reference/options.md` contains full explanations, not just quick lookup
   - `reference/time-format.md` duplicates `features/time-display.md`

3. **Configuration files that document features**
   - `customization/config-files.md` explains what each option does
   - `customization/environment.md` explains what each variable does

4. **Example files that re-explain features**
   - Examples should demonstrate, not teach
   - Feature explanations belong in feature files

## Quantitative Estimates

| Metric | Estimate |
|--------|----------|
| Total documentation files | ~45 |
| Files with significant duplication | ~25 |
| Percentage of duplicated content | 40-60% |
| Unique content percentage | 40-60% |
| Option descriptions duplicated | 15-20 options × 3-7 copies |

## Root Causes

1. **No single source of truth principle** — Each section tries to be self-contained
2. **Copy-paste documentation** — New sections copied from existing ones
3. **Missing cross-reference strategy** — Links used inconsistently
4. **Unclear section responsibilities** — Overlap between features/, customization/, reference/

## Recommendations

See [principles.md](./principles.md) for documentation principles and [action-items.md](./action-items.md) for specific remediation tasks.