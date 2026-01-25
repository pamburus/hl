# Filtering Examples

This page demonstrates practical filtering scenarios. For detailed syntax documentation, see:
- [Filtering by Log Level](../features/filtering-level.md)
- [Filtering by Field Values](../features/filtering-fields.md)
- [Filtering by Time Range](../features/filtering-time.md)
- [Complex Queries](../features/filtering-queries.md)

## Common Scenarios

### Debugging a Production Issue

```sh
# Start broad: recent errors
hl -l error --since -1h app.log

# Narrow to specific service
hl -l error --since -1h -f 'service = api' app.log

# Find the specific request
hl -f 'request-id = "abc-123"' app.log
```

### Investigating Slow Requests

```sh
# Find slow requests
hl -q 'duration > 1000' app.log

# Slow requests from a specific endpoint
hl -q 'duration > 1000 and path like "/api/v1/*"' app.log

# Slow or failed requests
hl -q 'duration > 1000 or status >= 500' app.log
```

### Tracking User Activity

```sh
# All activity for a user
hl -f 'user.id = 12345' app.log

# User activity in a time window
hl -f 'user.id = 12345' --since "2024-01-15 10:00" --until "2024-01-15 12:00" app.log
```

### Monitoring Authentication

```sh
# Failed login attempts
hl -f 'event = login and success = false' auth.log

# Failed logins in the last hour
hl -f 'event = login and success = false' --since -1h auth.log
```

### Filtering Out Noise

```sh
# Exclude health checks
hl -q 'path != "/health" and path != "/ping"' app.log

# Exclude debug logs from noisy component
hl -q 'not (level = debug and component = "cache")' app.log
```

### Multi-Service Debugging

```sh
# Trace a request across services
hl -s -f 'trace-id = "xyz-789"' api.log worker.log scheduler.log

# Errors from any of several services
hl -l error -q 'service in ("api", "worker", "scheduler")' *.log
```

## Combining Filters

Filters combine with AND logic. Layer them for precision:

```sh
# Level + time
hl -l error --since -1h app.log

# Level + time + field
hl -l error --since -1h -f 'service = api' app.log

# Level + time + field + query
hl -l error --since -1h -f 'service = api' -q 'status >= 500' app.log
```

## Piped Input

Filtering works the same with piped input:

```sh
kubectl logs my-pod | hl -P -l error
docker logs my-container | hl -P -q 'status >= 400'
```

## Next Steps

- [Query Examples](queries.md) — Complex query patterns
- [Time Filtering](time-filtering.md) — Time range specifications
- [Live Monitoring](live-monitoring.md) — Real-time filtering with follow mode