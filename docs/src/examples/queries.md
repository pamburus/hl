# Query Examples

This page demonstrates advanced query syntax for complex filtering scenarios.

## Query Basics

Queries use a simple expression language with operators, comparisons, and boolean logic.

### Basic Comparison Operators

```sh
# Equality
hl -q 'status = 200' app.log

# Inequality
hl -q 'status != 200' app.log

# Greater than
hl -q 'duration > 1000' app.log

# Greater than or equal
hl -q 'status >= 400' app.log

# Less than
hl -q 'response-size < 1024' app.log

# Less than or equal
hl -q 'retry-count <= 3' app.log
```

## String Operators

### String Contains

Use `~=` for substring matching (case-sensitive):

```sh
# Substring match
hl -q 'message ~= "database"' app.log
```

### Regular Expressions

Use `~~=` for regex matching (case-sensitive):

```sh
# Match pattern
hl -q 'url ~~= "^/api/v[0-9]+"' app.log

# Match multiple patterns
hl -q 'message ~~= "(error|warning|failure)"' app.log
```

### String Equality with Wildcards

The `like` operator supports glob-style wildcards:

```sh
# Match with wildcards
hl -q 'path like "/api/*/users"' app.log

# Wildcard pattern
hl -q 'filename like "*.json"' app.log
```

## Boolean Logic

### AND Operator

Combine conditions with `and` (or `&&`):

```sh
# Both conditions must be true
hl -q 'status >= 400 and duration > 15' app.log

# Multiple AND conditions
hl -q 'service = "api" and env = "production" and status >= 500' app.log
```

### OR Operator

Use `or` (or `||`) for alternative conditions:

```sh
# Either condition can be true
hl -q 'status = 404 or status = 500' app.log

# Multiple OR conditions
hl -q 'level = error or level = warning or level = critical' app.log
```

### NOT Operator

Negate conditions with `not` (or `!`):

```sh
# Exclude entries
hl -q 'not service = "health-check"' app.log

# Negate complex expressions
hl -q 'not (status >= 200 and status < 300)' app.log
```

## Operator Precedence

Operators are evaluated in this order (highest to lowest precedence):

1. Parentheses `()`
2. Comparisons and string operators (`=`, `!=`, `<`, `>`, `~=`, `~~=`, `like`, `in`)
3. `not` / `!`
4. `and` / `&&`
5. `or` / `||`

### Precedence Examples

```sh
# NOT binds tighter than AND/OR but looser than comparisons
# This means: (not (level = debug))
hl -q 'not level = debug' app.log

# AND binds tighter than OR
# This means: (a = 1) or ((b = 2) and (c = 3))
hl -q 'a = 1 or b = 2 and c = 3' app.log

# Use parentheses for clarity
hl -q '(a = 1 or b = 2) and c = 3' app.log
```

## Field Existence

### Check Field Presence

```sh
# Show entries that have an error field
hl -q 'exists(error)' app.log

# Show entries missing a user-id field
hl -q 'not exists(user-id)' app.log
```

### Optional Field Modifier

Use `?` to make comparisons include entries where the field is missing:

```sh
# Match if price > 100 OR if price field doesn't exist
hl -q '.price? > 100' app.log

# Equivalent explicit form
hl -q 'not exists(.price) or .price > 100' app.log
```

**Note**: Without `?`, comparisons implicitly require the field to exist:

```sh
# Only matches entries WHERE field exists AND value > 100
hl -q '.price > 100' app.log
```

## In Operator

Check if a value is in a list:

```sh
# Match multiple values
hl -q 'status in [200, 201, 204]' app.log

# Match multiple services
hl -q 'service in ["api", "web", "worker"]' app.log

# Combine with other operators
hl -q 'status in [400, 401, 403, 404] and path ~= "/admin"' app.log
```

## Level Filtering in Queries

### Semantic Level Comparisons

The `level` pseudo-field supports semantic comparisons:

```sh
# Exact level
hl -q 'level = warn' app.log

# Level and above
hl -q 'level >= warn' app.log

# Below a level
hl -q 'level < error' app.log
```

### Raw Level Field

