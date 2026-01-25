# Documentation Improvement Action Items

This document contains specific, actionable tasks to reduce duplication and improve the documentation structure.

## Priority 1: High-Impact Deduplication

These items address the most duplicated content with the highest maintenance burden.

### 1.1 Field Expansion (`--expansion`)

**Current state:** Full descriptions in 5 files
**Target state:** Full description in `features/field-expansion.md` only

- [x] `features/field-expansion.md` — Keep as authoritative source (already updated)
- [x] `features/formatting.md` — Replace full description with brief summary + link
  - Remove the 4-mode descriptions
  - Keep only: "Control how multi-line values are displayed. Options: `never`, `inline`, `auto`, `always`. See [Field Expansion](./field-expansion.md)."
- [x] `customization/config-files.md` — Replace mode descriptions with link
  - Keep the TOML syntax example
  - Replace mode list with: "See [Field Expansion](../features/field-expansion.md) for mode descriptions."
- [x] `customization/environment.md` — Replace mode descriptions with link
  - Keep the `export` examples
  - Replace mode list with link
- [x] `reference/options.md` — Simplify to quick reference format
  - Keep: option name, default, env var, values list
  - Remove: detailed mode descriptions
  - Add: "See [Field Expansion](../features/field-expansion.md) for details."

### 1.2 Time Format (`--time-format`)

**Current state:** Format specifiers documented in 4+ files
**Target state:** Full specifier table in `reference/time-format.md` only

- [x] `reference/time-format.md` — Authoritative source for format specifiers (complete reference)
- [x] `features/time-display.md` — Authoritative source for timezone handling, links to reference
- [x] `customization/config-files.md` — Already brief with link to time-display.md
- [x] `customization/environment.md` — Already has link to time-display.md
- [x] `features/time-handling-overview.md` — Already conceptual overview with appropriate links

### 1.3 Field Visibility (`--hide`)

**Current state:** Hide/reveal patterns explained in 6 files
**Target state:** Full explanation in `features/field-visibility.md` only

- [x] `features/field-visibility.md` — Keep as authoritative source (already updated)
- [x] `features/formatting.md` — Already brief with link to field-visibility.md
- [x] `examples/field-management.md` — Already focuses on practical examples with links
- [x] `customization/config-files.md` — Already has syntax with link
- [x] `reference/options.md` — Fixed broken link (was pointing to non-existent hiding-fields.md)
- [x] `help/faq.md` — Fixed broken link (was pointing to non-existent hiding-fields.md)

### 1.4 Time Filtering (`--since`, `--until`)

**Current state:** Supported formats listed in 5 files
**Target state:** Full documentation in `features/filtering-time.md` only

- [x] `features/filtering-time.md` — Authoritative source with complete format documentation
- [x] `features/filtering.md` — Already structured as overview with links to specific pages
- [x] `features/time-handling-overview.md` — Already conceptual overview with appropriate links
- [x] `examples/time-filtering.md` — Focuses on practical scenarios with links to feature docs
- [x] `reference/options.md` — Already has brief reference format

### 1.5 Timestamp Formats (input parsing)

**Current state:** Supported formats in 3 files with inconsistencies
**Target state:** Full documentation in `features/timestamps.md` only

- [x] `features/timestamps.md` — Authoritative source (comprehensive documentation)
- [x] `features/sorting.md` — Added link to timestamps.md for format details
- [x] `features/time-handling-overview.md` — Already has appropriate links

---

## Priority 2: Structural Improvements

These items address organizational issues and section overlap.

### 2.1 Consolidate Overview Files

**Problem:** `features/filtering.md` and `features/formatting.md` duplicate their child pages.

- [x] `features/filtering.md` — Already structured as true overview with links to specific pages
- [x] `features/formatting.md` — **Significantly deduplicated**: converted to true overview page (reduced from 320 lines to 95 lines), removed all duplicate examples, now links to authoritative feature pages

### 2.2 Refactor Reference Section

**Problem:** `reference/options.md` duplicates feature documentation.

