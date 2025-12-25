# Feature Specification: Theme Configuration System

**Feature Branch**: `007-theme-config-system`
**Created**: 2024-12-25  
**Status**: Draft  
**Input**: Define the complete theme configuration system including loading, validation, inheritance semantics, and versioning for both v0 (existing) and v1 (new) theme formats.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Theme File Loading and Validation (Priority: P1)

Users can create theme files in TOML format that define visual styles for log output elements. The system loads these themes, validates their structure against the appropriate schema version, and provides clear error messages for invalid configurations.

**Why this priority**: This is the foundation - without reliable theme loading, nothing else works. This documents the existing v0 behavior that already works in production.

**Independent Test**: Can be fully tested by creating valid and invalid theme files, attempting to load them, and verifying that valid themes load successfully while invalid themes produce specific error messages identifying the problem.

**Acceptance Scenarios**:

1. **Given** a valid v0 theme TOML file with element styles defined
   **When** the user loads the theme by name or path
   **Then** the theme loads successfully and all defined elements are available for styling

2. **Given** a theme file with invalid TOML syntax
   **When** the user attempts to load the theme
   **Then** the system reports a parse error with line number and description

3. **Given** a theme file with an undefined element name
   **When** the user loads the theme
   **Then** the system ignores the unknown element (graceful degradation)

4. **Given** a theme file with invalid color format
   **When** the user loads the theme
   **Then** the system reports an error identifying the invalid color value and expected format

---

### User Story 2 - V0 Parent-Inner Element Inheritance (Priority: P2)

Theme authors can define parent element styles (like `level`, `input-number`, `logger`, `caller`) and have corresponding inner elements (`level-inner`, `input-number-inner`, etc.) automatically inherit missing properties, enabling DRY theme authoring for common parent-inner pairs.

**Why this priority**: This is the core v0 inheritance feature that already exists. Documenting it properly is essential before adding v1 capabilities.

**Independent Test**: Can be tested by creating a theme with a parent element (e.g., `level`) having foreground and background, defining the inner element (`level-inner`) with only modes, and verifying the inner element displays with inherited colors plus explicit modes.

**Acceptance Scenarios**:

1. **Given** a parent element `level` with foreground=#FF0000 and background=#000000
   **When** an inner element `level-inner` is defined with only modes=[bold]
   **Then** `level-inner` displays with foreground=#FF0000 (inherited), background=#000000 (inherited), and modes=[bold] (explicit)

2. **Given** a parent element `input-number` with foreground=#00FF00
   **When** an inner element `input-number-inner` defines foreground=#FF0000 and modes=[italic]
   **Then** `input-number-inner` displays with foreground=#FF0000 (explicit override) and modes=[italic] (explicit)

3. **Given** a parent element `logger` with modes=[bold, underline]
   **When** an inner element `logger-inner` defines modes=[italic]
   **Then** `logger-inner` displays with modes=[italic] only (modes completely replace when non-empty, never merge in v0)

4. **Given** a parent element `caller` with modes=[bold]
   **When** an inner element `caller-inner` is defined with modes=[] or modes field absent
   **Then** `caller-inner` displays with modes=[bold] (empty array and absent field are identical - both inherit)

