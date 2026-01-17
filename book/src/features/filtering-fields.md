# Filtering by Field Values

Field filtering allows you to show only log entries where specific fields match certain conditions. This is one of the most common and useful filtering techniques in hl.

## Basic Syntax

Use the `-f` or `--filter` option to filter by field values:

```sh
hl -f KEY=VALUE application.log
```

You can specify multiple filters, and all must match (AND logic):

```sh
hl -f service=api -f environment=production application.log
```

## Filter Operators

### Exact Match (`=`)

Show entries where a field equals a specific value:

```sh
# Single value
hl -f status=200 application.log

# String value
hl -f method=GET application.log

# Numeric value
hl -f port=8080 application.log
```

### Not Equal (`!=`)

Show entries where a field exists and is NOT equal to a value:

```sh
# Exclude specific status
hl -f 'status!=200' application.log

# Exclude specific method
hl -f 'method!=GET' application.log
```

**Important:** This only matches entries that **have** the field. Entries without the field are excluded.

### Contains Substring (`~=`)

Show entries where a field contains a substring:

```sh
# Contains substring
hl -f 'message~=error' application.log

# Contains in any field
hl -f 'url~=/api/v1' application.log

# Case-sensitive
hl -f 'user~=Admin' application.log
```

### Does Not Contain (`!~=`)

Show entries where a field exists and does NOT contain a substring:

```sh
# Does not contain
hl -f 'message!~=debug' application.log

# Exclude pattern
hl -f 'path!~=/health' application.log
```

**Important:** Like `!=`, this requires the field to exist.

## Include Absent Modifier (`?`)

By default, field filters only match entries that **have** the field. The `?` modifier changes this behavior to include entries where the field is absent.

### Without `?` (Default)

```sh
# Only matches entries that HAVE status field AND status != 200
hl -f 'status!=200' application.log
```

Entries without a `status` field are **excluded**.

### With `?` (Include Absent)

```sh
# Matches entries WHERE status != 200 OR status field is absent
hl -f 'status?!=200' application.log
```

Entries without a `status` field are **included**.

### Common Use Cases

```sh
# Show entries with status=error OR no status field
hl -f 'status?=error' application.log

# Show entries without 'ok' status OR no status field
hl -f 'status?!=ok' application.log

# Show entries with price=0 OR no price field
hl -f 'price?=0' application.log
```

This is particularly useful for logfmt or logs where fields are optional.

## Nested Fields

Access nested fields using dot notation:

```sh
# Simple nested field
hl -f user.id=12345 application.log

# Deep nesting
hl -f request.headers.content-type=application/json application.log

# Nested with operators
hl -f 'response.error.code!=404' application.log
```

For JSON logs like:
```json
{"user": {"id": 12345, "name": "Alice"}}
```

You can filter with:
```sh
hl -f user.id=12345 application.log
hl -f user.name=Alice application.log
```

## Array Fields

### Any Element Match (`[]`)

Check if any element in an array matches:

```sh
# Any tag equals "error"
hl -f 'tags.[]=error' application.log

# Any IP in list
hl -f 'ip_addresses.[]=192.168.1.1' application.log
```

For JSON like:
```json
{"tags": ["info", "database", "slow"]}
```

This matches:
```sh
hl -f 'tags.[]=database' application.log
```

### Specific Index (`[N]`)

Access a specific array element (0-based):

```sh
# First element
hl -f 'tags.[0]=critical' application.log

# Second element
hl -f 'users.[1].name=admin' application.log

# Third element with nested field
hl -f 'items.[2].price=9.99' application.log
```

### Nested Objects in Arrays

```sh
# Any user with role=admin
hl -f 'users.[].role=admin' application.log

# Specific user's status
hl -f 'users.[0].status=active' application.log
```

For JSON like:
```json
{"users": [{"name": "Alice", "role": "admin"}, {"name": "Bob", "role": "user"}]}
```

## Multiple Filters (AND Logic)

All filters must match:

```sh
# Service=api AND environment=production
hl -f service=api -f environment=production application.log

# Service=api AND status!=200
hl -f service=api -f 'status!=200' application.log

# Three conditions
hl -f service=api -f method=POST -f 'status~=50' application.log
```

## Common Filtering Patterns

### Service/Component Filtering

```sh
# Specific service
hl -f service=payment application.log

# Specific component
hl -f component=database application.log

# Exclude component
hl -f 'component!=health-check' application.log
```

### HTTP Status Filtering

```sh
# Success
hl -f status=200 application.log

# Client errors (4xx)
hl -f 'status~=4' application.log

# Not successful
hl -f 'status!=200' application.log
```

### User/Request Tracking

```sh
# Specific user
hl -f user.id=12345 application.log

# Specific request
hl -f request.id=abc-123-def application.log

# Specific session
hl -f session.id=xyz789 application.log
```

### Environment Filtering

```sh
# Production only
hl -f environment=production application.log

# Exclude development
hl -f 'environment!=development' application.log
```

### Error Filtering

```sh
# Has error field
hl -f 'error~=' application.log

# Specific error type
hl -f error.type=DatabaseError application.log

# No errors (requires field to exist)
hl -f error= application.log
```

## Combining with Other Filters

### Field + Level

```sh
# Errors from payment service
hl -l e -f service=payment application.log
```

### Field + Time Range

```sh
# API calls in last hour
hl -f service=api --since -1h application.log
```

### Field + Query

```sh
# API service with slow requests
hl -f service=api -q 'duration > 0.5' application.log
```

### Multiple Field Filters + Level + Time

```sh
hl -l w \
   -f service=api \
   -f environment=production \
   --since -1h \
   application.log
```