- [x] `reference/options.md` — Simplified expansion section to quick reference format, fixed broken links
- [x] `reference/time-format.md` — Kept as authoritative time format reference (complements time-display.md)
- [x] `features/sorting.md` — **Deduplicated**: removed duplicate filter examples (time, level, query, field visibility), now links to filtering.md
- [x] `help/faq.md` — **Significantly deduplicated**: removed verbose code examples, replaced with brief answers + links to authoritative docs

### 2.3 Refactor Customization Section

**Problem:** `config-files.md` and `environment.md` explain features instead of just configuration.

- [x] `customization/config-files.md` — Simplified expansion section with link to feature docs
- [x] `customization/environment.md` — Simplified expansion section with link to feature docs

### 2.4 Refactor Examples Section

**Problem:** Example files re-explain features instead of demonstrating them.

- [x] `examples/field-management.md` — Already focuses on practical examples with links
- [x] `examples/time-filtering.md` — Already focuses on practical scenarios with links to feature docs
- [x] `examples/basic.md` — Reviewed, well-structured with Next Steps section
- [x] `examples/filtering.md` — **Significantly deduplicated**: removed syntax explanations, now focuses on practical scenarios with links to feature docs
- [x] `examples/queries.md` — Reviewed, well-structured with Next Steps section
- [x] `examples/live-monitoring.md` — Reviewed, well-structured with Next Steps section
- [x] `examples/multiple-logs.md` — Reviewed, well-structured with Next Steps section

---

## Priority 3: Content Quality

These items improve content accuracy and completeness.

### 3.1 Audit for Incorrect Information

- [x] `features/sorting.md` — Fixed unsupported timestamp formats, added link to timestamps.md
- [x] `features/timestamps.md` — Fixed unsupported timestamp formats
- [x] `features/timestamps.md` — Fixed incorrect sort mode behavior (said entries placed at beginning, actually discarded)
- [x] `features/filtering.md` — Fixed misleading "case-sensitive by default" (removed "by default")
- [x] Multiple files — Fixed field hiding examples for predefined fields
- [x] `reference/options.md` — Fixed broken link (hiding-fields.md → field-visibility.md)
- [x] `help/faq.md` — Fixed broken link (hiding-fields.md → field-visibility.md)
- [x] `help/faq.md` — Fixed broken link (getting-started/supported-formats.md → features/input-formats.md)
- [x] `help/faq.md` — Fixed broken link (examples/advanced.md → examples/queries.md)
- [x] `reference/options.md` — Fixed broken link (prefix-handling.md → prefixes.md)
- [x] `help/troubleshooting.md` — Fixed broken link (prefix-handling.md → prefixes.md)
- [x] Review remaining files for other potential inaccuracies — Completed audit of all example and reference files
- [ ] Test command examples to verify they work (lower priority)

### 3.2 Standardize Terminology

- [x] Create glossary of standard terms — DONE: Added `reference/glossary.md` with 19 term definitions
- [x] Audit terminology usage — DONE: Confirmed "entry/entries" is the dominant term (411 occurrences); glossary updated to reflect this as preferred term
- [ ] Apply consistent terminology across all files (lower priority)

### 3.3 Standardize Example Output Format

- [ ] Decide on standard time format for examples (lower priority)
- [ ] Update all example outputs to use consistent format (lower priority)

---

## Priority 4: Missing Cross-References

These items add links where content was removed or references are missing.

### 4.1 Add "See Also" Sections

- [x] Most feature files already have "Related" or "Related Topics" sections
- [ ] Audit for missing bidirectional links (lower priority)

### 4.2 Add Section Anchors

- [ ] Add explicit anchors to important sections for deep linking (lower priority)

---

## Checklist Summary by File

### High Priority Files — COMPLETED

