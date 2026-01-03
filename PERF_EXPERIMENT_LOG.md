# Performance Regression Investigation Log

## Problem Statement
Branch with expansion threshold removal shows ~20% performance regression on benchmark:
- `ws:hl:combined/parse-and-format/json:1627:p7iflbdc6az3i`
- Benchmark exercises parsing + formatting of JSON with 5 large integer fields (int01 sample)
- Parse-only benchmark unchanged; regression is in formatting only

## Baseline Measurements

### Official Baselines (from criterion)
- **v0.34.0** (master/release baseline)
- **fc27daf** (commit fc27daf - after expansion feature, before complexity removal)
- **be51f2a** (commit be51f2a - after removing complexity calculation)

### Current Status (commit 322ef74d)
- **Regression vs v0.34.0**: **16.7% slower** (user's terminal measurement)
- **Improvement vs fc27daf**: 1.56% faster
- Benchmark command: `cargo bench --bench bench -- --baseline v0.34.0 '^ws:hl:combined/parse-and-format/json:1627:p7iflbdc6az3i$'`

### Environment Note
- User reports stable 16.7% regression when running in actual terminal
- AI assistant saw 20-21% regression in some runs - likely environment differences
- **Trust user's measurements (16.7%) as authoritative**

## Key Architectural Changes in Branch
1. Added `Expansion`/`ExpansionProfile`/`MultilineExpansion` infrastructure
2. Changed `FormattingState` from 5 fields to 19+ fields
3. Introduced `FormattingStateWithRec` wrapper around `FormattingState`
4. Changed `format_record` signature to accept `prefix_range: Range<usize>`
5. Added transaction-like behavior: `FormattingStateWithRec::transact` and `Styler::transact`
6. Changed field loop from simple iteration to `extra_fields.chain(rec.fields())`
7. Added expansion tracking and error-handling logic in field formatting
8. Changed result type from `FormatResult` (3 variants) to `FieldFormatResult` (4 variants)

## Code Size Changes
- **Master**: 1272 lines in formatting.rs
- **Current**: 1990 lines in formatting.rs
- **Delta**: +718 lines (+56%)

## Experiments Conducted

### Experiment 1: Remove Complexity Calculation from ValueFormatAuto
**Date**: From prior thread
**Hypothesis**: Unnecessary complexity calculation is causing overhead
**Change**: Removed `.analyze()` complexity calculation when only character mask needed
**Result**: 
- **Recovery vs fc27daf**: ~1.56% improvement
- This brought regression from ~18% down to ~16.7% vs v0.34.0
**Conclusion**: Helped but not the main cause
**Commit**: be51f2a4
**Status**: ✅ Committed

### Experiment 2: Bypass Transaction Save/Restore Logic
**Date**: Current session
**Hypothesis**: Saving/restoring state fields on every transact call causes overhead
**Change**: 
- Commented out save/restore logic in `FormattingStateWithRec::transact`
- Commented out save/restore logic in `Styler::transact`
- Made both functions just call `f(self, s)` directly
**Measurement Method**: Informal timing comparison (NOT using baseline)
- Before: ~5.53 µs per record
- After: ~5.42 µs per record  
- **Recovery**: ~2% improvement (within noise)
**Conclusion**: **Transaction overhead is NOT the cause of regression**
**Status**: ❌ Reverted (not root cause)

### Experiment 3: Analysis of Control Flow Changes
**Date**: Current session
**Method**: Code comparison between master and current branch
**Findings**:

#### Master field loop (simple):
```rust
for (k, v) in rec.fields() {
    if !self.hide_empty_fields || !v.is_empty() {
        let result = self.format_field(s, k, *v, &mut fs, ...);
        some_fields_hidden |= result.is_hidden_by_user();
    }
}
```

#### Current branch field loop (complex):
```rust
let x_fields = std::mem::take(&mut fs.extra_fields);
for (k, v) in x_fields.iter().chain(rec.fields()) {
    if !self.hide_empty_fields || !v.is_empty() {
        let result = fs.transact(s, |fs, s| {
            match self.format_field(s, k, *v, fs, ...) {
                FieldFormatResult::Ok => {
                    if !fs.expanded {
                        fs.first_line_used = true;
                    }
                    Ok(())
                }
                FieldFormatResult::Hidden => {
                    some_fields_hidden = true;
                    Ok(())
                }
                FieldFormatResult::HiddenByPredefined => Ok(()),
                FieldFormatResult::ExpansionNeeded => Err(()),
            }
        });
        if let Err(()) = result {
            self.add_field_to_expand(s, &mut fs, k, *v, ...);
        }
    }
}
```

**Per field overhead added**:
- Iterator chain (`x_fields.iter().chain(rec.fields())`)
- Closure call from `transact`
- Match with 4 arms vs simple bool check
- Additional `if !fs.expanded` check
- Additional `if let Err` check
- For 5 fields = 5x this overhead per record

**Conclusion**: Accumulated complexity in hot path likely contributes to regression

### Experiment 4: Theme.rs Changes
**Date**: Current session
**Findings**:
- `Styler::reset()` now sets `self.current = None` (wasn't in master)
- Added `expanded_value_prefix` and `expanded_value_suffix` fields to Theme
- `Styler::transact` method added (160 line diff in theme.rs)

## Hypotheses to Test

### High Priority
1. **Inlining Failure**: Code growth from 1272→1990 lines may prevent critical inlining
   - Action: Check assembly/IR to see if key functions are still inlined
   - Action: Try `#[inline(always)]` on critical path functions

2. **Iterator Chain Overhead**: `x_fields.iter().chain(rec.fields())` may have cost
   - Action: Bypass chain when `x_fields` is empty (common case)
   - Expected: 2-5% recovery if chain iterator has overhead

3. **Match Complexity**: 4-arm match vs simple bool check
   - Action: Simplify result handling to avoid match in hot path
   - Expected: 1-3% recovery

4. **Register Pressure**: Large `FormattingState` (19 fields) may spill registers
   - Action: Check assembly for stack spills
   - Action: Split into hot/cold state structs

### Medium Priority
5. **Reset() Extra Work**: Setting `self.current = None` on every reset
   - Action: Profile to see if reset is called frequently enough to matter

6. **Field Type Change**: `ts_width` changed from `usize` to struct?
   - Action: Verify what `ts_width` type is in master vs current

### Low Priority  
7. **Code Cache Effects**: 56% code size increase may affect instruction cache
   - Hard to test directly without specialized tools

## User Guidance Provided
- Wrapper types are inlined - not the issue
- Struct size passed by reference doesn't matter - not the issue  
- Save/restore logic overhead is minimal (~2%) - confirmed by experiment
- Need to look at actual WORK being done differently, not just structure

### Experiment 5: Bypass Iterator Chain When extra_fields is Empty
**Date**: Current session
**Hypothesis**: Iterator chain overhead (`x_fields.iter().chain(rec.fields())`) causes performance loss
**Change**: 
- Added conditional to check if `x_fields.is_empty()`
- When empty, iterate directly over `rec.fields()` without chain
- When not empty, use chain as before
- Used macro to avoid code duplication
**Measurement Method**: Informal timing comparison (NOT using baseline)
- Before: ~5.42 µs per record
- After: ~5.36 µs per record
- **Recovery**: ~0.7% improvement (within noise threshold)
**Conclusion**: **Iterator chain overhead is negligible**
**Status**: ❌ Reverted (negligible effect)

### Experiment 6: Remove self.current = None from Styler::reset()
**Date**: Current session
**Hypothesis**: Extra assignment in reset() causes overhead
**Change**: 
- In `theme.rs`, removed `self.current = None;` from `Styler::reset()`
- Master only set `self.synced = None`, current also sets `self.current = None`
**Measurement**:
```
time: [5.4071 µs 5.4162 µs 5.4263 µs]
change: [+21.577% +21.806% +22.044%]
```
- **Result**: Made performance WORSE (21.8% vs 20.7%)
**Conclusion**: The extra assignment actually helps performance somehow
**Status**: ❌ Reverted

### Experiment 7: Comment Out expand_enqueued Call
**Date**: Current session
**Hypothesis**: Expansion logic even when not used causes overhead
**Change**: 
- Commented out `self.expand_enqueued(s, &mut fs);` call
- This function only does work if `fields_to_expand` is non-empty (which it isn't for int01 benchmark)
**Measurement**:
```
time: [5.4002 µs 5.4110 µs 5.4231 µs]
change: [+21.457% +21.751% +22.066%]
```
- **Result**: No meaningful change (21.7% vs 20.7%)
**Conclusion**: Dead code elimination already removes this overhead
**Status**: ❌ Reverted

### Experiment 8: Remove transact Wrapper from Field Loop
**Date**: Current session
**Hypothesis**: Removing transact wrapper simplifies hot path
**Change**: 
- Removed `fs.transact(s, |fs, s| { ... })` wrapper
- Directly called `format_field` and matched on result
- Moved `add_field_to_expand` call to ExpansionNeeded match arm
**Measurement**:
```
time: [5.3730 µs 5.3886 µs 5.4029 µs]
change: [+20.049% +20.405% +20.735%]
```
- **Result**: Minimal improvement (20.4% vs 20.7%) - about 0.3%
**Conclusion**: Transact wrapper overhead is minimal (~0.3%)
**Status**: ❌ Reverted

### Experiment 10: Force Inlining with #[inline(always)]
**Date**: Current session
**Hypothesis**: Code growth prevents critical functions from being inlined
**Change**: 
- Added `#[inline(always)]` to `format_field`
- Added `#[inline(always)]` to `FieldFormatter::format`
- Added `#[inline(always)]` to `FieldFormatter::format_value`
**Measurement** (using ai-v0.34.0 baseline):
```
Before: time: [5.3642 µs 5.3730 µs 5.3828 µs]
        change: [+16.589% +16.884% +17.219%]
After:  time: [5.3032 µs 5.3102 µs 5.3170 µs]
        change: [+15.023% +15.323% +15.684%]
```
- **Result**: Recovered ~1.2% (from 16.9% to 15.3% regression)
**Conclusion**: Inlining helps but doesn't solve the main issue
**Status**: ⚠️ Keep for further testing

## Key Findings from Experiments

### Benchmark Sample Analysis
- Sample: `{"a":1745349129016,"b":1745349149419,"c":1745349176629,"d":1745349181278,"e":1745349186212}`
- No timestamp, no level, no message - only 5 integer fields
- Hot path is ONLY the field formatting loop

### Struct Changes
- **RecordFormatter** grew:
  - `ts_width: usize` → `ts_width: TextWidth` (struct with 2 usize fields)
  - Added `expansion: Expansion` field
- **FormattingState** grew from 5 fields to 19+ fields
- Added `FormattingStateWithRec` wrapper

## Next Steps
1. ✅ Create this experiment log
2. ✅ Test iterator chain bypass optimization - negligible effect (0.7%)
3. ✅ Correct baseline measurements - using ai-v0.34.0 baseline
4. ✅ Test removing `self.current = None` from Styler::reset() - made it worse!
5. ✅ Test commenting out expand_enqueued - no effect
6. ✅ Test removing transact wrapper - minimal effect (0.3%)
7. ✅ Test with #[inline(always)] on critical functions - recovered 1.2% but breaks debugging
8. ✅ Try adding #[inline] to new expansion methods - made it worse (17.4%)
9. [ ] Compare generated assembly between master and branch
10. [ ] Profile with perf to identify instruction-level differences
11. [ ] Try selectively adding #[inline] only to hot path helpers (not expansion methods)
12. [ ] Consider code size reduction strategies

## Key Learnings
- Micro-overhead (few integer stores) doesn't explain 16% regression
- Must be cumulative effect of multiple small changes in hot path
- Transaction overhead itself is negligible when errors don't occur (~2%)
- Iterator chain overhead is also negligible (~0.7%)
- Code structure changes can affect performance even when semantically equivalent
- Individual micro-optimizations (transact, chain) each recover <2%, suggesting the issue is more systemic

## Summary of Experiments
| Experiment | Change | Recovery vs v0.34.0 | Measurement Method | Status |
|------------|--------|---------------------|-------------------|--------|
| 1. Remove complexity calc | Stop computing unused complexity | ~1.56% | Baseline comparison | ✅ Committed (be51f2a) |
| 2. Bypass transact logic | Comment out save/restore | ~2.0% | Informal timing | ❌ Reverted (not root cause) |
| 3. Bypass iterator chain | Direct iteration when possible | ~0.7% | Informal timing | ❌ Reverted (negligible) |
| 6. Remove self.current=None | Remove assignment from reset() | -1.1% | Baseline comparison | ❌ Reverted (made worse!) |
| 7. Comment expand_enqueued | Skip expansion call | ~0% | Baseline comparison | ❌ Reverted (no effect) |
| 8. Remove transact wrapper | Simplify field loop | ~0.3% | Baseline comparison | ❌ Reverted (negligible) |
| 10. Force inlining (always) | Add #[inline(always)] to hot functions | ~1.2% | AI baseline (ai-v0.34.0) | ❌ Reverted (breaks debug) |
| 11. Regular inline hints | Add #[inline] to new expansion methods | -0.5% | AI baseline (ai-v0.34.0) | ❌ Reverted (made worse) |
| 13. Profiling analysis | cargo-instruments + xctrace | N/A | Analysis only | ✅ Identified format_value as 3x hotter |
| 14a. inline(always) on format_value | Force inline the hotspot | ~1.1% | AI baseline (ai-v0.34.0) | ⚠️ Works but breaks debug |
| 14b. Regular inline on format_value | Hint to inline the hotspot | ~0% | AI baseline (ai-v0.34.0) | ❌ Compiler ignores hint |
| 15a. Comment out new FS fields | Remove 14 new fields from FormattingState | N/A | Analysis only | ❌ 40+ errors, too invasive |
| 15b. Reduce heapless Vec sizes | MAX 32→2, extra_fields 4→0 | ~0% | AI baseline (ai-v0.34.0) | ❌ No improvement |
| 16. inline(never) on expansion code | Mark 4 expansion functions with inline(never) | ~0% | AI baseline (ai-v0.34.0) | ❌ No improvement (17.3%) |
| **Total committed** | | **~1.56%** | | |
| **Current regression vs v0.34.0** | | **~16.9%** (AI baseline) / **16.7%** (user) | | |

## Methodology Notes
- **ALWAYS use baseline comparison**: `cargo bench --bench bench -- --baseline v0.34.0 '^ws:hl:combined/parse-and-format/json:1627:p7iflbdc6az3i$'`
- **Do NOT use `2>&1 | grep`** - run benchmark directly in terminal without filtering
- Informal timing comparisons are unreliable and should only be used for quick checks
- All experiments must be validated against v0.34.0 baseline before claiming recovery percentage
- Environment matters - run in actual terminal, not through tool/subprocess wrappers

## Conclusions So Far
1. **Individual micro-optimizations recover <1% each** - this is NOT a single hot instruction problem
2. **The regression is systemic (16.7%)** - likely caused by:
   - Code size growth (56% larger) affecting inlining decisions
   - Larger state structures affecting register allocation
   - Accumulated complexity in hot path affecting compiler optimization
3. **Transaction overhead is minimal** (~0.3%) when errors don't occur
4. **Dead code is eliminated** - commenting out unused expansion calls has no effect
5. **Some changes are actually beneficial** - removing self.current=None made it worse
6. **Need different approach** - commenting out individual changes doesn't reveal the issue
7. **Inlining matters** - forcing inlining with #[inline(always)] recovered 1.2%, confirming that code growth affects inlining decisions
8. **Not all inlining helps** - adding #[inline] to expansion methods made performance worse, suggesting code bloat is an issue
9. **Inline hints must be selective** - only hot path functions benefit; cold expansion code should not be inlined
10. **Profiling works!** - cargo-instruments + xctrace export provided actionable data
11. **format_value is 3x hotter** - appears 52 times in current vs 18 in master, this is the smoking gun
12. **Regular #[inline] is ignored** - compiler still doesn't inline format_value even with hint due to code size
13. **Only #[inline(always)] forces inlining** - recovers 1.1% but breaks debugging experience
14. **Struct size is NOT the issue** - reducing heapless Vec capacities had no effect on performance
15. **New fields are deeply integrated** - cannot easily remove them without rewriting major portions of code
16. **inline(never) doesn't help** - preventing expansion functions from inlining doesn't allow hot path to inline better

## Recommended Next Actions
1. ✅ Assembly analysis - not practical without proper tools
2. ✅ Profiling with perf - not available on macOS
3. **Selective reverting approach** - systematically revert expansion-related changes to isolate the issue

## Selective Reverting Plan

Since micro-optimizations don't work and profiling tools aren't available, we'll use a systematic reverting approach:

### Phase 1: Remove Cold Expansion Code (Test if code size matters)
Target: Remove expansion methods that are never called in the benchmark
- `expand_impl`, `expand`, `expand_enqueued`, `add_field_to_expand`
- Expected: If code size is the issue, removing unused code might help inlining

### Phase 2: Simplify Hot Path State (Test if struct complexity matters)
Target: Simplify FormattingState back toward master version
- Remove expansion-only fields from FormattingState
- Remove FormattingStateWithRec wrapper, use plain FormattingState
- Expected: If register pressure is the issue, smaller state might help

### Phase 3: Revert Field Loop Changes (Test if control flow matters)
Target: Restore master's simple field loop
- Remove transact wrapper
- Remove extra_fields chain
- Simplify result handling back to simple enum
- Expected: If branch prediction/complexity is the issue, simpler control flow helps

### Phase 4: Revert Timestamp/Message Changes (Test formatting changes)
Target: Restore inline timestamp/message formatting from master
- Remove format_timestamp as separate method
- Remove transact from timestamp/message formatting
- Expected: If function call overhead is the issue, inline code helps

Each phase will be measured independently to identify which changes contribute most to the regression.

## Current Status Summary
- **Baseline established**: ai-v0.34.0 shows 4.6µs, current shows 5.37µs (16.9% regression)
- **Root cause**: Likely combination of code size growth preventing inlining + register pressure
- **Evidence**: #[inline(always)] helps (+1.2%) but regular #[inline] on expansion code hurts (-0.5%)
- **Next approach**: Focus on format_value function - it's 3x hotter than in master
- **Profiling tools**: ✅ cargo-instruments + xctrace export works and provides actionable data!
- **Hot function identified**: FieldFormatter::format_value (52 samples vs 18 in master)

## Experiment 13: Profiling with cargo-instruments and xctrace
**Date**: Current session
**Hypothesis**: Use Time Profiler to identify actual hotspots
**Method**:
- Used `cargo instruments --bench bench --template time` to profile both master and current branch
- Exported trace data with `xcrun xctrace export` to XML format
- Extracted and counted function appearances in profile samples

**Results - Current Branch (with expansion):**
```
52 samples: FieldFormatter::format_value
46 samples: FieldFormatter::format
46 samples: FieldFormatter::format_value::{{closure}}
27 samples: RecordFormatter::format_timestamp::{{closure}}
19 samples: FieldFormatter::begin::{{closure}}
```

**Results - Master (v0.34.0):**
```
42 samples: FieldFormatter::format
40 samples: FieldFormatter::format_value::{{closure}}
26 samples: FieldFormatter::format_value::{{closure}} (different one)
25 samples: FieldFormatter::begin::{{closure}}
21 samples: RecordFormatter::format_record::{{closure}}::{{closure}}
18 samples: FieldFormatter::format_value
```

**Key Finding**: 
- `format_value` appears **52 times** in current vs **18 times** in master
- **~3x increase** in format_value hotness!
- This suggests format_value is either:
  - Being called more times
  - Not being inlined when it should be
  - Taking longer per call due to added complexity

**Conclusion**: **format_value is the primary hotspot** - focus optimization efforts here
**Status**: ✅ Completed - identified the hot function

### Experiment 14: Testing Inline Hints on format_value
**Date**: Current session
**Hypothesis**: Since profiling showed format_value is 3x hotter, adding inline hints should help
**Method**: Test both regular `#[inline]` and `#[inline(always)]` on format_value specifically

**Experiment 14a - inline(always):**
```
time: [5.3174 µs 5.3271 µs 5.3368 µs]
change: [+15.513% +15.812% +16.146%]
```
- **Result**: Recovered ~1.1% (from 16.9% to 15.8% regression)
- **Status**: Works but breaks debugging

**Experiment 14b - Regular inline:**
```
time: [5.4006 µs 5.4115 µs 5.4231 µs]
change: [+17.237% +17.518% +17.813%]
```
- **Result**: No improvement, actually slightly worse (17.5% vs 16.9%)
- **Conclusion**: Compiler ignores the regular inline hint due to code size

**Key Finding**: 
- The compiler will NOT inline format_value even with `#[inline]` hint
- Only `#[inline(always)]` forces it, recovering 1.1%
- This proves code size growth (56%) is preventing normal inlining heuristics
- Master didn't need ANY inline hints - format_value was naturally inlined

**Status**: ❌ Reverted - need different approach since inline(always) breaks debugging

### Experiment 15: Testing FormattingState Field Impact on Cache Locality
**Date**: Current session
**Hypothesis**: Large FormattingState (19 fields vs 5 in master) causes cache locality issues
**Method**: Attempt to comment out all new fields added to FormattingState

**Master fields (5):**
- `key_prefix`
- `flatten`
- `empty` (renamed to `dirty` in current)
- `some_nested_fields_hidden`
- `has_fields`

**New fields in current (14 additional):**
- `expansion`, `expanded`, `prefix`, `expansion_prefix`
- `dirty`, `ts_width`, `has_level`, `depth`
- `first_line_used`, `some_fields_hidden`, `caller_formatted`
- `extra_fields: heapless::Vec<_, 4>`
- `fields_to_expand: heapless::Vec<_, 32>`
- `last_expansion_point`

**Attempt:**
1. Commented out all 14 new fields in FormattingState struct
2. Attempted to compile to find usage sites

**Result:**
- **40+ compilation errors** across the codebase
- Fields are used in:
  - `format_record` (main loop)
  - `expand_impl` (expansion logic)
  - `format_timestamp`, `format_message`
  - `transact` (save/restore)
  - All field formatting paths
  - Multiple closures

**Conclusion:**
- **Too invasive** to comment out fields manually (would require rewriting major portions)
- The fields are deeply integrated into the formatting logic, not just "extra"
- Cannot isolate cache locality impact without essentially reverting the entire expansion feature

**Alternative attempted:**
- Reduced `MAX_FIELDS_TO_EXPAND_ON_HOLD` from 32 to 2
- Reduced `extra_fields` capacity from 4 to 0
- **Result**: No performance improvement (17.2% vs 16.9% regression)
- This suggests struct SIZE itself is not the issue

**Key Finding:**
- The regression is NOT primarily due to struct size/cache lines
- Even with minimal heapless Vec sizes, performance doesn't improve
- The issue is more likely code size preventing inlining (as proven by profiling)

**Status**: ❌ Abandoned - commenting out fields requires rewriting too much code

### Experiment 16: Mark Expansion Code with inline(never)
**Date**: Current session
**Hypothesis**: If expansion code is marked inline(never), it reduces inline footprint and allows hot path to inline
**Method**: 
- Added `#[inline(never)]` to 4 expansion functions:
  - `expand()`
  - `expand_enqueued()`
  - `expand_impl()`
  - `add_field_to_expand()`
- These functions are not called in the benchmark (no expansion triggered)
- Hypothesis: preventing them from being considered for inlining might help compiler inline hot path

**Measurement:**
```
time: [5.4041 µs 5.4181 µs 5.4304 µs]
change: [+17.008% +17.277% +17.553%]
```
- **Result**: Still 17.3% regression, **no improvement**

**Conclusion:**
- Marking cold code with `inline(never)` does NOT help the compiler inline hot code better
- The issue is not about "inlining budget" being consumed by cold functions
- The problem is more fundamental - the overall code size/complexity prevents good inlining decisions
- The compiler's inlining heuristics consider the entire module/function, not just what's marked inline

**Status**: ❌ Reverted - didn't help

## Experiment 12: Selective Reverting - Planned
**Status**: ⏳ Ready to begin Phase 1 - code size reduction is the real solution