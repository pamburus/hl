# Documentation Architecture

This document defines the architectural rules for the `hl` documentation book. All documentation changes must conform to these principles.

## Core Principle: Single Source of Truth

Every piece of information exists in **exactly one authoritative location**. All other locations **reference** that source via links.

## Section Responsibilities

### `features/` — Authoritative Feature Documentation

**Purpose:** The definitive source for how each feature works AND how to configure it.

**Each feature page contains:**

1. **What the feature does** — Complete explanation, behavior, edge cases
2. **How to use it** — Primary examples demonstrating the feature
3. **Configuration section** — All methods to configure this feature (placed immediately after the Overview/introduction):

```markdown
## Configuration

| Method | Setting |
|--------|---------|
| Config file | [`formatting.expansion.mode`](../customization/config-files.md#formatting-expansion-mode) |
| CLI option | [`-x, --expansion`](../reference/options.md#expansion) |
| Environment | [`HL_EXPANSION`](../customization/environment.md#hl-expansion) |

**Values:** `never`, `inline`, `auto` (default), `always`
```

Each entry in the Configuration table MUST link to the corresponding reference document:
- **Config file** → `config-files.md#option-name`
- **CLI option** → `reference/options.md#option-anchor`
- **Environment** → `environment.md#variable-name`

This is important because each configuration method may have unique details:
- Different value formats (e.g., TOML arrays vs comma-separated CLI values)
- Method-specific behavior or limitations
- Additional related options only available in that method

4. **Detailed explanation** — How the feature works, modes, behavior, edge cases
5. **Related Topics** — Links to related features and documentation

**Configuration Section Placement:**

The Configuration section MUST appear **immediately after the Overview/introduction** section, before detailed explanations. This ensures:
- Users can quickly find how to configure the feature
- Consistent navigation across all feature pages
- Clear separation between "how to configure" and "how it works"

**Standard feature page structure:**
1. Title (`# Feature Name`)
2. Overview/introduction (brief description)
3. **Configuration** (table with links to config-files.md, options.md, and environment.md anchors)
4. Detailed explanation sections
5. Examples and use cases
6. Tips (if any)
7. Related Topics

**Should NOT contain:**

- Configuration file format/syntax details (belongs in `customization/config-files.md`)
- Full environment variable reference (belongs in `customization/environment.md`)
- Quick lookup tables (belongs in `reference/`)

---

### `customization/config-files.md` — Configuration File Reference

**Purpose:** Document configuration file format, locations, layering, and provide an index of all configuration options.

**Contains:**

- Configuration file format (TOML/YAML/JSON)
- File locations and search order
- Layering and precedence rules
- Configuration schema reference
- **Index of all configuration options** with links to feature pages

**Option Index Format:**

Each configuration option MUST have its own fragment-linkable section with the **option name as the heading**. Options are NOT grouped by category in config-files.md — each option stands alone.

```markdown
### theme {#theme}

The color theme for output formatting.

- **Type:** string
- **Default:** `uni`
- **CLI:** `--theme <NAME>`
- **Env:** `HL_THEME`

See [Themes](./themes.md) for available themes and customization.

---

### theme-overlays {#theme-overlays}

Theme overlays to apply on top of the base theme.

- **Type:** array of strings
- **Default:** `["@accent-italic"]`

See [Theme Overlays](./themes-overlays.md) for details.

---

### time-format {#time-format}

Controls timestamp display format using strftime syntax.

- **Type:** string
- **Default:** `%b %d %T.%3N`
- **CLI:** `-t, --time-format <FORMAT>`
- **Env:** `HL_TIME_FORMAT`

See [Time Display](../features/time-display.md) for format examples.

---

### time-zone {#time-zone}

Timezone for displaying timestamps.

- **Type:** string
- **Default:** `UTC`
- **CLI:** `-Z, --time-zone <TZ>` or `-L, --local`
- **Env:** `HL_TIME_ZONE`

See [Time Display](../features/time-display.md) for timezone configuration.

---

### formatting.expansion.mode {#formatting-expansion-mode}

Controls field expansion behavior.

- **Type:** string
- **Default:** `auto`
- **Values:** `never`, `inline`, `auto`, `always`
- **CLI:** `-x, --expansion`
- **Env:** `HL_EXPANSION`

See [Field Expansion](../features/field-expansion.md) for detailed behavior.
```

**Key requirements:**
- Each option has its own `### option-name {#option-name}` section
- Options are listed alphabetically or in logical groups, but each is a separate section
- Every option links back to the authoritative feature page
- Fragment anchors use the option name (e.g., `#theme`, `#time-format`, `#formatting-expansion-mode`)

**Should NOT contain:**

- Full explanations of what features do
- Detailed examples of feature behavior
- Complete value descriptions (brief list + link instead)

---

### `customization/environment.md` — Environment Variable Reference

**Purpose:** Document all environment variables with links to feature pages.

**Contains:**

- List of all `HL_*` environment variables
- Brief description of each variable
- Cross-references to config file option and CLI option
- Link to feature page for detailed behavior

**Variable Entry Format:**

