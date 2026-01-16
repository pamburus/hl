# Query Syntax

The `--query` (or `-q`) option allows you to filter log entries using powerful query expressions that support logical operators, comparisons, set membership, string matching, and more.

## Basic Syntax

A query expression consists of field filters combined with logical operators.

```/dev/null/example.sh#L1-2
# Basic field comparison
hl -q 'status=200' app.log
```

## Field Names

Field names can be specified in two forms:

1. **Simple form**: Alphanumeric characters, underscores, hyphens, dots, brackets, and `@` symbol
   - Examples: `status`, `user-id`, `user.name`, `tags[0]`, `@timestamp`

2. **Quoted form**: JSON-style quoted strings for field names containing special characters
   - Examples: `"field with spaces"`, `"field:with:colons"`

```/dev/null/example.sh#L1-5
# Simple field name
hl -q 'status=200' app.log

# Quoted field name
hl -q '"request-id"="abc-123"' app.log
```

## Operators

### Comparison Operators

Comparison operators work with both numeric and string values.

| Operator | Aliases | Description | Example |
|----------|---------|-------------|---------|
| `=` | `eq` | Equal to | `status=200` |
| `!=` | `ne`, `not eq` | Not equal to | `status!=200` |
| `<` | `lt` | Less than | `status<400` |
| `<=` | `le` | Less than or equal to | `status<=299` |
| `>` | `gt` | Greater than | `status>399` |
| `>=` | `ge` | Greater than or equal to | `status>=500` |

```/dev/null/example.sh#L1-8
# Numeric comparisons
hl -q 'status>=500' app.log
hl -q 'duration>1.5' app.log

# String comparisons (lexicographic)
hl -q 'method=POST' app.log
hl -q 'level!=debug' app.log
```

### String Matching Operators

| Operator | Aliases | Description | Example |
|----------|---------|-------------|---------|
| `~=` | `contains`, `contain` | Substring match | `message~="error"` |
| `!~=` | `not contains`, `not contain` | Negated substring match | `message!~="debug"` |
| `~~=` | `matches`, `match` | Regular expression match | `user~~="^admin"` |
| `!~~=` | `not matches`, `not match` | Negated regex match | `path!~~="^/health"` |
| `like` | | Wildcard pattern match | `path like "/api/*"` |
| `not like` | | Negated wildcard match | `path not like "/internal/*"` |

```/dev/null/example.sh#L1-11
# Substring matching
hl -q 'message contains "timeout"' app.log
hl -q 'message~="connection refused"' app.log

# Regular expression matching
hl -q 'user matches "^admin.*"' app.log
hl -q 'path~~="^/api/v[0-9]+/"' app.log

# Wildcard matching
hl -q 'path like "/api/*/users"' app.log
hl -q 'hostname not like "prod-*"' app.log
```

### Set Membership Operators

| Operator | Aliases | Description | Example |
|----------|---------|-------------|---------|
| `in` | | Value is in set | `status in (500,502,503)` |
| `not in` | | Value is not in set | `method not in (GET,HEAD)` |

**Set sources**:
- **Literal**: `(value1,value2,value3)`
- **File**: `@filename.txt` (one value per line)
- **Stdin**: `@-` (read values from stdin)

```/dev/null/example.sh#L1-11
# Literal set
hl -q 'status in (500,502,503,504)' app.log

# Set from file
hl -q 'user-id in @user-ids.txt' app.log

# Set from stdin
echo -e "alice\nbob\ncharlie" | hl -q 'user in @-' app.log

# Negation
hl -q 'method not in (OPTIONS,HEAD)' app.log
```

### Existence Operators

| Operator | Aliases | Description | Example |
|----------|---------|-------------|---------|
| `exists(field)` | `exist(field)` | Field exists | `exists(user-id)` |
| `not exists(field)` | `not exist(field)` | Field does not exist | `not exists(trace-id)` |

```/dev/null/example.sh#L1-5
# Check if field exists
hl -q 'exists(user-id)' app.log

# Check if field does not exist
hl -q 'not exists(trace-id)' app.log
```

## Logical Operators

Combine multiple conditions using logical operators.

| Operator | Aliases | Description | Example |
|----------|---------|-------------|---------|
| `and` | `&&` | Logical AND | `status>=500 and method=POST` |
| `or` | `\|\|` | Logical OR | `status>=500 or duration>10` |
| `not` | `!` | Logical NOT | `not status=200` |

```/dev/null/example.sh#L1-8
# AND logic
hl -q 'status>=500 and method=POST' app.log

# OR logic
hl -q 'status>=500 or duration>10' app.log

# NOT logic
hl -q 'not status=200' app.log
```

## Grouping

Use parentheses to group expressions and control precedence.

```/dev/null/example.sh#L1-8
# Without grouping (AND has higher precedence than OR)
hl -q 'status=404 or status>=500 and method=POST' app.log
# Equivalent to: status=404 or (status>=500 and method=POST)

# With grouping to change precedence
hl -q '(status=404 or status>=500) and method=POST' app.log

# Complex grouped expression
hl -q '(status>=500 and status<600) or (status=404 and path contains "/api")' app.log
```

## Modifiers

### Include Absent Modifier (`?`)

The `?` modifier (placed after the field name) includes entries where the field is missing.

```/dev/null/example.sh#L1-5
# Match entries where user-id=123 OR user-id is absent
hl -q 'user-id?=123' app.log

# Match entries where status>=500 OR status field is absent
hl -q 'status?>=500' app.log
```

