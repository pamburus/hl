# Filtering

Filtering is one of hl's most powerful features, allowing you to focus on exactly the log entries you need. This page provides an overview of the different filtering methods available.

## Why Filter Logs?

Log files can contain thousands or millions of entries. Filtering helps you:

- Find specific events or errors
- Focus on relevant time periods
- Track particular services or components
- Investigate issues quickly
- Reduce noise and distractions

## Types of Filtering

hl provides several complementary filtering methods:

### 1. Log Level Filtering

Filter by severity level (trace, debug, info, warning, error):

```sh
hl -l e application.log
```

This is the quickest way to narrow down logs by importance. See [Filtering by Log Level](./filtering-level.md) for details.

### 2. Field-Based Filtering

Filter by specific field values:

```sh
hl -f service=api application.log
```

Perfect for focusing on a particular component, user, or request. Learn more in [Filtering by Field Values](./filtering-fields.md).

### 3. Time Range Filtering

Filter by timestamp range:

```sh
hl --since '2024-01-15 10:00:00' --until '2024-01-15 11:00:00' application.log
```

Essential for investigating incidents or analyzing specific time windows. See [Filtering by Time Range](./filtering-time.md).

### 4. Complex Queries

Build sophisticated filters with logical operators:

```sh
hl -q 'level >= warn and (status >= 400 or duration > 1)' application.log
```

The most flexible filtering option for complex scenarios. Covered in [Complex Queries](./filtering-queries.md).

## Combining Filters

All filter types can be combined:

```sh
hl -l w \
   -f service=api \
   --since -1h \
   -q 'duration > 0.5' \
   application.log
```

This shows warning-level or higher logs from the `api` service in the last hour where duration exceeds 0.5 seconds.

### How Filters Combine

When you use multiple filters, they work together using AND logic:

- **Level filter** AND **field filters** AND **time filters** AND **query filters**
- Within field filters (multiple `-f`): ALL must match (AND)
- Within queries: Use explicit `and`/`or` operators

## Filter Performance

hl is optimized for filtering:

- **Time range filters** are extremely fast when using sorted mode (`-s`)
- **Level filters** use bitmap indexing for quick elimination
- **Field filters** and **queries** are evaluated efficiently during parsing
- **Combined filters** short-circuit early when possible

## Common Filtering Patterns

### Finding Errors in a Time Window

```sh
hl -l e --since -2h application.log
```

### Excluding Debug Messages

```sh
hl -l i application.log
```

Info level and above (excludes trace and debug).

### Tracking a Specific Request

```sh
hl -f request.id=abc-123 application.log
```

### Finding Slow Operations

```sh
hl -q 'duration > 1.0' application.log
```

### Multiple Conditions

```sh
hl -q 'service = "api" and (status >= 400 or error != null)' application.log
```

### Investigating Recent Errors

```sh
hl -l e --since yesterday --until today application.log
```

## Filter Operators Quick Reference

### Field Filter Operators (`-f`)

- `key=value` - Exact match
- `key!=value` - Not equal
- `key~=substring` - Contains substring
- `key!~=substring` - Does not contain substring
- `key?=value` - Equals value OR field is absent
- `key?!=value` - Not equal to value OR field is absent

### Query Operators (`-q`)

- **Comparison**: `=`, `!=`, `>`, `>=`, `<`, `<=`
- **String**: `~=` (contains), `!~=`, `like` (wildcards), `match` (regex)
- **Logical**: `and`, `or`, `not`
- **Sets**: `in (...)`, `not in (...)`
- **Existence**: `exists(field)`

## Filtering Best Practices

1. **Start broad, then narrow** - Begin with level filtering, then add specifics
2. **Use time ranges** - Limit the data hl needs to process
3. **Combine filters** - More specific filters = faster results
4. **Test incrementally** - Add one filter at a time to verify results
5. **Use sorted mode for time filters** - Dramatically faster with `-s` flag

## Filter Examples by Use Case

### Security Monitoring

```sh
# Find failed authentication attempts
hl -q 'event = "auth_failed"' application.log

# Find access from suspicious IPs
hl -q 'ip in @suspicious-ips.txt' application.log
```

### Performance Analysis

```sh
# Find requests over 500ms
hl -q 'duration > 0.5' application.log

# Find slow database queries
hl -f component=database -q 'duration > 0.1' application.log
```

### Error Investigation

```sh
# All errors in the last hour
hl -l e --since -1h application.log

# Errors from specific service
hl -l e -f service=payment application.log

# Errors with stack traces
hl -l e -q 'exists(stack)' application.log
```

### User Activity Tracking

```sh
# All actions by specific user
hl -f user.id=12345 application.log

# User actions in time range
hl -f user.id=12345 --since '2024-01-15' --until '2024-01-16' application.log
```

### Multi-Service Debugging

```sh
# Trace a request across services
hl -f trace.id=xyz789 service1.log service2.log service3.log

# With chronological sorting
hl -s -f trace.id=xyz789 *.log
```

## Understanding Filter Results

When filters are applied:

- Matching entries are displayed normally
- Non-matching entries are completely omitted
- The total number of entries processed is not shown (for performance)
- Filters are case-sensitive by default for field values

## When to Use Each Filter Type

| Filter Type | Best For | Example |
|------------|----------|---------|
| Level (`-l`) | Quick severity filtering | `-l e` for errors only |
| Field (`-f`) | Specific field matching | `-f service=api` |
| Time (`--since/--until`) | Time-based investigation | `--since -2h` |
| Query (`-q`) | Complex conditions | `-q 'status >= 400 and duration > 1'` |

## Next Steps

Explore each filtering method in detail:

- [Filtering by Log Level](./filtering-level.md) - Master level-based filtering
- [Filtering by Field Values](./filtering-fields.md) - Learn field filter syntax
- [Filtering by Time Range](./filtering-time.md) - Work with time-based filters
- [Complex Queries](./filtering-queries.md) - Build sophisticated query expressions

See practical examples in the [Filtering Examples](../examples/filtering.md) section.