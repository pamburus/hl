# Theme Configuration Architecture Design

## Overview

This document describes the refactored architecture for the theme configuration system, separating v0 (legacy) and v1 (current) theme formats while maintaining clean boundaries and avoiding unnecessary complexity.

## Core Principles

1. **v0 format** = Simple, flat, element-based styling (historical format, no semantic roles)
2. **v1 format** = Semantic roles, base styles, mode diffs (current format with advanced features)
3. **Type lineage**: Common types in main module → v0 uses them → v1 extends them
4. **Conversion**: v0 themes are loaded and converted to v1 format at load time
5. **Simplicity**: No circular dependency concerns - modules can reference parent freely

## Module Structure

```
hl/src/themecfg.rs (main module)
├── Common types (used by both v0 and v1):
│   ├── Tag (theme metadata: dark, light, palette16, etc.)
│   ├── Mode (text styling: bold, italic, etc.)
│   ├── ModeSetDiff, ModeDiff, ModeDiffAction (mode diff system)
│   ├── Color, PlainColor, RGB (color types)
│   ├── ThemeVersion (version tracking)
│   ├── MergeFlag, MergeFlags (merge behavior control)
│   ├── Error, ThemeLoadError, ExternalError (all error types)
│   ├── Format (yaml, toml, json)
│   ├── MergedWith, Mergeable traits
├── Output types:
│   ├── Style (resolved style - was ResolvedStyle)
│   ├── StyleInventory (type alias)
│   ├── ThemeInfo (name, source, origin), ThemeOrigin, ThemeSource
├── Public API:
│   ├── Theme (fully resolved theme - was ResolvedTheme)
│   │   ├── Theme::load(name) -> Result<Theme> (loads, merges, resolves)
│   │   └── Theme::load_raw(name) -> Result<RawTheme> (loads, merges, not resolved)
│   ├── RawTheme (wrapper struct with ThemeInfo metadata)
│   │   ├── Fields: info (ThemeInfo), inner (v1::Theme)
│   │   ├── RawTheme::new(info, inner) -> RawTheme
│   │   ├── RawTheme::merged(other) -> RawTheme
│   │   ├── RawTheme::resolve() -> Result<Theme> (wraps errors with ThemeInfo)
│   │   └── Deref/DerefMut to v1::Theme for transparent field access
│   ├── RawStyle (type alias for v1::Style - unresolved style)
└── Re-exports for public API:
    └── pub use v1::{Element, Role, StylePack, ...}

hl/src/themecfg/v0/mod.rs
├── v0-specific types:
│   ├── Element (primitive, unchanged in v1)
│   ├── Style { modes: Vec<Mode>, foreground, background }
│   │   └── No 'base' field (v0 doesn't have inheritance)
│   │   └── Simple Vec<Mode> (no diff system)
│   ├── StylePack(HashMap<Element, Style>)
│   │   └── Not generic, just Element->Style mapping
│   ├── Indicator types (simple, no generics):
│   │   ├── IndicatorPack { sync: SyncIndicatorPack }
│   │   ├── SyncIndicatorPack { synced, failed }
│   │   ├── Indicator { outer, inner, text }
│   │   └── IndicatorStyle { prefix, suffix, style: Style }
│   └── Theme (v0 deserialization target, called "Theme" in v0 module):
│       ├── tags: EnumSet<Tag>
│       ├── version: ThemeVersion
│       ├── elements: StylePack (no 'styles' section in v0!)
│       ├── levels: HashMap<Level, StylePack>
│       └── indicators: IndicatorPack
├── Deserialization:
│   └── Lenient (ignores unknown keys for forward compatibility)
└── NO merge/resolution logic (all in v1)

hl/src/themecfg/v1/mod.rs
├── Re-export unchanged from v0:
│   └── pub use super::v0::Element;
├── v1-NEW types:
│   ├── Role (semantic styling - NEW in v1):
│   │   └── Default, Primary, Secondary, Strong, Muted, Accent, etc.
│   ├── StyleBase (base style inheritance - NEW in v1):
│   │   └── Vec<Role> for inheriting from role-based styles
│   ├── Style { base, modes: ModeSetDiff, foreground, background }
│   │   └── Uses StyleBase (can inherit from roles)
│   │   └── Uses ModeSetDiff from parent module (supports +/- prefix)
│   │   └── Called "Style" in v1 module, re-exported as "RawStyle" in main
│   ├── StylePack<K, S = Style> (generic version - NEW in v1):
│   │   └── Can map Role->Style or Element->Style
│   ├── Indicator<S = Style> types (generic - NEW in v1):
│   │   ├── IndicatorPack<S>
│   │   ├── SyncIndicatorPack<S>
│   │   ├── Indicator<S>
│   │   └── IndicatorStyle<S>
│   └── Theme (v1 deserialization target, called "Theme" in v1 module):
│       ├── tags: EnumSet<Tag>
│       ├── version: ThemeVersion
│       ├── styles: StylePack<Role, Style> (NEW - role-based styles!)
│       ├── elements: StylePack<Element, Style>
│       ├── levels: HashMap<Level, StylePack<Element, Style>>
│       └── indicators: IndicatorPack<Style>
├── Conversion from v0:
│   ├── impl From<v0::Theme> for v1::Theme
│   ├── convert_v0_style_to_v1() (Vec<Mode> -> ModeSetDiff)
│   ├── deduce_styles_from_elements() (map elements to roles)
│   └── convert_v0_indicators_to_v1()
├── Merging logic (ALL merge logic here):
│   ├── merge_themes(base, overlay) -> Theme
│   ├── StylePack::merge() implementations
│   ├── Indicator merge implementations
│   └── Style::merged() implementations
├── Resolution logic (ALL resolution here):
│   ├── resolve_theme() -> Result<super::Theme>
│   ├── StylePack::resolve() -> Result<StyleInventory> (can fail on recursion)
│   ├── Style::resolve() implementations
│   └── StyleResolver (helper for role resolution, checks recursion limit)
└── Deserialization:
    └── Strict (fails on unknown fields - proper versioning)
```

