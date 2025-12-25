# Feature Specification: Theme Configuration System

**Feature Branch**: `007-theme-config-system`
**Created**: 2024-12-25  
**Status**: Draft  
**Input**: Define the complete theme configuration system including loading, validation, inheritance semantics, and versioning for both v0 (existing) and v1 (new) theme formats.

## Clarifications

### Session 2024-12-25 (First Pass)

- Q: How are themes uniquely identified when users load them? → A: By filename (stem without extension) OR by full filename (with extension). When loading by stem, system tries extensions in priority order: .yaml, .toml, .json (first found wins).

- Q: What is the fallback behavior when no theme is specified or theme loading fails? → A: When no theme specified, use the theme setting from embedded config file (etc/defaults/config.yaml). When theme loading fails (specified theme not found or parse error), application exits with error to stderr - no fallback.

- Q: Where are custom theme files located on each platform? → A: macOS: ~/.config/hl/themes/*.{yaml,toml,json}, Linux: ~/.config/hl/themes/*.{yaml,toml,json}, Windows: %USERPROFILE%\AppData\Roaming\hl\themes\*.{yaml,toml,json}

- Q: Why does the boolean special case exist for boolean → boolean-true/boolean-false inheritance? → A: Backward compatibility + convenience. Initially only `boolean` existed, variants added later. Provides DRY for shared styling. In v0, this pattern is NOT applied to other parent-inner pairs like level/level-inner (v1 may generalize this to more element pairs).

- Q: How are theme loading errors communicated to users? → A: Application exits with error code and message to stderr. Error messages include suggestions for similar theme names when theme is not found.

### Session 2024-12-25 (Second Pass)

- Q: How do parent-inner element pairs like level/level-inner actually work in v0? → A: They use nested styling scope, not explicit inheritance. The inner element is rendered nested inside the parent element. If the inner element is not defined in the theme, the parent's style naturally continues to apply because rendering is still inside the parent's styling scope. This is different from the boolean special case which actively merges properties at load time.

- Q: What is the merge order when both level-specific overrides and parent-inner nesting are involved? → A: Base elements and level-specific overrides are merged first at load time (creating a complete StylePack for each level), then during rendering the parent-inner nesting naturally applies through nested styling scope.

- Q: How are duplicate modes in the modes array handled? → A: In v0, duplicates are allowed and all applied (terminal naturally ignores redundant mode codes). In v1, duplicates are allowed but deduplicated during load or merge with last occurrence winning.

- Q: How do YAML anchors ($palette) work across different file formats? → A: The $palette section is part of the schema and can be defined in all formats (TOML, YAML, JSON), but only YAML can use anchor/alias syntax to reference palette colors. TOML and JSON can include $palette for organization but must reference colors by value.

- Q: What information is displayed when listing themes? → A: Theme names only, grouped by origin (stock/custom). Each origin group shows themes in a compact multi-column layout with bullets. No tags or paths are shown in the listing.

### Session 2024-12-25 (Third Pass)

- Q: What should happen when a theme file exceeds safe size limits? → A: Accept any file size - rely on OS/filesystem limits only

- Q: What is the expected behavior for theme changes during runtime? → A: No runtime reload - theme loaded once at startup, restart required to change themes

- Q: What observability is needed for theme loading operations? → A: Silent on success - only log/output on errors (standard CLI behavior)

- Q: What should happen if filesystem operations fail during theme file reading (e.g., permission denied, I/O error, disk full)? → A: Exit with error to stderr reporting the specific filesystem error (permission denied, I/O error, etc.)

- Q: What happens when a v1 theme references a role that is not defined in the theme? → A: V1 uses embedded `@default` theme with defaults for all styles; undefined roles in user themes fall back to `@default` which defines all reasonable defaults with specific styles resolving to generic ones (primary/secondary)

### Session 2024-12-25 (Fourth Pass)

- Q: How does the `include` directive work in v1? → A: No custom `include` directive in v1; only `@default` theme inheritance. Custom inclusions may be considered later.

- Q: What is the schema for the `styles` section in v1 themes? → A: Object map where keys are role names, values are style objects with optional `style` field for parent/base inheritance: `styles: {warning: {style: "primary", foreground: "#FFA500", modes: [bold]}}`

- Q: What happens when role inheritance chains are deep or circular via the `style` field? → A: Maximum depth of 64 levels

- Q: How are theme name suggestions computed when a theme is not found? → A: Jaro similarity algorithm with minimum relevance threshold of 0.75, sorted by descending relevance

- Q: How should the system handle alternate file extensions like `.yml`? → A: Only accept `.yaml` - users must rename `.yml` files to `.yaml`

### Session 2024-12-25 (Fifth Pass)

- Q: What is the property precedence order when both an element and its referenced role define the same property? → A: Element explicit properties win (override role properties) - explicit is more specific

- Q: What is the order of modes in the result when merging modes arrays from role and element in v1? → A: V1 modes support +mode (add) and -mode (remove) prefixes; plain mode defaults to +mode. Internally represented as two unordered sets (adds/removes). During merge, -mode can turn off parent's mode. Final ANSI output uses only adds in enum declaration order (Bold, Faint, Italic, Underline, SlowBlink, RapidBlink, Reverse, Conceal, CrossedOut); removes only used during merge.

- Q: What should happen when the same mode appears in both +mode and -mode forms within the same modes array in v1? → A: Last occurrence wins - if modes=[+bold, -bold], bold is removed; if [-bold, +bold], bold is added

- Q: What happens in v0 when a mode has a +/- prefix (e.g., modes=[+bold])? → A: Error - v0 does not support +/- prefixes, exit with message suggesting to use v1 or remove prefix

- Q: Do level-specific overrides in v1 work the same way as v0, or can they use v1 features? → A: V1 extends v0 behavior - level-specific elements can use `style` field to reference roles

### Session 2024-12-25 (Sixth Pass)

- Q: Where are stock themes stored and what is the theme search priority? → A: Stock themes embedded in binary; custom directory searched first, then stock (custom wins)

- Q: What should happen with invalid color values like 3-digit hex (#FFF), 8-digit hex with alpha (#RRGGBBAA), out-of-range ANSI (256 or -1), or invalid hex (#GGGGGG)? → A: Exit with specific error for each case: "Invalid hex color #FFF (must be #RRGGBB)", "ANSI color 256 out of range (0-255)", etc.

- Q: Are there restrictions on role names in v1 (length, allowed characters, reserved words, case sensitivity)? → A: Kebab case, predefined list (enum): default, primary, secondary, emphasized, muted, accent, accent-secondary, syntax, status, info, warning, error. The `default` role is the implicit base for all styles that don't specify a base style explicitly via the `style` field.

- Q: What happens when a color palette anchor is referenced but not defined (YAML anchor edge case)? → A: YAML parser handles it - parse error with line number showing undefined anchor reference (treat as parse error)

- Q: What determines the layout and ordering of theme listing output? → A: Terminal-width-aware column count (fit max columns); alphabetically sorted within each group, if output is terminal. Plain list (without grouping or styling) with one theme name per line if output is not terminal.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Theme File Loading and Validation (Priority: P1)

Users can create theme files in TOML, YAML, or JSON format that define visual styles for log output elements. The system loads these themes by name (with automatic format detection) or by full filename, validates their structure against the appropriate schema version, and provides clear error messages for invalid configurations.

**Why this priority**: This is the foundation - without reliable theme loading, nothing else works. This documents the existing v0 behavior that already works in production.

**Independent Test**: Can be fully tested by creating valid and invalid theme files, attempting to load them, and verifying that valid themes load successfully while invalid themes produce specific error messages identifying the problem.

**Acceptance Scenarios**:

1. **Given** a valid v0 theme file (YAML, TOML, or JSON) with element styles defined
   **When** the user loads the theme by stem name (e.g., "my-theme")
   **Then** the system tries extensions in order (.yaml, .toml, .json) and loads the first found theme successfully

2. **Given** a theme file "my-theme.toml" in the custom themes directory
   **When** the user loads the theme by full filename "my-theme.toml"
   **Then** the system loads that specific file without trying other extensions

3. **Given** no theme is explicitly specified by the user
   **When** the application starts
   **Then** the system uses the theme specified by the `theme` setting in the embedded config file

4. **Given** a user specifies a theme name that doesn't exist
   **When** the system attempts to load the theme
   **Then** the application exits with error message to stderr including suggestions for similar theme names

5. **Given** a theme file with invalid syntax (TOML/YAML/JSON)
   **When** the user attempts to load the theme
   **Then** the system exits with parse error to stderr including line number and description

6. **Given** a theme file with an undefined element name
   **When** the user loads the theme
   **Then** the system ignores the unknown element (graceful degradation for forward compatibility)

7. **Given** a theme file with invalid color format
   **When** the user loads the theme
   **Then** the system exits with error to stderr identifying the invalid color value and expected format

---

### User Story 2 - V0 Parent-Inner Element Nested Styling (Priority: P2)

Theme authors can define parent element styles (like `level`, `input-number`, `logger`, `caller`) and corresponding inner elements (`level-inner`, `input-number-inner`, etc.). During rendering, inner elements are nested inside parent elements, so if an inner element is not defined in the theme, the parent's style naturally continues to apply (nested styling scope fallback).

**Why this priority**: This is the core v0 styling pattern that already exists. Understanding nested styling scope is essential before adding v1's explicit property inheritance.

**Independent Test**: Can be tested by creating a theme with a parent element (e.g., `level`) defined but the inner element (`level-inner`) not defined, and verifying that the parent's style continues to apply when the inner element is rendered (because it's nested inside the parent's styling scope).

**Acceptance Scenarios**:

1. **Given** a parent element `level` with foreground=#FF0000 and background=#000000
   **When** the inner element `level-inner` is not defined in the theme
   **Then** `level-inner` displays with the parent's style (foreground=#FF0000, background=#000000) because it's rendered nested inside the parent's styling scope

2. **Given** a parent element `level` with foreground=#FF0000
   **When** an inner element `level-inner` is defined with foreground=#00FF00 and modes=[bold]
   **Then** `level-inner` displays with foreground=#00FF00 (overrides parent) and modes=[bold]

3. **Given** a parent element `logger` with modes=[bold, underline]
   **When** an inner element `logger-inner` defines modes=[italic]
   **Then** `logger-inner` displays with modes=[italic] only (modes completely replace when non-empty in v0)

4. **Given** a parent element `caller` with modes=[bold]
   **When** an inner element `caller-inner` is defined with modes=[] or modes field absent
   **Then** `caller-inner` displays with modes=[bold] through nested styling scope (empty array and absent field both result in no mode override)

5. **Given** a parent element `input-number` with foreground=#00FF00 and background=#000000
   **When** an inner element `input-number-inner` defines only modes=[italic]
   **Then** `input-number-inner` displays with parent's colors (foreground=#00FF00, background=#000000) through nesting, plus modes=[italic]

---

### User Story 3 - Level-Specific Element Overrides (Priority: P3)

Theme authors can override element styles for specific log levels (trace, debug, info, warning, error), allowing different visual treatments for different severity levels while maintaining a consistent base theme.

**Why this priority**: This is a key v0 feature for semantic log coloring. It's lower priority than basic loading/inheritance but essential for practical theme authoring.

**Independent Test**: Can be tested by creating a theme with base element styles and level-specific overrides, then verifying that each level displays with the correct merged styles.

**Acceptance Scenarios**:

1. **Given** a base theme with element `message` having foreground=#FFFFFF
   **When** the `warning` level defines `message` with foreground=#FFA500
   **Then** warning-level messages display with foreground=#FFA500, other levels use #FFFFFF

2. **Given** a base theme with element `level` having foreground=#AAAAAA
   **When** the `error` level defines `level` with foreground=#FF0000, background=#000000, and modes=[bold]
   **Then** error-level logs display with the error-specific styling, other levels use base styling

3. **Given** base elements and `info` level overrides for multiple elements
   **When** an info-level log is displayed
   **Then** all overridden elements use info-specific styles, non-overridden elements use base styles

4. **Given** a level override that defines only `level-inner` without defining `level`
   **When** the level is displayed
   **Then** `level-inner` uses its level-specific override, and nested styling falls back to base `level` (because base elements and level overrides are merged first, then nesting applies during rendering)

---

### User Story 4 - Theme Metadata and Tags (Priority: P4)

Theme authors can add metadata tags to themes (dark, light, 16color, 256color, truecolor) to help users select appropriate themes for their terminal capabilities and preferences.

**Why this priority**: This is a convenience feature for theme discovery and filtering. It's useful but not essential for core functionality.

**Independent Test**: Can be tested by creating themes with various tag combinations and verifying the tags are correctly parsed and available for filtering.

**Acceptance Scenarios**:

1. **Given** a theme file with tags=["dark", "truecolor"]
   **When** the theme is loaded
   **Then** the tag metadata is available and can be queried

2. **Given** multiple themes with various tags
   **When** user lists themes with `--list-themes`
   **Then** themes are displayed grouped by origin (stock/custom), showing only theme names in compact multi-column layout with bullets (tags are not shown in listing)

3. **Given** a theme file with no tags specified
   **When** the theme is loaded
   **Then** the theme loads successfully with empty tag list

---

### User Story 5 - Theme Versioning and V1 Schema (Priority: P5)

Theme authors can specify theme version (e.g., "1.0") to opt into new v1 features while maintaining backward compatibility for v0 themes without version fields. The system validates versions and rejects incompatible themes with clear error messages.

**Why this priority**: This enables future v1 features while preserving v0 compatibility. It's foundational for v1 but can be implemented after v0 is fully documented.

**Independent Test**: Can be tested by creating themes with various version strings, attempting to load them, and verifying version validation and schema routing work correctly.

**Acceptance Scenarios**:

1. **Given** a theme file without a `version` field
   **When** the theme is loaded
   **Then** it is treated as v0 with v0 inheritance semantics

2. **Given** a theme file with version="1.0"
   **When** the theme is loaded
   **Then** it is validated against v1 schema and uses v1 semantics

3. **Given** a theme file with version="2.0"
   **When** the user attempts to load it
   **Then** the system rejects it with error "Unsupported theme version 2.0, maximum supported is 1.x"

4. **Given** a theme file with version="1.03" (invalid - leading zero in minor)
   **When** the user attempts to load it
   **Then** the system rejects it with error "Invalid version format, expected 1.0, 1.1, 1.2, etc."

---

### User Story 6 - V1 Enhanced Inheritance with Roles (Priority: P6)

Theme authors using v1 can define semantic roles (like "warning", "error", "success") that elements can reference via the `style` property, and can use comprehensive parent-inner inheritance that merges properties instead of replacing them entirely.

**Why this priority**: This is the new v1 feature set. It builds on v0 foundations and should be implemented after v0 is solid.

**Independent Test**: Can be tested by creating v1 themes with role definitions and property merging, verifying that inheritance works at the property level rather than element level.

**Acceptance Scenarios**:

1. **Given** a v1 theme with a role `warning` having foreground=#FFA500 and modes=[bold]
   **When** an element defines style="warning"
   **Then** the element displays with the warning role's properties

2. **Given** a v1 theme with parent element `level` having foreground=#FF0000 and background=#000000
   **When** child element `level-inner` defines only modes=[bold]
   **Then** `level-inner` displays with foreground=#FF0000 (inherited), background=#000000 (inherited), and modes=[bold] (explicit) - property-level merging

3. **Given** a v1 theme with parent element `level` having modes=[bold, underline]
   **When** child element `level-inner` defines modes=[italic]
   **Then** in v1, modes are added to parent modes: result has bold, underline, italic (contrast with v0 where modes=[italic] would replace entirely)

4. **Given** a v1 theme with parent element `level` having modes=[bold, underline]
   **When** child element `level-inner` defines modes=[-bold, italic]
   **Then** in v1, the -bold removes parent's bold, result has underline and italic only

5. **Given** a v1 theme that defines only 5 specific elements
   **When** the theme is loaded
   **Then** all non-defined elements inherit from the embedded `@default` theme (property-level merging for v1)

---

### Edge Cases

- What happens when a parent element is defined at the level-specific scope but not in base elements?
- How does the system handle a theme with both base `level` and level-specific `warning.level` when displaying a warning?
- What happens when modes contains duplicate values in v0 (e.g., modes=[bold, italic, bold])? Are they passed to terminal as-is or deduplicated?
- Can inner elements be defined without corresponding parent elements?
- What happens when trying to load a theme with an extension not in the priority list (.yaml, .toml, .json)?
- What happens when multiple theme files exist with the same stem but different extensions (e.g., theme.yaml and theme.toml)?
- What happens when filesystem operations fail (permission denied on theme file, I/O error during read, disk full)?
- What happens when the theme directory doesn't exist or isn't readable?
- What happens when a theme file exists but has restrictive permissions (not readable)?

- What happens when a user specifies a theme with `.yml` extension explicitly (e.g., `my-theme.yml`)? (Answer: file not found error - only `.yaml` extension is supported)


## Requirements *(mandatory)*

### Functional Requirements

#### V0 Theme Loading (Existing Behavior)

- **FR-001**: System MUST load theme files in TOML, YAML, or JSON format from user config directories and embedded resources at startup only (no runtime reloading)

- **FR-001a**: System MUST search for themes in this priority order: custom themes directory first, then stock themes embedded in binary (custom themes with same name override stock themes)

- **FR-002**: System MUST support loading themes by stem name (without extension) with automatic format detection in priority order: .yaml, .toml, .json (first found wins); alternate extension `.yml` is NOT supported

- **FR-003**: System MUST support loading themes by full filename (with extension) to load a specific format

- **FR-004**: System MUST load custom themes from platform-specific directories:
  - macOS: `~/.config/hl/themes/*.{yaml,toml,json}`
  - Linux: `~/.config/hl/themes/*.{yaml,toml,json}`
  - Windows: `%USERPROFILE%\AppData\Roaming\hl\themes\*.{yaml,toml,json}`

- **FR-005**: System MUST use the theme specified in the `theme` setting of the embedded configuration file when no theme is explicitly specified

- **FR-006**: System MUST exit with error to stderr when a specified theme cannot be loaded (no fallback to default)

- **FR-006a**: System MUST compute theme name suggestions using Jaro similarity algorithm with minimum relevance threshold of 0.75, presenting suggestions sorted by descending relevance score

- **FR-007**: System MUST exit with error to stderr when filesystem operations fail during theme loading, reporting the specific error (permission denied, I/O error, disk read failure, etc.)

- **FR-008**: System MUST include suggestions for similar theme names (computed via Jaro similarity ≥0.75) in error messages when theme is not found

- **FR-009**: System MUST be silent on successful theme loading (no output to stdout/stderr) following standard CLI behavior; errors only are reported to stderr

- **FR-010**: System MUST parse theme files with the following top-level sections: `elements`, `levels`, `indicators`, `tags`, `$palette` (optional YAML anchors)

- **FR-011**: System MUST support all v0 element names as defined in schema: input, input-number, input-number-inner, input-name, input-name-inner, time, level, level-inner, logger, logger-inner, caller, caller-inner, message, message-delimiter, field, key, array, object, string, number, boolean, boolean-true, boolean-false, null, ellipsis

- **FR-012**: System MUST support style properties: foreground (color), background (color), modes (array of mode enums in v0, array of mode operations in v1)

- **FR-013**: System MUST support color formats: ANSI basic colors (named), ANSI extended colors (0-255 integers), RGB colors (#RRGGBB hex)

- **FR-014**: System MUST support mode values in v0: bold, faint, italic, underline, slow-blink, rapid-blink, reverse, conceal, crossed-out (plain values only, no +/- prefixes supported)

- **FR-014a**: System MUST reject v0 themes that include mode prefixes (+/-) and exit with error message suggesting to use version="1.0" or remove the prefix

#### V0 Nested Styling Semantics

- **FR-015**: System MUST render inner elements nested inside parent elements for these specific pairs: (input-number, input-number-inner), (input-name, input-name-inner), (level, level-inner), (logger, logger-inner), (caller, caller-inner), so that when an inner element is not defined in the theme, the parent's style naturally continues to apply through nested styling scope

- **FR-016**: System MUST implement v0 style merging with these exact semantics:
  - When merging child into parent: if child has non-empty modes, child modes completely replace parent modes (no merging)
  - When merging child into parent: if child has foreground defined, child foreground replaces parent foreground
  - When merging child into parent: if child has background defined, child background replaces parent background

- **FR-017**: System MUST fall back to parent element when inner element is not defined (e.g., if `level-inner` is absent, use `level` style)

- **FR-018**: System MUST treat empty modes array `[]` identically to absent modes field (both inherit from parent)

#### V0 Level-Specific Overrides

- **FR-019**: System MUST support level-specific element overrides under `levels` section for: trace, debug, info, warning, error

- **FR-020**: System MUST merge level-specific elements with base elements at load time (creating a complete StylePack for each level) such that level overrides win for defined properties

- **FR-021**: System MUST apply nested styling during rendering after level-specific merging is complete, so that parent-inner nesting works with the merged element set for each level

- **FR-021a**: V1 level-specific overrides MUST support all v1 features including the `style` field to reference roles, mode operations (+mode/-mode), and property-level merging semantics

#### V0 Additional Features

- **FR-022**: System MUST support tags array with allowed values: dark, light, 16color, 256color, truecolor

- **FR-023**: System MUST support indicators section with sync.synced and sync.failed configurations

- **FR-024**: System MUST support boolean special case for backward compatibility in v0 only: if base `boolean` element is defined, automatically apply it to `boolean-true` and `boolean-false` at load time before applying their specific overrides (this is active property merging, different from the passive nested styling scope used for other parent-inner pairs; this pattern exists because `boolean` was added first, variants came later; v1 may generalize this pattern)

- **FR-025**: System MUST ignore unknown element names gracefully (forward compatibility)

- **FR-026**: System MUST validate color values and exit with clear error messages to stderr for invalid values, with format-specific error messages: invalid hex length (e.g., "#FFF must be #RRGGBB"), invalid hex characters (e.g., "#GGGGGG contains invalid hex characters"), out-of-range ANSI extended (e.g., "ANSI color 256 out of range (0-255)"), negative ANSI values, etc.

- **FR-027**: System MUST allow duplicate modes in the modes array in v0 (all duplicates passed to terminal which naturally ignores redundant mode codes)

- **FR-028**: System MUST support $palette section in theme schema for all formats (TOML, YAML, JSON), but only YAML can use anchor/alias syntax to reference palette colors; TOML and JSON can define $palette for organization but must reference colors by value

- **FR-029**: System MUST report file format errors (TOML/YAML/JSON syntax errors) to stderr with line numbers and exit; YAML undefined anchor references are treated as parse errors

- **FR-029a**: System MUST rely on YAML parser to detect and report undefined anchor references in $palette section as parse errors with line numbers

- **FR-030**: System MUST provide theme listing grouped by origin (stock/custom) showing theme names only in compact multi-column layout with bullets (no tags or paths in listing output) when output is a terminal; when output is not a terminal (pipe/redirect), output plain list with one theme name per line without grouping or styling

- **FR-030a**: System MUST detect whether output is a terminal and adjust theme listing format accordingly: terminal output uses terminal-width-aware multi-column layout with alphabetical sorting within each group; non-terminal output uses plain list format (one name per line) without grouping

#### V1 Versioning

- **FR-031**: System MUST treat themes without `version` field as v0

- **FR-032**: System MUST support version field with format "major.minor" where major=1 and minor is non-negative integer without leading zeros (e.g., "1.0", "1.5", "1.12")

- **FR-033**: System MUST validate version string against pattern `^1\.(0|[1-9][0-9]*)$` and reject malformed versions

- **FR-034**: System MUST check theme version compatibility against the supported version range and reject themes with unsupported major or minor versions

- **FR-035**: System MUST provide error message format: "Unsupported theme version X.Y, maximum supported is A.B"

#### V1 Enhanced Inheritance (Future)

- **FR-036**: V1 system MUST include an embedded `@default` theme that defines defaults for all styles and roles; this theme is invisible when listing themes (not shown in stock or custom groups)

- **FR-037**: V1 themes MUST support `styles` section as an object map where keys are role names (from predefined enum) and values are style objects containing optional foreground, background, modes, and an optional `style` field that references another role for parent/base inheritance (e.g., `styles: {warning: {style: "primary", foreground: "#FFA500", modes: [bold]}}`)

- **FR-037a**: V1 role names MUST be from the predefined enum (kebab-case): default, primary, secondary, emphasized, muted, accent, accent-secondary, syntax, status, info, warning, error. Undefined role names are rejected with error.

- **FR-037b**: V1 `default` role serves as the implicit base for all roles that do not explicitly specify a `style` field; properties set in `default` (foreground, background, modes) apply to all other roles unless overridden

- **FR-038**: V1 themes MUST support `style` property on elements to reference role names

- **FR-039**: V1 themes MUST resolve element styles in this order: 1) resolve element's `style` field to role (recursively following role-to-role `style` references up to 64 levels), 2) merge with `@default` theme for undefined roles, 3) apply element's explicit properties (foreground, background, modes) which override role properties

- **FR-039a**: V1 property precedence: element explicit properties MUST override role properties when both are defined (e.g., if element has `style: "warning"` and `foreground: "#FF0000"`, and warning role has `foreground: "#FFA500"`, the element uses `#FF0000`)

- **FR-040**: V1 `@default` theme MUST define reasonable defaults for all roles, with the `default` role serving as the implicit base for all other roles that don't specify a `style` field, and specific roles using the `style` field to reference more generic ones (e.g., `warning: {style: "primary", ...}` where `primary` is also defined in `@default`)

- **FR-041**: V1 themes MUST support mode operations with +mode (add) and -mode (remove) prefixes; plain mode defaults to +mode. Modes are internally represented as two unordered sets (adds/removes). During style merging, child -mode removes parent's mode, child +mode adds mode. Final ANSI output includes only added modes in enum declaration order: Bold, Faint, Italic, Underline, SlowBlink, RapidBlink, Reverse, Conceal, CrossedOut. Remove operations are only used during merge process.

- **FR-041a**: V1 mode operations contrast with v0 replacement semantics: v0 child modes completely replace parent modes (no merging), v1 child modes modify parent modes (additive/subtractive operations)

- **FR-041b**: V1 themes MUST resolve conflicting mode operations within the same modes array using last occurrence wins semantics (e.g., modes=[+bold, -bold] results in bold removed; modes=[-bold, +bold] results in bold added)

- **FR-042**: V1 does NOT support custom `include` directive for referencing other themes; only `@default` theme inheritance is supported (custom includes may be considered for future versions)

- **FR-043**: V1 inheritance chain is simple: user theme → `@default` theme (no circular dependency detection needed)

- **FR-044**: V1 role-to-role inheritance via the `style` field MUST support a maximum depth of 64 levels

- **FR-045**: V1 themes MUST detect circular role references (e.g., `warning: {style: "error"}` and `error: {style: "warning"}`) and exit with error message showing the circular dependency chain

- **FR-046**: V1 themes MUST exit with error when a role's `style` field references a role name that is not in the predefined role enum or when the referenced role is not defined in either the user theme or `@default` theme

### Key Entities

- **Theme**: Complete theme configuration containing element styles, level-specific overrides, indicators, version, and metadata tags

- **@default Theme** (v1 only): Embedded theme that provides default definitions for all styles and roles. Not visible in theme listings. All v1 user themes implicitly inherit from `@default` when roles or styles are not explicitly defined. More specific styles in `@default` resolve to generic ones (e.g., `primary`, `secondary`).

- **Theme Version**: Version identifier following "major.minor" format (e.g., "1.0", "1.5") where major=1 and minor has no leading zeros. Used to determine which schema and merge semantics apply.

- **Element**: Named visual element in log output (28 distinct elements in v0) including: input, input-number, input-number-inner, input-name, input-name-inner, time, level, level-inner, logger, logger-inner, caller, caller-inner, message, message-delimiter, field, key, array, object, string, number, boolean, boolean-true, boolean-false, null, ellipsis

- **Style**: Visual appearance specification with optional foreground color, optional background color, and optional text modes list. In v0, modes is a simple array of mode names. In v1, modes is an array of mode operations (+mode to add, -mode to remove, plain mode defaults to +mode), and styles can have an optional `style` field that references a parent/base style for inheritance.

- **Role** (v1 only): Named style defined in the `styles` section that can be referenced by elements or other roles. Role names must be from the predefined enum (kebab-case): default, primary, secondary, emphasized, muted, accent, accent-secondary, syntax, status, info, warning, error. The `default` role is the implicit base for all roles that don't specify a `style` field - properties set in `default` apply to all other roles unless overridden. Roles support inheritance via the optional `style` field (e.g., `warning: {style: "primary", foreground: "#FFA500", modes: [+bold, -italic]}`).

- **Color**: Visual color value in one of three formats:
  - ANSI basic: named colors (default, black, red, green, yellow, blue, magenta, cyan, white, bright-black, bright-red, bright-green, bright-yellow, bright-blue, bright-magenta, bright-cyan, bright-white)
  - ANSI extended: integer value 0-255 (values outside this range are rejected with specific error)
  - RGB: hex format #RRGGBB (exactly 6 hex digits; other formats like #FFF, #RRGGBBAA are rejected with specific error messages)

- **Mode**: Text rendering mode (bold, faint, italic, underline, slow-blink, rapid-blink, reverse, conceal, crossed-out). In v0, modes are plain values in an array. In v1, modes are operations: +mode (add), -mode (remove), or plain mode (defaults to +mode). V1 modes are internally stored as two unordered sets (adds/removes) and final output uses only adds in enum declaration order.

- **Level**: Log severity level (trace, debug, info, warning, error)

- **Tag**: Theme classification metadata (dark, light, 16color, 256color, truecolor)

### Non-Functional Requirements

- **NFR-001**: Theme loading MUST complete in under 50ms for typical themes (50-100 elements)

- **NFR-002**: Theme validation errors MUST include specific location (field name, element name) and expected format

- **NFR-003**: Style merge operations MUST be deterministic and produce identical results across all platforms. In v1, mode output order is deterministic (enum declaration order: Bold, Faint, Italic, Underline, SlowBlink, RapidBlink, Reverse, Conceal, CrossedOut) regardless of input order.

- **NFR-004**: The system MUST base inheritance decisions on semantic property values (whether colors/modes are defined) not on internal representation details

- **NFR-005**: Code implementing inheritance MUST achieve >95% test coverage with unit tests for all edge cases

- **NFR-006**: V0 themes MUST continue to render identically after any refactoring (pixel-perfect regression tests)

- **NFR-007**: Invalid theme files MUST NOT cause application crashes - all errors MUST be handled gracefully with clear error messages

- **NFR-008**: Successful theme loading operations MUST produce no output to stdout or stderr (silent success following standard CLI conventions); only errors are reported

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All existing v0 themes in production render identically after implementation (100% visual regression test pass rate)

- **SC-002**: Theme authors can create a new v0 theme with 20-30 elements in under 30 minutes using only schema and examples

- **SC-003**: Theme loading performance remains under 50ms for themes with up to 100 elements

- **SC-004**: Invalid theme files produce error messages that allow theme authors to fix issues in under 2 minutes (90% success rate in user testing)

- **SC-005**: Unit test suite achieves >95% code coverage for theme loading and inheritance logic

- **SC-006**: All v0 inheritance edge cases documented in this spec have corresponding passing unit tests

- **SC-007**: Version validation correctly accepts all valid v1 version strings (1.0 through 1.999) and rejects all invalid formats (100% accuracy)

- **SC-008**: Theme schema validation via tombi/taplo produces zero false positives for valid v0 themes

- **SC-009**: Documentation includes complete examples of all v0 inheritance patterns with expected visual output

- **SC-010**: All inheritance decisions based solely on semantic property presence (whether foreground/background/modes are defined), verified through code review

## Assumptions

- Theme files are valid TOML, YAML, or JSON format (format parsing errors are handled and reported to stderr)
- When loading by stem name, the first format found in priority order (.yaml, .toml, .json) is used
- When multiple files with same stem but different extensions exist, only the highest priority extension is loaded
- Color values are valid when specified (validation occurs during parsing)
- Users have appropriate terminal capabilities for the colors they choose (no runtime terminal capability detection required)
- Mode values are from the known set (unknown modes are rejected during parsing)
- Theme files are UTF-8 encoded
- Theme file size limits are enforced by OS/filesystem only; no application-level size validation or limits are imposed (parser will handle files of any size that the OS allows to be read)
- In v0, parent-inner pairs use nested styling scope (inner rendered inside parent) for these specific pairs listed in FR-013; if inner element is not defined, parent style continues through nesting (v1 adds explicit property-level inheritance)
- In v0, boolean special case (boolean → boolean-true/boolean-false) uses active property merging at load time, different from the passive nested styling used for other pairs; this exists for backward compatibility because `boolean` was added first and variants came later (v1 may generalize active inheritance to more element pairs)
- Empty modes array `[]` is semantically identical to absent modes field in v0 (both result in no mode override, so parent style continues through nesting or no modes applied)
- In v0, duplicate modes in modes array are allowed and all passed to terminal (terminal ignores redundant codes); v0 modes are plain values only (no +/- prefixes supported); if a mode with +/- prefix is detected in v0 theme, system exits with error suggesting to use v1 or remove prefix
- In v1, modes are operations: +mode (add mode), -mode (remove mode), or plain mode (defaults to +mode). Modes are internally represented as two unordered sets (adds/removes). During merge, child -mode can turn off parent's mode. When the same mode appears in both +mode and -mode forms within the same array, last occurrence wins. Final ANSI output uses only added modes in enum declaration order (Bold, Faint, Italic, Underline, SlowBlink, RapidBlink, Reverse, Conceal, CrossedOut); remove operations are only used during merge.
- The $palette section is part of the schema for all formats, but only YAML can use anchor/alias syntax; TOML and JSON can define $palette for organization but must reference colors by explicit values; YAML undefined anchor references are detected by the YAML parser and reported as parse errors with line numbers
- Color validation provides format-specific error messages: RGB hex colors must be exactly #RRGGBB format (6 hex digits), ANSI extended colors must be integers 0-255 (inclusive), ANSI basic colors must match known color names; invalid formats exit with specific error describing the issue and expected format
- Stock themes are embedded in the application binary; custom themes are searched first, allowing users to override stock themes by creating a custom theme with the same name
- Level-specific overrides are merged with base elements at load time, creating a complete element set for each level; nested styling then applies during rendering (v0 and v1)
- In v1, level-specific element overrides can use the `style` field to reference roles, enabling level-specific elements to inherit from semantic roles (e.g., `levels.error.message: {style: "error-text", modes: [+bold]}`)
- The system has access to all theme files at load time (no lazy loading)
- Theme files are relatively small (< 10KB typical, < 100KB expected maximum in practice, but no hard limits enforced)
- Themes are loaded once at application startup and remain constant for the lifetime of the process; changing themes requires restarting the application
- In v1, all user themes implicitly inherit from the embedded `@default` theme, which provides sensible defaults for all roles and styles; undefined roles/elements in user themes fall back to `@default` definitions
- Theme listing format is terminal-aware: when output is a terminal, use multi-column layout (terminal-width-aware) with alphabetical sorting within groups (stock/custom); when output is not a terminal (pipe/redirect), use plain list format with one theme name per line without grouping or styling
- V1 role names are restricted to a predefined enum (kebab-case): default, primary, secondary, emphasized, muted, accent, accent-secondary, syntax, status, info, warning, error. User themes can only define roles from this list; undefined role names are rejected with error. The `default` role is the implicit base for all roles that don't specify a `style` field - properties set in `default` (foreground, background, modes) apply to all other roles unless explicitly overridden.
- V1 property precedence: element explicit properties override role properties; this allows elements to reference a role for base styling while overriding specific properties
- V1 does NOT support custom `include` directive for theme-to-theme inheritance; only `@default` inheritance is available (custom includes may be added in future versions)
- V1 role-to-role inheritance chains via the `style` field support a maximum depth of 64 levels; deeper chains or circular references cause theme loading to fail with error
- The `@default` theme is not visible in theme listings (it's an internal/system theme)
- Theme name suggestions use Jaro similarity algorithm with minimum relevance threshold of 0.75; suggestions are sorted by descending relevance score
- Only `.yaml` extension is supported for YAML files; alternate `.yml` extension is NOT supported (users must rename `.yml` files to `.yaml`)
- YAML anchors ($palette) are a convenience feature - themes can be written without them
- The embedded configuration file (etc/defaults/config.yaml) specifies the default theme used when no theme is explicitly specified
- Theme loading failures (file not found, parse errors, invalid color values) cause the application to exit with error messages to stderr - no silent fallbacks
- Theme listing shows names only, grouped by origin (stock/custom), in compact multi-column layout; no tags or paths shown in listing
- V1 features (roles, includes, property-level merging) are additive - v0 behavior remains unchanged
- Version checking occurs before any inheritance processing
- The default/embedded themes serve as reference implementations
- Theme authors understand basic color theory and terminal capabilities