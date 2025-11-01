# 🧪 Unit Testing Guidelines

This document provides guidelines for writing unit tests in the HL project, based on established patterns and conventions used throughout the codebase.

---

## 1. Test Organization

### Test File Location (Required)

Tests **MUST** be placed in a separate `tests.rs` file in the same directory as the module being tested. This is required because test code must be excluded from coverage analysis, which cannot be done with inline test modules.

**File structure:**
- Main code: `src/module_name.rs`
- Tests: `src/module_name/tests.rs`

Or for directories:
- Main code: `src/module_name/mod.rs`
- Tests: `src/module_name/tests.rs`

**tests.rs template:**

```rust
use super::*;

#[test]
fn test_something() {
    // test code
}
```

### Why Separate Files?

- ✅ Test code is excluded from coverage analysis
- ✅ Cleaner code organization and easier to navigate
- ✅ Build system can properly exclude test artifacts
- ✅ Consistent with Rust project best practices

---

## 2. Test Naming Conventions

### Naming Pattern

Use the `test_` prefix followed by a descriptive name indicating what is being tested:

**Pattern:** `test_[subject_or_function]_[scenario_or_behavior]`

### Examples

```rust
#[test]
fn test_filter() { }

#[test]
fn test_raw_record_parser_empty_line() { }

#[test]
fn test_relaxed_level_from_conversion() { }

#[test]
fn test_query_and() { }

#[test]
fn test_raw_value_auto() { }
```

Names should be self-documenting. Reading the test name should tell you what behavior is being verified.

---

## 3. Test Structure: Arrange-Act-Assert

Organize all tests into three clear sections:

```rust
#[test]
fn test_relaxed_level_conversion() {
    // Arrange: Set up test data and objects
    let relaxed = RelaxedLevel(Level::Info);
    
    // Act: Execute the function or behavior being tested
    let level: Level = relaxed.into();
    
    // Assert: Verify the results
    assert_eq!(level, Level::Info);
}
```

This structure improves readability and makes it easy to understand:
- What conditions are being set up
- What action is being performed
- What the expected outcome is

---

## 4. Assertions

### Choosing the Right Assertion

| Assertion | Use Case |
|-----------|----------|
| `assert!(condition)` | Boolean conditions or truthy checks |
| `assert_eq!(left, right)` | Comparing two values for equality |
| `assert_ne!(left, right)` | Verifying values are not equal |
| `assert!(result.is_ok())` | Checking Result is Ok variant |
| `assert!(result.is_err())` | Checking Result is Err variant |
| `assert!(option.is_some())` | Checking Option is Some |
| `assert!(option.is_none())` | Checking Option is None |

### Using assert_eq! vs Unwrap

```rust
// Good: Clear assertions about equality
assert_eq!(ab.setting(), IncludeExcludeSetting::Include);
assert_eq!(level, Level::Info);

// Acceptable: Using is_ok/is_err for Results
assert!(result.is_ok());
assert!(result.is_err());

// Good: Unwrap for expected success (test setup)
let record = parse(r#"{"a":1}"#);  // If parsing fails, test fails - that's OK
assert!(record.matches(&query));
```

### Testing Errors with Pattern Matching

When verifying specific error details, use pattern matching:

```rust
#[test]
fn test_invalid_file_handling() {
    let result = Query::parse("v in @src/testing/assets/query/set-invalid").unwrap();
    assert!(result.is_err());
    
    if let Error::FailedToParseJsonLine { line, source } = result.unwrap_err() {
        assert_eq!(line, 2);
        assert!(source.is_eof());
    } else {
        panic!("unexpected error type");
    }
}
```

---

## 5. Test Data and Fixtures

### Inline Test Data

For small, focused tests, define test data inline:

```rust
#[test]
fn test_raw_value_auto() {
    assert_eq!(RawValue::auto("123"), RawValue::Number("123"));
    assert_eq!(RawValue::auto("true"), RawValue::Boolean(true));
    assert_eq!(RawValue::auto("null"), RawValue::Null);
}
```

### Helper Functions

Create helper functions in the `tests.rs` file to reduce duplication and improve readability:

```rust
// In tests.rs file
fn parse(s: &str) -> Record<'_> {
    let settings = ParserSettings::default();
    let mut parser = settings
        .new_parser(crate::format::Auto::default(), s.as_bytes())
        .unwrap();
    parser.next().unwrap().unwrap()
}

#[test]
fn test_query_matches() {
    let query = Query::parse(".a=1").unwrap();
    let record = parse(r#"{"a":1}"#);
    assert!(record.matches(&query));
}
```

### External Test Assets

For larger fixture data, create files in `src/testing/assets/`:

```rust
#[test]
fn test_load_config_from_file() {
    let settings = super::at(["src/testing/assets/configs/issue-288.yaml"])
        .load()
        .unwrap();
    assert_eq!(settings.fields.predefined.level.variants.len(), 1);
}
```

