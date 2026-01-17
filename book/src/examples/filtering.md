# Filtering Examples

This page demonstrates how to filter log entries by level, field values, and other criteria.

## Filtering by Log Level

### Show Only Errors

Display only error-level entries:

```hl/dev/null/shell.sh#L1
hl --level error app.log
```

Short form:

```hl/dev/null/shell.sh#L1
hl -l error app.log
```

### Show Warnings and Errors

Show entries at warning level or higher:

```hl/dev/null/shell.sh#L1
hl --level warn app.log
```

This includes both `warning` and `error` level entries.

### Show Everything Except Debug

Show all entries except debug level:

```hl/dev/null/shell.sh#L1
hl --level info app.log
```

This shows `info`, `warning`, and `error` entries, but excludes `trace` and `debug`.

### Multiple Level Filters

You can't specify multiple `--level` options (the last one wins), but you can use queries for complex level filtering. See [Query Examples](queries.md) for details.

## Filtering by Field Values

### Filter by Exact Field Value

Show only entries where `service` equals `"api"`:

```hl/dev/null/shell.sh#L1
hl --filter 'service = api' app.log
```

**Note**: Field names use hyphens. Underscores and hyphens are interchangeable when querying (e.g., `user-id` matches both `user-id` and `user_id` in the source), but examples use hyphens for consistency with display output.

Or using the short form:

```hl/dev/null/shell.sh#L1
hl -f 'service = api' app.log
```

### Filter by Multiple Fields

Combine filters with `and`:

```hl/dev/null/shell.sh#L1
hl -f 'service = api and environment = production' app.log
```

### Filter with OR Logic

Show entries matching any of several conditions:

```hl/dev/null/shell.sh#L1
hl -f 'service = api or service = web' app.log
```

### Numeric Comparisons

Filter by numeric field values:

```hl/dev/null/shell.sh#L1
# Show requests taking longer than 1 second
hl -f 'duration > 1000' app.log

# Show HTTP errors (status >= 400)
hl -f 'status >= 400' app.log

# Show successful responses
hl -f 'status >= 200 and status < 300' app.log
```

### String Matching

Filter using string patterns:

```hl/dev/null/shell.sh#L1
# Contains substring (case-sensitive)
hl -f 'message ~= "database"' app.log

# Regex matching (case-sensitive)
hl -f 'url ~~ "^/api/v[0-9]+"' app.log
```

### Filtering by Field Presence

Show entries that have a specific field:

```hl/dev/null/shell.sh#L1
hl -f 'exists(error)' app.log
```

Show entries that don't have a field:

```hl/dev/null/shell.sh#L1
hl -f 'not exists(user-id)' app.log
```

### Nested Field Filtering

Filter by nested JSON fields:

```hl/dev/null/shell.sh#L1
# Dot notation for nested fields
hl -f 'user.id = 12345' app.log

# Deeper nesting
hl -f 'request.headers.authorization ~= "Bearer"' app.log
```

## Combining Level and Field Filters

Combine log level and field filters:

```hl/dev/null/shell.sh#L1
# Show errors from the API service
hl -l error -f 'service = api' app.log

# Show warnings and errors with slow responses
hl -l warn -f 'duration > 2000' app.log
```

## Filtering by Time Range

### Show Logs After a Specific Time

Display entries after a given timestamp:

```hl/dev/null/shell.sh#L1
hl --since "2024-01-15 10:00:00" app.log
```

### Show Logs Before a Specific Time

Display entries before a given timestamp:

```hl/dev/null/shell.sh#L1
hl --until "2024-01-15 18:00" app.log
```

### Show Logs in a Time Range

Combine `--since` and `--until`:

```hl/dev/null/shell.sh#L1
hl --since "2024-01-15 10:00" --until "2024-01-15 12:00:00" app.log
```

### Relative Time Filters

Use relative time specifications:

```hl/dev/null/shell.sh#L1
# Last hour
hl --since "1h ago" app.log

# Last 30 minutes
hl --since "30m ago" app.log

# Last 5 days
hl --since "5d ago" app.log
```

See [Time Filtering](time-filtering.md) for more time format examples.

## Practical Filtering Examples

### Find Failed Login Attempts

```hl/dev/null/shell.sh#L1
hl -f 'event = login and success = false' auth.log
```

### Monitor High-Error-Rate Periods

```hl/dev/null/shell.sh#L1
hl -l error --since "1h ago" app.log
```

### Debug a Specific User Session

```hl/dev/null/shell.sh#L1
hl -f 'session-id = "abc123xyz"' app.log
```

### Find Slow Database Queries

```hl/dev/null/shell.sh#L1
hl -f 'query-time > 1000 and operation ~= "SELECT"' db.log
```

### Show API Errors by Endpoint

```hl/dev/null/shell.sh#L1
hl -l error -f 'path ~= "/api/"' app.log
```

### Filter Multiple Services

```hl/dev/null/shell.sh#L1
hl -f 'service in ["api", "worker", "scheduler"]' app.log
```

### Exclude Health Checks

```hl/dev/null/shell.sh#L1
hl -f 'not (path = "/health" or path = "/ping")' app.log
```

### Show Only Production Errors

```hl/dev/null/shell.sh#L1
hl -l error -f 'env = production' app.log
```

### Find Entries with High Memory Usage

```hl/dev/null/shell.sh#L1
hl -f 'memory-mb > 500' app.log
```

### Complex Multi-Condition Filter

```hl/dev/null/shell.sh#L1
hl -f '(level >= warn and service = api) or (duration > 5000)' app.log
```

## Filtering Piped Input

When reading from a pipe, filtering works the same way:

```hl/dev/null/shell.sh#L1
kubectl logs my-pod | hl -P -l error

docker logs my-container | hl -P -f 'status >= 400'

tail -f app.log | hl -P --since "5m ago"
```

## Performance Tips

- **Field filters** are applied during parsing, so they're very efficient.
- **Time filters** (`--since`, `--until`) benefit from the index when using sorted files.
- **Combine filters** to reduce the dataset early:
  ```hl/dev/null/shell.sh#L1
  hl -l error --since "1h ago" -f 'service = api' large.log
  ```

## Next Steps

- [Query Examples](queries.md) — Advanced query syntax and complex filtering.
- [Time Filtering](time-filtering.md) — Detailed time range specifications.
- [Field Management](field-management.md) — Control which fields are displayed in filtered results.
