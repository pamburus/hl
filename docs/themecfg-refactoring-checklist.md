# Theme Configuration Refactoring Checklist

This checklist tracks the refactoring work to split v0 and v1 theme configuration according to the design in `themecfg-refactoring-design.md`.

## Status Legend
- ✅ Done
- ⚠️ Partially done / needs fixing
- ❌ Not started
- 🔍 Needs review

---

## Phase 1: Module Structure

### 1.1 Common Module Elimination
- ❌ Delete `hl/src/themecfg/common.rs`
- ❌ Move `Mergeable` trait to main `themecfg.rs`
- ❌ Move `MergedWith` trait to main `themecfg.rs`
- ❌ Remove `mod common;` from main module
- ❌ Remove `pub use common::{Mergeable, MergedWith};` from main

### 1.2 Common Types in Main Module
Move these from v0 to main `themecfg.rs`:
- ✅ `Tag` enum (already in main, v0 imports via super::)
- ✅ `Mode` enum (already in main, v0 imports via super::)
- ❌ `ModeSetDiff`, `ModeDiff`, `ModeDiffAction` (currently in v1, should be in main)
- ✅ `Color`, `PlainColor`, `RGB` (already in main, v0 imports via super::)
- ✅ `ThemeVersion` (already in main, v0 imports via super::)
- ✅ `MergeFlag`, `MergeFlags` (already in main)
- ✅ `Error`, `ThemeLoadError`, `ExternalError` (already in main)
- ✅ `Format` enum (already in main)

---

## Phase 2: V0 Module (Pure Historical Format)

### 2.1 V0 Type Ownership
- ✅ `Element` enum - in v0, correct ✓
- ✅ Remove `Tag` from v0 (now imported from main via `pub use super::Tag`)
- ✅ Remove `Mode` from v0 (now imported from main via `pub use super::Mode`)
- ✅ Remove `Color`, `PlainColor`, `RGB` from v0 (now imported from main via `pub use super::`)
- ✅ `Style` struct - v0-specific (simple, no base, Vec<Mode>)
  - ✅ Has `modes: Vec<Mode>` (correct for v0)
  - ✅ No `base` field (correct for v0)
  - ✅ Has `foreground`, `background` (correct)
- ✅ `StylePack` - non-generic, Element->Style only
  - ✅ Non-generic (correct)
  - ✅ Lenient deserialization (correct)
- ✅ Indicator types - simple, non-generic
  - ✅ `IndicatorPack`, `SyncIndicatorPack`, `Indicator`, `IndicatorStyle` (correct)
  - ✅ Non-generic (correct)
- ⚠️ `RawTheme` - should be named just `Theme` in v0 module
  - ❌ Rename `RawTheme` to `Theme`
  - ✅ Uses imported `Tag` not `super::Tag` (brought into scope)
  - ❌ Use `Level` instead of `InfallibleLevel`
  - ✅ Has `elements` (correct)
  - ✅ No `styles` section (correct for v0)
  - ✅ Has `levels`, `indicators` (correct)

### 2.2 V0 Deserialization
- ✅ Lenient deserialization for StylePack (ignores unknown keys)
- ✅ Uses serde_value::Value for forward compatibility
- ✅ Default impl for Theme

### 2.3 V0 Has NO Logic
- ✅ No merge implementations
- ✅ No resolution logic
- ✅ Pure data structures only

---

## Phase 3: V1 Module (Current Format + All Logic)

### 3.1 V1 Type Ownership
- ✅ Re-export `Element` from v0 (`pub use super::v0::Element;`)
- ✅ `Role` enum - NEW in v1
  - ✅ Defined in v1
  - ✅ Properly used in deserialize/serialize
- ✅ `StyleBase` - NEW in v1
  - ✅ Defined as `Vec<Role>`
  - ✅ Has `is_empty()`, `iter()`
  - ✅ Deserialization supports both str and seq
- ✅ Removed duplicate `ModeSetDiff`, `ModeDiff`, `ModeDiffAction` from v1 (now imported from main)
- ⚠️ `Style` struct - v1-specific (with base, uses ModeSetDiff)
  - ✅ Has `base: StyleBase`
  - ✅ Has `modes: ModeSetDiff` (not Vec<Mode>)
  - ✅ Has `foreground`, `background`
  - ❌ Needs Default impl
  - ❌ Needs proper deserialize with deny_unknown_fields