| File | Status |
|------|--------|
| `features/formatting.md` | ✅ **Major deduplication**: 320→95 lines, converted to overview with links |
| `features/sorting.md` | ✅ **Deduplicated**: removed duplicate filter examples, added links |
| `examples/filtering.md` | ✅ **Major deduplication**: 274→105 lines, focuses on scenarios with links |
| `help/faq.md` | ✅ **Deduplicated**: removed verbose examples, brief answers with links |
| `customization/config-files.md` | ✅ Simplified expansion section with link |
| `customization/environment.md` | ✅ Simplified expansion section with link |
| `reference/options.md` | ✅ Simplified expansion section, fixed broken links |

### Medium Priority Files — COMPLETED/VERIFIED

| File | Status |
|------|--------|
| `features/time-handling-overview.md` | ✅ Already overview-only with links |
| `reference/time-format.md` | ✅ Kept as authoritative reference |
| `features/field-visibility.md` | ✅ Authoritative source for --hide |
| `features/field-expansion.md` | ✅ Authoritative source for --expansion |
| `features/timestamps.md` | ✅ Authoritative source for input parsing |
| `features/time-display.md` | ✅ Authoritative source for timezone handling |
| `features/filtering-time.md` | ✅ Authoritative source for --since/--until |
| `examples/field-management.md` | ✅ Focuses on practical examples |
| `examples/time-filtering.md` | ✅ Focuses on practical scenarios |

### Low Priority Files — COMPLETED

| File | Status |
|------|--------|
| `help/troubleshooting.md` | ✅ Reviewed, fixed broken link (prefix-handling.md → prefixes.md) |
| `quick-start.md` | ✅ Reviewed, all links valid |
| `intro.md` | ✅ Reviewed, all links valid |

---

## Priority 6: Architecture Implementation

These tasks implement the documentation architecture defined in [architecture.md](../constitution/architecture.md).

### 6.1 Add Configuration Sections to Feature Pages

Each feature page needs a "Configuration" section showing all three configuration methods (config file, CLI, environment variable), placed immediately after the Overview/introduction.

| Feature Page | Has Config Section | Status |
|--------------|-------------------|--------|
| `features/field-expansion.md` | ✅ | Complete with links to all 3 refs |
| `features/field-visibility.md` | ✅ | Complete with links to all 3 refs |
| `features/time-display.md` | ✅ | Complete with links to all 3 refs |
| `features/filtering-level.md` | ✅ | Complete with links to CLI + env |
| `features/filtering-time.md` | ✅ | Complete with links to CLI |
| `features/filtering-fields.md` | ✅ | Complete with links to CLI |
| `features/filtering-queries.md` | ✅ | Complete with links to CLI |
| `features/sorting.md` | ✅ | Complete with links to CLI |
| `features/follow-mode.md` | ✅ | Complete with links to CLI |
| `features/pager.md` | ✅ | Complete with links to CLI + env |
| `features/raw-output.md` | ✅ | Complete with links to CLI |
| `features/input-formats.md` | ✅ | Complete with links to CLI + env |
| `features/prefixes.md` | ✅ | Complete with links to all 3 refs, moved to top |
| `features/multiple-files.md` | ✅ | Complete with links to CLI + config |
| `features/timestamps.md` | ✅ | Complete with links to all 3 refs, moved to top |
| `customization/themes.md` | ✅ | Complete with links to all 3 refs |

**Note:** Overview pages (`filtering.md`, `input.md`, `time-handling-overview.md`, `viewing-logs.md`, `streaming.md`) and format description pages (`compressed.md`) don't need Configuration sections as they link to specific feature pages.

**Configuration section template:**

```markdown
## Configuration

| Method | Setting |
|--------|---------|
| Config file | `[section]` → `option = "value"` |
| CLI option | `-x, --option <VALUE>` |
| Environment | `HL_OPTION` |

**Values:** `value1`, `value2` (default), `value3`
```

### 6.2 Add Fragment Anchors to Config File Reference

Each configuration option in `customization/config-files.md` needs a fragment-linkable anchor.

**Format:**
```markdown
### formatting.expansion.mode {#formatting-expansion-mode}
```

