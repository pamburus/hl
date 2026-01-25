# Documentation Principles

> **Note:** This document has been superseded by [architecture.md](../constitution/architecture.md) for detailed architectural rules. This file is kept for reference but `docs/constitution/architecture.md` is the authoritative source.

This document establishes the principles for organizing and maintaining the `hl` documentation to avoid duplication and ensure consistency.

## Core Principle: Single Source of Truth

> Every piece of information should exist in **exactly one place** in the documentation.

All other locations should **reference** that single source, not duplicate it.

See [Documentation Architecture](../constitution/architecture.md) for the complete architectural specification.

## Section Responsibilities

### `features/` — Authoritative Feature Documentation

**Purpose:** The definitive source for how each feature works.

**Should contain:**
- Complete explanation of the feature
- All available options and their values
- Detailed behavior descriptions
- Edge cases and limitations
- Primary examples demonstrating the feature

**Should NOT contain:**
- Configuration file syntax (belongs in `customization/`)
- Environment variable names (belongs in `customization/`)
- Quick reference tables (belongs in `reference/`)

### `customization/` — Configuration Methods

**Purpose:** How to configure `hl` through files and environment variables.

**Should contain:**
- Configuration file format and syntax
- Environment variable names and formats
- Where configuration files are located
- Configuration precedence rules
- Brief mention of what each setting controls with **link to feature**

**Should NOT contain:**
- Full explanations of what features do
- Complete lists of option values with descriptions
- Feature-specific examples

**Pattern to follow:**
```markdown
### Expansion Mode

```toml
[formatting.expansion]
mode = "auto"
```

Controls how multi-line field values are displayed. Options: `never`, `inline`, `auto`, `always`.

See [Field Expansion](../features/field-expansion.md) for detailed behavior.
```

### `reference/` — Quick Lookup

**Purpose:** Fast reference for users who already know what they're looking for.

**Should contain:**
- Tables and lists for quick lookup
- Option names, short forms, defaults
- Brief one-line descriptions
- Links to detailed documentation

**Should NOT contain:**
- Detailed explanations
- Multiple examples per option
- Feature tutorials

**Pattern to follow:**
```markdown
### `-x, --expansion [<MODE>]`

Control field expansion mode.

- **Default:** `auto`
- **Values:** `never`, `inline`, `auto`, `always`
- **Environment:** `HL_EXPANSION`

See [Field Expansion](../features/field-expansion.md) for details.
```

### `examples/` — Practical Scenarios

**Purpose:** Show how to combine features for real-world tasks.

**Should contain:**
- Goal-oriented examples ("How to debug a specific request")
- Combinations of multiple features
- Real-world scenarios
- Brief context for why this combination is useful

**Should NOT contain:**
- Feature explanations (link to features instead)
- Complete option documentation
- Single-feature demonstrations (those belong in feature docs)

**Pattern to follow:**
```markdown
## Debugging a Slow Request

Find all log entries for a specific request and show timing details:

```sh
hl -q 'request-id = "abc-123"' \
   --hide '*' --hide '!latency' --hide '!endpoint' \
   --since "1 hour ago" \
   app.log
```

This combines [query filtering](../features/filtering-queries.md), 
[field visibility](../features/field-visibility.md), and 
[time filtering](../features/filtering-time.md).
```

### `help/` — Problem-Solving

**Purpose:** Help users solve specific problems.

**Should contain:**
- FAQ with common questions and brief answers
- Troubleshooting guides for common issues
- Links to relevant feature documentation

**Should NOT contain:**
- Complete feature documentation
- Detailed option explanations

## Cross-Reference Guidelines

### When to Link vs. Repeat

| Situation | Action |
|-----------|--------|
| Explaining a feature | Write once in `features/`, link from elsewhere |
| Listing option values | Full list in `features/`, just names elsewhere with link |
| Showing examples | Primary examples in `features/`, scenario examples in `examples/` |
| Describing behavior | Always in `features/`, brief summary + link elsewhere |

### Link Text Patterns

**Good:**
- "See [Field Expansion](../features/field-expansion.md) for details."
- "For all available modes, see [Expansion Modes](../features/field-expansion.md#expansion-modes)."
- "Learn more about [time filtering](../features/filtering-time.md)."

**Avoid:**
- "Click here for more information."
- "See the documentation."
- Links without context.

## Content Guidelines

### Option Value Lists

**In feature documentation (full):**
```markdown
- `never` — keep everything on a single line, escape newlines as `\n`
- `inline` — preserve actual newlines in multi-line values, surrounded by backticks
- `auto` — expand only fields with multi-line values, keep single-line fields inline
- `always` — display each field on its own indented line
```

**Everywhere else (brief + link):**
```markdown
Options: `never`, `inline`, `auto`, `always`. See [Field Expansion](../features/field-expansion.md).
```

### Examples

**In feature documentation:**
- Show the feature in isolation
- Demonstrate each mode/option
- Include output examples

**In examples section:**
- Show features combined for real tasks
- Focus on the goal, not the feature
- Keep feature explanations minimal (link instead)

## Maintenance Rules

1. **Before adding content:** Check if it already exists. If yes, link to it.

2. **When updating a feature:** Update only the feature file. Other files should just have links.

3. **When fixing an error:** Fix in the authoritative location. If duplicates exist, remove them.

4. **When adding a new feature:** 
   - Create the feature file in `features/`
   - Add configuration syntax to `customization/config-files.md`
   - Add environment variable to `customization/environment.md`
   - Add to reference tables in `reference/options.md`
   - All additions should link back to the feature file

## Anti-Patterns to Avoid

### ❌ Explanatory Comments in Examples

Don't repeat feature explanations in code comments:
```sh
# Hide all custom fields (predefined fields like time, level, message are always shown)
hl -h '*' app.log
```

### ✅ Simple Comments

Use brief, action-focused comments:
```sh
# Hide all fields
hl -h '*' app.log
```

The explanation about predefined fields belongs in the feature documentation, not repeated in every example.

### ❌ Self-Contained Sections

Don't make each section complete on its own:
```markdown
## Environment Variables
### HL_EXPANSION
Control field expansion mode.
- `never` — keep everything on a single line...
- `inline` — preserve actual newlines...
- `auto` — expand only fields with...
- `always` — display each field...
```

### ✅ Reference with Links

Do reference the authoritative source:
```markdown
## Environment Variables
### HL_EXPANSION
Control field expansion mode. Values: `never`, `inline`, `auto`, `always`.
See [Field Expansion](../features/field-expansion.md).
```

### ❌ Copied Examples

Don't copy the same example to multiple files.

### ✅ Unique Examples with Links

Create unique examples that serve each section's purpose, linking to features for explanation.