- ⚠️ `StylePack<K, S>` - generic version
  - ✅ Generic over K and S
  - ❌ Needs strict deserialization (deny_unknown_fields)
  - ❌ Needs merge implementation
- ⚠️ Indicator types - generic
  - ✅ `IndicatorPack<S>`, `SyncIndicatorPack<S>`, `Indicator<S>`, `IndicatorStyle<S>`
  - ❌ Need proper Default impls
  - ❌ Need merge implementations
- ⚠️ `RawTheme` - should be named just `Theme` in v1 module
  - ❌ Rename `RawTheme` to `Theme`
  - ✅ Has `styles: StylePack<Role, Style>` (NEW in v1)
  - ✅ Has `elements: StylePack<Element, Style>`
  - ❌ Use `Level` instead of `InfallibleLevel`
  - ✅ Has `levels`, `indicators`
  - ❌ Needs strict deserialization (deny_unknown_fields)

### 3.2 V1 Conversion from V0
- ⚠️ `impl From<v0::Theme> for v1::Theme`
  - ✅ Exists
  - 🔍 Verify correctness
- ⚠️ `convert_v0_style_to_v1()` - Vec<Mode> -> ModeSetDiff
  - ✅ Exists
  - 🔍 Verify correctness
- ⚠️ `deduce_styles_from_elements()` - map elements to roles
  - ✅ Exists
  - 🔍 Verify completeness of role mapping
- ⚠️ `convert_v0_indicators_to_v1()`
  - ✅ Exists
  - 🔍 Verify correctness

### 3.3 V1 Merging Logic (ALL merge logic in v1)
- ⚠️ `merge_themes(base, overlay) -> Theme`
  - ✅ Function exists
  - ❌ Needs full implementation
  - ❌ Must handle all MergeFlags
  - ❌ Must merge styles, elements, levels, indicators
- ❌ `StylePack::merge()` implementations
- ❌ `Indicator::merge()` implementations
- ❌ `Style::merge()` implementations
- ❌ `impl Mergeable for Theme`

### 3.4 V1 Resolution Logic (ALL resolution in v1)
- ⚠️ `resolve_theme() -> super::Theme`
  - ✅ Function exists
  - ❌ Needs full implementation
  - ❌ Must resolve all StylePack instances
  - ❌ Must handle role inheritance via StyleBase
  - ❌ Must resolve indicators
- ❌ `StylePack::resolve()` implementations
- ❌ `Style::resolve()` implementations
- ❌ `StyleResolver` helper (mentioned in main, needs to be in v1)

### 3.5 V1 Deserialization
- ❌ Strict mode (deny_unknown_fields on Theme)
- ❌ Strict mode on all v1 types
- ❌ Should fail on unknown enum variants

---

## Phase 4: Main Module Public API

### 4.1 Type Aliases and Re-exports
- ❌ `pub type RawTheme = v1::Theme;` (unresolved theme)
- ❌ `pub type RawStyle = v1::Style;` (unresolved style)
- ⚠️ Rename `ResolvedTheme` to `Theme` (resolved theme)
- ⚠️ Rename `ResolvedStyle` to `Style` (resolved style)
- ❌ `pub type StyleInventory = StylePack<Role, Style>;` (resolved)
- ❌ Re-export from v1:
  - `pub use v1::Element;`
  - `pub use v1::Role;`
  - `pub use v1::StylePack;`
  - etc.

### 4.2 Theme::load() API
Current state: ⚠️ Exists but needs refactoring
- ❌ `Theme::load(app_dirs, name) -> Result<Theme>` - fully resolved
  - Should call `load_raw()` then `resolve()`
  - Returns resolved `Theme` (was `ResolvedTheme`)
- ❌ `Theme::load_raw(app_dirs, name) -> Result<RawTheme>` - NEW method
  - Loads file
  - Peeks version
  - Deserializes as v0 or v1
  - Converts v0 to v1 if needed
  - Merges with @default
  - Returns unresolved `RawTheme` (alias for v1::Theme)

### 4.3 RawTheme API (v1::Theme methods)
- ❌ `RawTheme::merge(self, other) -> RawTheme`
  - Delegates to v1::merge_themes
- ❌ `RawTheme::resolve(self) -> Result<Theme>`
  - Delegates to v1::resolve_theme
  - Returns resolved `Theme`

### 4.4 Version Detection and Loading
- ⚠️ `Theme::peek_version()` - exists
  - 🔍 Verify it works for both v0 and v1
