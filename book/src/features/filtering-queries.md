# Complex Queries

Complex queries allow you to build sophisticated filtering expressions using logical operators, comparisons, and pattern matching. This is hl's most powerful filtering feature.

## Basic Query Syntax

Use the `-q` or `--query` option to specify a query:

```sh
hl -q 'level > info' application.log
```

Queries are expressions that evaluate to true or false for each log entry. Only entries where the query evaluates to true are displayed.

## Field References

### Predefined Fields

These special field names reference standard log fields regardless of the source field names:

- `level` - Log level (supports semantic level comparisons)
- `message` - Log message
- `caller` - Caller information
- `logger` - Logger name

Example:

```sh
hl -q 'level = error' application.log
```

**Important:** Predefined fields like `level` perform semantic comparisons. For example, `level > info` correctly compares log levels (debug < info < warn < error), not string values. The actual field name and format in your logs can vary (e.g., `"PRIORITY": 6`, `"severity": "ERROR"`) as long as it's recognized by hl's configuration.

### Source Fields

Reference source fields by prefixing with a period (`.`):

```sh
hl -q '.status = 200' application.log
```

This queries the actual `status` field in your logs as a raw value (string or number).

**Important distinction with predefined fields:**

```sh
# Semantic level comparison (recognizes different formats)
hl -q 'level = info' application.log
# Matches: "level":"info", "severity":"INFO", "PRIORITY":6, etc.
# Supports: level > info, level >= warn

# Raw string/number comparison (exact field name and value)
hl -q '.level = info' application.log
# Matches only: "level":"info" (exact, case-sensitive)
# Does NOT support semantic comparisons like .level > info
```

When using source field selectors (`.level`), you're operating on raw field values as strings or numbers. There's no semantic interpretation - it's a direct value comparison.

### Exact Field Names

Use JSON-formatted strings to avoid special syntax or match exact field names:

```sh
# Match field literally named ".level" (with the dot)
hl -q '".level" = info' application.log

# Match source field named "level" (without dot prefix)
hl -q '.level = info' application.log

# Match predefined level field
hl -q 'level = info' application.log
```

The `".level"` syntax (JSON-escaped) matches a field literally named `".level"`, while `.level` matches a field named `"level"` in the source.

**Field name matching rules:**
- **Underscores and hyphens** are treated interchangeably: `user_name` matches `user-name` (applies with or without JSON-escaping)
- **Dot-delimited names** match both hierarchical and flat fields automatically:
  - `user.id` matches `{"user":{"id":123}}` (hierarchical/nested)
  - `user.id` matches `{"user.id":123}` (flat with dot in name)
  - Even with JSON-escaping: `"user.id"` matches both formats
  - This allows hl to work seamlessly with different log formats

**Semantic vs. Raw Field Access:**

| Syntax | Field Name | Comparison Type | Example |
|--------|------------|-----------------|---------|
| `level` | Configured level field | Semantic level | `level > info` works |
| `.level` | Literal "level" field | Raw string/number | `.level = "info"` (case-sensitive) |
| `".level"` | Literal ".level" field | Raw string/number | Only exact match |

Predefined fields like `level`, `message`, `caller`, and `logger` have semantic meaning and special comparison behavior. Source field selectors (`.field` or `"field"`) always use raw value comparison.

## Comparison Operators

### Equality and Inequality

```sh
# Equal to
hl -q 'status = 200' application.log
hl -q 'status eq 200' application.log

# Not equal to
hl -q 'status != 200' application.log
hl -q 'status ne 200' application.log
```

### Numeric Comparisons

```sh
# Greater than
hl -q 'status > 400' application.log
hl -q 'status gt 400' application.log

# Greater than or equal
hl -q 'status >= 400' application.log
hl -q 'status ge 400' application.log

# Less than
hl -q 'duration < 0.5' application.log
hl -q 'duration lt 0.5' application.log

# Less than or equal
hl -q 'duration <= 1.0' application.log
hl -q 'duration le 1.0' application.log
```

### Semantic Level Comparisons

The predefined `level` field supports semantic comparisons that understand log level hierarchy:

```sh
# Show warnings and errors (level >= warn)
hl -q 'level >= warn' application.log

# Show info and above (excludes debug and trace)
hl -q 'level >= info' application.log

# Show only errors (level higher than warn)
hl -q 'level > warn' application.log

# Show debug and trace (lower levels)
hl -q 'level < info' application.log
```

Level hierarchy (from lowest to highest):
- `trace` < `debug` < `info` < `warn` < `error`

**These comparisons work regardless of the actual field format in your logs:**
- `"level": "info"`, `"severity": "INFO"`, `"PRIORITY": 6` all match `level = info`
- Case-insensitive: `INFO`, `Info`, `info` all match
- Different field names configured in hl settings