**Status:** ✅ Complete — All options have fragment anchors (e.g., `#theme`, `#time-format`, `#formatting-expansion-mode`, `#allow-prefix`, `#unix-timestamp-unit`)

### 6.3 Add Cross-References to Config File Reference

Each option entry in `config-files.md` should include:
- CLI option equivalent
- Environment variable equivalent
- Link to feature page

**Status:** ✅ Complete — All options link back to feature pages

### 6.4 Add Cross-References to Environment Variable Reference

Each variable entry in `environment.md` should include:
- Config file option equivalent
- CLI option equivalent
- Link to feature page
- Fragment anchor with hyphens (e.g., `{#hl-expansion}`)

**Status:** ✅ Complete — All variables have fragment anchors and cross-references

### 6.5 Add Cross-References to CLI Options Reference

Each option entry in `reference/options.md` should include:
- Config file option equivalent (linked)
- Environment variable equivalent
- Link to feature page
- Clean fragment anchor (e.g., `{#theme}`, `{#time-format}`, `{#expansion}`)

**Status:** ✅ Complete — All 44 CLI options have clean fragment anchors and cross-references

---

## Priority 5: File Audit Checklist

Each documentation file must be audited against the principles. Use this checklist for systematic review.

### Audit Checklist Template

For each file, verify:

1. **Single Source of Truth Compliance**
   - [ ] No full feature explanations (should be in `features/` only)
   - [ ] No complete option value lists with descriptions (brief list + link instead)
   - [ ] No feature-specific examples that duplicate authoritative docs

2. **Appropriate Content for Section**
   - [ ] `features/`: Contains complete feature documentation
   - [ ] `customization/`: Contains only configuration syntax + brief description + link
   - [ ] `reference/`: Contains only quick lookup tables + links
   - [ ] `examples/`: Contains only practical scenarios with links to features
   - [ ] `help/`: Contains only brief answers with links

3. **Cross-References**
   - [ ] All feature mentions link to authoritative feature page
   - [ ] All links are valid (no broken links)
   - [ ] Related Topics section present where appropriate

4. **No Internal Duplication**
   - [ ] No repeated examples within the same file
   - [ ] No repeated configuration snippets

### Files Requiring Audit

| File | Audited | Issues Found | Fixed |
|------|---------|--------------|-------|
| **Customization** | | | |
| `customization/config-files.md` | ✅ | See below | ✅ |
| `customization/environment.md` | ✅ | Minor - added links | ✅ |
| `customization/themes.md` | ✅ | See below | ✅ |
| `customization/themes-custom.md` | ✅ | None | ✅ |
| `customization/themes-overlays.md` | ✅ | None | ✅ |
| `customization/themes-selecting.md` | ✅ | None | ✅ |
| `customization/themes-stock.md` | ✅ | None | ✅ |
| **Examples** | | | |
| `examples/basic.md` | ✅ | None | ✅ |
| `examples/field-management.md` | ✅ | None | ✅ |
| `examples/filtering.md` | ✅ | None | ✅ |
| `examples/live-monitoring.md` | ✅ | None | ✅ |
| `examples/multiple-logs.md` | ✅ | None | ✅ |
| `examples/queries.md` | ✅ | None | ✅ |
| `examples/time-filtering.md` | ✅ | None | ✅ |
| **Features** | | | |
| `features/compressed.md` | ✅ | None | ✅ |
| `features/field-expansion.md` | ✅ | None | ✅ |
| `features/field-visibility.md` | ✅ | None | ✅ |
| `features/filtering.md` | ✅ | Misleading "by default" | ✅ |
| `features/filtering-fields.md` | ✅ | None | ✅ |
| `features/filtering-level.md` | ✅ | None | ✅ |
| `features/filtering-queries.md` | ✅ | None | ✅ |
| `features/filtering-time.md` | ✅ | None | ✅ |
| `features/follow-mode.md` | ✅ | None | ✅ |
| `features/formatting.md` | ✅ | None | ✅ |
| `features/input.md` | ✅ | None | ✅ |
| `features/input-formats.md` | ✅ | None | ✅ |
| `features/multiple-files.md` | ✅ | None | ✅ |
| `features/pager.md` | ✅ | None | ✅ |
| `features/prefixes.md` | ✅ | Config section moved | ✅ |
| `features/raw-output.md` | ✅ | None | ✅ |
| `features/sorting.md` | ✅ | None | ✅ |
| `features/sorting-chrono.md` | ✅ | Empty placeholder | N/A |
| `features/streaming.md` | ✅ | None | ✅ |
| `features/time-display.md` | ✅ | None | ✅ |
| `features/time-handling-overview.md` | ✅ | None | ✅ |
| `features/timestamps.md` | ✅ | Sort mode error, config moved | ✅ |
| `features/viewing-logs.md` | ✅ | None | ✅ |
| **Help** | | | |
| `help/faq.md` | ✅ | None | ✅ |
| `help/troubleshooting.md` | ✅ | None | ✅ |
| **Reference** | | | |
| `reference/options.md` | ✅ | Broken links | ✅ |
| `reference/performance.md` | ✅ | None | ✅ |
| `reference/query-syntax.md` | ✅ | None | ✅ |
| `reference/time-format.md` | ✅ | None | ✅ |
| **Root** | | | |
| `intro.md` | ✅ | None | ✅ |
| `installation.md` | ✅ | None | ✅ |
| `quick-start.md` | ✅ | None | ✅ |
| `SUMMARY.md` | ✅ | N/A (navigation) | N/A |