```markdown
### HL_EXPANSION

Control field expansion mode.

- **Values:** `never`, `inline`, `auto`, `always`
- **Default:** `auto`
- **Config:** `formatting.expansion.mode`
- **CLI:** `-x, --expansion`

See [Field Expansion](../features/field-expansion.md) for details.
```

**Should NOT contain:**

- Full explanations of what features do
- Multiple examples per variable
- Detailed value descriptions

---

### `customization/themes.md` — Theme Configuration

**Purpose:** Document how to select, list, and create themes.

**Contains:**

- How to select a theme (CLI, config, env)
- How to list available themes
- Default theme information
- Theme overlays
- Link to custom theme creation guide

**Should NOT contain:**

- Descriptions of individual built-in themes (future: separate theme catalog)
- Recommendations for which theme to use when

---

### `reference/options.md` — Command-Line Options Reference

**Purpose:** Quick lookup for CLI users.

**Contains:**

- All command-line options with short/long forms
- Default values
- Environment variable equivalents
- Brief one-line descriptions
- Links to feature pages for details

**Option Entry Format:**

```markdown
### `-x, --expansion [<MODE>]`

Control field expansion mode.

- **Default:** `auto`
- **Values:** `never`, `inline`, `auto`, `always`
- **Environment:** `HL_EXPANSION`
- **Config:** `formatting.expansion.mode`

See [Field Expansion](../features/field-expansion.md) for details.
```

**Should NOT contain:**

- Detailed feature explanations
- Multiple examples per option
- Feature tutorials

---

### `reference/` — Other Reference Documents

**Purpose:** Quick lookup tables and formal specifications.

**Contains:**

- `query-syntax.md` — Formal query grammar
- `time-format.md` — Time format specifiers (authoritative)
- `performance.md` — Performance tips and benchmarks

---

### `examples/` — Practical Scenarios

**Purpose:** Show how to combine features for real-world tasks.

**Contains:**

- Goal-oriented examples ("Debug a slow request")
- Combinations of multiple features
- Real-world scenarios
- Brief context for why the combination is useful

**Should NOT contain:**

- Feature explanations (link to features instead)
- Complete option documentation
- Single-feature demonstrations (those belong in feature docs)

**Pattern:**

```markdown
## Debugging a Slow Request

Find all log entries for a specific request and show timing details:

```sh
hl -q 'request-id = "abc-123"' \
   -h '*' -h '!latency' -h '!endpoint' \
   --since "1 hour ago" \
   app.log
```

This combines [query filtering](../features/filtering-queries.md),
[field visibility](../features/field-visibility.md), and
[time filtering](../features/filtering-time.md).
```

---

### `help/` — Problem-Solving

**Purpose:** Help users solve specific problems.

**Contains:**

- `faq.md` — Common questions with brief answers + links
- `troubleshooting.md` — Problem-solution guides

**Should NOT contain:**

- Complete feature documentation
- Detailed option explanations

---

## Cross-Reference Rules

### When to Link vs. Describe

| Content Type | Authoritative Location | Elsewhere |
|--------------|----------------------|-----------|
| Feature behavior | `features/*.md` | Brief summary + link |
| Option values list | `features/*.md` | Value names only + link |
| Configuration syntax | `customization/config-files.md` | Link |
| Time format specifiers | `reference/time-format.md` | Link |
| Query syntax | `reference/query-syntax.md` | Link |

### Link Format

