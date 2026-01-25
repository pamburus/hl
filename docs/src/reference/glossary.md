# Glossary

Standard terminology used throughout hl documentation.

---

**Caller**
: The source code location (file, line, function) that generated a log entry. See [Field Visibility](../features/field-visibility.md).

**Expansion**
: Displaying nested or structured field values inline or across multiple lines. Controlled by `--expansion`. See [Field Expansion](../features/field-expansion.md).

**Field**
: A key-value pair within a log record. Examples: `level`, `message`, `timestamp`, `caller`, or custom application-specific fields like `user_id` or `request_id`. See [Field Visibility](../features/field-visibility.md).

**Entry**
: A single log entry containing a timestamp, level, message, and optional fields. Also referred to as *log record* or *log line* in some contexts. Preferred term in this documentation: **entry** (plural: **entries**).

**Filter**
: A condition that selects which log entries to display. Includes level filters (`-l`), field filters (`-f`), time filters (`--since`/`--until`), and queries (`-q`). See [Filtering](../features/filtering.md).

**Follow mode**
: Continuously watching a file for new content, similar to `tail -f`. Enabled with `-F` or `--follow`. See [Follow Mode](../features/follow-mode.md).

**Input format**
: The structure of incoming log data. Supported formats: JSON, logfmt, and CLF (Common Log Format). See [Input Formats](../features/input-formats.md).

**Level**
: The severity of a log entry. Standard levels in ascending severity: `debug`, `info`, `warning`, `error`. Also called *log level* or *severity*. See [Filtering by Log Level](../features/filtering-level.md).

**Message**
: The primary human-readable text content of a log entry, typically stored in a `message` or `msg` field.

**Overlay**
: A partial theme definition that modifies specific colors without replacing the entire theme. See [Theme Overlays](../customization/themes-overlays.md).

**Pager**
: An external program (such as `less`) used to scroll through output interactively. See [Automatic Pager Integration](../features/pager.md).

**Predicate**
: A single condition within a query expression, such as `level = error` or `message contains "timeout"`. See [Query Syntax](./query-syntax.md).

**Prefix**
: Non-JSON text that appears before a JSON log entry on the same line, such as timestamps added by Docker or systemd. See [Non-JSON Prefixes](../features/prefixes.md).

**Query**
: A structured expression for filtering logs, supporting boolean logic and field comparisons. Specified with `-q` or `--query`. See [Complex Queries](../features/filtering-queries.md).

**Sort mode**
: Processing mode where all input is read and buffered before producing output, enabling chronological ordering across multiple sources. Enabled with `-s` or `--sort`. Contrast with *streaming mode*. See [Sorting](../features/sorting.md).

**Streaming mode**
: Default processing mode where records are output immediately as they are read, without buffering the entire input. Contrast with *sort mode*. See [Live Streaming](../features/streaming.md).

**Theme**
: A color scheme defining how log output is displayed, including colors for levels, fields, timestamps, and other elements. See [Themes](../customization/themes.md).

**Timestamp**
: The time associated with a log entry. hl auto-detects various timestamp formats including ISO 8601, RFC 3339, and Unix timestamps. See [Timestamp Handling](../features/timestamps.md).

**Time zone**
: The offset from UTC used when displaying timestamps. Controlled by `--time-zone` or the `HL_TIME_ZONE` environment variable. See [Time Display](../features/time-display.md).

**Unix timestamp**
: A timestamp expressed as the number of seconds (or milliseconds, microseconds, nanoseconds) since January 1, 1970 00:00:00 UTC (the Unix epoch).