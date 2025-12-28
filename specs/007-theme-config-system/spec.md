# Feature Specification: Theme Configuration System

**Feature Branch**: `007-theme-config-system`
**Created**: 2024-12-25  
**Status**: Draft  
**Input**: Define the complete theme configuration system including loading, validation, inheritance semantics, and versioning for both v0 (existing) and v1 (new) theme formats.

## Clarifications

### Session 2024-12-25 (First Pass)

- Q: How are themes uniquely identified when users load them? → A: By filename (stem without extension) OR by full filename (with extension). When loading by stem, system tries extensions in priority order: .yaml, .toml, .json (first found wins).

- Q: What is the fallback behavior when no theme is specified or theme loading fails? → A: When no theme specified, use the theme setting from embedded config file (etc/defaults/config.yaml). When theme loading fails (specified theme not found or parse error), application exits with error to stderr - no fallback.

- Q: Where are custom theme files located on each platform? → A: macOS: ~/.config/hl/themes/*.{yaml,yml,toml,json}, Linux: ~/.config/hl/themes/*.{yaml,yml,toml,json}, Windows: %USERPROFILE%\AppData\Roaming\hl\themes\*.{yaml,yml,toml,json}

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

- Q: How should the system handle alternate file extensions like `.yml`? → A: Support both `.yml` and `.yaml` extensions - both are valid YAML file extensions and use the same YAML parser

### Session 2024-12-25 (Fifth Pass)

- Q: What is the property precedence order when both an element and its referenced role define the same property? → A: Element explicit properties win (override role properties) - explicit is more specific

- Q: What is the order of modes in the result when merging modes arrays from role and element in v1? → A: V1 modes support +mode (add) and -mode (remove) prefixes; plain mode defaults to +mode. Internally represented as two unordered sets (adds/removes). During merge, -mode can turn off parent's mode. Final ANSI output uses only adds in enum declaration order (Bold, Faint, Italic, Underline, SlowBlink, RapidBlink, Reverse, Conceal, CrossedOut); removes only used during merge.

- Q: What should happen when the same mode appears in both +mode and -mode forms within the same modes array in v1? → A: Last occurrence wins - if modes=[+bold, -bold], bold is removed; if [-bold, +bold], bold is added

- Q: What happens in v0 when a mode has a +/- prefix (e.g., modes=[+bold])? → A: Error - v0 does not support +/- prefixes, exit with message suggesting to use v1 or remove prefix

- Q: Do level-specific overrides in v1 work the same way as v0, or can they use v1 features? → A: V1 extends v0 behavior - level-specific elements can use `style` field to reference roles

### Session 2024-12-25 (Sixth Pass)

- Q: Where are stock themes stored and what is the theme search priority? → A: Stock themes embedded in binary; custom directory searched first, then stock (custom wins)

- Q: What should happen with invalid color values like 3-digit hex (#FFF), 8-digit hex with alpha (#RRGGBBAA), out-of-range ANSI (256 or -1), or invalid hex (#GGGGGG)? → A: Exit with specific error for each case: "Invalid hex color #FFF (must be #RRGGBB)", "ANSI color 256 out of range (0-255)", etc.

- Q: Are there restrictions on role names in v1 (length, allowed characters, reserved words, case sensitivity)? → A: Kebab case, predefined list (enum): default, primary, secondary, strong, muted, accent, accent-secondary, message, syntax, status, level, trace, debug, info, warning, error. The `default` role is the implicit base for all styles that don't specify a base style explicitly via the `style` field.

- Q: What happens when a color palette anchor is referenced but not defined (YAML anchor edge case)? → A: YAML parser handles it - parse error with line number showing undefined anchor reference (treat as parse error)

- Q: What determines the layout and ordering of theme listing output? → A: Terminal-width-aware column count (fit max columns); alphabetically sorted within each group, if output is terminal. Plain list (without grouping or styling) with one theme name per line if output is not terminal.

### Session 2024-12-25 (Seventh Pass)

- Q: Are element names, role names, and ANSI color names case-sensitive or case-insensitive? → A: Case-sensitive for all: element names, role names, color names (e.g., "primary" ≠ "Primary", "black" ≠ "Black")

- Q: What is the behavior for empty/missing sections (elements, styles, levels)? → A: For v1: missing sections treated as empty; empty sections allowed; theme inherits from @default for undefined parts. For v0: all sections are optional; if missing then all elements inside are considered missing; elements with parent/inner relations or boolean special case inherit from parent if missing; others use empty style (default terminal background and foreground, no modes).

- Q: Does the boolean→boolean-true/boolean-false special case still occur in v1, or does v1 use only role-based inheritance? → A: V1 keeps boolean special case for backward compatibility (active merging still happens)

- Q: What is the indicators feature referenced in the indicators section? → A: Out of scope for detailed specification - indicators are a separate feature (--follow mode). Brief description: When --follow option is used, application processes inputs simultaneously, sorting entries chronologically. A sync indicator placeholder at line start shows two states: in sync (default) and out of sync (typically `!` with warning style). Themes provide only styling for these indicator states.

- Q: Does @default theme define all 28 elements and all 16 roles explicitly, or just a subset? → A: @default defines all 28 elements and all 16 roles explicitly with reasonable defaults. Styles with more specific roles usually just inherit styles with more generic roles by default - this provides better flexibility, old themes may still be compatible with newer app versions and look consistently even without defining explicitly styles for new roles.

### Session 2024-12-25 (Eighth Pass)

- Q: Does v1 still use nested styling scope for parent/inner pairs, or does v1 replace it entirely with property-level merging? → A: V1 keeps nested styling scope for parent/inner pairs AND adds property-level merging for roles

- Q: What are the exact version validation rules and maximum supported version? → A: Support only v1.0 initially; reject any other version (v1.1+, v2.0+) until implemented

- Q: Are theme names case-sensitive when loading (e.g., "MyTheme" vs "mytheme")? → A: Platform-dependent - case-sensitive on Linux/macOS, case-insensitive on Windows

- Q: What happens with unknown tags, empty tag arrays, or conflicting tags (e.g., both "dark" and "light")? → A: Validate known tags only, allow empty array, allow conflicting tags (theme author's choice). Note: dark+light are not conflicting - means theme is compatible with both dark and light modes.

- Q: What is the absolute minimum valid theme that can be successfully loaded? → A: Empty file OR minimal version declaration (v1 requires `version: "1.0"`, v0 can be completely empty)

### Session 2024-12-25 (Ninth Pass)

- Q: Are mode names case-sensitive? → A: Case-sensitive - "bold" is valid, "Bold" or "BOLD" are invalid and cause error

- Q: How should unknown top-level sections be handled in theme files? → A: Ignore unknown top-level sections when app knows the version (forward compatible within same version). If app doesn't know the theme version, fail with error. Exception: unknown level names in `levels` section cause error (levels must be from known set: trace, debug, info, warning, error).

- Q: Does the `$palette` section work the same in v1 as in v0? → A: No - $palette is NOT supported in v1; it's only supported in v0 as a YAML anchor/alias organization feature; v1 strict parsing rejects $palette as an unknown top-level section per FR-028a

- Q: Can users create a custom theme file named `@default` or is this name reserved/protected? → A: The embedded `@default` theme is excluded from theme listings (special hidden base theme), but users CAN create custom themes named `@default` which will be loaded and merged with the embedded `@default` following normal custom theme priority rules. Other theme names starting with `@` are not reserved and can be used normally.

- Q: What happens when a file's extension doesn't match its content (e.g., `theme.yaml` contains TOML content)? → A: Parse error from format parser (YAML parser fails on TOML content) - exit with error to stderr

### Session 2024-12-25 (Tenth Pass)

- Q: What is the complete property resolution order for a v1 element when combining roles, @default, element explicit properties, and level-specific overrides? → A: Likely B (@default → level-specific → role → element explicit), but exact resolution order needs further clarification after defining complete list of user stories and use cases. Alternative: level-specific first, then @default, then merge parent roles recursively, then explicit element properties.

- Q: Does the <50ms performance requirement apply to all theme configurations including edge cases like 64-level role chains? → A: Yes - <50ms for all scenarios including 64-level role chains and maximum complexity

- Q: If both `theme.yaml` and `theme.toml` exist when loading by stem, is this silent or does the system provide indication? → A: Silent - loads theme.yaml (highest priority), theme.toml ignored without indication

- Q: When listing themes with `--list-themes`, if both `theme.yaml` and `theme.toml` exist, how are they displayed? → A: Show only "theme" once (represents the loadable theme, .yaml would be loaded)

- Q: What happens when a custom theme has the same name as a stock theme? → A: Complete replacement - custom theme fully replaces stock theme, no merging or inheritance

### Session 2024-12-25 (Eleventh Pass)

- Q: Are level names in the `levels` section case-sensitive? → A: Case-sensitive - "error" is valid, "Error" or "ERROR" are invalid. However, correction: unknown/invalid level names are ignored (not error as previously stated).

- Q: Can boolean-true and boolean-false use the v1 `style` field to reference roles? → A: Yes - boolean-true and boolean-false can use `style` field in v1 like any other element

- Q: How should unknown element properties be handled (e.g., properties other than foreground, background, modes, style)? → A: Ignore unknown properties silently (forward compatibility - newer themes work on older apps). Note: strict validation (D) may be considered later.

- Q: Can a role name be the same as an element name (e.g., both element "primary" and role "primary")? → A: Separate namespaces - element and role names can overlap without conflict

- Q: Is ANSI extended color value 0 valid? → A: Yes - 0 is valid (0-255 means inclusive range, 0 is black in ANSI 256-color palette)

### Session 2024-12-25 (Twelfth Pass)

- Q: What is the exact v1 element property resolution order when combining @default, base element, level-specific override, role reference, and explicit properties? → A: Proposed Order A (merge elements first, then resolve role): 1) Start with @default element, 2) Merge base element from user theme, 3) Merge level-specific element, 4) Resolve `style` field if present (role resolution recursive), 5) Apply explicit properties from merged element (override role properties)

### Session 2024-12-26 (Thirteenth Pass)

- Q: When does the boolean active merge happen relative to level-specific element merging? → A: After level merging - Boolean merge happens on each level's merged StylePack; level overrides to `boolean` DO affect variants at that level

### Session 2025-01-07 (Fourteenth Pass)

- Q: Should v0 themes automatically deduce style roles from element definitions to improve consistency with @default? → A: Yes - before merging with @default, deduce styles.secondary from time, styles.primary from string, styles.strong from message, styles.accent from key, and styles.syntax from array; this makes undefined elements use colors consistent with v0 theme's aesthetic

- Q: How should elements not defined in v0 themes fall back to @default when they reference styles? → A: Fix style resolution to preserve inherited modes from base styles; ReplaceModes flag should only apply to theme-level merging, not within-theme style resolution; when merging style's own properties onto resolved base, use additive mode merging

- Q: Should style deduction override explicitly defined styles in v0 themes? → A: No - deduction only creates a style if: (1) element is defined in v0 theme, AND (2) style is not already defined; explicit definitions take precedence

- Q: When exactly does style deduction happen? → A: After loading v0 theme file but before merging with @default; deduced styles become part of v0 theme's inventory and override @default's corresponding styles during merge

- Q: What does "with empty base" mean in FR-031's style deduction description? → A: The deduced style has no reference to a parent style (no base roles to inherit from); internally this means the base field is empty/default, not referencing any other style role; this is an artificially created style during deduction logic, not present in v0 theme schema

- Q: Should v0 themes ignore the `styles` section if present, or treat it as an error? → A: Ignore silently - v0 schema does NOT include styles section; if present in v0 theme file, ignore entire section per forward compatibility rule (FR-010c for unknown top-level sections)

### Session 2025-12-28 (Fifteenth Pass)

- Q: Can inner elements be defined without corresponding parent elements (e.g., can you define `level-inner` in a theme without defining `level`)? → A: Inner elements are valid on their own; in v1, they fall back to @default theme's parent element for the parent's style; in v0, orphaned inner elements use empty/terminal default for the parent since v0 has no @default theme to fall back to

- Q: What happens when the theme directory doesn't exist or isn't readable? → A: Skip custom themes silently, continue with stock themes only (no error, no directory creation)

- Q: What happens when a theme file exists but has restrictive permissions (not readable)? → A: Exit with permission denied error to stderr (consistent with FR-007 filesystem error handling)

- Q: How does a custom @default theme merge with the embedded @default theme? → A: Custom @default merges like any other custom theme - no special treatment for the name. At theme merge level (FR-001a): custom theme elements completely replace corresponding embedded @default elements (using extend, not property merge). Property-level merging happens later during style resolution. The merge strategy depends on custom theme version: v0 uses element replacement semantics (FR-016), v1 uses property-level merge during resolution (FR-041). The embedded @default is v1, so it has styles and hierarchical inheritance. After merging, the result can have hierarchical styles requiring resolution. Cross-references: FR-001b (custom @default allowed), FR-045 (v1 inheritance chain), FR-041 (v1 resolution order)

- Q: What happens when a parent element is defined at the level-specific scope but not in base elements (e.g., theme defines levels.error.level but not elements.level)? → A: Level-specific element merges with @default base element - follows FR-041 resolution order which starts with @default element, then merges base element from user theme (if present), then merges level-specific element; absence of base element in custom theme doesn't prevent level-specific elements from working

- Q: How does the system handle a theme with both base `level` and level-specific `warning.level` when displaying a warning? → A: Level-specific properties override base properties - the system merges base `level` element with `warning.level` override per FR-020 where level-specific properties win for defined properties; the merged result is used for rendering warning-level logs; undefined properties in level-specific inherit from base

- Q: What happens when modes contains duplicate values in v0 (e.g., modes=[bold, italic, bold])? Are they passed to terminal as-is or deduplicated? → A: Deduplicated during theme loading with last occurrence kept - this ensures consistent behavior and minimal processing; v1 uses the same deduplication strategy (FR-027 allows duplicates but they are deduplicated with last occurrence winning)

- Q: Should the system support .yml extension alongside .yaml for YAML theme files? → A: Yes - support both .yml and .yaml extensions with same priority in the extension search order; when loading by stem name, try extensions in order: .yaml, .yml, .toml, .json (first found wins); both extensions use the YAML parser

- Q: What happens when trying to load a theme with an unsupported extension (not .yaml, .yml, .toml, or .json)? → A: Exit with specific error message to stderr: "Unsupported theme file extension '.ext' - supported extensions are: .yaml, .yml, .toml, .json"

- Q: What happens when multiple theme files exist with the same stem but different extensions (e.g., theme.yaml and theme.toml)? → A: Silent - load highest priority extension (.yaml) without warning or indication that other files were ignored; this keeps behavior simple and predictable; users who want a specific format can use full filename

- Q: Should custom @default theme files created by users appear in theme listings? → A: Yes - show custom @default in custom themes list; while embedded @default is hidden (system default), custom @default is user-created and should be visible so users can see and manage it

- Q: When does color validation occur - during initial theme file parsing, during merge, or during resolution? → A: During initial theme file parsing (fail-fast approach) - if a theme file contains invalid color values, the system exits with error immediately when parsing that file, providing immediate feedback to theme authors per FR-026

- Q: When a custom theme (e.g., v0) merges with the embedded @default theme (v1), what version does the resulting merged theme have? → A: Custom theme's version wins - the merged result uses the version from the custom theme that was explicitly requested by the user; for example, v0 custom theme + v1 @default = v0 result; this ensures the custom theme's semantics (v0 replacement vs v1 property-level merge) are applied correctly

- Q: Should circular reference detection apply to the embedded @default theme, or only to user themes? → A: Circular reference detection occurs after merging user theme with @default - user theme overrides can create circular role references that didn't exist before merge (e.g., user overrides role A to reference role B, while @default has B reference A); detection must happen on the merged result, not on individual themes separately

- Q: In v1, what's the difference between an element having modes=[] (empty array) versus not having a modes field at all? → A: Same as v0 - both empty array and absent field inherit modes from parent/role; in v1 modes is a diff set (add/remove operations), so an empty modes array means "no mode operations specified" which results in inheriting from the base style/role, just like an absent field; modes never replace the whole set in v1, only modify it

- Q: Should theme listing output use case-sensitive or case-insensitive alphabetical sorting? → A: Case-sensitive sorting (ASCII order) - themes are sorted with uppercase letters before lowercase, following standard ASCII ordering; this provides deterministic and predictable ordering

- Q: When an element references a role name via the `style` field, is the role name validated immediately during parsing or later during style resolution? → A: During initial theme file parsing (fail-fast like color validation) - if an element references an invalid role name (not in the predefined role enum), the system exits with error immediately when parsing that file; this provides immediate feedback to theme authors and catches typos early

- Q: What happens if the `default` role is not defined in the theme (neither in user theme nor in @default)? → A: The embedded @default theme MUST define the `default` role - this is a guaranteed invariant tested during development; since @default defines all 16 roles explicitly (per FR-038), the `default` role is always available as the implicit base for other roles

- Q: Are palette color values in the `$palette` section validated the same way as element colors? → A: No - palette colors are not validated; $palette is only supported in v0 themes (YAML anchor/alias organization feature); v1 strict parsing rejects the `$palette` section as an unknown top-level section per FR-010c forward compatibility rules (v1 themes must not include $palette)

- Q: Are file extensions case-sensitive (e.g., does .YAML or .Yaml work on any platform)? → A: Strict - only lowercase extensions are accepted (.yaml, .yml, .toml, .json) on all platforms; variations like .YAML, .Yaml, .YML are not recognized and result in unsupported extension error per FR-002c; this provides consistent behavior across platforms

- Q: In the boolean special case merge, what is the merge direction and how are undefined properties handled? → A: The `boolean` element serves as the base style, and `boolean-true`/`boolean-false` variants merge over it (override/extend); the merge is: boolean (base) → merged with → boolean-true/false (patch); variant properties override corresponding boolean properties; undefined properties in variants inherit from boolean; undefined properties in boolean resolve to base style (if specified via `style` field in v1) or terminal defaults

- Q: What is the complete schema for the indicators section? → A: Complete schema: `indicators.sync.synced` and `indicators.sync.failed`, each with: `text` (string, the indicator character/text), `outer.prefix` (string), `outer.suffix` (string), `outer.style` (style object), `inner.prefix` (string), `inner.suffix` (string), `inner.style` (style object); all fields optional with defaults (empty strings for text/prefix/suffix, empty style for style objects); this supports --follow mode sync state indicators

- Q: What happens if the embedded configuration file is missing, corrupted, or doesn't have a theme setting? → A: Embedded config file cannot be missing or corrupted - if this happens it means the application build is invalid; if embedded config is missing the `theme` field, the application exits with error: "failed to load configuration: missing configuration field 'theme'" (unless theme is explicitly specified in user configuration file which overrides embedded config)

- Q: What error message is shown when role inheritance depth exceeds 64 levels? → A: Specific error with depth information and affected roles: "Role inheritance depth exceeded: maximum 64 levels (chain: role1 → role2 → ... → roleN)" showing the actual depth reached and the inheritance chain that caused the problem; this helps theme authors debug deep or infinite inheritance chains

- Q: Can theme names contain special characters like spaces, dots, unicode, or other non-ASCII characters? → A: Theme names can contain any valid filename characters per platform filesystem rules (no restrictions beyond filesystem limitations like /, \, null); however, the recommendation is to use lowercase kebab-case style (e.g., "my-theme", "dark-blue") for consistency and portability across platforms; dots in names are allowed but may create ambiguity with extensions (e.g., "theme.backup.yaml" has stem "theme.backup")

- Q: What happens if no theme names meet the Jaro similarity threshold of 0.75 for suggestions? → A: Omit suggestions entirely - if no themes meet the 0.75 threshold, the error message shows only "Theme not found" without a suggestions section; showing an empty suggestion list or lowering the threshold doesn't help the user, so suggestions are only included when at least one theme meets the quality threshold

### Session 2025-12-28 (Sixteenth Pass)

- Q: How many v1 roles are actually defined in the implementation? → A: 16 roles total - the implementation defines: default, primary, secondary, strong, muted, accent, accent-secondary, message, syntax, status, level, trace, debug, info, warning, error. The spec previously mentioned only 12 roles, omitting the newer roles: message, level, trace, debug.

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

- What happens when trying to load a theme with an extension not in the supported list (.yaml, .yml, .toml, .json)? (Answer: Exit with specific error message to stderr: "Unsupported theme file extension '.ext' - supported extensions are: .yaml, .yml, .toml, .json")
- What happens when multiple theme files exist with the same stem but different extensions (e.g., theme.yaml and theme.toml)? (Answer: Silent - load highest priority extension without warning per FR-002a; users can specify full filename for specific format)
- Should custom @default theme files created by users appear in theme listings? (Answer: Yes - custom @default shown in custom themes list per FR-030c; embedded @default remains hidden)
- When does color validation occur - during initial theme file parsing, during merge, or during resolution? (Answer: During initial theme file parsing per FR-026a - fail-fast approach provides immediate feedback)
- When a custom theme (e.g., v0) merges with the embedded @default theme (v1), what version does the resulting merged theme have? (Answer: Custom theme's version wins per FR-045a - v0 custom + v1 @default = v0 result)
- Should circular reference detection apply to the embedded @default theme, or only to user themes? (Answer: Detection occurs after merging per FR-047a - user overrides can create loops that didn't exist before merge)
- In v1, what's the difference between an element having modes=[] (empty array) versus not having a modes field at all? (Answer: Same as v0 - both inherit from parent/role per FR-018b; in v1 modes is a diff set, so empty array = no operations = inherit)
- Should theme listing output use case-sensitive or case-insensitive alphabetical sorting? (Answer: Case-sensitive sorting per FR-030d - ASCII order with uppercase before lowercase for deterministic ordering)
- When an element references a role name via the `style` field, is the role name validated immediately during parsing or later during style resolution? (Answer: During initial parsing per FR-040a - fail-fast validation like colors, catches invalid role names immediately)
- What happens if the `default` role is not defined in the theme (neither in user theme nor in @default)? (Answer: Guaranteed invariant - @default theme MUST define default role per FR-038a, tested during development)
- Are palette color values in the `$palette` section validated the same way as element colors? (Answer: No - palette colors not validated; $palette only supported in v0, v1 rejects it per FR-028a)
- Are file extensions case-sensitive (e.g., does .YAML or .Yaml work on any platform)? → A: Strict - only lowercase extensions accepted per FR-002c; .YAML, .Yaml, .YML not recognized)
- In the boolean special case merge, what is the merge direction and how are undefined properties handled? → A: Boolean is base, variants override per FR-024b - boolean → merged with → boolean-true/false; undefined variant properties inherit from boolean)
- What is the complete schema for the indicators section? (Answer: Complete schema per FR-023a - indicators.sync.synced/failed each with text, outer/inner prefix/suffix/style; all fields optional)
- What happens if the embedded configuration file is missing, corrupted, or doesn't have a theme setting? (Answer: Build validation error if missing/corrupted; runtime error if missing theme field per FR-005a - exits with "failed to load configuration: missing configuration field 'theme'")
- What error message is shown when role inheritance depth exceeds 64 levels? (Answer: Specific error per FR-046a: "Role inheritance depth exceeded: maximum 64 levels (chain: role1 → role2 → ... → roleN)" with depth info and affected roles)
- Can theme names contain special characters like spaces, dots, unicode, or other non-ASCII characters? (Answer: Yes per FR-002d - any valid filename characters allowed; recommendation is lowercase kebab-case for portability)
- What happens if no theme names meet the Jaro similarity threshold of 0.75 for suggestions? (Answer: Omit suggestions entirely per FR-006c - error shows only "Theme not found" without suggestions section if none meet threshold)
- Can inner elements be defined without corresponding parent elements? (Answer: Yes - inner elements are valid on their own; in v1 they fall back to @default theme's parent element, in v0 they use empty/terminal default for the parent)
- What happens when the theme directory doesn't exist or isn't readable? (Answer: Skip custom themes silently, continue with stock themes only - no error, no directory creation)
- How does a custom @default theme merge with the embedded @default theme? (Answer: Custom @default merges like any other theme - elements from custom theme completely replace embedded @default elements at theme merge level; property-level merging happens during resolution; merge strategy depends on custom theme version per FR-016 (v0) or FR-041 (v1))
- What happens when a parent element is defined at the level-specific scope but not in base elements? (Answer: Level-specific element merges with @default base element per FR-041 resolution order)
- How does the system handle a theme with both base `level` and level-specific `warning.level` when displaying a warning? (Answer: Level-specific properties override base properties per FR-020 - merged result used for rendering where level-specific wins for defined properties, undefined properties inherit from base)
- What happens when modes contains duplicate values in v0 (e.g., modes=[bold, italic, bold])? (Answer: Deduplicated during theme loading with last occurrence kept per FR-027)
- Should the system support .yml extension alongside .yaml for YAML theme files? (Answer: Yes - both .yml and .yaml supported with same priority in extension search order per updated FR-002)
- What happens when filesystem operations fail (permission denied on theme file, I/O error during read, disk full)? (Answer: Exit with error to stderr per FR-007)
- What happens when a theme file exists but has restrictive permissions (not readable)? (Answer: Exit with permission denied error to stderr - same as other filesystem errors per FR-007)


## Requirements *(mandatory)*

### Functional Requirements

#### V0 Theme Loading (Existing Behavior)

- **FR-001**: System MUST load theme files in TOML, YAML (both .yaml and .yml extensions), or JSON format from user config directories and embedded resources at startup only (no runtime reloading)

- **FR-001a**: System MUST search for themes in this priority order: custom themes directory first, then stock themes embedded in binary (custom themes with same name completely replace stock themes - no merging or inheritance)

- **FR-001b**: System MUST exclude the embedded `@default` from theme listings (it is a special hidden base theme); however, users MAY create custom themes named `@default` which will be loaded and merged with the embedded `@default` theme following normal custom theme priority rules (FR-001a); custom `@default` theme files MUST appear in theme listings under the custom themes group (FR-030c) since they are user-created; the system MUST load custom `@default` themes consistently whether loaded by stem name (`@default`) or full filename (`@default.yaml`), both methods should check for custom theme files first and merge with embedded `@default`; the embedded `@default` theme itself MUST NOT merge with itself (no recursion); other theme names starting with `@` are not reserved and can be used normally

- **FR-002**: System MUST support loading themes by stem name (without extension) with automatic format detection in priority order: .yaml, .yml, .toml, .json (first found wins); both .yaml and .yml extensions are supported for YAML files and use the YAML parser; theme name matching is case-sensitive on Linux/macOS and case-insensitive on Windows (follows platform filesystem conventions)

- **FR-002c**: System MUST only accept lowercase file extensions (.yaml, .yml, .toml, .json) on all platforms; variations with uppercase letters (.YAML, .Yaml, .YML, .TOML, .JSON, etc.) are NOT recognized as valid theme files and result in unsupported extension error per FR-006b; this provides consistent behavior across all platforms regardless of filesystem case sensitivity

- **FR-002d**: System MUST allow theme names (file stems) to contain any characters that are valid in filenames on the platform filesystem, with no additional restrictions beyond platform filesystem rules (e.g., no /, \, null, or other OS-specific forbidden characters); theme names can include spaces, dots, unicode characters, and other non-ASCII characters; however, the RECOMMENDED naming convention is lowercase kebab-case (e.g., "my-theme", "dark-blue", "solarized-dark") for consistency, portability across platforms, and avoiding potential filesystem compatibility issues; dots in theme names are allowed but may create ambiguity (e.g., "theme.backup.yaml" has stem "theme.backup")

- **FR-002a**: System MUST silently load the highest priority format when multiple theme files with the same stem but different extensions exist (e.g., if theme.yaml, theme.yml, and theme.toml all exist, load theme.yaml without warning or indication that others were ignored; if only theme.yml and theme.toml exist, load theme.yml)

- **FR-002b**: System MUST use the file extension to determine which parser to use (YAML parser for .yaml and .yml files, TOML parser for .toml files, JSON parser for .json files); if file content doesn't match the extension, the parser will fail with parse error to stderr

- **FR-003**: System MUST support loading themes by full filename (with extension) to load a specific format

- **FR-004**: System MUST load custom themes from platform-specific directories:
  - macOS: `~/.config/hl/themes/*.{yaml,yml,toml,json}`
  - Linux: `~/.config/hl/themes/*.{yaml,yml,toml,json}`
  - Windows: `%USERPROFILE%\AppData\Roaming\hl\themes\*.{yaml,yml,toml,json}`

- **FR-004a**: System MUST silently skip custom theme directory search if the theme directory does not exist or is not readable (permission denied); the system continues with stock themes only without errors or warnings; the system does NOT automatically create the theme directory

- **FR-005**: System MUST use the theme specified in the `theme` setting of the embedded configuration file when no theme is explicitly specified

- **FR-005a**: The embedded configuration file is part of the application build and MUST be valid (not missing or corrupted); if the embedded config file is missing or corrupted, this indicates an invalid application build; if the embedded config file is present but missing the `theme` field, the system exits with error: "failed to load configuration: missing configuration field 'theme'" unless the theme is explicitly specified in the user configuration file (user config overrides embedded config)

- **FR-006**: System MUST exit with error to stderr when a specified theme cannot be loaded (no fallback to default)

- **FR-006a**: System MUST compute theme name suggestions using Jaro similarity algorithm with minimum relevance threshold of 0.75, presenting suggestions sorted by descending relevance score

- **FR-006b**: System MUST exit with specific error message to stderr when a user explicitly specifies a theme file with an unsupported extension (any extension other than .yaml, .yml, .toml, or .json): "Unsupported theme file extension '.ext' - supported extensions are: .yaml, .yml, .toml, .json"

- **FR-006c**: System MUST omit the suggestions section entirely from the error message if no theme names meet the Jaro similarity threshold of 0.75; when no themes meet the quality threshold, the error message shows only "Theme not found: <theme-name>" without including a suggestions section; suggestions are only included when at least one theme meets the 0.75 threshold, ensuring that only helpful suggestions are shown to users

- **FR-007**: System MUST exit with error to stderr when filesystem operations fail during theme loading, reporting the specific error (permission denied, I/O error, disk read failure, etc.); this applies both when a requested theme file exists but cannot be read due to permissions, and when other I/O errors occur during the read operation

- **FR-008**: System MUST include suggestions for similar theme names (computed via Jaro similarity ≥0.75) in error messages when theme is not found

- **FR-009**: System MUST be silent on successful theme loading (no output to stdout/stderr) following standard CLI behavior; errors only are reported to stderr

- **FR-010**: System MUST parse theme files with the following top-level sections: `elements`, `levels`, `indicators`, `tags`, `$palette` (all sections optional); an empty theme file is valid for v0, v1 requires at minimum `version: "1.0"`

- **FR-010a**: System MUST accept completely empty theme files as valid v0 themes (all sections missing, inherits from terminal defaults and parent/inner relationships)

- **FR-010b**: System MUST accept v1 theme files with only `version: "1.0"` field as valid (all other sections optional, inherits from `@default` theme)

- **FR-010c**: System MUST ignore unknown top-level sections in theme files when the theme version is supported by the application (forward compatibility within same version)

- **FR-010d**: System MUST reject themes with unsupported version numbers (e.g., v1.1+ or v2.0+ when not implemented) before parsing sections; if version is unsupported, exit with error without processing unknown sections

- **FR-010e**: System MUST treat level names in `levels` section as case-sensitive (valid: trace, debug, info, warning, error; invalid: Trace, ERROR, etc.); unknown or invalid level names are ignored (not loaded) rather than causing error

- **FR-010f**: System MUST recognize that v0 theme schema does NOT include a `styles` section (only v1 and later versions support styles); if a v0 theme file contains a `styles` section, the system MUST ignore it silently per the forward compatibility rule for unknown top-level sections (FR-010c); this ensures v0 themes cannot accidentally define styles, and style deduction (FR-031) remains the only mechanism for creating style roles in v0 themes

- **FR-011**: System MUST support all v0 element names as defined in schema (case-sensitive): input, input-number, input-number-inner, input-name, input-name-inner, time, level, level-inner, logger, logger-inner, caller, caller-inner, message, message-delimiter, field, key, array, object, string, number, boolean, boolean-true, boolean-false, null, ellipsis

- **FR-011a**: System MUST treat element names as case-sensitive; "message" and "Message" are different identifiers (unknown element "Message" would be ignored per forward compatibility)

- **FR-011b**: V0 themes MUST allow all sections (`elements`, `levels`, `indicators`, `tags`) to be optional or empty; missing sections are treated as if all elements inside are missing; elements with parent/inner relations inherit from parent if missing; elements with boolean special case inherit from `boolean` if missing; other elements use empty style (default terminal background and foreground, no modes)

- **FR-011c**: System MUST ignore unknown element properties silently (forward compatibility allows newer themes with additional properties to work on older app versions); known properties in v0: foreground, background, modes; known properties in v1: foreground, background, modes, style

- **FR-012**: System MUST support style properties: foreground (color), background (color), modes (array of mode enums in v0, array of mode operations in v1)

- **FR-013**: System MUST support color formats: ANSI basic colors (named, case-sensitive), ANSI extended colors (0-255 integers inclusive), RGB colors (#RRGGBB hex, case-insensitive for A-F)

- **FR-013b**: System MUST accept ANSI extended color value 0 as valid (0 is black in the ANSI 256-color palette); valid range is 0-255 inclusive

- **FR-013a**: System MUST treat ANSI basic color names as case-sensitive; "black" is valid, "Black" or "BLACK" are invalid and cause error

- **FR-014**: System MUST support mode values in v0 (case-sensitive): bold, faint, italic, underline, slow-blink, rapid-blink, reverse, conceal, crossed-out (plain values only, no +/- prefixes supported)

- **FR-014a**: System MUST treat mode names as case-sensitive; "bold" is valid, "Bold" or "BOLD" are invalid and cause error

- **FR-014b**: System MUST reject v0 themes that include mode prefixes (+/-) and exit with error message suggesting to use version="1.0" or remove the prefix

#### V0 Nested Styling Semantics

- **FR-015**: System MUST render inner elements nested inside parent elements for these specific pairs: (input-number, input-number-inner), (input-name, input-name-inner), (level, level-inner), (logger, logger-inner), (caller, caller-inner), so that when an inner element is not defined in the theme, the parent's style naturally continues to apply through nested styling scope; v1 retains this nested styling scope behavior for backward compatibility

- **FR-016**: System MUST implement v0 style merging with these exact semantics:
  - When merging child into parent: if child has non-empty modes, child modes completely replace parent modes (no merging)
  - When merging child into parent: if child has foreground defined, child foreground replaces parent foreground
  - When merging child into parent: if child has background defined, child background replaces parent background

- **FR-017**: System MUST fall back to parent element when inner element is not defined (e.g., if `level-inner` is absent, use `level` style)

- **FR-017a**: System MUST allow inner elements to be defined without corresponding parent elements (orphaned inner elements); in v1, orphaned inner elements fall back to the `@default` theme's parent element for the parent's style (leveraging v1's @default inheritance); in v0, orphaned inner elements use empty/terminal default for the parent since v0 has no @default theme

- **FR-018**: System MUST treat empty modes array `[]` identically to absent modes field (both inherit from parent)

- **FR-018a**: System MUST treat level names in the `levels` section as case-sensitive (valid: trace, debug, info, warning, error); unknown or invalid level names (e.g., "critical", "Error") are silently ignored and not loaded

- **FR-018b**: In v1, system MUST treat empty modes array `[]` identically to absent modes field (both inherit from parent/role); since v1 modes are diff operations (add/remove), an empty array means "no mode operations specified" which results in inheriting all modes from the base style/role; v1 modes never replace the whole set, only modify it through +/- operations

#### V0 Level-Specific Overrides

- **FR-019**: System MUST support level-specific element overrides under `levels` section for: trace, debug, info, warning, error

- **FR-020**: System MUST merge level-specific elements with base elements at load time (creating a complete StylePack for each level) such that level overrides win for defined properties

- **FR-020a**: System MUST allow level-specific elements to be defined without corresponding base elements in the custom theme; level-specific elements merge with @default theme's base element following the resolution order in FR-041 (start with @default element, merge base element if present in custom theme, then merge level-specific element); this allows themes to define level-specific overrides without requiring base element definitions

- **FR-020b**: System MUST merge base elements with level-specific elements using property-level override semantics: when both base element and level-specific element exist, level-specific properties win for defined properties (foreground, background, modes), while undefined properties in level-specific inherit from base; the merged result is used for rendering at that level; for example, if base `level` has foreground=#AAAAAA and modes=[italic], and `warning.level` has foreground=#FFA500, the warning level renders with foreground=#FFA500 (overridden) and modes=[italic] (inherited from base)

- **FR-021**: System MUST apply nested styling during rendering after level-specific merging is complete, so that parent-inner nesting works with the merged element set for each level

- **FR-021a**: V1 level-specific overrides MUST support all v1 features including the `style` field to reference roles, mode operations (+mode/-mode), and property-level merging semantics

#### V0 Additional Features

- **FR-022**: System MUST support tags array with allowed values: dark, light, 16color, 256color, truecolor; tags are optional metadata for theme classification

- **FR-022a**: System MUST validate that tag values are from the allowed set (dark, light, 16color, 256color, truecolor) and reject themes with unknown tag values

- **FR-022b**: System MUST allow empty tags array; tags are purely informational metadata

- **FR-022c**: System MUST allow multiple tags including combinations like dark+light (theme compatible with both modes), dark+256color, etc.; no tag combinations are considered conflicting

- **FR-023**: System MUST support indicators section with sync.synced and sync.failed configurations; indicators are a separate application feature (--follow mode) where sync state markers are displayed at the start of each line; themes provide only the visual styling for these indicator states (in sync vs out of sync)

- **FR-023a**: System MUST support the complete indicators schema structure: `indicators.sync.synced` and `indicators.sync.failed`, where each indicator has: `text` (string, the indicator character/text displayed), `outer.prefix` (string), `outer.suffix` (string), `outer.style` (style object with optional foreground, background, modes, and in v1: style field), `inner.prefix` (string), `inner.suffix` (string), `inner.style` (style object); all fields are optional with defaults: empty strings for text/prefix/suffix, empty style objects for outer/inner styles; the outer/inner distinction supports nested styling where the indicator text is rendered with inner style nested inside outer style

- **FR-024**: System MUST support boolean special case for backward compatibility in v0 and v1: if base `boolean` element is defined, automatically apply it to `boolean-true` and `boolean-false` during theme structure creation (after level-specific merging) before applying the variants' specific element-level overrides (this is active property merging, different from the passive nested styling scope used for other parent-inner pairs; this pattern exists because `boolean` was added first, variants came later); in v1, boolean-true and boolean-false can also use `style` field to reference roles like any other element

- **FR-024a**: When level-specific overrides include a `boolean` element override, the boolean active merge for that level uses the level-merged `boolean` element (base + level override) as the base for `boolean-true` and `boolean-false` at that level; this allows level-specific customization of boolean styling across all variants (e.g., if error level defines `boolean: {background: "#440000"}` and base defines `boolean-true: {foreground: "#00ffff"}`, then at error level boolean-true gets foreground from base boolean-true and background from error level's boolean)

- **FR-024b**: The boolean merge direction is: `boolean` element serves as base → `boolean-true`/`boolean-false` variants merge over it (clone boolean, then merge variant properties); variant properties override corresponding boolean properties; undefined properties in variants inherit from boolean; undefined properties in boolean resolve to base style (if variant specifies `style` field in v1) or terminal defaults; this is standard property-level override semantics where the patch (variant) overrides the base (boolean)

- **FR-025**: System MUST ignore unknown element names gracefully (forward compatibility)

- **FR-026**: System MUST validate color values and exit with clear error messages to stderr for invalid values, with format-specific error messages: invalid hex length (e.g., "#FFF must be #RRGGBB"), invalid hex characters (e.g., "#GGGGGG contains invalid hex characters"), out-of-range ANSI extended (e.g., "ANSI color 256 out of range (0-255)"), negative ANSI values, etc.

- **FR-026a**: System MUST perform color validation during initial theme file parsing (fail-fast approach); if a theme file contains invalid color values, the system exits with error immediately when parsing that file, before any merge or resolution operations; this provides immediate feedback to theme authors

- **FR-027**: System MUST allow duplicate modes in the modes array in v0 but deduplicate them during theme loading with last occurrence kept (e.g., modes=[bold, italic, bold] becomes [italic, bold]); v1 uses the same deduplication strategy for consistency

- **FR-028**: System MUST support $palette section in v0 theme schema for all formats (TOML, YAML, JSON), but only YAML can use anchor/alias syntax to reference palette colors; TOML and JSON can define $palette for organization but must reference colors by value; palette color values are NOT validated - $palette is purely an organizational/referencing feature in v0

- **FR-028a**: V1 themes MUST NOT include the `$palette` section; v1 strict parsing treats `$palette` as an unknown top-level section and rejects it per forward compatibility rules (FR-010c applies only when version is supported, but v1 doesn't support $palette); if a v1 theme includes `$palette`, the system exits with error indicating it's not supported in v1

- **FR-029**: System MUST report file format errors (TOML/YAML/JSON syntax errors) to stderr with line numbers and exit; YAML undefined anchor references are treated as parse errors

- **FR-029a**: System MUST rely on YAML parser to detect and report undefined anchor references in $palette section as parse errors with line numbers

- **FR-030**: System MUST provide theme listing grouped by origin (stock/custom) showing theme names only in compact multi-column layout with bullets (no tags or paths in listing output) when output is a terminal; when output is not a terminal (pipe/redirect), output plain list with one theme name per line without grouping or styling

- **FR-030a**: System MUST detect whether output is a terminal and adjust theme listing format accordingly: terminal output uses terminal-width-aware multi-column layout with alphabetical sorting within each group; non-terminal output uses plain list format (one name per line) without grouping

- **FR-030b**: System MUST display each theme by stem name only once in theme listings, even when multiple file formats exist for the same stem (e.g., if both theme.yaml and theme.toml exist, list shows "theme" once, representing the loadable theme per extension priority)

- **FR-030c**: System MUST include custom `@default` theme files in theme listings under the custom themes group; only the embedded `@default` theme is excluded from listings; this allows users to see and manage their custom @default themes

- **FR-030d**: System MUST sort themes alphabetically within each group (stock/custom) using case-sensitive ASCII ordering where uppercase letters come before lowercase letters (e.g., "MyTheme", "Theme", "another", "basic"); this provides deterministic and predictable ordering

- **FR-031**: System MUST automatically deduce style roles from element definitions in v0 themes before merging with `@default` theme, using these mappings: if v0 theme defines `time` element → deduce `styles.secondary` from it; if defines `string` → deduce `styles.primary`; if defines `message` → deduce `styles.strong`; if defines `key` → deduce `styles.accent`; if defines `array` → deduce `styles.syntax`; deduction copies foreground, background, and modes from the element definition to create the corresponding style role with no base (the deduced style has no reference to parent styles, i.e., no base roles to inherit from); note that v0 theme schema does not include a styles section, so deduced styles are created artificially during theme loading and cannot be explicitly defined in v0 theme files

- **FR-031a**: Style deduction MUST only create a style role if: (1) the corresponding element is defined in the v0 theme, AND (2) the style role is not already defined in the v0 theme; if a v0 theme explicitly defines both the element and its corresponding style, the explicit style definition takes precedence (no deduction)

- **FR-031b**: Style deduction MUST happen after loading the v0 theme file but before merging with `@default` theme; the deduced styles become part of the v0 theme's style inventory and override corresponding styles from `@default` during merge

- **FR-031c**: Deduced styles MUST be used by all elements in `@default` that reference those style roles, making elements not defined in the v0 theme (like new elements added to `@default`) use colors and modes consistent with the v0 theme's aesthetic; for example, if v0 theme defines `time: {foreground: 30}`, this deduces `secondary: {foreground: 30}`, and then `input` element from `@default` (which has `style: "secondary"`) will use foreground 30

- **FR-031d**: Style deduction MUST NOT affect elements that are explicitly defined in the v0 theme; elements defined in v0 themes are complete and render exactly as specified (no inheritance from deduced or `@default` styles)

- **FR-032**: System MUST preserve inherited modes from base styles when resolving element styles that reference roles; when a style's own properties (foreground, background, modes) are merged onto the resolved base, mode merging MUST be additive (not replacement) regardless of the `ReplaceModes` flag; the `ReplaceModes` flag applies only to theme-level merging (child theme replacing parent theme), not to within-theme style resolution

- **FR-032a**: For elements not defined in v0 themes, the system MUST fall back to `@default` theme's element definitions, preserving all properties including modes inherited from referenced styles; for example, if v0 theme doesn't define `input`, it falls back to `@default`'s `input = {style: "secondary"}` which resolves to include faint mode from the secondary style (unless overridden by deduced secondary style per FR-031)

#### V1 Versioning

- **FR-033**: System MUST treat themes without `version` field as v0

- **FR-034**: System MUST support version field with format "major.minor" where major=1 and minor is non-negative integer without leading zeros (e.g., "1.0")

- **FR-035**: System MUST validate version string against pattern `^1\.(0|[1-9][0-9]*)$` and reject malformed versions

- **FR-036**: System MUST support only version="1.0" initially; reject v1.1+ with error "Unsupported theme version 1.X, only version 1.0 is currently supported"; reject v2.0+ with error "Unsupported theme version 2.0, maximum supported major version is 1"

- **FR-036a**: System MUST treat future minor versions (v1.1, v1.2, etc.) as unsupported until explicitly implemented; version support can be extended in future releases

- **FR-037**: System MUST provide version-specific error messages: for v1.1+: "Unsupported theme version 1.X, only version 1.0 is currently supported"; for v2.0+: "Unsupported theme version 2.Y, maximum supported major version is 1"

#### V1 Enhanced Inheritance (Future)

- **FR-038**: V1 system MUST include an embedded `@default` theme that explicitly defines all 28 v0 elements and all 16 v1 roles with reasonable defaults; this theme is invisible when listing themes (not shown in stock or custom groups)

- **FR-038a**: V1 `@default` theme MUST define roles with inheritance chains where more specific roles inherit from more generic ones (e.g., specific roles reference `primary` or `secondary` via `style` field), providing flexibility so old themes remain compatible with newer app versions and look consistent even without defining new roles explicitly; the `default` role MUST always be defined in the embedded @default theme (guaranteed invariant, tested during development) since it serves as the implicit base for all other roles per FR-039b

- **FR-039**: V1 themes MUST support `styles` section as an object map where keys are role names (from predefined enum) and values are style objects containing optional foreground, background, modes, and an optional `style` field that references another role for parent/base inheritance (e.g., `styles: {warning: {style: "primary", foreground: "#FFA500", modes: [bold]}}`)

- **FR-039a**: V1 role names MUST be from the predefined enum (kebab-case, case-sensitive): default, primary, secondary, strong, muted, accent, accent-secondary, message, syntax, status, level, trace, debug, info, warning, error. Undefined role names or incorrect case (e.g., "Primary") are rejected with error. Element and role names exist in separate namespaces and can overlap without conflict.

- **FR-039b**: V1 `default` role serves as the implicit base for all roles that do not explicitly specify a `style` field; properties set in `default` (foreground, background, modes) apply to all other roles unless overridden

- **FR-039c**: V1 themes MUST allow `elements`, `styles`, and `levels` sections to be optional or empty; missing sections are treated as empty; undefined elements/roles inherit from the `@default` theme

- **FR-039d**: V1 MUST retain nested styling scope for parent/inner element pairs (same as v0) in addition to the new property-level merging available through roles; both inheritance mechanisms coexist in v1

- **FR-040**: V1 themes MUST support `style` property on elements to reference role names

- **FR-040a**: System MUST validate role names in the `style` field during initial theme file parsing (fail-fast approach); if an element's `style` field references a role name that is not in the predefined role enum (default, primary, secondary, strong, muted, accent, accent-secondary, message, syntax, status, level, trace, debug, info, warning, error), the system exits with error immediately when parsing that file; this provides immediate feedback to theme authors and catches typos early, before any merge or resolution operations

- **FR-041**: V1 themes MUST resolve element styles using the following order: 1) Start with element from @default theme (if defined), 2) Merge with base element from user theme (properties in base override @default), 3) Merge with level-specific element for the current level (level-specific properties override base), 4) If the merged element has a `style` field, resolve the role recursively (following role-to-role `style` references up to 64 levels depth), applying role properties to fill in undefined properties, 5) Apply explicit properties from the merged element (foreground, background, modes) which override role properties

- **FR-041a**: V1 element merging (steps 1-3) follows standard property override semantics: later sources override earlier sources for defined properties; undefined properties are inherited from earlier sources

- **FR-041b**: V1 role resolution (step 4) fills in properties not explicitly defined in the merged element; if merged element has foreground defined, role's foreground is not applied; if merged element lacks foreground but role defines it, role's foreground is used

- **FR-041c**: V1 property precedence: explicit properties in the merged element (result of steps 1-3) MUST override role properties when both are defined (e.g., if merged element has `style: "warning"` and `foreground: "#FF0000"`, and warning role has `foreground: "#FFA500"`, the final result uses `#FF0000` from the merged element)

- **FR-042**: V1 `@default` theme MUST define reasonable defaults for all roles, with the `default` role serving as the implicit base for all other roles that don't specify a `style` field, and specific roles using the `style` field to reference more generic ones (e.g., `warning: {style: "primary", ...}` where `primary` is also defined in `@default`)

- **FR-043**: V1 themes MUST support mode operations with +mode (add) and -mode (remove) prefixes; plain mode defaults to +mode. Modes are internally represented as two unordered sets (adds/removes). During style merging, child -mode removes parent's mode, child +mode adds mode. Final ANSI output includes only added modes in enum declaration order: Bold, Faint, Italic, Underline, SlowBlink, RapidBlink, Reverse, Conceal, CrossedOut. Remove operations are only used during merge process.

- **FR-043a**: V1 mode operations contrast with v0 replacement semantics: v0 child modes completely replace parent modes (no merging), v1 child modes modify parent modes (additive/subtractive operations)

- **FR-043b**: V1 themes MUST resolve conflicting mode operations within the same modes array using last occurrence wins semantics (e.g., modes=[+bold, -bold] results in bold removed; modes=[-bold, +bold] results in bold added)

- **FR-044**: V1 does NOT support custom `include` directive for referencing other themes; only `@default` theme inheritance is supported (custom includes may be considered for future versions)

- **FR-045**: V1 inheritance chain is simple: user theme → `@default` theme (no circular dependency detection needed)

- **FR-045a**: When a custom theme merges with the embedded `@default` theme, the resulting merged theme MUST use the version from the custom theme; this ensures the custom theme's merge semantics are applied correctly (e.g., v0 custom theme + v1 @default = v0 result with v0 replacement semantics; v1 custom theme + v1 @default = v1 result with v1 property-level merge semantics)

- **FR-046**: V1 role-to-role inheritance via the `style` field MUST support a maximum depth of 64 levels

- **FR-046a**: When role inheritance depth exceeds 64 levels, the system MUST exit with a specific error message including: the actual depth reached, the maximum allowed depth (64), and the inheritance chain showing affected roles; error format: "Role inheritance depth exceeded: maximum 64 levels (chain: role1 → role2 → ... → roleN)" where the chain shows the sequence of role references that caused the limit to be exceeded; this helps theme authors identify and fix deep or accidentally infinite inheritance chains

- **FR-047**: V1 themes MUST detect circular role references (e.g., `warning: {style: "error"}` and `error: {style: "warning"}`) and exit with error message showing the circular dependency chain

- **FR-047a**: Circular reference detection MUST occur after merging the user theme with the embedded `@default` theme, not before; user theme overrides can create circular role references that didn't exist in either theme individually (e.g., if @default has `A: {style: "B"}` and user theme overrides with `B: {style: "A"}`, the circular reference only exists after merge); the detection applies to the complete merged role inventory

- **FR-048**: V1 themes MUST exit with error when a role's `style` field references a role name that is not in the predefined role enum or when the referenced role is not defined in either the user theme or `@default` theme

### Key Entities

- **Theme**: Complete theme configuration containing element styles, level-specific overrides, indicators, version, and metadata tags

- **@default Theme** (v1 only): Embedded theme that explicitly defines all 28 v0 elements and all 16 v1 roles with reasonable defaults. Not visible in theme listings. All user themes (both v0 and v1) implicitly inherit from `@default` when roles or styles are not explicitly defined. More specific roles in `@default` typically inherit from more generic ones via `style` field (e.g., `warning: {style: "primary", ...}`), ensuring old themes remain compatible with newer app versions by falling back to consistent generic styles. Users CAN create custom themes named `@default` which merge with the embedded `@default` following normal theme merge rules (FR-001b): at theme merge level, custom elements completely replace embedded elements; property-level merging happens during style resolution based on custom theme's version.

- **Theme Version**: Version identifier following "major.minor" format (e.g., "1.0") where major=1 and minor is non-negative integer without leading zeros. Currently only version="1.0" is supported; future minor versions (1.1, 1.2, etc.) will be added as needed. Used to determine which schema and merge semantics apply.

- **Element**: Named visual element in log output (28 distinct elements in v0) including: input, input-number, input-number-inner, input-name, input-name-inner, time, level, level-inner, logger, logger-inner, caller, caller-inner, message, message-delimiter, field, key, array, object, string, number, boolean, boolean-true, boolean-false, null, ellipsis

- **Style**: Visual appearance specification with optional foreground color, optional background color, and optional text modes list. In v0, modes is a simple array of mode names. In v1, modes is an array of mode operations (+mode to add, -mode to remove, plain mode defaults to +mode), and styles can have an optional `style` field that references a parent/base style for inheritance.

- **Role** (v1 only): Named style defined in the `styles` section that can be referenced by elements or other roles. Role names must be from the predefined enum (kebab-case, case-sensitive): default, primary, secondary, strong, muted, accent, accent-secondary, message, syntax, status, level, trace, debug, info, warning, error. The `default` role is the implicit base for all roles that don't specify a `style` field - properties set in `default` apply to all other roles unless overridden. Roles support inheritance via the optional `style` field (e.g., `warning: {style: "primary", foreground: "#FFA500", modes: [+bold, -italic]}`).

- **Color**: Visual color value in one of three formats:
  - ANSI basic: named colors (case-sensitive: default, black, red, green, yellow, blue, magenta, cyan, white, bright-black, bright-red, bright-green, bright-yellow, bright-blue, bright-magenta, bright-cyan, bright-white)
  - ANSI extended: integer value 0-255 inclusive (0 is black in ANSI 256-color palette; values outside this range are rejected with specific error)
  - RGB: hex format #RRGGBB (exactly 6 hex digits; hex letters A-F are case-insensitive; other formats like #FFF, #RRGGBBAA are rejected with specific error messages)

- **Mode**: Text rendering mode (case-sensitive: bold, faint, italic, underline, slow-blink, rapid-blink, reverse, conceal, crossed-out). In v0, modes are plain values in an array. In v1, modes are operations: +mode (add), -mode (remove), or plain mode (defaults to +mode). V1 modes are internally stored as two unordered sets (adds/removes) and final output uses only adds in enum declaration order.

- **Level**: Log severity level (trace, debug, info, warning, error)

- **Tag**: Theme classification metadata with allowed values: dark, light, 16color, 256color, truecolor. Tags are optional and can be combined (e.g., dark+light means theme works in both modes, dark+256color means dark theme optimized for 256-color terminals). Empty tag array is valid.

### Non-Functional Requirements

- **NFR-001**: Theme loading MUST complete in under 50ms for all theme configurations including typical themes (50-100 elements), complex v1 themes with deep role inheritance chains (up to 64 levels), and maximum complexity scenarios

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
- When multiple files with same stem but different extensions exist, only the highest priority extension is loaded silently without warning or indication
- Color values are valid when specified (validation occurs during parsing)
- Users have appropriate terminal capabilities for the colors they choose (no runtime terminal capability detection required)
- Mode values are from the known set (unknown modes are rejected during parsing in v0; in v1 unknown mode operations are also rejected)
- All theme sections (`elements`, `levels`, `indicators`, `tags`, `styles`) are optional and can be empty or missing
- V0: missing sections mean all elements inside are considered missing; elements with parent/inner relations or boolean special case inherit from parent; others use empty style (default terminal colors, no modes)
- Indicators section (`indicators.sync.synced` and `indicators.sync.failed`) provides styling for sync state markers used in --follow mode; this is a separate application feature where themes only define visual appearance
- V1: missing sections are treated as empty; all undefined elements/roles inherit from `@default` theme
- Theme files are UTF-8 encoded
- Theme file size limits are enforced by OS/filesystem only; no application-level size validation or limits are imposed (parser will handle files of any size that the OS allows to be read)
- Both v0 and v1 use nested styling scope for parent-inner pairs (inner rendered inside parent) for these specific pairs listed in FR-015; if inner element is not defined, parent style continues through nesting; v1 adds property-level merging via roles as an additional inheritance mechanism
- Boolean special case (boolean → boolean-true/boolean-false) uses active property merging at load time in both v0 and v1, different from the passive nested styling used for other pairs; this exists for backward compatibility because `boolean` was added first and variants came later; in v1, boolean-true and boolean-false can also use the `style` field to reference roles like any other element (boolean special case merging occurs first, then role resolution)
- Empty modes array `[]` is semantically identical to absent modes field in v0 (both result in no mode override, so parent style continues through nesting or no modes applied)
- In v0, duplicate modes in modes array are allowed and all passed to terminal (terminal ignores redundant codes); v0 modes are plain values only (no +/- prefixes supported); if a mode with +/- prefix is detected in v0 theme, system exits with error suggesting to use v1 or remove prefix
- In v1, modes are operations: +mode (add mode), -mode (remove mode), or plain mode (defaults to +mode). Modes are internally represented as two unordered sets (adds/removes). During merge, child -mode can turn off parent's mode. When the same mode appears in both +mode and -mode forms within the same array, last occurrence wins. Final ANSI output uses only added modes in enum declaration order (Bold, Faint, Italic, Underline, SlowBlink, RapidBlink, Reverse, Conceal, CrossedOut); remove operations are only used during merge.
- The $palette section is part of the schema for all formats in both v0 and v1, but only YAML can use anchor/alias syntax; TOML and JSON can define $palette for organization but must reference colors by explicit values; YAML undefined anchor references are detected by the YAML parser and reported as parse errors with line numbers; v1 $palette works identically to v0
- Color validation provides format-specific error messages: RGB hex colors must be exactly #RRGGBB format (6 hex digits), ANSI extended colors must be integers 0-255 inclusive (0 is valid and represents black in ANSI 256-color palette), ANSI basic colors must match known color names; invalid formats exit with specific error describing the issue and expected format
- Stock themes are embedded in the application binary; custom themes are searched first, allowing users to completely replace stock themes by creating a custom theme with the same name (no merging - complete replacement)
- Level-specific overrides are merged with base elements at load time, creating a complete element set for each level; nested styling then applies during rendering (v0 and v1)
- In v1, level-specific element overrides can use the `style` field to reference roles, enabling level-specific elements to inherit from semantic roles (e.g., `levels.error.message: {style: "error-text", modes: [+bold]}`)
- The system has access to all theme files at load time (no lazy loading)
- Theme files are relatively small (< 10KB typical, < 100KB expected maximum in practice, but no hard limits enforced)
- Theme loading performance requirement (<50ms) applies to all scenarios including edge cases with 64-level role chains and maximum complexity
- Themes are loaded once at application startup and remain constant for the lifetime of the process; changing themes requires restarting the application
- Minimum valid theme: v0 can be completely empty file (inherits terminal defaults); v1 requires at minimum `version: "1.0"` field (inherits from `@default` theme)
- V1 property resolution order: 1) @default element, 2) merge base element, 3) merge level-specific element, 4) resolve `style` field (role resolution fills undefined properties), 5) explicit properties from merged element override role; this ensures level-specific can change role reference and explicit properties always win
- Custom theme files named `@default` ARE allowed and merge with the embedded v1 @default theme following normal theme merge rules (FR-001b); the embedded `@default` is excluded from theme listings but custom `@default` themes are loaded and merged; other theme names starting with `@` can be used normally
- File extension determines which parser is used (YAML/TOML/JSON); if file content doesn't match extension, parser fails with error to stderr (no auto-detection of actual format)
- Unknown top-level sections in theme files are ignored when the theme version is supported (forward compatibility); if theme version is unsupported, error occurs before section parsing; level names in `levels` section are case-sensitive (trace, debug, info, warning, error); unknown or invalid level names are silently ignored
- Unknown element properties (properties other than foreground, background, modes, and in v1: style) are silently ignored for forward compatibility; this allows newer themes with additional properties to work on older app versions
- Element names and role names exist in separate namespaces; element and role names can overlap without conflict (e.g., can have both an element named "message" and a role named "message" in v1)
- In v1, all user themes implicitly inherit from the embedded `@default` theme, which explicitly defines all 28 v0 elements and all 16 v1 roles with reasonable defaults; undefined roles/elements in user themes fall back to `@default` definitions
- V1 `@default` theme defines roles with inheritance chains where more specific roles inherit from more generic ones (e.g., `info: {style: "primary"}`, `warning: {style: "accent"}`), providing flexibility and forward compatibility - old themes work with newer app versions by falling back to consistent generic role styles
- Theme name matching when loading themes follows platform filesystem conventions: case-sensitive on Linux/macOS (e.g., "MyTheme" ≠ "mytheme"), case-insensitive on Windows (e.g., "MyTheme" matches "mytheme.yaml")
- Tags are validated against allowed values (dark, light, 16color, 256color, truecolor); unknown tags cause error; empty array is allowed; multiple tags including combinations like dark+light (compatible with both modes) are allowed; no tag combinations are considered conflicting
- Theme listing format is terminal-aware: when output is a terminal, use multi-column layout (terminal-width-aware) with alphabetical sorting within groups (stock/custom); when output is not a terminal (pipe/redirect), use plain list format with one theme name per line without grouping or styling; each theme shown by stem name once even if multiple formats exist
- All identifiers are case-sensitive: element names (e.g., "message" ≠ "Message"), role names (e.g., "primary" ≠ "Primary"), mode names (e.g., "bold" ≠ "Bold"), level names (e.g., "error" ≠ "Error"), ANSI basic color names (e.g., "black" ≠ "Black"); RGB hex color codes are case-insensitive for letters A-F
- Currently only version="1.0" is supported for v1 themes; version="1.1" or higher minor versions are rejected until implemented; version="2.0" or higher major versions are rejected as unsupported
- V1 role names are restricted to a predefined enum (kebab-case, case-sensitive): default, primary, secondary, strong, muted, accent, accent-secondary, message, syntax, status, level, trace, debug, info, warning, error. User themes can only define roles from this list; undefined role names or incorrect case are rejected with error. The `default` role is the implicit base for all roles that don't specify a `style` field - properties set in `default` (foreground, background, modes) apply to all other roles unless explicitly overridden.
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
