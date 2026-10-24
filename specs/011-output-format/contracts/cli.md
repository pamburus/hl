# Contract: Command Line, Environment and Configuration

**Feature**: [../spec.md](../spec.md)

## New option

```text
--output-format <FORMAT>  Output format [env: HL_OUTPUT_FORMAT=] [default: pretty] [possible values: pretty, json, logfmt]
```

- **Help heading**: Output.
- **Precedence**: the command line, then `HL_OUTPUT_FORMAT`, then the default. A repeated option overrides the earlier one.
- **Configuration**: none.
- **Invalid values**: rejected by argument parsing with the usual suggestion of valid values.

## Changed option

```text
--flatten <WHEN>  Whether to flatten objects [env: HL_FLATTEN=] [default: auto] [possible values: auto, never, always]
```

- **`auto`**: resolves to `always` for `pretty` and `logfmt` and to `never` for `json`.
- **Default**: taken from `formatting.flatten` in the configuration, falling back to `auto`.

## Interaction with existing options in structured formats (`json`, `logfmt`)

| Option | Effect |
|---|---|
| `-r`, `--raw` | Takes precedence: raw source entries are emitted exactly as today, and the output format is ignored |
| `--color`, `-c` | Ignored; no ANSI sequences |
| `--paging`, `-P` | Ignored; no pager |
| `-o`, `--output` | Same bytes as stdout |
| `--output-delimiter` | Separates records (`newline` or `nul`) |
| `--input-info` | Adds an input field only for explicitly requested layouts (FR-029) |
| `-h`, `--hide`, `--hide-empty-fields`, `--show-empty-fields` | Apply as described in FR-014 |
| `--flatten` | See above |
| `-l`, `--since`, `--until`, `-q`, `-f`, `-s`, `-F`, `--tail`, `--allow-prefix`, `--input-format`, `--delimiter`, `--unix-timestamp-unit` | Unchanged behavior; same record set as `pretty` |
| `--theme`, `--time-format`, `--time-zone`, `-L`, `--expansion`, `--ascii`, `--raw-fields` | No effect |

## Configuration additions

```toml
[formatting]
flatten = "auto"          # new value; new default in etc/defaults/config.toml

[formatting.json]
unique-keys = true        # new table; false writes repeated keys as they occur (FR-009a)

[formatting.logfmt]
unique-keys = true        # new table; same for the logfmt output format

[fields.predefined.time]
output-key = "timestamp"  # optional, per field: time, level, logger, message, caller, caller-file, caller-line

[fields.predefined.input]
output-key = "input"      # optional; new table
```

**Errors**: rejected at load, with a message naming the setting:
- an empty `output-key`, e.g. `fields.predefined.time.output-key must not be empty`;
- two predefined fields with the same effective output key.

**JSON schema**: `schema/json/config.schema.json` gains:
- `"auto"` in the `formatting.flatten` enum;
- the `formatting.json` and `formatting.logfmt` objects with an optional boolean `unique-keys`;
- an optional `output-key` string (`minLength: 1`) in each predefined field object;
- the `fields.predefined.input` object with only `output-key`.

## Preset values

| Preset | time | level | logger | message | caller | caller-file | caller-line |
|---|---|---|---|---|---|---|---|
| default | `timestamp` | `level` | `logger` | `message` | `caller` | n/a (combined) | n/a (combined) |
| k8s | `ts` | `level` | (unset) | `msg` | `caller` | n/a (combined) | n/a (combined) |
| ECS | `@timestamp` | `log.level` | `log.logger` | `message` | `log.origin.function` | `log.origin.file.name` | `log.origin.file.line` |

## Exit status

The exit status is the same as for `pretty` with the same inputs and options. A run that drops every line still succeeds.