Use `.level` (with a dot) to access the raw source field:

```sh
# Raw string comparison (case-sensitive)
hl -q '.level = "WARNING"' app.log

# Semantic comparison (recognizes multiple formats)
hl -q 'level = warn' app.log
```

## Nested Field Access

### Dot Notation

Access nested JSON fields with dot notation:

```sh
# Nested object fields
hl -q 'user.name = "alice"' app.log

# Deeply nested fields
hl -q 'request.headers.content-type ~= "json"' app.log

# Numeric nested fields
hl -q 'response.body.total-count > 100' app.log
```

**Note**: Dot notation matches both:
- Hierarchical JSON: `{"user": {"id": 123}}`
- Flat fields with dots: `{"user.id": 123}`

### Underscore/Hyphen Equivalence

Field names treat underscores and hyphens as interchangeable when matching. Use hyphens in examples for consistency with display output:

```sh
# Both match fields named 'user-id' or 'user-id' in the source
hl -q 'user-id = 123' app.log
```

## Complex Query Examples

### Multi-Condition Error Detection

```sh
# Errors in production with high duration
hl -q 'level >= error and env = "production" and duration > 5000' app.log
```

### Exclude Health Checks and Monitoring

```sh
hl -q 'not (path = "/health" or path = "/metrics" or path = "/ping")' app.log
```

### Find Slow or Failed Requests

```sh
hl -q '(duration > 3000) or (status >= 500)' app.log
```

### Complex Service and User Filtering

```sh
hl -q '(service = "api" and user.role = "admin") or (service = "worker" and priority = "high")' app.log
```

### Find Requests with Specific Patterns

```sh
# API calls to user endpoints with errors
hl -q 'path ~~= "^/api/.*/users" and status >= 400' app.log
```

### Database Query Analysis

```sh
# Slow queries or errors
hl -q 'operation = "query" and (duration > 1000 or exists(error))' app.log
```

### Authentication Failures

```sh
hl -q '(event = "login" or event = "auth") and success = false' auth.log
```

### High-Value Transaction Monitoring

```sh
hl -q 'transaction-type = "payment" and amount > 10000 and status != "completed"' payments.log
```

## Practical Patterns

### Debugging a Specific Request

```sh
# Track a request by ID across services
hl -q 'request-id = "abc-123-xyz"' service-*.log
```

### Finding Anomalies

```sh
# Requests that took longer than average and failed
hl -q 'duration > 2000 and status >= 500' app.log
```

### Security Audit

```sh
# Failed authentication attempts from specific IPs
hl -q 'event = "auth-failed" and ip ~~= "^192\\.168\\."' security.log
```

### Rate Limiting Detection

```sh
# Too many requests from a user
hl -q 'status = 429 or message ~= "rate limit"' app.log
```

### Service Dependency Issues

```sh
# Errors calling downstream services
hl -q 'exists(downstream-service) and (status >= 500 or exists(timeout))' app.log
```

## Combining Queries with Other Filters

Queries can be combined with level and time filters:

```sh
# Errors in the last hour from the API service
hl -l error --since "1h ago" -q 'service = "api"' app.log

# Production warnings in a time range
hl -l warn --since "2024-01-15 10:00" --until "2024-01-15 12:00" -q 'env = "production"' app.log
```

## Query Performance Tips

- **Simple queries are faster** — `status = 200` is faster than complex regex.
- **Use exists() sparingly** — Field comparisons implicitly check existence.
- **Avoid redundant conditions** — `exists(.field) and .field = value` is redundant; just use `.field = value`.
- **Filter early** — Combine level filters (`-l error`) with queries to reduce the dataset.

## Testing Your Queries

Use small sample logs to test complex queries:

```sh
# Test on a small sample first
echo '{"level":"error","status":500,"service":"api"}' | hl -P -q 'status >= 400 and service = "api"'
```

## Next Steps

- [Filtering Examples](filtering.md) — Simpler filtering patterns and level-based filtering.
- [Time Filtering](time-filtering.md) — Time range specifications and relative time.
- [Reference: Query Syntax](../reference/query-syntax.md) — Formal grammar and complete operator reference.