### Using the Sample Trait

The project provides a `Sample` trait for generating test instances:

```rust
/// Trait that provides a method to generate a sample instance.
pub trait Sample {
    /// Returns a sample instance of the implementing type.
    fn sample() -> Self;
}
```

Implement this for types that benefit from a standard test instance.

---

## 6. Common Testing Patterns

### Testing Happy Path

Test normal, expected behavior with valid inputs:

```rust
#[test]
fn test_raw_value_auto() {
    let value = RawValue::auto("123");
    assert_eq!(value, RawValue::Number("123"));

    let value = RawValue::auto("true");
    assert_eq!(value, RawValue::Boolean(true));
}
```

### Testing Edge Cases and Error Conditions

Verify behavior with invalid or boundary inputs:

```rust
#[test]
fn test_raw_record_parser_invalid_type() {
    let parser = RawRecordParser::new().format(Some(InputFormat::Json));
    let mut stream = parser.parse(b"12");
    assert!(matches!(stream.next(), Some(Err(Error::JsonParseError(_)))));
}
```

### Testing Multiple Scenarios with rstest

Use `rstest` for parameterized tests with multiple input/output combinations. This provides better isolation and clearer failure messages:

```rust
// In src/query/tests.rs
use super::*;
use rstest::rstest;

#[rstest]
#[case("mod=test", "test")]
#[case(r#"mod="test""#, "test")]
fn test_query_json_str_simple(#[case] query_str: &str, #[case] expected_value: &str) {
    let query = Query::parse(query_str).unwrap();
    let record = parse(format!(r#"{{"mod":"{}"}}"#, expected_value).as_str());
    assert!(record.matches(&query));
    
    let record = parse(r#"{"mod":"test2"}"#);
    assert!(!record.matches(&query));
}
```

**Benefits of `rstest` over loops:**
- ✅ Each case runs as a separate test (better test isolation)
- ✅ Failure in one case doesn't skip others
- ✅ Clear output showing which case failed
- ✅ More idiomatic Rust testing style

### Testing Hierarchical/Nested Structures

Test both direct and nested access:

```rust
#[test]
fn test_filter_nested_access() {
    let mut filter = IncludeExcludeKeyFilter::new(MatchOptions::default());
    filter.entry("a.b").include();

    // Test nested access through parent
    let a = filter.get("a").unwrap();
    let ab = a.get("b").unwrap();
    assert_eq!(ab.setting(), IncludeExcludeSetting::Include);

    // Test direct access
    let ab = filter.get("a.b").unwrap();
    assert_eq!(ab.setting(), IncludeExcludeSetting::Include);
}
```

### Testing Operator Overloading

Verify operators work the same as underlying methods:

```rust
#[test]
fn test_query_bitwise_operators() {
    let q1 = Query::parse(".a=1").unwrap();
    let q2 = Query::parse(".b=2").unwrap();

    // Test both operator and method form
    let and_query1 = q1.clone() & q2.clone();
    let and_query2 = q1.clone().and(q2.clone());

    let record = parse(r#"{"a":1,"b":2}"#);
    assert!(record.matches(&and_query1));
    assert!(record.matches(&and_query2));
}
```

### Testing Trait Implementations

```rust
#[test]
fn test_relaxed_level_from_conversion() {
    let relaxed = RelaxedLevel(Level::Info);
    let level: Level = relaxed.into();
    assert_eq!(level, Level::Info);

    let level: Level = Level::from(RelaxedLevel(Level::Error));
    assert_eq!(level, Level::Error);
}

#[test]
fn test_relaxed_level_deref() {
    let relaxed = RelaxedLevel(Level::Warning);
    assert_eq!(*relaxed, Level::Warning);
    assert_eq!(relaxed.deref(), &Level::Warning);
}
```

---

## 7. Macros for Common Patterns

### Simple Test Macros

For repetitive patterns, create concise macros in your `tests.rs` file:

```rust
// In src/lexer/tests.rs
use super::*;

macro_rules! next {
    ($expression:expr) => {
        (&mut $expression).next().unwrap().unwrap()
    };
}

#[test]
fn test_lexer_tokens() {
    let input = br#"{"a":1}"#;
    let mut lexer = Lexer::from_slice(input);
    assert_eq!(next!(lexer), EntryBegin);
    assert_eq!(next!(lexer), CompositeBegin(Field(...)));
}
```

### Parameterized Tests with rstest

Use `rstest` for multiple input/output combinations in your `tests.rs` file:

```rust
// In src/timezone/tests.rs
use super::*;
use rstest::rstest;

#[rstest]
#[case("+03:00", Some(b'+'), Some(3))]
#[case("-05:00", Some(b'-'), Some(5))]
#[case("Z", None, None)]
fn test_timezone_parsing(
    #[case] input: &str,
    #[case] expected_sign: Option<u8>,
    #[case] expected_hour: Option<u32>,
) {
    let tz = Timezone::parse(input).unwrap();
    assert_eq!(tz.sign(), expected_sign);
    assert_eq!(tz.hour().map(|x| x.value()), expected_hour);
}
```

