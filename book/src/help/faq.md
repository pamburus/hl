# FAQ

This page answers frequently asked questions about using `hl`.

## General Questions

### What log formats does `hl` support?

`hl` supports:
- **JSON**: Including pretty-printed and compact JSON
- **logfmt**: Key-value pairs in logfmt format
- **Auto-detection**: Automatically detects the format by default

See [Supported Formats](../getting-started/supported-formats.md) for details.

### Does `hl` work with compressed files?

Yes! `hl` automatically detects and decompresses common formats:
- **gzip** (`.gz`)
- **bzip2** (`.bz2`)
- **xz** (`.xz`)

```bash
# Works automatically
hl app.log.gz
hl app.log.bz2
```

### Can I use `hl` with stdin?

Yes, `hl` reads from stdin when no files are specified:

```bash
kubectl logs my-pod | hl
docker logs container-id | hl
cat app.log | hl --level error
```

### How do I disable the pager?

Use `-P` or `--paging=never`:

```bash
# Short form
hl -P app.log

# Long form
hl --paging=never app.log
```

This is especially useful in scripts and pipelines.

## Filtering and Querying

### What's the difference between `--filter` and `--query`?

- **`--filter` (`-f`)**: Simple field matching with basic operators (`=`, `~=`, `~~=`)
- **`--query` (`-q`)**: Complex expressions with boolean logic, comparisons, set membership, etc.

```bash
# Simple filter
hl -f 'status=500' app.log

# Complex query
hl -q 'status>=500 and method in (POST,PUT)' app.log
```

Use `--filter` for simple cases, `--query` when you need boolean logic or comparisons.

See [Query Syntax](../reference/query-syntax.md) for complete details.

### How do I filter by multiple values?

Use set membership in a query:

```bash
# Multiple status codes
hl -q 'status in (500,502,503,504)' app.log

# Multiple HTTP methods
hl -q 'method in (POST,PUT,DELETE)' app.log
```

### How do I search for text in log messages?

Use the substring operator (`~=` or `contains`):

```bash
# Using operator
hl -f 'message~=timeout' app.log

# Using query with 'contains'
hl -q 'message contains "connection refused"' app.log
```

For regular expressions, use `~~=` or `matches`:

```bash
hl -q 'message matches "error.*timeout"' app.log
```

### How do I combine multiple filters?

Use multiple `--filter` options (they are ANDed together):

```bash
# Both conditions must match
hl -f 'status=500' -f 'method=POST' app.log
```

Or use `--query` for more complex logic:

```bash
# AND logic
hl -q 'status=500 and method=POST' app.log

# OR logic
hl -q 'status>=500 or duration>10' app.log

# Complex expression
hl -q '(status>=500 and method=POST) or level=error' app.log
```

### How do I exclude certain entries?

Use negation in filters or queries:

```bash
# Using filter (negate with !)
hl -f 'status!=200' app.log

# Using query (negate with != or 'not')
hl -q 'status!=200' app.log
hl -q 'not status=200' app.log

# Exclude substring
hl -q 'message not contains "health check"' app.log
```

## Timestamps and Sorting

### What timestamp formats does `hl` support?

`hl` supports:
- **RFC 3339**: `2024-01-15T10:30:45.123Z`
- **ISO 8601-like with space**: `2024-01-15 10:30:45.123Z` (relaxed variant)
- **Unix timestamps**: Seconds, milliseconds, microseconds, nanoseconds

The timezone component is mandatory for RFC 3339-like timestamps.

See [Timestamp Formats](../features/timestamps.md) for complete details.

### How do I sort logs from multiple files chronologically?

Use the `--sort` flag:

```bash
hl --sort app1.log app2.log app3.log
```

This builds a timestamp index and merges entries chronologically across all files.

### What happens to entries without timestamps when sorting?

Entries without recognized timestamps are **discarded** in `--sort` mode.