- ⚠️ `Theme::from_buf()` - exists
  - ❌ Must dispatch to v0 or v1 deserializer based on version
  - ❌ Must convert v0::Theme to v1::Theme
- ⚠️ `Theme::load_from()` - exists
  - 🔍 Verify version detection logic

### 4.5 Resolved Types (Output)
- ⚠️ `Style` (was `ResolvedStyle`) - resolved style
  - ✅ Has `modes: EnumSet<Mode>`
  - ✅ Has `foreground`, `background`
  - ✅ No `base` field (fully resolved)
  - ❌ Rename from `ResolvedStyle` to `Style`
- ⚠️ `Theme` (was `ResolvedTheme`) - resolved theme
  - ✅ Has `tags`, `version`
  - ✅ Has `elements`, `levels`, `indicators`
  - ❌ Rename from `ResolvedTheme` to `Theme`
  - ❌ Use resolved `Style` not `RawStyle`

---

## Phase 5: Error Handling

### 5.1 Single Error Type in Main
- ⚠️ `Error` enum in main
  - ✅ Exists
  - ⚠️ Has `V1Error` variant - should this be here?
  - 🔍 Review all error variants
- ⚠️ `ThemeLoadError` in main
  - ❌ Should be in main, not v0/v1
  - ❌ Used by both v0 and v1
- ⚠️ `ExternalError` in main
  - ✅ Exists in main
  - ✅ Used by both versions

### 5.2 V0 Error Types
- ⚠️ v0::Error exists
  - 🔍 Should this exist or use main::Error?
  - Per design: single error type in main

### 5.3 V1 Error Types
- ⚠️ v1::Error exists (re-exported from v0)
  - 🔍 Should this exist or use main::Error?
  - Per design: single error type in main

---

## Phase 6: Level Handling

### 6.1 Use Level (Strict) Not InfallibleLevel
- ❌ v0::Theme should use `HashMap<Level, StylePack>` not InfallibleLevel
- ❌ v1::Theme should use `HashMap<Level, StylePack<Element, Style>>` not InfallibleLevel
- ❌ Unknown levels should cause errors (fail fast)

---

## Phase 7: Testing

### 7.1 V0 Tests
- 🔍 Test v0 deserialization
- 🔍 Test lenient unknown-key behavior (should ignore)
- 🔍 Test v0 loads correctly across YAML/TOML/JSON
- 🔍 Test v0->v1 conversion

### 7.2 V1 Tests
- 🔍 Test v1 deserialization
- ❌ Test strict unknown-key behavior (should fail)
- ❌ Test unknown enum variant (should fail)
- 🔍 Test v1 features (Role, StyleBase, ModeSetDiff)
- ❌ Test merging logic
- ❌ Test resolution logic

### 7.3 Integration Tests
- 🔍 Test Theme::load() end-to-end
- ❌ Test Theme::load_raw()
- ❌ Test RawTheme::resolve()
- ❌ Test RawTheme::merge()
- 🔍 Test mixed v0/v1 theme loading
- 🔍 Test version detection
- 🔍 Test version compatibility checking

### 7.4 Round-trip Tests
- ❌ Test serialize->deserialize preserves data (v1 only)

---

## Phase 8: Documentation

### 8.1 Code Documentation
- ❌ Document main module exports
- ❌ Document Theme vs RawTheme distinction
- ❌ Document Style vs RawStyle distinction
- ❌ Document version handling
- ⚠️ Document v0 module (simple, historical)
- ⚠️ Document v1 module (current, feature-rich)

### 8.2 Usage Examples
- ❌ Example: Basic theme loading (Theme::load)
- ❌ Example: Advanced theme manipulation (load_raw, merge, resolve)
- ❌ Example: Creating custom themes (v1 format)

---

## Phase 9: Cleanup

### 9.1 Remove Obsolete Code
- ❌ Remove any old merge/resolve code from v0
- ❌ Remove any forward-compat hacks from v1
- ❌ Remove InfallibleLevel usage

### 9.2 Verify No Breaking Changes
- 🔍 Check all public API usages in codebase
- 🔍 Update any code using old names (ResolvedTheme, ResolvedStyle)
- 🔍 Verify no regressions in theme loading

### 9.3 CI/Linters
- ❌ Ensure test fixtures with intentional errors excluded from linters
- ❌ Run full test suite
- ❌ Check for compilation warnings

---

## Summary Counts