This is useful when you want to include log entries that don't have a particular field.

## Level Filtering

Query expressions support special level filtering with the `level` pseudo-field.

```/dev/null/example.sh#L1-8
# Filter by log level
hl -q 'level=error' app.log
hl -q 'level>=warn' app.log

# Combine with other filters
hl -q 'level>=error and status>=500' app.log
hl -q 'level in (error,fatal)' app.log
```

## Value Types

### Numbers

Numbers can be:
- **Integers**: `42`, `-17`, `0`
- **Decimals**: `3.14`, `-0.5`, `2.0`
- **Scientific notation**: `1.5e10`, `2e-3`, `-3.14E+2`

```/dev/null/example.sh#L1-5
# Integer comparison
hl -q 'status>=500' app.log

# Decimal comparison
hl -q 'duration>1.5' app.log

# Scientific notation
hl -q 'response-size>1e6' app.log
```

### Strings

Strings can be specified in two forms:

1. **Simple strings**: Letters, numbers, and select special characters (`@`, `.`, `_`, `-`, `:`, `/`, `!`, `#`, `%`, `$`, `*`, `+`, `?`)
   - Examples: `POST`, `error`, `192.168.1.1`, `/api/users`

2. **JSON strings**: Quoted strings with escape sequences
   - Examples: `"hello world"`, `"path with\nline break"`, `"quote: \"hi\""`

```/dev/null/example.sh#L1-5
# Simple string
hl -q 'method=POST' app.log

# JSON string with spaces
hl -q 'message="connection timeout"' app.log

# JSON string with escapes
hl -q 'message="error: \"connection refused\""' app.log
```

## Complete Examples

### Error Analysis

```/dev/null/example.sh#L1-8
# All 5xx errors
hl -q 'status>=500 and status<600' app.log

# Errors with slow response times
hl -q 'status>=500 and duration>2' app.log

# POST requests that failed
hl -q 'method=POST and status>=400' app.log
```

### User Activity

```/dev/null/example.sh#L1-8
# Specific user's requests
hl -q 'user-id=12345' app.log

# Multiple users
hl -q 'user-id in (123,456,789)' app.log

# Authenticated requests (user-id exists)
hl -q 'exists(user-id)' app.log
```

### Pattern Matching

```/dev/null/example.sh#L1-8
# API endpoints
hl -q 'path like "/api/*"' app.log

# Specific error messages
hl -q 'message contains "timeout" or message contains "refused"' app.log

# Requests to versioned API endpoints
hl -q 'path matches "^/api/v[0-9]+/"' app.log
```

### Complex Filtering

```/dev/null/example.sh#L1-8
# Errors excluding health checks
hl -q 'status>=500 and path not like "/health*"' app.log

# High-priority issues
hl -q '(level=error or level=fatal) and (status>=500 or exists(exception))' app.log

# Suspicious activity
hl -q '(status=401 or status=403) and method in (POST,PUT,DELETE)' app.log
```

## Operator Precedence

From highest to lowest precedence:

1. **Parentheses**: `( )`
2. **Unary**: `not`, `!`
3. **Comparison**: `=`, `!=`, `<`, `<=`, `>`, `>=`, `~=`, `~~=`, `in`, `exists()`
4. **AND**: `and`, `&&`
5. **OR**: `or`, `||`

```/dev/null/example.sh#L1-5
# These are equivalent:
hl -q 'a=1 and b=2 or c=3' app.log
hl -q '(a=1 and b=2) or c=3' app.log

# Different from:
hl -q 'a=1 and (b=2 or c=3)' app.log
```

## Comparison with `--filter`

The `--query` option is more powerful than `--filter`:

| Feature | `--filter` | `--query` |
|---------|------------|-----------|
| Field matching | ✓ | ✓ |
| Regex matching | ✓ | ✓ |
| Comparisons (`<`, `>`, etc.) | ✗ | ✓ |
| Logical operators | ✗ | ✓ |
| Set membership | ✗ | ✓ |
| Existence checks | ✗ | ✓ |
| Grouping | ✗ | ✓ |

**Use `--filter`** for simple field matching:
```/dev/null/example.sh#L1-2
# Simple exact match
hl -f 'status=200' app.log
```

**Use `--query`** for complex expressions:
```/dev/null/example.sh#L1-2
# Complex expression with multiple conditions
hl -q 'status>=500 and (method=POST or method=PUT)' app.log
```

## Tips

1. **Quote the entire query** to avoid shell interpretation:
   ```/dev/null/example.sh#L1-2
   # Good
   hl -q 'status>=500 and method=POST' app.log
   
   # Bad (shell may interpret > as redirection)
   hl -q status>=500 and method=POST app.log
   ```

2. **Use literal sets for multiple values** instead of multiple OR conditions:
   ```/dev/null/example.sh#L1-5
   # Good
   hl -q 'status in (500,502,503,504)' app.log
   
   # Less efficient
   hl -q 'status=500 or status=502 or status=503 or status=504' app.log
   ```

3. **Combine with other options** for powerful filtering:
   ```/dev/null/example.sh#L1-5
   # Filter by level AND query
   hl -l error -q 'status>=500' app.log
   
   # Filter by time range AND query
   hl --since "2024-01-15 10:00:00" -q 'method=POST' app.log
   ```

4. **Use `--raw` with queries** to export filtered data:
   ```/dev/null/example.sh#L1-2
   # Export matching entries as JSON
   hl --raw -q 'status>=500' app.log > errors.json
   ```