### Audit Results: `customization/config-files.md`

**Issues Found:**

1. **Lines 325-338: Field Flattening** — Contains full behavioral explanation with input→output examples
   - Violation: Full feature explanation in customization file
   - Fix: Brief description + link to feature page (or formatting.md)
   - ✅ Fixed: Now links to `formatting.md#object-flattening`

2. **Lines 339-354: Message Format** — Contains full mode descriptions
   - Violation: Complete list of option values with descriptions
   - Fix: Brief description + link
   - ✅ Fixed: Simplified to brief description

3. **Lines 213-230: Input Information Display** — Contains full mode descriptions
   - Violation: Complete list of option values with descriptions
   - Fix: Brief description + link to multiple-files.md
   - ✅ Fixed: Now links to `multiple-files.md`

4. **Lines 230-243: ASCII Mode** — Contains full mode descriptions
   - Violation: Complete list of option values with descriptions
   - Fix: Brief description (no feature page exists, keep minimal)
   - ✅ Fixed: Simplified to single-line description

5. **Lines 245-260: Hiding and Ignoring Fields** — Contains behavioral explanation
   - Violation: Feature explanation in customization file
   - Fix: Brief description + link to field-visibility.md
   - ✅ Fixed: Now links to `field-visibility.md`

6. **Lines 459-461 and 516-517: Duplicate hide example**
   - Violation: Same example repeated within file
   - Fix: Use different field names in second example
   - ✅ Fixed: Second example now uses different field names

7. **Configuration File Format section**
   - Issue: Only mentions TOML format
   - Fix: Add brief note that YAML and JSON are also supported, but TOML is recommended
   - Do NOT elaborate on other formats, just mention they exist

---

### Audit Results: `customization/themes.md`

**Planned Refactoring:**

The Themes section should be restructured to focus on:

1. **How to list available themes** — `hl --list-themes` with filtering options
2. **How to select a theme** — command-line, config file, environment variable
3. **Default theme** — what theme is used by default (`uni`)
4. **Theme overlays** — how to combine built-in themes with overlays
5. **Combining built-in with custom themes** — explain layering
6. **Custom themes** — link to `themes-custom.md` guide (already exists)

**Note:** Default values can be found in the embedded config at `etc/defaults/config.toml`. The default theme is `uni` (a slightly improved version of `universal-blue`).

