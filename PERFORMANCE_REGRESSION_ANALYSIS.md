# Performance Regression Analysis - Expansion Feature Branch

## Summary

The expansion feature branch introduced a **~20% performance regression** in the benchmark `ws:hl:combined/parse-and-format/json:1627:p7iflbdc6az3i` (inline mode with simple integer fields).

**Root causes identified:**
1. **Unnecessary complexity calculation** (fixed - improves by ~3.6%)  
2. **Transactional wrapping overhead** (not yet fixed - accounts for remaining ~13-14% regression)

## Benchmark Details

- **Benchmark:** `ws:hl:combined/parse-and-format/json:1627:p7iflbdc6az3i`
- **Mode:** `ExpansionMode::Inline`
- **Input:** `{"a":1745349129016,"b":1745349149419,"c":1745349176629,"d":1745349181278,"e":1745349186212}`
- **Characteristics:** 5 integer fields, no strings, no expansion expected

## Performance Results

| Version | Time (µs) | Throughput (MiB/s) | vs Master |
|---------|-----------|-------------------|-----------|
| Master  | 4.59      | 338              | baseline  |
| Branch (before fix) | 5.55 | 280 | +20.9% |
| Branch (after fix) | 5.35 | 289 | +16.6% |

## Issue #1: Unnecessary Complexity Calculation (FIXED)

### Problem

In `ValueFormatAuto::format()`, the code was calling `.analyze()` which calculates both character mask AND complexity:

```rust
let analysis = buf[begin..].analyze();
let mask = analysis.chars;  // only this is used
```

The `analyze()` method:
```rust
fn analyze(&self) -> Analysis {
    let mut chars = Mask::empty();
    let mut complexity = 0;  // ← never used in ValueFormatAuto
    self.iter()
        .map(|&c| (CHAR_GROUPS[c as usize], COMPLEXITY[c as usize]))
        .for_each(|(group, cc)| {
            chars |= group;
            complexity += cc;  // ← wasted work
        });
    Analysis { chars, complexity }
}
```

### Fix

Replace with mask-only calculation:

```rust
let mut mask = Mask::empty();
buf[begin..].iter().map(|&c| CHAR_GROUPS[c as usize]).for_each(|group| {
    mask |= group;
});
```

### Impact

- **Improvement:** ~3.6% (5.55 µs → 5.35 µs)
- **Status:** ✅ Fixed in current commit

## Issue #2: Transactional Wrapping Overhead (NOT YET FIXED)

### Problem

Every field formatting call is now wrapped in `fs.transact(...)`:

```rust
let result = fs.transact(s, |fs, s| {
    match self.format_field(s, k, *v, fs, ...) {
        // ...
    }
});
```

This transactional wrapper:
1. Saves 4 fields from `FormattingStateWithRec` (dirty, depth, first_line_used, ts_width)
2. Calls `s.transact()` which saves 3 more fields from `Styler` (current, synced, buffer length)
3. Restores all 7 fields on error

**In master:** No transactional wrapping existed at all.

**For inline mode:** All transactions succeed, so we pay the cost of saving state that we never restore.

### Performance Impact

- **Estimated overhead:** ~13-14% of total regression
- **Reason:** Saving/restoring 7 values per field, even when unnecessary

### Struct Size Increase

`FormattingState` grew significantly:

| Version | Fields | Approximate Size |
|---------|--------|-----------------|
| Master | 5 | ~40-80 bytes |
| Branch | 19 | ~800-1200 bytes (includes 2 heapless::Vec) |

While the heapless::Vecs start empty, the increased struct size may affect:
- Cache locality
- Passing by reference overhead
- Memory bandwidth

### Potential Fixes

1. **Short-term:** Conditionally skip transact when `expansion.expand_all == false && multiline == Inline`
2. **Medium-term:** Lazy transaction - only save state when actually needed
3. **Long-term:** Restructure to avoid transaction overhead in common case

## Recommendations

1. ✅ **DONE:** Remove unnecessary complexity calculation
2. ⚠️ **TODO:** Optimize or eliminate transactional wrapping for inline mode
3. **Consider:** Profile-guided optimization to identify other hot paths
4. **Consider:** Benchmark other expansion modes to ensure they didn't regress

## Notes

- The complexity calculation was originally added to support expansion heuristics
- Those heuristics were removed but the complexity calculation remained
- The transactional system was added to support expansion rollback
- For modes that never expand, this overhead is pure waste