## Logical Operators

### AND

Combine conditions that must all be true:

```sh
hl -q 'level = error and status >= 500' application.log
hl -q 'level = error && status >= 500' application.log
```

### OR

Match if any condition is true:

```sh
hl -q 'level = error or status >= 500' application.log
hl -q 'level = error || status >= 500' application.log
```

### NOT

Negate a condition:

```sh
# NOT has lower precedence than comparison operators
hl -q 'not level = debug' application.log
hl -q '!level = debug' application.log

# Use parentheses for complex expressions
hl -q 'not (level = debug and status >= 400)' application.log
hl -q '!(level = debug and status >= 400)' application.log

# Or use inequality operator for simple equality checks
hl -q 'level != debug' application.log
```

**Note:** `not` has lower precedence than comparison operators, so `not level = debug` is parsed as `not (level = debug)`. Use explicit parentheses for clarity in complex expressions.

### Combining Operators

Use parentheses to control precedence:

```sh
hl -q '(level = error or level = warn) and status >= 400' application.log
```

## String Matching

### Substring Match

Check if a field contains a substring:

```sh
# Contains
hl -q 'message contain "error"' application.log
hl -q 'message ~= "error"' application.log

# Does not contain
hl -q 'message not contain "debug"' application.log
hl -q 'message !~= "debug"' application.log
```

### Wildcard Match

Use `*` for zero or more characters and `?` for a single character:

```sh
# Matches "user123", "user456", etc.
hl -q 'username like "user*"' application.log

# Matches "user1", "user2", but not "user12"
hl -q 'username like "user?"' application.log

# Negation
hl -q 'username not like "admin*"' application.log
```

### Regular Expression Match

Use regex patterns for complex matching:

```sh
# Matches email addresses
hl -q 'email match "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"' application.log
hl -q 'email ~~= "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"' application.log

# Does not match pattern
hl -q 'username not match "^admin"' application.log
hl -q 'username !~~= "^admin"' application.log
```

## Set Operations

### In-List Matching

Check if a value is in a set:

```sh
# Value in list
hl -q 'status in (200, 201, 204)' application.log

# String values
hl -q 'method in (GET, POST, PUT)' application.log

# Not in list
hl -q 'status not in (200, 304)' application.log
```

### Loading Sets from Files

Load values from a file (one per line):

```sh
# Create a file with allowed IPs
echo "192.168.1.1" > allowed-ips.txt
echo "10.0.0.1" >> allowed-ips.txt

# Query using the file
hl -q 'ip in @allowed-ips.txt' application.log
hl -q 'ip not in @blocked-ips.txt' application.log
```

File format:
- One value per line
- Can be plain strings or JSON strings
- Empty lines are ignored

### Loading Sets from stdin

```sh
# Read values from stdin
echo -e "error\nwarn" | hl -q 'level in @-' application.log
```

## Field Existence

### Checking if Fields Exist

```sh
# Field exists (regardless of value)
hl -q 'exists(.price)' application.log
hl -q 'exist(.price)' application.log

# Field does not exist
hl -q 'not exists(.internal)' application.log

# Show entries with errors OR high status codes
hl -q 'exists(.error) or status >= 400' application.log
```

### Combining with Other Conditions

```sh
# Show if no price field OR price > 100
hl -q 'not exists(.price) or .price > 100' application.log

# Equivalent using ? modifier
hl -q '.price? > 100' application.log

# Show entries with errors OR failed requests
hl -q 'exists(.error) or status >= 500' application.log

# Show entries with stack trace (for debugging)
hl -q 'exists(.stack)' application.log
```

**Important:** By default, any field comparison **implicitly requires the field to exist**. This means:

- `.price > 100` already means "field exists AND value > 100"
- `exists(.price) and .price > 100` is **redundant** — the `exists()` does nothing useful
- To include records without the field, use `?` modifier: `.price? > 100`
- Or use explicit logic: `not exists(.price) or .price > 100`

Use `exists()` **only** when:
- Checking existence is the sole condition: `exists(.error)`, `exists(.stack)`
- Combining with `or` for complex logic: `exists(.error) or status >= 500`
- The field check stands alone, not combined with a value comparison on the same field

## Include Absent Modifier

The `?` modifier after a field name changes how missing fields are handled.

### Without `?` (Default Behavior)

```sh
# Only matches records WHERE .status exists AND equals "error"
hl -q '.status = error' application.log
```

Records without a `status` field are excluded.

### With `?` (Include Absent)

```sh
# Matches records WHERE .status = "error" OR .status doesn't exist
hl -q '.status?=error' application.log
```

This is useful when you want to include records that might not have the field.

### Common Use Cases