**Should NOT contain:**
- Descriptions of all built-in themes (remove "Stock Themes" section with individual theme descriptions)
- Recommendations for which theme works best in which context (remove "Theme Use Cases" section)
- Detailed previews of each theme
- "Best Practices" recommendations about theme selection

**Sections to Remove/Simplify:**
- "Stock Themes" (lines ~70-150) — Remove individual theme descriptions entirely
- "Theme Use Cases" (lines ~320-370) — Remove use case recommendations
- "Best Practices" (lines ~375-380) — Remove or simplify
- Terminal compatibility table — Keep but simplify

**Sections to Keep:**
- "Selecting a Theme" — Keep as-is (command line, env var, config file)
- "Listing Available Themes" — Keep as-is
- "Theme Overlays" — Keep as-is
- "Theme Structure" — Keep (useful for understanding)
- "Custom Themes" — Keep brief intro + link to themes-custom.md
- "Related Topics" — Keep links

**Rationale:** A theme catalog with preview screenshots would be valuable but is out of scope for the initial book release. This can be added later as a separate section with dedicated pages per theme.

✅ **Fixed:** Refactored themes.md:
- Added "Default Theme" section documenting `uni` as the default
- Simplified "Selecting a Theme" section
- Simplified "Listing Available Themes" section
- Kept "Theme Overlays" section with brief explanation
- Removed "Stock Themes" section with individual theme descriptions
- Removed "Theme Use Cases" section with recommendations
- Removed "Best Practices" section
- Simplified "Terminal Compatibility" section
- Simplified "Theme Structure Reference" section (kept color specs and modes for custom theme creation)
- Kept "Custom Themes" section with link to detailed guide
- Updated "Related Topics" to remove links to stock themes and selecting themes pages

### Audit Results: `customization/config-files.md` — Configuration File Format

**Issue:** The "Configuration File Format" section only mentions TOML format.

**Fix:** Add brief note that YAML and JSON formats are also supported, with TOML being recommended.

**Example text to add:**
> Configuration files can be written in TOML (recommended), YAML, or JSON format. This documentation uses TOML examples throughout.

**Do NOT:**
- Show YAML/JSON examples
- Elaborate on format differences
- Explain when to use which format

✅ **Fixed:** Added note about YAML/JSON support to config-files.md

### Audit Results: `customization/environment.md`

**Issues Found:**

1. **HL_FLATTEN** — Had full value descriptions without link
   - ✅ Fixed: Simplified to brief list + link to `formatting.md#object-flattening`

2. **HL_LEVEL** — Had value list without link to feature page
   - ✅ Fixed: Added link to `filtering-level.md`

3. **HL_INPUT_FORMAT** — Had full value descriptions without link
   - ✅ Fixed: Simplified to brief list + link to `input-formats.md`

4. **HL_PAGING** — Had full value descriptions without link
   - ✅ Fixed: Simplified to brief list + link to `pager.md`

**Overall Assessment:** The file was already well-structured with brief descriptions per value. Added links to feature pages where appropriate to follow the single-source-of-truth principle.

### Audit Results: `reference/options.md`

**Issues Found:**

1. **Line 31: Broken link** — `../configuration/overview.md` does not exist
   - ✅ Fixed: Changed to `../customization/config-files.md`

2. **Line 290: Broken link** — `../configuration/themes.md` does not exist
   - ✅ Fixed: Changed to `../customization/themes.md`

**Overall Assessment:** The file is well-structured as a quick reference with option names, defaults, environment variables, and links to feature pages. No content violations of the single-source-of-truth principle.

### Audit Results: `help/faq.md`

**Issues Found:** None

**Overall Assessment:** The file is well-structured with brief answers and links to feature pages. Follows the single-source-of-truth principle correctly — provides brief answers with links to authoritative documentation rather than duplicating content. All links verified to be valid.

### Audit Results: `help/troubleshooting.md`

**Issues Found:** None

**Overall Assessment:** The file is comprehensive and well-structured with problem-solution format. All links verified to be valid. Follows the single-source-of-truth principle — provides specific troubleshooting guidance without duplicating feature documentation.