5. **Given** a parent element `caller` with all properties defined
   **When** the inner element `caller-inner` is not defined in the theme
   **Then** references to `caller-inner` use the parent `caller` style as fallback

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
   **Then** `level-inner` still inherits from base `level` (level overrides don't affect inheritance relationships)

---

### User Story 4 - Theme Metadata and Tags (Priority: P4)

Theme authors can add metadata tags to themes (dark, light, 16color, 256color, truecolor) to help users select appropriate themes for their terminal capabilities and preferences.

**Why this priority**: This is a convenience feature for theme discovery and filtering. It's useful but not essential for core functionality.

**Independent Test**: Can be tested by creating themes with various tag combinations and verifying the tags are correctly parsed and available for filtering.

**Acceptance Scenarios**:

1. **Given** a theme file with tags=["dark", "truecolor"]
   **When** the theme is loaded
   **Then** the tag metadata is available and can be queried

2. **Given** a theme file with tags=["light", "256color", "16color"]
   **When** listing available themes
   **Then** the theme appears in filtered lists for each tag category

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
   **Then** in v1, modes are merged: [bold, underline, italic] (contrast with v0 where modes=[italic] would replace entirely)

4. **Given** a v1 theme that includes a default theme via `include` directive
   **When** the theme overrides only 5 specific elements
   **Then** all non-overridden elements inherit from the default theme (property-level merging for v1)

---

### Edge Cases

- What happens when a parent element is defined at the level-specific scope but not in base elements?
- How does the system handle a theme with both base `level` and level-specific `warning.level` when displaying a warning?
- What happens when modes contains duplicate values (e.g., modes=[bold, italic, bold])?
- Can inner elements be defined without corresponding parent elements?
- What happens when a v1 theme with `style="warning"` reference has no `warning` role defined?
- How are circular includes detected in v1 (Theme A includes Theme B which includes Theme A)?
- What happens when v1 theme includes a v0 theme or vice versa?
- How does the system handle multi-level inheritance (grandparent → parent → child) in v1?
- What happens when a color palette anchor is referenced but not defined (YAML anchor edge case)?
- How are mode duplicates handled when merging in v1 (e.g., parent has [bold], child adds [bold, italic])?

## Requirements *(mandatory)*

### Functional Requirements

#### V0 Theme Loading (Existing Behavior)

- **FR-001**: System MUST load theme files in TOML format from user config directories and embedded resources

- **FR-002**: System MUST parse theme files with the following top-level sections: `elements`, `levels`, `indicators`, `tags`, `$palette` (optional YAML anchors)

- **FR-003**: System MUST support all v0 element names as defined in schema: input, input-number, input-number-inner, input-name, input-name-inner, time, level, level-inner, logger, logger-inner, caller, caller-inner, message, message-delimiter, field, key, array, object, string, number, boolean, boolean-true, boolean-false, null, ellipsis

- **FR-004**: System MUST support style properties: foreground (color), background (color), modes (array of mode enums)

- **FR-005**: System MUST support color formats: ANSI basic colors (named), ANSI extended colors (0-255 integers), RGB colors (#RRGGBB hex)

- **FR-006**: System MUST support mode values: bold, faint, italic, underline, slow-blink, rapid-blink, reverse, conceal, crossed-out

#### V0 Inheritance Semantics

- **FR-007**: System MUST apply parent-to-inner inheritance for these specific pairs only: (input-number, input-number-inner), (input-name, input-name-inner), (level, level-inner), (logger, logger-inner), (caller, caller-inner)

- **FR-008**: System MUST implement v0 style merging with these exact semantics:
  - When merging child into parent: if child has non-empty modes, child modes completely replace parent modes (no merging)
  - When merging child into parent: if child has foreground defined, child foreground replaces parent foreground
  - When merging child into parent: if child has background defined, child background replaces parent background

- **FR-009**: System MUST fall back to parent element when inner element is not defined (e.g., if `level-inner` is absent, use `level` style)

- **FR-010**: System MUST treat empty modes array `[]` identically to absent modes field (both inherit from parent)

#### V0 Level-Specific Overrides

- **FR-011**: System MUST support level-specific element overrides under `levels` section for: trace, debug, info, warning, error

- **FR-012**: System MUST merge level-specific elements with base elements such that level overrides win for defined properties

- **FR-013**: System MUST apply level overrides independently - overriding `level` at warning level does not affect `level-inner` inheritance from base `level`

#### V0 Additional Features

- **FR-014**: System MUST support tags array with allowed values: dark, light, 16color, 256color, truecolor

- **FR-015**: System MUST support indicators section with sync.synced and sync.failed configurations

- **FR-016**: System MUST support boolean special case: if base `boolean` is defined, apply it to `boolean-true` and `boolean-false` before applying their specific overrides

- **FR-017**: System MUST ignore unknown element names gracefully (forward compatibility)

- **FR-018**: System MUST validate color and mode values and reject invalid values with clear error messages

#### V1 Versioning

- **FR-019**: System MUST treat themes without `version` field as v0

- **FR-020**: System MUST support version field with format "major.minor" where major=1 and minor is non-negative integer without leading zeros (e.g., "1.0", "1.5", "1.12")

- **FR-021**: System MUST validate version string against pattern `^1\.(0|[1-9][0-9]*)$` and reject malformed versions

- **FR-022**: System MUST check theme version compatibility against the supported version range and reject themes with unsupported major or minor versions

- **FR-023**: System MUST provide error message format: "Unsupported theme version X.Y, maximum supported is A.B"

#### V1 Enhanced Inheritance (Future)

- **FR-024**: V1 themes MUST support `styles` section for defining semantic roles (warning, error, success, etc.)

- **FR-025**: V1 themes MUST support `style` property on elements to reference role names

- **FR-026**: V1 themes MUST resolve styles in this order: role resolution → parent inheritance → explicit overrides

- **FR-027**: V1 themes MUST use property-level merging for modes (union of parent and child modes) instead of replacement

- **FR-028**: V1 themes MUST support `include` directive to reference parent themes

- **FR-029**: V1 themes MUST detect circular includes and report error with dependency chain

- **FR-030**: V1 cross-theme merging MUST preserve property-level granularity (child overrides only specified properties, inherits others)

### Key Entities

- **Theme**: Complete theme configuration containing element styles, level-specific overrides, indicators, version, and metadata tags

- **Theme Version**: Version identifier following "major.minor" format (e.g., "1.0", "1.5") where major=1 and minor has no leading zeros. Used to determine which schema and merge semantics apply.

- **Element**: Named visual element in log output (28 distinct elements in v0) including: input, input-number, input-number-inner, input-name, input-name-inner, time, level, level-inner, logger, logger-inner, caller, caller-inner, message, message-delimiter, field, key, array, object, string, number, boolean, boolean-true, boolean-false, null, ellipsis

- **Style**: Visual appearance specification with optional foreground color, optional background color, and optional text modes list

- **Color**: Visual color value in one of three formats:
  - ANSI basic: named colors (default, black, red, green, yellow, blue, magenta, cyan, white, bright-black, bright-red, bright-green, bright-yellow, bright-blue, bright-magenta, bright-cyan, bright-white)
  - ANSI extended: integer value 0-255
  - RGB: hex format #RRGGBB

- **Mode**: Text rendering mode (bold, faint, italic, underline, slow-blink, rapid-blink, reverse, conceal, crossed-out)

- **Level**: Log severity level (trace, debug, info, warning, error)

- **Tag**: Theme classification metadata (dark, light, 16color, 256color, truecolor)

### Non-Functional Requirements

- **NFR-001**: Theme loading MUST complete in under 50ms for typical themes (50-100 elements)

- **NFR-002**: Theme validation errors MUST include specific location (field name, element name) and expected format

- **NFR-003**: Style merge operations MUST be deterministic and produce identical results across all platforms

- **NFR-004**: The system MUST base inheritance decisions on semantic property values (whether colors/modes are defined) not on internal representation details

- **NFR-005**: Code implementing inheritance MUST achieve >95% test coverage with unit tests for all edge cases

- **NFR-006**: V0 themes MUST continue to render identically after any refactoring (pixel-perfect regression tests)

- **NFR-007**: Invalid theme files MUST NOT cause application crashes - all errors MUST be handled gracefully with clear error messages

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

- Theme files are valid TOML format (TOML parsing errors are handled but not part of theme validation)
- Color values are valid when specified (validation occurs during parsing)
- Users have appropriate terminal capabilities for the colors they choose (no runtime terminal capability detection required)
- Mode values are from the known set (unknown modes are rejected during parsing)
- Theme files are UTF-8 encoded
- Parent-inner inheritance applies only to the specific pairs listed in FR-007 (not all *-inner elements)
- Empty modes array `[]` is semantically identical to absent modes field in v0 (both result in inheriting parent modes)
- Level-specific overrides are independent - overriding one element doesn't affect inheritance of related elements
- The system has access to all theme files at load time (no lazy loading)
- Theme files are relatively small (< 10KB typical, < 100KB maximum)
- YAML anchors ($palette) are a convenience feature - themes can be written without them
- Boolean special case (boolean → boolean-true/boolean-false) is unique to that element and not generalized to other parent-child relationships
- V1 features (roles, includes, property-level merging) are additive - v0 behavior remains unchanged
- Version checking occurs before any inheritance processing
- The default/embedded themes serve as reference implementations
- Theme authors understand basic color theory and terminal capabilities