## Type Ownership Matrix

| Type | Location | Generic? | Reason |
|------|----------|----------|--------|
| `Element` | v0 | No | Primitive enum, unchanged in v1 |
| `Mode` | themecfg | No | Primitive enum, used by both v0 and v1 |
| `ModeSetDiff`, `ModeDiff` | themecfg | No | Common type, used by v1 (v0 uses Vec<Mode>) |
| `Color`, `RGB`, `PlainColor` | themecfg | No | Primitive types, unchanged |
| `Tag` | themecfg | No | Metadata, unchanged |
| `Role` | v1 | No | **NEW in v1** - semantic styling concept |
| `StyleBase` | v1 | No | **NEW in v1** - base style inheritance |
| `v0::Style` | v0 | No | `{ modes: Vec<Mode>, foreground, background }` |
| `v1::Style` (RawStyle) | v1 | No | `{ base: StyleBase, modes: ModeSetDiff, ... }` |
| `v0::StylePack` | v0 | No | Simple `HashMap<Element, Style>` |
| `v1::StylePack` | v1 | Yes `<K, S>` | Generic over key and style type |
| `v0::Indicator` | v0 | No | Simple, concrete style field |
| `v1::Indicator` | v1 | Yes `<S>` | Generic over style type |
| `v0::Theme` | v0 | No | v0 deserialization target |
| `v1::Theme` | v1 | No | v1 deserialization target |
| `ThemeVersion` | themecfg | No | Version tracking, common |
| `MergeFlag` | themecfg | No | Merge behavior, common |
| `Error` | themecfg | No | Single error type for all versions |
| `ThemeLoadError` | themecfg | No | Single error type |
| `Style` | themecfg | No | **Resolved style** (was ResolvedStyle) |
| `Theme` | themecfg | No | **Resolved theme** (public API, was ResolvedTheme) |
| `RawTheme` | themecfg | No | **Wrapper struct** with ThemeInfo + v1::Theme |
| `RawStyle` | themecfg | No | Type alias for `v1::Style` (unresolved) |
| `ThemeInfo` | themecfg | No | Theme metadata (name, source, origin) |
| `ThemeSource` | themecfg | No | Embedded or Custom file path |

## Key Architectural Decisions

### 1. No Common Module Needed
- Common types live in `themecfg.rs` main module
- Sub-modules reference parent via `super::`
- No artificial separation needed

### 2. Single Error Type
- One `Error` enum in main `themecfg` module
- Used by both v0 and v1
- Simpler than separate v0::Error and v1::Error

### 3. RawTheme Wrapper with Metadata
- `RawTheme` is a wrapper struct (not a type alias) containing:
  - `info: ThemeInfo` - metadata (name, source, origin)
  - `inner: v1::Theme` - the actual theme data