If you need to preserve all entries, use streaming mode (no `--sort`):

```bash
# Preserves all entries
hl app.log
```

### How do I change the timezone for displayed timestamps?

Use `--time-zone` or `--local`:

```bash
# Specific timezone
hl --time-zone "America/New_York" app.log

# Local system timezone
hl --local app.log
```

**Note**: The value `local` is not a valid timezone. Use `--local` instead of `--time-zone local`.

### How do I change the timestamp display format?

Use `--time-format` with strftime format specifiers:

```bash
# ISO 8601 format
hl --time-format "%Y-%m-%dT%H:%M:%S%z" app.log

# Custom format
hl -t "%b %d %H:%M:%S" app.log
```

See `man date` or [strftime documentation](https://man7.org/linux/man-pages/man1/date.1.html) for format specifiers.

## Output and Formatting

### How do I get the original JSON/logfmt output?

Use `--raw` (or `-r`):

```bash
# Output original JSON for matching entries
hl --raw -q 'status>=500' app.log
```

Filtering still applies, but the output is in the original format.

### How do I pipe `hl` output to `jq` or other JSON tools?

Use `--raw` with `--input-info json`:

```bash
# Clean JSON output for strict processors
hl --raw --input-info json -q 'status>=500' app.log | jq '.status'
```

This ensures only valid JSON objects are output (no logfmt, no prefix text).

### How do I hide specific fields?

Use `--hide`:

```bash
# Hide one field
hl --hide caller app.log

# Hide multiple fields
hl --hide caller --hide pid app.log
```

See [Hiding Fields](../features/hiding-fields.md) for details.

### How do I show only specific fields?

Hide all fields, then reveal the ones you want:

```bash
# Show only message and level
hl --hide '*' --hide '!message' --hide '!level' app.log
```

Or configure default hidden fields in your config file.

### Can I customize the colors?

Yes! Use themes:

```bash
# List available themes
hl --list-themes

# Use a specific theme
hl --theme hl-light app.log
```

You can also create custom themes or theme overlays. See [Themes](../customization/themes.md) for details.

## Performance

### Why is `--sort` slow on large files?

`--sort` builds a timestamp index before processing. For very large files (>10GB), this takes time.

Strategies for large files:
- Use time ranges: `--since` and `--until`
- Pre-filter with `ripgrep` (`rg`) before piping to `hl` (much faster than `grep`)
- Increase `--concurrency` for faster processing

See [Performance Tips](../reference/performance.md) for optimization strategies.

### How do I make filtering faster?

For best filtering performance:

1. **Use `--sort` with `--level`**: Index-based level filtering is extremely fast
   ```bash
   hl --sort --level error large-file.log
   ```

2. **Use simple filters** when possible instead of complex queries

3. **Use set membership** instead of multiple OR conditions:
   ```bash
   # Fast
   hl -q 'status in (500,502,503)' app.log
   
   # Slower
   hl -q 'status=500 or status=502 or status=503' app.log
   ```

### How do I reduce memory usage?

- Decrease `--buffer-size`: `hl --buffer-size "64 KiB" app.log`
- Decrease `--concurrency`: `hl --concurrency 2 app.log`
- Use streaming mode instead of `--sort` when possible

## Configuration

### Where is the configuration file located?

`hl` looks for configuration in these locations (in order):

1. Path specified by `--config` option
2. Path specified by `HL_CONFIG` environment variable
3. `~/.config/hl/config.yaml` (or `$XDG_CONFIG_HOME/hl/config.yaml`)
4. `~/.hl/config.yaml`

See [Configuration Files](../customization/config-files.md) for details.

### How do I create a configuration file?

Create `~/.config/hl/config.yaml`:

```bash
mkdir -p ~/.config/hl
cat > ~/.config/hl/config.yaml << 'EOF'
theme: hl-dark
hide-empty-fields: true
time-zone: America/New_York
EOF
```

See [Configuration Files](../customization/config-files.md) for all available options.

### Can I override configuration with environment variables?

Yes, many options support environment variables:

```bash
export HL_THEME=hl-light
export HL_LEVEL=warn
export HL_TIME_ZONE=UTC

hl app.log
```

Command-line options override environment variables, which override config file settings.

### How do I reset all configuration?

Use `--config -` to ignore all configuration:

```bash
hl --config - app.log
```

This uses only default settings and command-line options.

## Troubleshooting

### Why aren't my logs being parsed?

Common causes:

1. **Unsupported format**: `hl` only supports JSON and logfmt
2. **Mixed formats**: File contains both JSON and logfmt (use `--input-format` to force one)
3. **Malformed entries**: Check for incomplete JSON objects or syntax errors

Try viewing in raw mode to see what `hl` is receiving:

```bash
head app.log
```

### Why are some entries missing in `--sort` mode?

Entries without recognized timestamps are discarded in `--sort` mode.

Check if your timestamps are in a supported format. If not, use streaming mode:

```bash
hl app.log  # Preserves all entries
```

### Why isn't my filter matching anything?

Common issues:

1. **Field name mismatch**: Check the actual field names in your logs
2. **Case sensitivity**: Filters are case-sensitive by default
3. **Nested fields**: Use dot notation: `user.name=alice`
4. **Wrong operator**: Use `~=` for substring match, not `=`

View formatted output to see actual field names:

```bash
hl app.log | head
```

### How do I see what fields are available?

Just run `hl` on a sample entry:

```bash
head -1 app.log | hl
```

This shows all fields with their values.

### Why isn't my theme working?

Check:

1. **Theme exists**: Run `hl --list-themes` to see available themes
2. **Terminal support**: Some themes require 256-color or truecolor support
3. **Configuration**: Make sure theme is set correctly in config or via `--theme`

Test theme support:

```bash
# Check TERM variable
echo $TERM

# Try a different theme
hl --theme hl-dark app.log
```

## Advanced Usage

### Can I use `hl` in follow mode like `tail -f`?

Yes, use `--follow` (or `-F`):

```bash
hl --follow app.log
```

This is similar to `tail -f` but with chronological sorting within a time window.

### How do I process logs from multiple sources simultaneously?

Use process substitution:

```bash
hl --sort <(kubectl logs pod1) <(kubectl logs pod2)
```

Or merge logs and pipe to `hl`:

```bash
cat app1.log app2.log | hl
```

### Can I save filtered output to a file?

Yes, use `-o` or shell redirection:

```bash
# Using -o option
hl -q 'status>=500' app.log -o errors.log

# Using shell redirection
hl -q 'status>=500' app.log > errors.log
```

For JSON output, use `--raw`:

```bash
hl --raw -q 'status>=500' app.log > errors.json
```

### How do I configure custom timestamp field names?

Create or edit your config file (`~/.config/hl/config.yaml`):

```yaml
fields:
  predefined:
    time:
      names:
        - timestamp
        - time
        - ts
        - "@timestamp"  # Add custom field name
```

See [Configuration Files](../customization/config-files.md) for details.

### Can I use regular expressions in field names?

No, field names must be literal. However, you can use regular expressions in field **values**:

```bash
# Regex in value (not field name)
hl -q 'message matches "error.*timeout"' app.log
```

## Getting Help

### Where can I find more examples?

See:
- [Basic Examples](../examples/basic.md)
- [Filtering Examples](../examples/filtering.md)
- [Advanced Examples](../examples/advanced.md)

### How do I report a bug or request a feature?

Visit the [GitHub repository](https://github.com/pamburus/hl) to:
- Report bugs
- Request features
- Contribute improvements

### Where can I find the complete option reference?

See [Command-Line Options](../reference/options.md) for a complete reference of all options.

### How do I get help on the command line?

```bash
# Short help
hl --help

# Detailed help
hl --help=long
```