## Case Sensitivity

Field filters are **case-sensitive** for values:

```sh
# These are different
hl -f method=GET application.log
hl -f method=get application.log

# Field names are also case-sensitive
hl -f Status=200 application.log  # Different from status=200
```

## Quoting Values

Use quotes for values with special characters or spaces:

```sh
# Spaces
hl -f 'message=Connection timeout' application.log

# Special characters
hl -f 'path=/api/v1/users?active=true' application.log

# Shell-safe
hl -f "user.name=O'Brien" application.log
```

## Performance Tips

1. **Use field filters for simple equality** - Faster than queries:
   ```sh
   # Prefer this
   hl -f service=api application.log
   
   # Over this
   hl -q 'service = "api"' application.log
   ```

2. **Apply most selective filters first** - Though order doesn't matter for correctness, hl is optimized

3. **Combine with level filtering** - Quick reduction of data:
   ```sh
   hl -l e -f service=api application.log
   ```

4. **Use time ranges** - Dramatically reduces processing:
   ```sh
   hl -f service=api --since -1h application.log
   ```

## Examples by Use Case

### Debugging Specific Service

```sh
# All logs from payment service
hl -f service=payment application.log

# Payment service errors
hl -l e -f service=payment application.log

# Payment service excluding health checks
hl -f service=payment -f 'endpoint!=/health' application.log
```

### Tracking Requests

```sh
# Specific request ID
hl -f request.id=abc-123 application.log

# All requests from user
hl -f user.id=12345 application.log

# Requests with specific trace
hl -f trace.id=xyz789 application.log
```

### HTTP API Monitoring

```sh
# All POST requests
hl -f method=POST application.log

# Non-GET requests
hl -f 'method!=GET' application.log

# Specific endpoint
hl -f path=/api/v1/users application.log

# Endpoint pattern
hl -f 'path~=/api/v1' application.log
```

### Error Investigation

```sh
# Entries with errors
hl -f 'error~=' application.log

# Specific error type
hl -f error.type=ValidationError application.log

# Database errors
hl -f 'error.message~=database' application.log
```

### Multi-Tenant Applications

```sh
# Specific tenant
hl -f tenant.id=acme-corp application.log

# Exclude system tenant
hl -f 'tenant.id!=system' application.log

# Tenant with errors
hl -l e -f tenant.id=acme-corp application.log
```

## Troubleshooting

### No Results

If you get no output:

1. **Check field exists**:
   ```sh
   hl --raw application.log | head
   ```

2. **Check field name casing**:
   ```sh
   # Try different cases
   hl -f Service=api application.log
   hl -f service=api application.log
   ```

3. **Check value format**:
   ```sh
   # Numeric vs string
   hl -f status=200 application.log
   hl -f status="200" application.log
   ```

4. **Remove filters one by one** to find which is too restrictive

### Unexpected Results

If you see unexpected entries:

1. **Remember `!=` requires field to exist** - Use `?!=` to include absent

2. **Check for partial matches** with `~=` operator

3. **Verify nested field syntax**:
   ```sh
   # Correct
   hl -f user.name=Alice application.log
   
   # Incorrect
   hl -f user[name]=Alice application.log
   ```

### Field Not Filtering

1. **Check if field is nested**:
   ```sh
   # May need dot notation
   hl -f response.status=200 application.log
   ```

2. **Check if field is in array**:
   ```sh
   # May need array syntax
   hl -f 'tags.[]=error' application.log
   ```

## Limitations

1. **No OR logic** - All filters must match (use `-q` for OR):
   ```sh
   # This is AND (both must match)
   hl -f service=api -f service=web application.log  # No matches!
   
   # Use query for OR
   hl -q 'service = "api" or service = "web"' application.log
   ```

2. **No regex** - Use query with `match` operator:
   ```sh
   hl -q 'message match "error|warning"' application.log
   ```

3. **No numeric comparisons** - Use query:
   ```sh
   hl -q 'status >= 400' application.log
   ```

4. **No wildcards** - Use `~=` for substring or query with `like`:
   ```sh
   hl -q 'path like "/api/v*"' application.log
   ```

## When to Use Field Filters vs Queries

### Use Field Filters (`-f`) When:
- ✓ Checking exact equality or inequality
- ✓ Simple substring matching
- ✓ Filtering by known field values
- ✓ Combining multiple simple conditions (AND)
- ✓ Performance is critical

### Use Queries (`-q`) When:
- ✓ Need OR logic
- ✓ Need numeric comparisons (`>`, `<`, `>=`, `<=`)
- ✓ Need regex or wildcard matching
- ✓ Need complex nested conditions
- ✓ Need field existence checks

See [Complex Queries](./filtering-queries.md) for advanced filtering.

## Best Practices

1. **Start with one filter** - Add more incrementally:
   ```sh
   hl -f service=api application.log
   hl -f service=api -f method=POST application.log
   ```

2. **Use meaningful field names** from your logs

3. **Quote complex values** to avoid shell interpretation:
   ```sh
   hl -f 'message=Error: Connection failed' application.log
   ```

4. **Combine with level filtering** for quick reduction:
   ```sh
   hl -l e -f service=payment application.log
   ```

5. **Use `?` modifier for optional fields** in sparse logs:
   ```sh
   hl -f 'error?!=' application.log  # Include entries without error field
   ```

## Next Steps

- [Filtering by Time Range](./filtering-time.md) - Time-based filtering
- [Complex Queries](./filtering-queries.md) - Advanced query language
- [Filtering Examples](../examples/filtering.md) - Real-world scenarios
- [Filtering Overview](./filtering.md) - All filtering methods