```sh
# Show non-errors OR records without status field
hl -q '.status?!=error' application.log

# Show records with price=0 OR no price field
hl -q '.price?=0' application.log
```

## Nested Fields

Access nested JSON fields using dot notation:

```sh
# Simple nested field
hl -q 'user.id = 12345' application.log

# Deep nesting
hl -q 'request.headers.authorization ~= "Bearer"' application.log
```

### Automatic Matching: Hierarchical and Flat Fields

hl automatically matches dot-delimited field names against **both** hierarchical JSON objects and flat fields with dots in their names:

```sh
# This query: user.id = 12345
# Matches BOTH of these log formats:

# 1. Hierarchical JSON
{"user": {"id": 12345, "name": "Alice"}}

# 2. Flat field with dot in name
{"user.id": 12345, "user.name": "Alice"}
```

**This works even with JSON-escaped field names:**

```sh
# Using JSON-escaping still matches both formats
hl -q '"user.id" = 12345' application.log

# Matches: {"user": {"id": 12345}}
# Also matches: {"user.id": 12345}
```

**Why this matters:**
- Different logging frameworks use different structures
- hl works seamlessly with both formats
- You don't need to know how fields are stored internally
- Queries work consistently across different log sources

**Examples:**

```sh
# Match request.method in either format
hl -q 'request.method = POST' application.log
# Hierarchical: {"request": {"method": "POST"}}
# Flat: {"request.method": "POST"}

# Deep nesting works the same way
hl -q 'a.b.c.d = value' application.log
# Hierarchical: {"a": {"b": {"c": {"d": "value"}}}}
# Flat: {"a.b.c.d": "value"}
# Mixed: {"a": {"b.c": {"d": "value"}}}
```

## Array Fields

### Array Element Access

```sh
# Access specific array index (0-based)
hl -q 'tags.[0] = "important"' application.log

# Second element
hl -q 'tags.[1] = "verified"' application.log
```

### Array Contains

```sh
# Check if any array element matches
hl -q 'tags.[] = "error"' application.log

# Nested object in array
hl -q 'users.[].role = "admin"' application.log
```

## Special Characters in Values

Use JSON-formatted strings for values with special characters:

```sh
# Newlines in strings
hl -q 'message contain "Error:\nConnection failed"' application.log

# Quotes in strings
hl -q 'message = "He said \"hello\""' application.log

# Tabs and special characters
hl -q 'data contain "\t\r\n"' application.log
```

## Semantic vs Raw Field Access - Practical Examples

### When to Use Predefined `level` Field

```sh
# Show all warnings and errors (semantic comparison)
hl -q 'level >= warn' application.log

# Works with any log format
# Matches: "level":"WARN", "severity":"ERROR", "PRIORITY":4, etc.

# Show errors only
hl -q 'level = error' application.log
# Case-insensitive, format-agnostic

# Exclude debug logs
hl -q 'level > debug' application.log
```

### When to Use Source `.level` Field

```sh
# Match exact string value in "level" field
hl -q '.level = "INFO"' application.log
# Only matches: "level":"INFO" (case-sensitive)
# Does NOT match: "level":"info" or "severity":"INFO"

# Match custom level value (field must exist)
hl -q '.level = "custom-level"' application.log

# Or include entries without level field
hl -q '.level? = "custom-level"' application.log

# Match non-standard level values
hl -q '.level = "VERBOSE"' application.log
```

### Comparing the Difference

```sh
# Semantic (predefined field)
hl -q 'level >= info' application.log
# ✓ Matches: trace=false, debug=false, info=true, warn=true, error=true
# ✓ Works across different log formats
# ✓ Understands level hierarchy

# Raw (source field)
hl -q '.level >= "info"' application.log
# ✓ Alphabetical string comparison
# ✗ "error" < "info" < "warn" (alphabetically wrong!)
# ✗ Only matches exact "level" field name
# ✗ Case-sensitive
```

**Use predefined `level`** for log level filtering (almost always what you want).

**Use source `.level`** only when you need exact raw value matching or the field has non-standard values.

## Complete Examples

### Error Investigation

```sh
# All errors with stack traces
hl -q 'level = error and exists(stack)' application.log

# Errors from specific service
hl -q 'level = error and service = "payment"' application.log

# Errors excluding known issues
hl -q 'level = error and message not contain "Expected timeout"' application.log
```

### Performance Analysis

```sh
# Slow requests
hl -q 'duration > 1.0' application.log

# Slow OR failed requests
hl -q 'duration > 1.0 or status >= 500' application.log

# Database queries over threshold
hl -q 'component = "database" and duration > 0.1' application.log
```

### Security Monitoring

