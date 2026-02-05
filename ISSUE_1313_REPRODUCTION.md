# Issue #1313: Panic with Escaped Strings

## Problem Summary

**NOTE: This analysis may be incomplete. The test created demonstrates a related issue with `logfmt::raw::RawValue`, but the actual panic location uses `json::value::RawValue` which should handle escapes. More information about the actual input data is needed for proper reproduction.**

The application panics with the following error:

```
thread '<unnamed>' panicked at src/formatting.rs:1065:42:
called `Result::unwrap()` on an `Err` value: JsonParseError(Error("invalid type: string \"k:{\\\"uid\\\":\\\"def35946-79ca-4e02-8aeb-ef79db20c081\\\"}\", expected a borrowed string", line: 1, column: 62))
```

## Root Cause

The panic occurs due to an incompatibility between how `serde_json` deserializes strings with escape sequences and how `logfmt::raw::RawValue` expects to receive strings.

### Detailed Explanation

1. **Location**: `src/formatting.rs:1065` - when calling `value.parse().unwrap()` on a `RawObject`

2. **The Issue**: In `src/model.rs:1202`, the `LogfmtRawRecord` deserializer uses:
   ```rust
   ObjectVisitor::<logfmt::raw::RawValue, N>::new(&mut target.0.fields)
   ```

3. **Visitor Implementation**: The `logfmt::raw::RawValue`'s `ReferenceFromString` visitor (in `crate/serde-logfmt/src/logfmt/raw.rs:208-220`) only implements `visit_borrowed_str()`:
   ```rust
   fn visit_borrowed_str<E>(self, s: &'de str) -> Result<Self::Value, E>
   ```

   It does NOT implement `visit_str()`.

4. **The Conflict**: When JSON contains escape sequences like `\"`, `\\`, or `\n`, the `serde_json` deserializer must:
   - Unescape the string (convert `\"` to `"`, `\\` to `\`, etc.)
   - This produces an owned `String`, not a borrowed `&str`
   - Therefore, it calls `visit_str()` instead of `visit_borrowed_str()`

5. **The Panic**: Since `ReferenceFromString` doesn't implement `visit_str()`, serde returns an error: "invalid type: string \"...\", expected a borrowed string"

## Examples of Problematic Input

Any JSON object with string values containing escape sequences will trigger this issue:

```json
{"field": "k:{\"uid\":\"def35946-79ca-4e02-8aeb-ef79db20c081\"}"}
{"data": "some\\escaped\\path"}
{"value": "v:\"projection.genctl.ibm.com/02k7\""}
{"path": "C:\\Users\\name\\file.txt"}
```

## Test Case

A reproduction test has been added to `crate/serde-logfmt/src/logfmt/raw/tests.rs`:

```rust
#[test]
fn test_raw_value_from_json_with_escapes_issue_1313()
```

This test demonstrates the issue by attempting to deserialize JSON with escaped strings into `&RawValue`.

## Running the Test

```bash
cargo test -p serde-logfmt test_raw_value_from_json_with_escapes_issue_1313
```

## Potential Solutions

1. **Add `visit_str()` to ReferenceFromString**: Implement the `visit_str()` method to handle owned strings by converting them to owned `RawValue` (using `to_owned()`).

2. **Use `Box<RawValue>` instead of `&RawValue`**: The `BoxedFromString` visitor already implements `visit_str()`, so it can handle escaped strings.

3. **Use `json::value::RawValue`**: When deserializing JSON specifically, use `json::value::RawValue` (serde_json's RawValue) instead of `logfmt::raw::RawValue`, which is what the regular `Object::from_json()` method does.

4. **Avoid the unwrap()**: Replace the `.unwrap()` at `src/formatting.rs:1065` with proper error handling to prevent the panic and provide a better error message to users.

## Files Modified

- `crate/serde-logfmt/Cargo.toml` - Added `serde_json` as dev-dependency for testing
- `crate/serde-logfmt/src/logfmt/raw/tests.rs` - Added reproduction test
- `crate/serde-logfmt/src/logfmt/de/tests.rs` - Added exploratory tests (later removed as they didn't reproduce the actual issue)

## Related Code Locations

- `src/formatting.rs:1065` - The panic location
- `src/model.rs:1202` - Where `ObjectVisitor<logfmt::raw::RawValue>` is used
- `src/model.rs:230` - `RawObject::parse()` which calls `Object::from_json()`
- `crate/serde-logfmt/src/logfmt/raw.rs:208-220` - `ReferenceFromString` visitor implementation
