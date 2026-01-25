# Troubleshooting

This page provides solutions to common problems when using `hl`.

## Parsing Issues

### Logs Not Being Parsed

**Symptoms**: Logs appear as plain text instead of formatted output, or nothing is displayed.

**Common Causes**:

1. **Unsupported format**: `hl` only supports JSON and logfmt
   ```sh
   # Check if your logs are in a supported format
   head -5 app.log
   ```

2. **Mixed formats in one file**: File contains both JSON and logfmt entries
   ```sh
   # Force a specific format
   hl --input-format json app.log
   hl --input-format logfmt app.log
   ```

3. **Malformed JSON**: Incomplete or invalid JSON objects
   ```sh
   # Validate JSON
   head -1 app.log | jq .
   ```

4. **Unusual delimiters**: Logs use non-standard entry delimiters
   ```sh
   # Try different delimiters
   hl --delimiter lf app.log
   hl --delimiter nul app.log
   ```

**Solution**:
- Verify your log format matches JSON or logfmt
- Use `--input-format` to force a specific format
- Check for incomplete or malformed entries
- Adjust `--delimiter` if needed

### JSON with Prefixes Not Recognized

**Symptoms**: Lines like `2024-01-15 10:30:45 {"level":"info",...}` are not parsed.

**Solution**: Enable prefix handling:

```sh
hl --allow-prefix app.log
```

The prefix text is preserved in the output.

See [Non-JSON Prefixes](../features/prefixes.md) for details.

### Pretty-Printed JSON Not Detected

**Symptoms**: Multi-line pretty-printed JSON appears as separate entries or plain text.

**Solution**: The default auto-delimiter should handle this. If not, try:

```sh
# Auto-delimiter (default)
hl app.log

# If still not working, check for unusual formatting
cat app.log | head -20
```

The default delimiter handles pretty-printed JSON that starts with `{` and ends with `}` on separate lines.

## Filtering Issues

### Filter Not Matching Anything

**Symptoms**: Filter or query returns no results even though entries should match.

**Common Causes**:

1. **Field name mismatch**: The field name in your filter doesn't match the actual field name
   ```sh
   # View actual field names
   head -1 app.log | hl
   ```

2. **Case sensitivity**: Filters are case-sensitive
   ```sh
   # Won't match if actual value is "Error" with capital E
   hl -f 'level=error' app.log
   ```

3. **Wrong operator**: Using `=` for substring match instead of `~=`
   ```sh
   # Wrong: exact match
   hl -f 'message=timeout' app.log
   
   # Correct: substring match
   hl -f 'message~=timeout' app.log
   ```

4. **Nested fields**: Not using dot notation
   ```sh
   # Correct way to access nested fields
   hl -f 'user.name=alice' app.log
   ```

**Solution**:
- Check actual field names by viewing formatted output
- Ensure exact case match for values
- Use `~=` for substring matching
- Use dot notation for nested fields

### Query Syntax Errors

**Symptoms**: Error message about query parsing or invalid syntax.

**Common Issues**:

1. **Missing quotes around query**:
   ```sh
   # Wrong: shell interprets special characters
   hl -q status>=500 app.log
   
   # Correct: quote the entire query
   hl -q 'status>=500' app.log
   ```

2. **Invalid operator combinations**:
   ```sh
   # Wrong: 'and' requires two expressions
   hl -q 'and status=500' app.log
   
   # Correct
   hl -q 'status=500 and method=POST' app.log
   ```

3. **Unbalanced parentheses**:
   ```sh
   # Wrong
   hl -q '(status>=500 and method=POST' app.log
   
   # Correct
   hl -q '(status>=500) and method=POST' app.log
   ```

**Solution**:
- Always quote your query expressions
- Check for balanced parentheses
- Verify operator syntax in [Query Syntax](../reference/query-syntax.md)

## Sorting Issues

### Missing Entries in Sort Mode

**Symptoms**: Some log entries disappear when using `--sort`.

**Cause**: Entries without recognized timestamps are discarded in `--sort` mode.

**Solution**:

1. **Check timestamp format**:
   ```sh
   # View raw entries to see timestamp format
   head -5 app.log
   ```

2. **Use streaming mode** if timestamps aren't in a supported format:
   ```sh
   # Preserves all entries
   hl app.log
   ```

3. **Configure custom timestamp field names** if using non-standard field names:
   ```yaml
   # ~/.config/hl/config.yaml
   fields:
     predefined:
       time:
         names:
           - timestamp
           - time
           - "@timestamp"
   ```

### Incorrect Chronological Order

**Symptoms**: Entries appear out of chronological order even with `--sort`.

**Common Causes**:

1. **Using streaming mode**: Without `--sort`, entries are displayed in file order
   ```sh
   # Correct: use --sort for chronological ordering
   hl --sort app1.log app2.log
   ```

2. **Mixed timezones**: Logs from different sources with different timezones
   ```sh
   # Normalize to UTC
   hl --sort --time-zone UTC app.log
   ```

3. **Clock skew**: Different servers had unsynchronized clocks when logs were written
   - This is a data issue, not an `hl` issue
   - Logs will be sorted by their recorded timestamps