- `RawTheme::resolve()` automatically wraps errors with `ThemeInfo` context
- Deref/DerefMut provide transparent access to inner v1::Theme fields
- **No code duplication** - context added in one place (RawTheme::resolve)
- **Works everywhere** - any call to resolve() gets proper error context

### 4. Type Location Strategy
**themecfg (main)**: Primitives and common infrastructure
- Types that don't change between versions
- Types used by both v0 and v1
- Output types (ResolvedTheme, ResolvedStyle)

**v0**: v0-specific simple types
- Element (stays here as it's fundamental to v0)
- Simple Style without base field
- Non-generic StylePack and Indicator types
- NO merge/resolution logic

**v1**: v1 extensions and features
- NEW types: Role, StyleBase
- Extended Style with base field
- Generic versions of StylePack and Indicator
- ALL merge and resolution logic

### 5. No Logic in v0
- v0 module only has data structures and deserialization
- NO merge implementations
- NO resolution logic
- Clean, simple, historical format representation

### 6. All Logic in v1
- ALL merging logic
- ALL resolution logic
- Conversion from v0
- This is where the complexity lives

### 7. Clear Naming: Resolved vs Raw
- **In v0 and v1 modules**: Types are named naturally (`Theme`, `Style`)
- **In main themecfg module**: 
  - `Theme` = **resolved** theme (what was `ResolvedTheme`)
  - `Style` = **resolved** style (what was `ResolvedStyle`)
  - `RawTheme` = **wrapper struct** with metadata + v1::Theme
  - `RawStyle` = type alias for `v1::Style` (unresolved)
- `Theme::load()` returns fully resolved theme (load + merge + resolve)
- `Theme::load_raw()` returns unresolved theme (load + merge only)
- `RawTheme::resolve()` returns resolved `Theme`

## Data Flow

### Loading a v0 Theme (Resolved)
```
1. User calls Theme::load(name)
2. Theme::load_raw(name) loads and merges:
   a. Theme::load_from() reads file
   b. Theme::peek_version() detects version 0.0
   c. Deserialize as v0::Theme (lenient)
   d. Convert to v1::Theme via From impl:
      - Vec<Mode> -> ModeSetDiff
      - No styles section -> deduce from elements
      - Simple indicators -> generic indicators
   e. Merge with @default theme
   f. Returns RawTheme (alias for v1::Theme)
3. RawTheme::resolve() called
4. v1::resolve_theme() resolves all styles
5. Returns Theme (resolved, with Style not RawStyle)
```

### Loading a v1 Theme (Resolved)
```
1. User calls Theme::load(name)
2. Theme::load_raw(name) loads and merges:
   a. Theme::load_from() reads file
   b. Theme::peek_version() detects version 1.0
   c. Deserialize as v1::Theme (strict)
   d. Merge with @default theme
   e. Returns RawTheme (alias for v1::Theme)
3. RawTheme::resolve() called
4. v1::resolve_theme() resolves all styles
5. Returns Theme (resolved, with Style not RawStyle)
```

### Loading Unresolved (for advanced use)
```
1. User calls Theme::load_raw(name)
2. Same as above but stops at step 2.e
3. Returns RawTheme (unresolved)
4. User can manually merge, modify, then resolve
```

### Merging Themes
```
1. User has raw_theme: RawTheme (v1::Theme)
2. Calls raw_theme.merge(other)
3. Delegates to v1::merge_themes(self, other)
4. Returns merged RawTheme (v1::Theme)
```

### Resolving a Theme
```
1. User has raw_theme: RawTheme (v1::Theme)
2. Calls raw_theme.resolve()
3. Delegates to v1::resolve_theme(self)
4. Returns Theme (resolved, with Style not RawStyle)
```

## Migration from Old Code

### What Moves Where

**From old themecfg.rs to themecfg.rs (stays):**
- Error, ThemeLoadError, ExternalError
- Tag, Mode, Color, RGB, PlainColor
- ModeSetDiff, ModeDiff, ModeDiffAction
- ThemeVersion, Format
- MergeFlag, MergeFlags
- Theme (renamed from ResolvedTheme - the **resolved** theme)
- Style (renamed from ResolvedStyle - the **resolved** style)
- RawTheme (type alias for v1::Theme - **unresolved**)
- RawStyle (type alias for v1::Style - **unresolved**)
- MergedWith, Mergeable traits

**From old themecfg.rs to v0:**
- Element enum
- Simple Style struct (without base)
- Non-generic StylePack
- Non-generic Indicator types
- Theme (for v0 deserialization)

**From old themecfg.rs to v1:**
- Role enum
- StyleBase
- Extended Style (with base field) - becomes RawStyle in main
- Generic StylePack<K, S>
- Generic Indicator<S> types
- Theme (for v1 deserialization) - becomes RawTheme in main
- ALL merge implementations
- ALL resolution logic
- StyleResolver
- Conversion functions from v0

## Public API

Users of the `themecfg` module see:

```rust
use hl::themecfg::{
    Theme,           // RESOLVED theme (main API) - load() returns this
    Style,           // RESOLVED style (used in Theme)
    RawTheme,        // Unresolved theme (v1::Theme) - load_raw() returns this
    RawStyle,        // Unresolved style (v1::Style) - used in RawTheme
    Element,         // Re-exported from v1
    Role,            // Re-exported from v1
    Color,           // From main module
    Mode,            // From main module
    Error,           // From main module
    // ... etc
};

// Typical usage (most common):
let theme: Theme = Theme::load(&app_dirs, "my-theme")?;  
// ↑ Fully resolved, ready to use
// Contains Style (resolved), not RawStyle

// Advanced usage (for customization):
let raw: RawTheme = Theme::load_raw(&app_dirs, "my-theme")?;  
// ↑ Unresolved, can be modified
// Contains RawStyle (unresolved), not Style
let modified = raw.merge(custom_overrides);
let theme: Theme = modified.resolve()?;  
// ↑ Now resolved (Style, not RawStyle)
```

They don't need to know about v0 vs v1 internals - that's all implementation detail.

## Benefits of This Architecture

1. **Clean separation**: v0 is pure historical format, v1 is current
2. **No duplication**: Common types in one place (main module)
3. **No artificial complexity**: No need for common module or duplicate errors
4. **Clear upgrade path**: v0 -> v1 conversion is explicit
5. **Type safety**: Generic v1 types enforce correct usage
6. **Maintainable**: All logic for each version in its own module
7. **Future-proof**: Adding v2 would follow same pattern
8. **Simple public API**: Users just use current version types
9. **Resolved by default**: Theme::load() returns resolved theme (most common case)
10. **Advanced control**: load_raw() + manual resolve for power users
11. **Type clarity**: Theme = resolved, RawTheme = unresolved (clear naming)
12. **Fail fast**: Use Level instead of InfallibleLevel - unknown levels are errors
13. **Error handling**: Recursion limit violations return ThemeLoadError::StyleRecursionLimitExceeded
    - Resolution errors are wrapped in FailedToResolveTheme with full ThemeInfo
    - ThemeInfo contains name, source (Embedded or Custom{path}), and origin (Stock or Custom)
    - Error message includes theme name and the problematic role (displayed in kebab-case)
    - Role implements Display using serde to match user input format
    - RawTheme::resolve() automatically adds context - no code duplication
    - Example: "failed to resolve theme 'my-theme': style recursion limit exceeded while resolving role primary"
14. **Clear naming**: Raw vs Resolved is explicit in type names
    - `v1::Theme` → main's `RawTheme` (unresolved)
    - `v1::Style` → main's `RawStyle` (unresolved)
    - main's `Theme` = resolved (was ResolvedTheme)
    - main's `Style` = resolved (was ResolvedStyle)

## Documentation

- **Module-level docs**: Complete overview of theme system architecture
- **Public API docs**: Comprehensive docs for `Theme`, `RawTheme`, `Style`, `RawStyle`
- **Examples**: Usage examples for common and advanced scenarios
- **Error docs**: Detailed error type documentation with example messages
- **Format docs**: V0 vs V1 format differences with YAML examples
- **$schema support**: Documented for IDE integration
- **Role Display**: User-friendly kebab-case formatting using serde

## Testing Strategy

- **v0 tests**: Test v0 deserialization, ensure it matches historical behavior
- **v1 tests**: Test v1 features, merging, resolution
- **Integration tests**: Test v0 -> v1 conversion, mixed theme loading
- **Round-trip tests**: Ensure serialize -> deserialize preserves data

## Future: Adding v2

When we add v2:
1. Create `themecfg/v2/mod.rs`
2. Define v2-specific types (or reuse from v1)
3. Add `impl From<v1::RawTheme> for v2::RawTheme`
4. Move current v1 logic to v2 if needed
5. Update `themecfg.rs` to `pub use v2::*` for public API
6. No changes needed to v0 or v1 modules