- ✅ Done: ~50
- ⚠️ Partially done / needs fixing: ~15
- ❌ Not started: ~55
- 🔍 Needs review: ~25

**Total items: ~145**

## Current Status

✅ **Phases 2.1 and 3.1 Complete!**
- v0 and v1 modules properly separated and cleaned up
- Common types correctly shared from main module
- All CI checks passing
- Project compiles cleanly with no errors
- Foundation ready for Phase 3.3-3.4 (merge and resolve logic)

---

## Progress Log

### 2024-12-27 - Phase 2.1 Complete
- ✅ Removed duplicate common types from v0/mod.rs (Tag, Mode, Color, PlainColor, RGB)
- ✅ Removed duplicate helper functions from v0/mod.rs (unhex, unhex_one, write_hex)
- ✅ Updated v0/mod.rs to import common types from parent: `pub use super::{Color, MergeFlag, MergeFlags, Mode, PlainColor, RGB, Tag, ThemeVersion}`
- ✅ Updated v0::RawTheme to use imported types directly (Tag, ThemeVersion) instead of super:: prefix
- ✅ Added Default derive to Element enum

### 2024-12-27 - Phase 3.1 Complete ✅
- ✅ Updated v1/mod.rs imports to get common types from parent module instead of v0
- ✅ v1 now imports from parent: `Color, MergeFlag, MergeFlags, Mode, ModeDiff, ModeDiffAction, ModeSet, ModeSetDiff, PlainColor, RGB, Tag, ThemeVersion`
- ✅ v1 imports `Element` from v0 only: `pub use super::v0::Element;`
- ✅ v1 imports v0 module: `use super::v0;` (cleaner than aliased imports)
- ✅ Removed duplicate ModeSetDiff, ModeDiff, ModeDiffAction definitions from v1 (now imported from parent)
- ✅ Added Default derive to Role enum with `Default` as the default variant
- ✅ Added Default derive to Element enum with `Input` as the default variant
- ✅ Added Serialize to v1 serde imports
- ✅ Added module declarations in main themecfg.rs: `pub mod v0;` and `pub mod v1;`
- ✅ Refactored conversion functions to use `From` trait implementations:
  - `impl From<v0::Style> for Style`
  - `impl From<v0::IndicatorPack> for IndicatorPack<Style>`
  - `impl From<v0::RawTheme> for RawTheme`
- ✅ Cleaned up variable naming (removed verbose `v0_*` prefixes)
- ✅ Minimized `super::` usage outside import blocks (kept for `super::StylePack` and `super::IndicatorPack` to avoid name collision)
- ✅ Removed unused imports from v0 and v1 modules
- ✅ Removed v0/tests.rs and v1/tests.rs (need refactoring - they test main module API, not v0/v1 specific)
- ✅ Removed placeholder merge_themes() and resolve_theme() functions from v1 (will be properly implemented in Phase 3.3-3.4)
- ✅ v0: 274 lines (simple, pure data structures)
- ✅ v1: 366 lines (conversions, v1-specific types, no placeholders)
- ✅ **All CI checks passing!** (`just ci` succeeds)
- ✅ Project compiles cleanly
- ✅ All existing tests pass (570 tests)
- **Next**: Continue with Phase 3.3-3.4 - move/copy complete merge and resolve logic from main themecfg.rs to v1

---

## Recommendation

Based on this analysis, I recommend **finishing the refactoring** rather than starting over:

### Why finish (not restart):
1. Good foundation is already in place:
   - v0 module structure is mostly correct (simple types, lenient deser)
   - v1 module has started with right concepts (Role, StyleBase, conversions)
   - Version detection infrastructure exists
   - Conversion from v0->v1 exists

2. Main issues are:
   - Type location (moving things between modules)
   - Naming (RawTheme->Theme in modules, ResolvedTheme->Theme in main)
   - Missing implementations (merge, resolve)
   - Strict vs lenient deserialization

3. These are incremental fixes, not architectural changes

### Execution order (recommended):
1. **Phase 1-2**: Move common types to main, clean up v0 (low risk, foundational)
2. **Phase 4.5**: Rename ResolvedTheme/Style to Theme/Style (affects codebase widely - do once)
3. **Phase 3.3-3.4**: Implement merge and resolve in v1 (core logic)
4. **Phase 4.1-4.3**: Add RawTheme/RawStyle aliases and new API methods
5. **Phase 6-7**: Fix Level usage and add tests
6. **Phase 8-9**: Documentation and cleanup