---

## 8. What to Test

### ✅ Test These

- **Happy path**: Normal, expected behavior
- **Edge cases**: Boundary values, empty inputs, extreme values
- **Error conditions**: Invalid inputs, parsing failures, resource errors
- **State changes**: Verify side effects and state modifications
- **Interactions**: How multiple components work together
- **Type conversions**: From/Into trait implementations
- **Trait methods**: All public trait implementations

### ❌ Don't Test These

- Standard library behavior (e.g., `Vec::push()`)
- Third-party crate behavior (trust external dependencies)
- Trivial getters/setters without logic
- Implementation details (test behavior instead)
- Private functions (test through public API)

---

## 9. Test Independence and Best Practices

### Independence

- Each test should be independent and runnable in any order
- Tests should not share mutable state
- Each test should set up its own context
- Avoid test interdependencies

### Clarity and Maintainability

- **Use descriptive variable names** instead of `x`, `y`, `z`
- **Extract common setup** into helper functions
- **Add comments** only for non-obvious test logic
- **Keep tests focused** - one logical behavior per test
- **Group related tests** together in the same module

### Good Practices

```rust
#[test]
fn test_filter_includes_nested_path() {
    // Clear variable names make intent obvious
    let mut filter = IncludeExcludeKeyFilter::new(MatchOptions::default());
    filter.entry("parent").exclude();
    filter.entry("parent.child").include();

    let child = filter.get("parent.child").unwrap();
    assert_eq!(child.setting(), IncludeExcludeSetting::Include);
}
```

---

## 10. Running Tests

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p crate_name

# Run a specific test by name
cargo test test_filter_

# Run tests with output
cargo test -- --nocapture

# Run tests single-threaded
cargo test -- --test-threads=1
```

---

## 11. Checklist for New Tests

Before committing a test, verify:

- ✅ Test is named with `test_` prefix and descriptive name
- ✅ Test uses arrange-act-assert pattern
- ✅ Test data is clearly defined and readable
- ✅ Assertions are focused and specific
- ✅ Error cases are tested alongside happy paths
- ✅ Test is independent and can run in any order
- ✅ No testing of standard library or third-party behavior
- ✅ Helper functions are used to reduce duplication
- ✅ Tests are in a separate `tests.rs` file (not inline modules)
- ✅ Comments explain non-obvious test setup or assertions
- ✅ Test compiles without warnings
- ✅ Test passes consistently

---

## 12. Complete Example

Here's a complete `tests.rs` file demonstrating these principles:

**File: `src/query/tests.rs`**

```rust
use super::*;

/// Helper function to create a test query
fn create_query(filter_str: &str) -> Query {
    Query::parse(filter_str).expect("valid filter")
}

/// Helper function to parse test JSON
fn parse_record(json: &str) -> Record<'_> {
    let settings = ParserSettings::default();
    let mut parser = settings
        .new_parser(crate::format::Auto::default(), json.as_bytes())
        .expect("valid parser");
    parser.next().expect("record").expect("parsing")
}

#[test]
fn test_query_matches_simple_field() {
    // Arrange
    let query = create_query(".a=1");
    let record = parse_record(r#"{"a":1}"#);

    // Act & Assert
    assert!(record.matches(&query));
}

#[test]
fn test_query_does_not_match_different_value() {
    // Arrange
    let query = create_query(".a=1");
    let record = parse_record(r#"{"a":2}"#);

    // Act & Assert
    assert!(!record.matches(&query));
}

#[test]
fn test_query_and_requires_both_conditions() {
    // Arrange
    let query = create_query(".a=1 and .b=2");

    // Act & Assert: Both match
    assert!(parse_record(r#"{"a":1,"b":2}"#).matches(&query));

    // Only first matches
    assert!(!parse_record(r#"{"a":1,"b":3}"#).matches(&query));

    // Only second matches
    assert!(!parse_record(r#"{"a":2,"b":2}"#).matches(&query));
}

#[test]
fn test_query_or_requires_at_least_one_condition() {
    // Arrange
    let query = create_query(".a=1 or .b=2");

    // Act & Assert: Both match
    assert!(parse_record(r#"{"a":1,"b":2}"#).matches(&query));

    // Only first matches
    assert!(parse_record(r#"{"a":1,"b":3}"#).matches(&query));

    // Only second matches
    assert!(parse_record(r#"{"a":2,"b":2}"#).matches(&query));

    // Neither matches
    assert!(!parse_record(r#"{"a":2,"b":3}"#).matches(&query));
}
```

---

## 13. References

- [Rust Book: Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Rust API Guidelines: Examples](https://rust-lang.github.io/api-guidelines/documentation.html#examples-use-///-doc-comments-u-ex)
- Project codebase examples in `src/*/tests.rs` files