**Good:**
- "See [Field Expansion](../features/field-expansion.md) for details."
- "See [Configuration Files](../customization/config-files.md#formatting-expansion-mode) for syntax."
- Config file link in feature page: [`time-format`](../customization/config-files.md#time-format)

**Bad:**
- "Click here for more information."
- "See the documentation."
- Generic links without fragment anchors to config options

### Fragment URLs

Configuration options in `config-files.md` and `environment.md` must have fragment-linkable anchors using the **option name itself**:

```markdown
### theme {#theme}
### time-format {#time-format}
### time-zone {#time-zone}
### formatting.expansion.mode {#formatting-expansion-mode}
```

This allows deep linking:
- `config-files.md#theme`
- `config-files.md#time-format`
- `config-files.md#formatting-expansion-mode`

**Feature pages MUST link to these anchors** in their Configuration table.

Similarly, `environment.md` and `reference/options.md` should have fragment-linkable anchors:

```markdown
### HL_EXPANSION {#hl-expansion}
### `-x, --expansion [<MODE>]` {#expansion}
```

**CLI option anchors** should use clean, descriptive names that match the config-files.md convention where applicable:
- `#theme` (not `#--theme`)
- `#time-format` (not `#-t---time-format`)
- `#expansion` (not `#-x---expansion`)

**Complete Configuration table example with all cross-references:**

```markdown
| Method | Setting |
|--------|---------|
| Config file | [`theme`](../customization/config-files.md#theme) |
| CLI option | [`--theme`](../reference/options.md#theme) |
| Environment | [`HL_THEME`](../customization/environment.md#hl-theme) |
```

---

## Code Block Formatting

All code blocks in documentation MUST use standard markdown syntax with a **language identifier** after the opening triple backticks.

### Language Identifiers

Use the appropriate language identifier for syntax highlighting:

| Content Type | Language ID |
|--------------|-------------|
| Shell commands | `sh` |
| TOML configuration | `toml` |
| JSON data | `json` |
| YAML configuration | `yaml` |
| Markdown examples | `markdown` |
| Plain text output | `text` |
| Log output examples | `text` |

### Correct Format

```markdown
```sh
hl --level error app.log
```

```toml
[formatting.expansion]
mode = "auto"
```

```json
{"level": "info", "message": "hello"}
```
```

### Incorrect Formats

**Never use:**
- Path-based code blocks (e.g., `` ```path/to/file.sh ``)
- Code blocks without language identifiers (bare `` ``` ``)
- Fake paths like `/dev/null/example.sh`

These formats may be used by AI assistants in conversations but are **not valid** for documentation files.

---

## Content Guidelines

### Option Value Lists

**In feature documentation (authoritative):**
```markdown
- `never` — keep everything on a single line, escape newlines as `\n`
- `inline` — preserve actual newlines in multi-line values
- `auto` — expand only fields with multi-line values (default)
- `always` — display each field on its own indented line
```

**In config/env/options reference (brief):**
```markdown
**Values:** `never`, `inline`, `auto` (default), `always`

See [Field Expansion](../features/field-expansion.md) for descriptions.
```

### Examples

**In feature documentation:**
- Show the feature in isolation
- Demonstrate each mode/option
- Include output examples

**In examples section:**
- Show features combined for real tasks
- Focus on the goal, not the feature
- Link to features for explanation

---

## Default Values Reference

When documenting default values, always verify against the source of truth:

- **Embedded configuration:** `etc/defaults/config.toml`
- **Theme files:** `etc/defaults/themes/`
- **Schema files:** `schema/json/`

**Never guess default values** — always check the source files.

---

## File Naming Conventions

- Use kebab-case: `field-expansion.md`, `config-files.md`
- Feature files: `features/<feature-name>.md`
- Related features can have prefixes: `filtering-level.md`, `filtering-time.md`, `filtering-queries.md`

---

## Maintenance Rules

1. **Before adding content:** Check if it already exists. If yes, link to it.

2. **When updating a feature:** Update only the feature file. Other files should have links.

3. **When fixing an error:** Fix in the authoritative location. Remove duplicates.

4. **When adding a new feature:**
   - Create `features/<feature>.md` with full documentation
   - Place Configuration section immediately after Overview/introduction
   - Configuration table links to specific anchors in ALL THREE reference docs:
     - `config-files.md#option-name`
     - `reference/options.md#option-name` (use clean names matching config-files.md where applicable)
     - `environment.md#hl-variable-name` (use hyphens, not underscores)
   - Add configuration option entry to `customization/config-files.md`:
     - Each option gets its own `### option-name {#option-name}` section
     - Section links back to the feature file
   - Add environment variable entry to `customization/environment.md`:
     - Each variable gets its own `### HL_VARIABLE {#hl-variable}` section (use hyphens in anchor)
     - Section links back to the feature file
   - Add CLI option entry to `reference/options.md`:
     - Each option gets its own section with clean fragment anchor (e.g., `{#time-format}`)
     - Section links back to the feature file
   - All entries link back to the feature file

5. **When renaming/restructuring:**
   - Update all cross-references
   - Verify links with `mdbook build`

---

## Anti-Patterns to Avoid

### ❌ Self-Contained Sections

Don't make each section complete on its own:

```markdown
## HL_EXPANSION

Control field expansion mode.

- `never` — keep everything on a single line, escape newlines as `\n`
- `inline` — preserve actual newlines in multi-line values
- `auto` — expand only fields with multi-line values
- `always` — display each field on its own indented line
```

### ✅ Reference with Links

```markdown
## HL_EXPANSION

Control field expansion mode.

**Values:** `never`, `inline`, `auto` (default), `always`

See [Field Expansion](../features/field-expansion.md) for mode descriptions.
```

### ❌ Duplicate Examples

Don't copy the same example to multiple files.

### ✅ Unique Examples with Context

Create unique examples that serve each section's purpose, linking to features for explanation.

### ❌ Explanatory Comments in Examples

```sh
# Hide all custom fields (predefined fields like time, level, message are always shown)
hl -h '*' app.log
```

### ✅ Brief Comments

```sh
# Hide all fields
hl -h '*' app.log
```

The explanation belongs in the feature documentation, not repeated in examples.

---

## Verification Checklist

Before merging documentation changes:

- [ ] Feature explanations only in `features/`
- [ ] Configuration section appears immediately after Overview in feature pages
- [ ] Configuration table links to all three reference docs:
  - [ ] `config-files.md#option-name`
  - [ ] `reference/options.md#option-name` (clean names)
  - [ ] `environment.md#hl-variable-name`
- [ ] Config/env/CLI references link to feature pages
- [ ] Each config option has its own section with `{#option-name}` anchor in `config-files.md`
- [ ] No duplicate value descriptions across files
- [ ] All links valid (`mdbook build` succeeds)
- [ ] Default values verified against `etc/defaults/config.toml`
- [ ] Bidirectional links: feature → config-files.md AND config-files.md → feature