**Solution**:
- Always use `--sort` for chronological ordering
- Consider timezone differences in your logs
- Be aware of potential clock skew in distributed systems

## Timestamp Issues

### Timestamps Not Displayed in Correct Timezone

**Symptoms**: Timestamps show UTC when you want local time, or vice versa.

**Solution**:

```sh
# Use local timezone
hl --local app.log

# Use specific timezone
hl --time-zone "America/New_York" app.log
```

**Note**: Don't use `--time-zone local` (it's not a valid IANA timezone). Use `--local` instead.

### Timestamp Format Not Applied

**Symptoms**: `--time-format` option appears to be ignored.

**Common Causes**:

1. **Invalid format string**:
   ```sh
   # Check format string syntax
   man date  # or search for strftime documentation
   ```

2. **Environment variable override**:
   ```sh
   # Check for conflicting environment variable
   echo $HL_TIME_FORMAT
   
   # Unset if needed
   unset HL_TIME_FORMAT
   ```

**Solution**:
- Verify format string is valid strftime format
- Check for environment variable overrides
- Command-line options should override environment variables

### Unix Timestamps Not Recognized

**Symptoms**: Numeric timestamps aren't parsed correctly.

**Solution**: Specify the timestamp unit:

```sh
# Milliseconds (common in JavaScript/Node.js)
hl --unix-timestamp-unit ms app.log

# Seconds (common in Unix/Python)
hl --unix-timestamp-unit s app.log

# Nanoseconds (common in Go)
hl --unix-timestamp-unit ns app.log
```

## Output Issues

### Colors Not Showing

**Symptoms**: Output is plain text without colors.

**Common Causes**:

1. **Not a terminal**: Output is redirected or piped
   ```sh
   # Force colors
   hl --color=always app.log | less -R
   ```

2. **Terminal doesn't support colors**:
   ```sh
   # Check TERM environment variable
   echo $TERM
   
   # Try setting TERM
   export TERM=xterm-256color
   ```

3. **Color disabled in config**:
   ```yaml
   # Check ~/.config/hl/config.yaml
   color: never  # Remove or change this
   ```

**Solution**:
- Use `--color=always` when piping
- Ensure your terminal supports colors
- Check configuration file

### Pager Not Working

**Symptoms**: Output scrolls past instead of using a pager.

**Common Causes**:

1. **Paging disabled**:
   ```sh
   # Check if -P was used
   hl app.log  # Should use pager if output is long
   ```

2. **No pager configured**:
   ```sh
   # Set pager environment variable
   export PAGER=less
   # or
   export HL_PAGER=less
   ```

3. **Output is not a terminal**:
   ```sh
   # Pager is auto-disabled when piping
   hl app.log | grep ERROR  # No pager (correct behavior)
   ```

**Solution**:
- Don't use `-P` if you want paging
- Set `PAGER` or `HL_PAGER` environment variable
- Use `--paging=always` to force pager even when piping

### Garbled Output

**Symptoms**: Strange characters or broken formatting in output.

**Common Causes**:

1. **Terminal encoding mismatch**:
   ```sh
   # Ensure UTF-8 encoding
   export LANG=en_US.UTF-8
   export LC_ALL=en_US.UTF-8
   ```

2. **ASCII mode enabled**:
   ```sh
   # Check if ASCII mode is forcing character substitution
   hl --ascii=never app.log
   ```

3. **Theme incompatibility**:
   ```sh
   # Try a different theme
   hl --theme hl-dark app.log
   ```

**Solution**:
- Ensure terminal uses UTF-8 encoding
- Disable ASCII mode if you want Unicode characters
- Try different themes

## Performance Issues

### Slow Processing

**Symptoms**: `hl` takes a long time to process logs.

**Common Causes**:

1. **Large files with `--sort`**: Building index takes time
   ```sh
   # Use time ranges to limit scope
   hl --sort --since "2024-01-15 10:00:00" app.log
   ```

2. **Low concurrency**:
   ```sh
   # Increase thread count
   hl --sort --concurrency 16 app.log
   ```

3. **Complex queries**:
   ```sh
   # Simplify query if possible
   # Use set membership instead of multiple ORs
   hl -q 'status in (500,502,503)' app.log
   ```

**Solution**:
- Use time ranges to limit scope
- Increase `--concurrency` for large files
- Optimize query expressions
- See [Performance Tips](../reference/performance.md)

### High Memory Usage

**Symptoms**: `hl` uses excessive memory.

**Common Causes**:

1. **Large buffer size**:
   ```sh
   # Reduce buffer size
   hl --buffer-size "64 KiB" app.log
   ```

2. **High concurrency**:
   ```sh
   # Reduce thread count
   hl --concurrency 4 app.log
   ```

3. **Very large log entries**:
   ```sh
   # Limit max message size
   hl --max-message-size "16 MiB" app.log
   ```

**Solution**:
- Reduce buffer size
- Reduce concurrency
- Limit max message size
- Use streaming mode instead of `--sort` when possible

