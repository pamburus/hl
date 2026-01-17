# Query Examples

This page demonstrates advanced query syntax for complex filtering scenarios.

## Query Basics

Queries use a simple expression language with operators, comparisons, and boolean logic.

### Basic Comparison Operators

```hl/dev/null/shell.sh#L1
# Equality
hl -f 'status = 200' app.log

# Inequality
hl -f 'status != 200' app.log

# Greater than
hl -f 'duration > 1000' app.log

# Greater than or equal
hl -f 'status >= 400' app.log

# Less than
hl -f 'response_size < 1024' app.log

# Less than or equal
hl -f 'retry_count <= 3' app.log
```

## String Operators

### String Contains

Use `~=` for substring matching (case-sensitive):

```hl/dev/null/shell.sh#L1
# Substring match
hl -f 'message ~= "database"' app.log
```

### Regular Expressions

Use `~~=` for regex matching (case-sensitive):

```hl/dev/null/shell.sh#L1
# Match pattern
hl -f 'url ~~= "^/api/v[0-9]+"' app.log

# Match multiple patterns
hl -f 'message ~~= "(error|warning|failure)"' app.log
```

### String Equality with Wildcards

The `like` operator supports glob-style wildcards:

```hl/dev/null/shell.sh#L1
# Match with wildcards
hl -f 'path like "/api/*/users"' app.log

# Wildcard pattern
hl -f 'filename like "*.json"' app.log
```

## Boolean Logic

### AND Operator

Combine conditions with `and` (or `&&`):

```hl/dev/null/shell.sh#L1
# Both conditions must be true
hl -f 'status >= 400 and duration > 1000' app.log

# Multiple AND conditions
hl -f 'service = "api" and env = "production" and status >= 500' app.log
```

### OR Operator

Use `or` (or `||`) for alternative conditions:

```hl/dev/null/shell.sh#L1
# Either condition can be true
hl -f 'status = 404 or status = 500' app.log

# Multiple OR conditions
hl -f 'level = error or level = warning or level = critical' app.log
```

### NOT Operator

Negate conditions with `not` (or `!`):

```hl/dev/null/shell.sh#L1
# Exclude entries
hl -f 'not service = "health-check"' app.log

# Negate complex expressions
hl -f 'not (status >= 200 and status < 300)' app.log
```

## Operator Precedence

Operators are evaluated in this order (highest to lowest precedence):

1. Parentheses `()`
2. Comparisons and string operators (`=`, `!=`, `<`, `>`, `~=`, `~~=`, `like`, `in`)
3. `not` / `!`
4. `and` / `&&`
5. `or` / `||`

### Precedence Examples

```hl/dev/null/shell.sh#L1
# NOT binds tighter than AND/OR but looser than comparisons
# This means: (not (level = debug))
hl -f 'not level = debug' app.log

# AND binds tighter than OR
# This means: (a = 1) or ((b = 2) and (c = 3))
hl -f 'a = 1 or b = 2 and c = 3' app.log

# Use parentheses for clarity
hl -f '(a = 1 or b = 2) and c = 3' app.log
```

## Field Existence

### Check Field Presence

```hl/dev/null/shell.sh#L1
# Show entries that have an error field
hl -f 'exists(error)' app.log

# Show entries missing a user_id field
hl -f 'not exists(user_id)' app.log
```

### Optional Field Modifier

Use `?` to make comparisons include entries where the field is missing:

```hl/dev/null/shell.sh#L1
# Match if price > 100 OR if price field doesn't exist
hl -f '.price? > 100' app.log

# Equivalent explicit form
hl -f 'not exists(.price) or .price > 100' app.log
```

**Note**: Without `?`, comparisons implicitly require the field to exist:

```hl/dev/null/shell.sh#L1
# Only matches entries WHERE field exists AND value > 100
hl -f '.price > 100' app.log
```

## In Operator

Check if a value is in a list:

```hl/dev/null/shell.sh#L1
# Match multiple values
hl -f 'status in [200, 201, 204]' app.log

# Match multiple services
hl -f 'service in ["api", "web", "worker"]' app.log

# Combine with other operators
hl -f 'status in [400, 401, 403, 404] and path ~= "/admin"' app.log
```

## Level Filtering in Queries

### Semantic Level Comparisons

The `level` pseudo-field supports semantic comparisons:

```hl/dev/null/shell.sh#L1
# Exact level
hl -f 'level = warn' app.log

# Level and above
hl -f 'level >= warn' app.log

# Below a level
hl -f 'level < error' app.log
```

### Raw Level Field

Use `.level` (with a dot) to access the raw source field:

```hl/dev/null/shell.sh#L1
# Raw string comparison (case-sensitive)
hl -f '.level = "WARNING"' app.log

# Semantic comparison (recognizes multiple formats)
hl -f 'level = warn' app.log
```

## Nested Field Access

### Dot Notation

Access nested JSON fields with dot notation:

```hl/dev/null/shell.sh#L1
# Nested object fields
hl -f 'user.name = "alice"' app.log

# Deeply nested fields
hl -f 'request.headers.content-type ~= "json"' app.log

# Numeric nested fields
hl -f 'response.body.total-count > 100' app.log
```

**Note**: Dot notation matches both:
- Hierarchical JSON: `{"user": {"id": 123}}`
- Flat fields with dots: `{"user.id": 123}`

### Underscore/Hyphen Equivalence

Field names treat underscores and hyphens as interchangeable when matching. Use hyphens in examples for consistency with display output:

```hl/dev/null/shell.sh#L1
# Both match fields named 'user_id' or 'user-id' in the source
hl -f 'user-id = 123' app.log
```

## Complex Query Examples

### Multi-Condition Error Detection

```hl/dev/null/shell.sh#L1
# Errors in production with high duration
hl -f 'level >= error and env = "production" and duration > 5000' app.log
```

### Exclude Health Checks and Monitoring

```hl/dev/null/shell.sh#L1
hl -f 'not (path = "/health" or path = "/metrics" or path = "/ping")' app.log
```

### Find Slow or Failed Requests

```hl/dev/null/shell.sh#L1
hl -f '(duration > 3000) or (status >= 500)' app.log
```

### Complex Service and User Filtering

```hl/dev/null/shell.sh#L1
hl -f '(service = "api" and user.role = "admin") or (service = "worker" and priority = "high")' app.log
```

### Find Requests with Specific Patterns

```hl/dev/null/shell.sh#L1
# API calls to user endpoints with errors
hl -f 'path ~ "^/api/.*/users" and status >= 400' app.log
```

### Database Query Analysis

```hl/dev/null/shell.sh#L1
# Slow queries or errors
hl -f 'operation = "query" and (duration > 1000 or exists(error))' app.log
```

### Authentication Failures

```hl/dev/null/shell.sh#L1
hl -f '(event = "login" or event = "auth") and success = false' auth.log
```

### High-Value Transaction Monitoring

```hl/dev/null/shell.sh#L1
hl -f 'transaction_type = "payment" and amount > 10000 and status != "completed"' payments.log
```

## Practical Patterns

### Debugging a Specific Request

```hl/dev/null/shell.sh#L1
# Track a request by ID across services
hl -f 'request-id = "abc-123-xyz"' service-*.log
```

### Finding Anomalies

```hl/dev/null/shell.sh#L1
# Requests that took longer than average and failed
hl -f 'duration > 2000 and status >= 500' app.log
```

### Security Audit

```hl/dev/null/shell.sh#L1
# Failed authentication attempts from specific IPs
hl -f 'event = "auth-failed" and ip ~~ "^192\\.168\\."' security.log
```

### Rate Limiting Detection

```hl/dev/null/shell.sh#L1
# Too many requests from a user
hl -f 'status = 429 or message ~= "rate limit"' app.log
```

### Service Dependency Issues

```hl/dev/null/shell.sh#L1
# Errors calling downstream services
hl -f 'exists(downstream-service) and (status >= 500 or exists(timeout))' app.log
```

## Combining Queries with Other Filters

Queries can be combined with level and time filters:

```hl/dev/null/shell.sh#L1
# Errors in the last hour from the API service
hl -l error --since "1h ago" -f 'service = "api"' app.log

# Production warnings in a time range
hl -l warn --since "2024-01-15 10:00" --until "2024-01-15 12:00" -f 'env = "production"' app.log
```

## Query Performance Tips

- **Simple queries are faster** — `status = 200` is faster than complex regex.
- **Use exists() sparingly** — Field comparisons implicitly check existence.
- **Avoid redundant conditions** — `exists(.field) and .field = value` is redundant; just use `.field = value`.
- **Filter early** — Combine level filters (`-l error`) with queries to reduce the dataset.

## Testing Your Queries

Use small sample logs to test complex queries:

```hl/dev/null/shell.sh#L1
# Test on a small sample first
echo '{"level":"error","status":500,"service":"api"}' | hl -P -f 'status >= 400 and service = "api"'
```

## Next Steps

- [Filtering Examples](filtering.md) — Simpler filtering patterns and level-based filtering.
- [Time Filtering](time-filtering.md) — Time range specifications and relative time.
- [Reference: Query Syntax](../reference/query-syntax.md) — Formal grammar and complete operator reference.
