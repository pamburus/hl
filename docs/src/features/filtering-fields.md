# Filtering by Field Values

Field filtering allows you to show only log entries where specific fields match certain conditions. This is one of the most common and useful filtering techniques in hl.

## Configuration

| Method | Setting |
|--------|---------|
| CLI option | [`-f, --filter`](../reference/options.md#filter) |

## Basic Syntax

Use the `-f` or `--filter` option to filter by field values:

```sh
hl -f KEY=VALUE app.log
```

You can specify multiple filters, and all must match (AND logic):

```sh
hl -f service=api -f environment=production app.log
```

## Filter Operators

### Exact Match (`=`)

Show entries where a field equals a specific value:

```sh
# Single value
hl -f status=200 app.log

# String value
hl -f method=GET app.log

# Numeric value
hl -f port=8080 app.log
```

### Not Equal (`!=`)

Show entries where a field exists and is NOT equal to a value:

```sh
# Exclude specific status
hl -f 'status!=200' app.log

# Exclude specific method
hl -f 'method!=GET' app.log
```

**Important:** This only matches entries that **have** the field. Entries without the field are excluded.

### Contains Substring (`~=`)

Show entries where a field contains a substring:

```sh
# Contains substring
hl -f 'message~=error' app.log

# Contains /api/v1/ in URL
hl -f 'url~=/api/v1/' app.log

# Case-sensitive
hl -f 'user~=Admin' app.log
```

### Does Not Contain (`!~=`)

Show entries where a field exists and does NOT contain a substring:

```sh
# Does not contain
hl -f 'message!~=debug' app.log

# Exclude pattern
hl -f 'path!~=/health' app.log
```

**Important:** Like `!=`, this requires the field to exist.

## Include Absent Modifier (`?`)

By default, field filters only match entries that **have** the field. The `?` modifier changes this behavior to include entries where the field is absent.

### Without `?` (Default)

```sh
# Only matches entries that HAVE status field AND status != 200
hl -f 'status!=200' app.log
```

Entries without a `status` field are **excluded**.

### With `?` (Include Absent)

```sh
# Matches entries WHERE status != 200 OR status field is absent
hl -f 'status?!=200' app.log
```

Entries without a `status` field are **included**.

### Common Use Cases

```sh
# Show entries with status=error OR no status field
hl -f 'status?=error' app.log

# Show entries without 'ok' status OR no status field
hl -f 'status?!=ok' app.log

# Show entries with price=0 OR no price field
hl -f 'price?=0' app.log
```

This is particularly useful for logfmt or logs where fields are optional.

## Nested Fields

Access nested fields using dot notation:

```sh
# Simple nested field
hl -f user.id=12345 app.log

# Deep nesting
hl -f request.headers.content-type=application/json app.log

# Nested with operators
hl -f 'response.error.code!=404' app.log
```

For JSON logs like:
```json
{"user": {"id": 12345, "name": "Alice"}}
```

You can filter with:
```sh
hl -f user.id=12345 app.log
hl -f user.name=Alice app.log
```

## Array Fields

### Any Element Match (`[]`)

Check if any element in an array matches:

```sh
# Any tag equals "error"
hl -f 'tags.[]=error' app.log

# Match an IP address in the list of addresses
hl -f 'addresses.[]=192.168.1.1' app.log
```

For JSON like:
```json
{"tags": ["info", "database", "slow"]}
```

This matches:
```sh
hl -f 'tags.[]=database' app.log
```

### Specific Index (`[N]`)

Access a specific array element (0-based):

```sh
# First element
hl -f 'tags.[0]=critical' app.log

# Second element
hl -f 'users.[1].name=admin' app.log

# Third element with nested field
hl -f 'items.[2].price=9.99' app.log
```

### Nested Objects in Arrays

```sh
# Any user with role=admin
hl -f 'users.[].role=admin' app.log

# Specific user's status
hl -f 'users.[0].status=active' app.log
```

For JSON like:
```json
{"users": [{"name": "Alice", "role": "admin"}, {"name": "Bob", "role": "user"}]}
```

## Multiple Filters (AND Logic)

All filters must match:

```sh
# Service=api AND environment=production
hl -f service=api -f environment=production app.log

# Service=api AND status!=200
hl -f service=api -f 'status!=200' app.log

# Three conditions
hl -f service=api -f method=POST -f 'status~=50' app.log
```

## Common Filtering Patterns

### Service/Component Filtering

```sh
# Specific service
hl -f service=payment app.log

# Specific component
hl -f component=database app.log

# Exclude component
hl -f 'component!=health-check' app.log
```

### HTTP Status Filtering

```sh
# Success
hl -f status=200 app.log

# Client errors (4xx)
hl -f 'status~~=^4' app.log

# Not successful
hl -f 'status!=200' app.log
```

### User/Request Tracking

```sh
# Specific user
hl -f user.id=12345 app.log

# Specific request
hl -f request.id=abc-123-def app.log

# Specific session
hl -f session.id=xyz789 app.log
```

### Environment Filtering

```sh
# Production only
hl -f environment=production app.log

# Exclude development
hl -f 'environment!=development' app.log
```

### Error Filtering

```sh
# Has error field
hl -f 'error~=' app.log

# Specific error code
hl -f error.code=DatabaseError app.log
```

## Combining with Other Filters

### Field + Level

```sh
# Errors from payment service
hl -l e -f service=payment app.log
```

### Field + Time Range

```sh
# API calls in last hour
hl -f service=api --since -1h app.log
```

### Field + Query

```sh
# API service with slow requests
hl -f service=api -q 'duration > 0.5' app.log
```

### Multiple Field Filters + Level + Time

```sh
hl -l w \
   -f service=api \
   -f environment=production \
   --since -1h \
   app.log
```

## Case Sensitivity

Field filters are **case-sensitive** for values:

```sh
# These are different
hl -f method=GET app.log
hl -f method=get app.log

# Field names are also case-sensitive
hl -f Status=200 app.log  # Different from status=200
```

## Quoting Values

Use quotes for values with special characters or spaces:

```sh
# Spaces
hl -f 'message=Connection timeout' app.log

# Special characters
hl -f 'url=/api/v1/users?active=true' app.log

# Shell-safe
hl -f "user.name=O'Brien" app.log
```

## Performance Tips

1. **Use chronologocal sorting** - Quick builds index for time-based logs:
   ```sh
   hl -s app.log
   ```

1. **Combine with level filtering** - Quick reduction of data:
   ```sh
   hl -s -l e -f service=api app.log
   ```

2. **Use time ranges** - Dramatically reduces processing:
   ```sh
   hl -s -f service=api --since -1h app.log
   ```

## Examples by Use Case

### Debugging Specific Service

```sh
# All logs from payment service
hl -f service=payment app.log

# Payment service errors
hl -l e -f service=payment app.log

# Payment service excluding health checks
hl -f service=payment -f 'endpoint!=/health' app.log
```

### Tracking Requests

```sh
# Specific request ID
hl -f request.id=abc-123 app.log

# All requests from user
hl -f user.id=12345 app.log

# Requests with specific trace
hl -f trace.id=xyz789 app.log
```

### HTTP API Monitoring

```sh
# All POST requests
hl -f method=POST app.log

# Non-GET requests
hl -f 'method!=GET' app.log

# Specific endpoint
hl -f path=/api/v1/users app.log

# Endpoint pattern
hl -f 'path~=/api/v1' app.log
```

### Error Investigation

```sh
# Entries with errors
hl -f 'error~=' app.log

# Specific error type
hl -f error.type=ValidationError app.log

# Database errors
hl -f 'error.message~=database' app.log
```

### Multi-Tenant Applications

```sh
# Specific tenant
hl -f tenant.name=acme-corp app.log

# Exclude system tenant
hl -f 'tenant.name!=system' app.log

# Tenant with errors
hl -l e -f tenant.name=acme-corp app.log
```

## Troubleshooting

### No Results

If you get no output:

1. **Check field exists**:
   ```sh
   hl --raw app.log | head
   ```

2. **Check field name casing**:
   ```sh
   # Try different cases
   hl -f Service=api app.log
   hl -f service=api app.log
   ```

3. **Remove filters one by one** to find which is too restrictive

### Unexpected Results

If you see unexpected entries:

1. **Remember `!=` requires field to exist** - Use `?!=` to include absent

2. **Check for partial matches** with `~=` operator

3. **Verify nested field syntax**:
   ```sh
   # Correct
   hl -f user.name=Alice app.log
   
   # Incorrect
   hl -f user[name]=Alice app.log
   ```

### Field Not Filtering

1. **Check if field is nested**:
   ```sh
   # May need dot notation
   hl -f response.status=200 app.log
   ```

2. **Check if field is in array**:
   ```sh
   # May need array syntax
   hl -f 'tags.[]=error' app.log
   ```

## Limitations

1. **No OR logic** - All filters must match (use `-q` for OR):
   ```sh
   # This is AND (both must match)
   hl -f service=api -f service=web app.log  # No matches!
   
   # Use query for OR
   hl -q 'service = "api" or service = "web"' app.log
   ```

2. **No numeric comparisons** - Use query:
   ```sh
   hl -q 'status >= 400' app.log
   ```

3. **No wildcards** - Use `~=` for substring or query with `like`:
   ```sh
   hl -q 'path like "/api/v*/*"' app.log
   ```

## When to Use Field Filters vs Queries

### Use Field Filters (`-f`) When:
- ✓ Checking exact equality or inequality
- ✓ Simple substring matching
- ✓ Filtering by known field values
- ✓ Combining multiple simple conditions (AND)

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
   hl -f service=api app.log
   hl -f service=api -f method=POST app.log
   ```

2. **Use meaningful field names** from your logs

3. **Quote complex values** to avoid shell interpretation:
   ```sh
   hl -f 'message=Error: Connection failed' app.log
   ```

4. **Combine with level filtering** for quick reduction:
   ```sh
   hl -l e -f service=payment app.log
   ```

5. **Use `?` modifier for optional fields** in sparse logs:
   ```sh
   hl -f 'error?!=' app.log  # Include entries without error field
   ```

## Next Steps

- [Filtering by Time Range](./filtering-time.md) - Time-based filtering
- [Complex Queries](./filtering-queries.md) - Advanced query language
- [Filtering Examples](../examples/filtering.md) - Real-world scenarios
- [Filtering Overview](./filtering.md) - All filtering methods