### Audit Results: `examples/basic.md`

**Issues Found:** None

**Overall Assessment:** The file demonstrates common everyday use cases with practical examples. Follows the single-source-of-truth principle — shows how to use features without explaining them in detail. Has "Next Steps" section linking to other example pages.

### Audit Results: `examples/filtering.md`

**Issues Found:** None

**Overall Assessment:** The file demonstrates practical filtering scenarios with links to detailed syntax documentation at the top. Follows the single-source-of-truth principle — focuses on real-world scenarios, not syntax documentation. Links to feature pages for detailed information.

### Audit Results: `examples/queries.md`

**Issues Found:** None

**Overall Assessment:** The file demonstrates advanced query syntax with practical examples. While comprehensive, it appropriately focuses on query patterns and usage rather than duplicating the formal grammar from `reference/query-syntax.md`. Has "Next Steps" section with link to the formal reference.

### Audit Results: `examples/field-management.md`

**Issues Found:** None

**Overall Assessment:** The file demonstrates practical field hiding/revealing patterns with clear examples. Links to feature pages (`field-expansion.md`, `formatting.md`) for detailed explanations. Has "Next Steps" section with relevant links.

### Audit Results: `examples/live-monitoring.md`

**Issues Found:** None

**Overall Assessment:** Comprehensive examples for follow mode and piped streaming. Includes helpful comparison table of `-F` vs `-P` modes. Links to feature pages for detailed documentation. Has "Next Steps" section.

### Audit Results: `examples/multiple-logs.md`

**Issues Found:** None

**Overall Assessment:** Demonstrates multi-file viewing, sorting, and source identification. Links to feature pages for detailed documentation. Well-structured with practical examples.

### Audit Results: `examples/time-filtering.md`

**Issues Found:** None

**Overall Assessment:** Clear distinction at the top between `--since`/`--until` formats and log entry timestamp formats. Links to `timestamps.md` for log entry formats. Comprehensive examples of absolute and relative time formats.

### Audit Results: `reference/performance.md`

**Issues Found:** None

**Overall Assessment:** Provides performance optimization guidance. Correctly states that entries without timestamps are discarded in sort mode. Links to relevant feature documentation.

### Audit Results: `reference/query-syntax.md`

**Issues Found:** None

**Overall Assessment:** Authoritative reference for query syntax. Comprehensive operator tables and examples. Appropriately detailed as a reference document.

### Audit Results: `reference/time-format.md`

**Issues Found:** None

**Overall Assessment:** Authoritative reference for strftime format specifiers. Includes helpful note about format usage for both display and filter parsing.

### Audit Results: `features/filtering.md`

**Issues Found:**

1. **Line 219:** Said "case-sensitive by default" implying there's a way to make it case-insensitive
   - ✅ Fixed: Changed to "case-sensitive" (no "by default")

---

## Reference: Where to Find Default Values

When documenting default values, always verify against the source of truth:

- **Embedded configuration defaults:** `etc/defaults/config.toml`
- **Theme files:** `etc/defaults/themes/`
- **Schema files:** `schema/json/`

Do NOT guess default values — always check the source files.

---

## Verification Steps

After completing the above tasks:

1. [x] Build the book and verify all links work — ✅ `mdbook build` succeeds
2. [x] Search for common duplicated phrases to find remaining issues:
   - "escape newlines" — ✅ only in `field-expansion.md`
   - "keep everything on a single line" — ✅ only in `field-expansion.md`
   - "preserve actual newlines" — ✅ only in `field-expansion.md`
3. [ ] Read through each section type to verify consistent style
4. [ ] Test 10 random command examples to verify accuracy
5. [x] Review word count reduction:
   - `features/formatting.md`: 320→95 lines (~70% reduction)
   - `examples/filtering.md`: 274→105 lines (~62% reduction)
   - `features/sorting.md`: ~260→210 lines (~20% reduction)
   - `help/faq.md`: significant reduction in duplicate examples