```sh
# Failed login attempts
hl -q 'event = "login" and success = false' application.log

# Suspicious IP addresses
hl -q 'ip not in @allowed-ips.txt and level = warn' application.log

# Admin actions
hl -q 'user.role = "admin" and action not in (login, logout)' application.log
```

### Request Tracing

```sh
# Trace specific request
hl -q 'request.id = "abc-123-def"' application.log

# Requests from specific user
hl -q 'user.id = 12345 and method != GET' application.log

# Multi-step request flow
hl -q 'trace.id = "xyz789" and (step in (auth, process, respond))' application.log
```

### HTTP API Monitoring

```sh
# Client errors (4xx)
hl -q 'status >= 400 and status < 500' application.log

# Server errors (5xx)
hl -q 'status >= 500' application.log

# Slow or failed requests
hl -q 'status >= 400 or duration > 0.5' application.log

# Specific endpoints
hl -q 'path like "/api/v1/*" and method = POST' application.log
```

## Query Best Practices

1. **Use parentheses for clarity** - Make complex queries readable
   ```sh
   hl -q '(level = error or level = warn) and (status >= 400 or duration > 1)' application.log
   ```

2. **Start simple, then refine** - Test basic queries before adding complexity
   ```sh
   # Start with
   hl -q 'level = error' application.log
   
   # Then add conditions
   hl -q 'level = error and service = "api"' application.log
   ```

3. **Use exists() for optional fields** - Prevent unexpected filtering
   ```sh
   hl -q 'not exists(.optional) or .optional != "skip"' application.log
   ```

4. **Quote strings with spaces** - Avoid parsing issues
   ```sh
   hl -q 'message = "Connection timeout"' application.log
   ```

5. **Combine with other filters** - Layer filtering methods
   ```sh
   hl -l e -q 'duration > 1' --since -1h application.log
   ```

## Operator Precedence

From highest to lowest:

1. Parentheses `()`
2. Field access `.field`
3. Function calls `exists()`
4. Comparison operators `=`, `!=`, `>`, `<`, etc.
5. String operators `~=`, `like`, `match`
6. `not`, `!` (lower than comparisons)
7. `and`, `&&`
8. `or`, `||`

This means `not level = debug` is parsed as `not (level = debug)`, which is usually what you want.

When in doubt, use parentheses to be explicit.

## Performance Tips

1. **Put selective filters first** - Help short-circuit evaluation
   ```sh
   hl -q 'status = 500 and exists(stack)' application.log
   ```

2. **Use field filters for simple cases** - Faster than queries
   ```sh
   # Prefer this
   hl -f service=api application.log
   
   # Over this
   hl -q 'service = "api"' application.log
   ```

3. **Combine with time ranges** - Reduce data to process
   ```sh
   hl -q 'status >= 400' --since -1h application.log
   ```

4. **Use sorted mode for time-sensitive queries** - Much faster
   ```sh
   hl -s -q 'level = error' --since -1d *.log
   ```

### Common Mistakes

1. **Forgetting quotes for values with spaces**
   ```sh
   # Wrong
   hl -q 'message = Connection timeout' application.log
   
   # Correct
   hl -q 'message = "Connection timeout"' application.log
   ```

2. **Using wrong field reference**
   ```sh
   # Query predefined field
   hl -q 'level = error' application.log
   
   # Query source field named "level"
   hl -q '.level = error' application.log
   ```

3. **Not handling missing fields**
   ```sh
   # Excludes records without .price
   hl -q '.price > 100' application.log
   
   # Include records without .price
   hl -q 'not exists(.price) or .price > 100' application.log
   ```

4. **Confusing NOT operator precedence**
   ```sh
   # This works - NOT has lower precedence than comparison
   hl -q 'not level = debug' application.log
   
   # But use parentheses for clarity in complex expressions
   hl -q 'not (level = debug or level = trace)' application.log
   
   # Without parentheses, this would be parsed incorrectly:
   # hl -q 'not level = debug or level = trace'  # means: (not level = debug) or (level = trace)
   ```

5. **Using raw field instead of predefined field for level comparisons**
   ```sh
   # Wrong - alphabetical string comparison
   hl -q '.level > info' application.log
   # "error" < "info" < "warn" (alphabetically wrong!)
   
   # Correct - semantic level comparison
   hl -q 'level > info' application.log
   # Understands: trace < debug < info < warn < error
   
   # Use .level only for exact raw value matching
   hl -q '.level = "CUSTOM_LEVEL"' application.log
   ```

## Next Steps

- [Query Syntax Reference](../reference/query-syntax.md) - Complete syntax specification
- [Query Examples](../examples/queries.md) - More real-world examples
- [Filtering by Field Values](./filtering-fields.md) - Simpler field-based filtering
- [Filtering by Log Level](./filtering-level.md) - Level-based filtering