## Configuration Issues

### Configuration File Not Loaded

**Symptoms**: Settings in config file are ignored.

**Debugging Steps**:

1. **Check config file location**:
   ```sh
   # Default locations (in order):
   # 1. --config <path>
   # 2. $HL_CONFIG
   # 3. ~/.config/hl/config.yaml
   # 4. ~/.hl/config.yaml
   
   ls -la ~/.config/hl/config.yaml
   ```

2. **Check config file syntax**:
   ```sh
   # Validate YAML syntax
   cat ~/.config/hl/config.yaml
   ```

3. **Check for environment variable override**:
   ```sh
   # See if HL_CONFIG points elsewhere
   echo $HL_CONFIG
   ```

**Solution**:
- Ensure config file exists in a standard location
- Verify YAML syntax is correct
- Check for environment variable overrides

### Theme Not Applied from Config

**Symptoms**: Theme set in config file is not used.

**Common Causes**:

1. **Command-line override**:
   ```sh
   # Command-line --theme overrides config
   hl app.log  # Uses config theme
   hl --theme hl-light app.log  # Overrides config
   ```

2. **Environment variable override**:
   ```sh
   # Check for HL_THEME
   echo $HL_THEME
   unset HL_THEME  # Remove if needed
   ```

3. **Theme doesn't exist**:
   ```sh
   # List available themes
   hl --list-themes
   ```

**Solution**:
- Check for command-line or environment overrides
- Verify theme name is correct
- Ensure theme is available

## Follow Mode Issues

### Follow Mode Not Updating

**Symptoms**: `--follow` mode doesn't show new entries as they're written.

**Common Causes**:

1. **File not being appended**: Application isn't writing new entries
   ```sh
   # Test with tail
   tail -f app.log
   ```

2. **Wrong file**: Following the wrong file
   ```sh
   # Check which file is actually being written
   lsof | grep app.log
   ```

3. **File rotation**: Log file was rotated and new entries go to a different file
   - Follow mode doesn't handle file rotation
   - Consider using tools designed for log rotation (e.g., `tail -F`)

**Solution**:
- Verify the application is writing to the file
- Ensure you're following the correct file
- Be aware that follow mode doesn't handle log rotation

## Integration Issues

### Piping to `jq` Fails

**Symptoms**: `jq` reports "parse error" when reading `hl` output.

**Common Causes**:

1. **Formatted output instead of JSON**:
   ```sh
   # Wrong: formatted output is not JSON
   hl app.log | jq
   
   # Correct: use --raw for JSON output
   hl --raw app.log | jq
   ```

2. **Mixed formats in output**:
   ```sh
   # Ensure JSON-only output (force JSON format)
   hl --raw --input-format json app.log | jq
   ```

**Solution**:
- Use `--raw` to output original JSON
- Use `--input-format json` to ensure JSON-only output (no logfmt)

### Shell Redirects Not Working as Expected

**Symptoms**: Output redirects produce unexpected results.

**Common Issues**:

1. **Special characters in shell**:
   ```sh
   # Be careful with special characters in filenames
   hl app.log > output.txt
   ```

2. **Special characters in query**:
   ```sh
   # Wrong: shell interprets >
   hl -q status>500 app.log
   
   # Correct: quote the query
   hl -q 'status>500' app.log
   ```

**Solution**:
- Quote query expressions to prevent shell interpretation
- Note: The pager is automatically disabled when output is redirected, so `-P` is not needed

## Getting Help

If you encounter an issue not covered here:

1. **Check the documentation**:
   - [FAQ](./faq.md) for common questions
   - [Command-Line Options](../reference/options.md) for option details
   - [Examples](../examples/basic.md) for usage patterns

2. **Enable debug logging** (if available):
   ```sh
   # Check for verbose/debug options
   hl --help=long | grep -i debug
   ```

3. **Create a minimal reproduction**:
   - Use a small sample log file
   - Identify the minimal command that reproduces the issue

4. **Report the issue**:
   - Visit the [GitHub repository](https://github.com/pamburus/hl)
   - Provide your minimal reproduction
   - Include `hl` version: `hl --version`
   - Include OS and terminal information

## Common Error Messages

### "Failed to parse query"

**Cause**: Query syntax error.

**Solution**: Check query syntax in [Query Syntax](../reference/query-syntax.md). Ensure proper quoting.

### "No such file or directory"

**Cause**: Specified file doesn't exist or path is incorrect.

**Solution**: Verify file path. Use tab completion or `ls` to confirm.

### "Permission denied"

**Cause**: No read permission for the file.

**Solution**:
```sh
# Check permissions
ls -l app.log

# Fix permissions if appropriate
chmod +r app.log
```

### "Broken pipe"

**Cause**: Output consumer (pager, pipeline) terminated early.

**Solution**: This is normal when using a pager and quitting early (e.g., pressing `q` in `less`). Not an error.

### "Invalid timestamp unit"

**Cause**: Unrecognized value for `--unix-timestamp-unit`.

**Solution**: Use one of: `auto`, `s`, `ms`, `us`, `ns`.
