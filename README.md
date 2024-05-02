# hl [![Build Status][ci-img]][ci] [![Coverage Status][cov-img]][cov]

A fast and powerful log viewer and processor that translates JSON or logfmt logs into a pretty human-readable format.
High performance and convenient features are the main goals.

## Features overview

* [Automatic usage](#automatic-usage-of-pager) of the [less](https://github.com/vbwagner/less) pager by default for convenience.
* Log streaming with the `-P` flag that disables the pager.
* Log record [filtering by field key/value pairs](#filtering-by-field-values) with the `-f` option with support for hierarchical keys.
* Quick and easy [filtering by level](#quick-filtering-by-log-level) with the `-l` option.
* Quick and easy [filtering by timestamp range](#filtering-by-time-range) using the `--since` and `--until` options and intuitive formats:
    * RFC-3339 timestamp format.
    * Current configured timestamp output format with the `-t` option or environment variable.
    * Human friendly shortcuts like `today`, `yesterday`, `friday` or relative offsets like `-3h` or `-14d`.
* Quick and easy [hiding and revealing](#hiding-or-revealing-selected-fields) of fields with the `-h` option.
* Hide empty fields with the `-e` flag.
* Lightning fast [message sorting](#sorting-messages-chronologically) with automatic indexing for local files using the `-s` flag.
    * Handles ~1 GiB/s for the first scan and allows fast filtering by timestamp range and level without scanning the data afterwards.
    * Works fast with hundreds of local files containing hundreds of gigabytes of data.
    * Reindexes large, growing files at lightning speed, skipping unmodified blocks, ~10 GiB/s.
* [Follow mode](#sorting-messages-chronologically-with-following-the-changes) with live message sorting by timestamp from different sources using the `-F` flag and preview of several recent messages with the `--tail` option.
* Custom complex [queries](#performing-complex-queries) that can include and/or conditions and much more.
* Non-JSON prefixes with `--allow-prefix` flag.
* Displays timestamps in UTC by default and supports easy timezone switching with the `-Z` option and the `-L` flag for a local timezone.
* Customizable via [configuration](#configuration-files) file and environment variables, supports easy [theme switching](#selecting-current-theme) and custom [themes](#custom-themes).

## Performance comparison chart

### Performance comparison with [humanlog](https://github.com/humanlogio/humanlog), [hlogf](https://github.com/ssgreg/hlogf) and [fblog](https://github.com/brocode/fblog) on a 2.3 GiB log file

![performance chart](doc/performance-chart.svg)

* See [performance](#performance) section for more details.

## Installation options

* Install using [homebrew](https://brew.sh) on macOS or Linux

    ```
    brew install pamburus/tap/hl
    ```

* Download and extract using `curl` and `tar` on macOS

    ```
    curl -sSfL https://github.com/pamburus/hl/releases/latest/download/hl-macos.tar.gz | tar xz
    ```

* Download and extract using `curl` and `tar` on Linux

    ```
    curl -sSfL https://github.com/pamburus/hl/releases/latest/download/hl-linux-x86_64-musl.tar.gz | tar xz
    ```

* Install [AUR package](https://aur.archlinux.org/packages/hl-log-viewer-bin) on Arch Linux

    ```
    yay -S hl-log-viewer-bin
    ```

* Install using [cargo](https://www.rust-lang.org/tools/install)

    ```
    cargo install --locked --git https://github.com/pamburus/hl.git
    ```

* Download latest release from [download page](https://github.com/pamburus/hl/releases/latest)

## Examples

### Screenshot

![screenshot-light](https://github.com/pamburus/hl-extra/raw/90be58af2fb91d5b5e7ce3b74c3b567611379c40/screenshot/universal/light.png#gh-light-mode-only)
![screenshot-dark](https://github.com/pamburus/hl-extra/raw/90be58af2fb91d5b5e7ce3b74c3b567611379c40/screenshot/universal/dark.png#gh-dark-mode-only)

See other [screenshots](https://github.com/pamburus/hl-extra/tree/90be58af2fb91d5b5e7ce3b74c3b567611379c40/screenshot#readme)


## Features and usage

### Concatenation of multiple log files

- Concatenate all log files

    Command

    ```
    $ hl $(ls -tr /var/log/example/*.log)
    ```
    Concatenates and humanizes all `*.log` files found in `/var/log/example/`.

### Support for gzipped log files

- Concatenate all log files including gzipped log files

    Command

    ```
    $ hl $(ls -tr /var/log/example/*.{log,log.gz})
    ```
    Concatenates and humanizes all `*.log` and `*.log.gz` files found in `/var/log/example/`.

### Automatic usage of pager

- Use default pager with default parameters

    Command

    ```
    $ hl example.log
    ```
    Automatically opens `less` pager with default parameters.

- Override options for default pager
    
    Command

    ```
    $ LESS=-SR hl example.log
    ```
    Opens `less` pager with disabled line wrapping.

- Use custom pager
    
    Command

    ```
    $ PAGER=bat hl example.log
    ```
    Opens `bat` pager.

### Quick filtering by log level

- Errors only

    Command 

    ```
    $ hl -l e
    ```

    Displays only error log level messages.

- Errors and warnings

    Command 

    ```
    $ hl -l w
    ```
    Displays only warning and error log level messages.

- Errors, warnings and informational

    Command 

    ```
    $ hl -l i
    ```
    Displays all log messages except debug level messages.

### Using live log streaming

- Command

    ```
    $ tail -f example.log | hl -P
    ```
    Tracks changes in the example.log file and displays them immediately.
    Flag `-P` disables automatic using of pager in this case.


### Filtering by field values

- Command

    ```
    $ hl example.log --filter component=tsdb
    ```
    Displays only messages where the `component` field has the value `tsdb`.

- Command

    ```
    $ hl example.log -f component!=tsdb -f component!=uninteresting
    ```
    Displays only messages where the `component` field has a value other than `tsdb` or `uninteresting`.

- Command

    ```
    $ hl example.log -f provider~=string
    ```
    Displays only messages where the `provider` field contains the `string` sub-string.

- Command

    ```
    $ hl example.log -f 'provider!~=string'
    ```
    Displays only messages where the `provider` field does not contain the `string` sub-string.


### Performing complex queries

- Command

    ```
    $ hl my-service.log --query 'level > info or status-code >= 400 or duration > 0.5'
    ```
    Displays messages that either have a level higher than info (i.e. warning or error) or have a status code field with a numeric value >= 400 or a duration field with a numeric value >= 0.5.

- Command

    ```
    $ hl my-service.log -q '(request in (95c72499d9ec, 9697f7aa134f, bc3451d0ad60)) or (method != GET)'
    ```
    Displays all messages that have the 'request' field with one of these values, or the 'method' field with a value other than 'GET'.

- Complete set of supported operators

    * Logical operators
        * Logical conjunction - `and`, `&&`
        * Logical disjunction - `or`, `||`
        * Logical negation - `not`, `!`
    * Comparison operators
        * Equal - `eq`, `=`
        * Not equal - `ne`, `!=`
        * Greater than - `gt`, `>`
        * Greater or equal - `ge`, `>=`
        * Less than - `lt`, `<`
        * Less or equal - `le`, `<=`
    * String matching operators
        * Sub-string check - (`contain`, `~=`), (`not contain`, `!~=`)
        * Wildcard match - (`like`), (`not like`)
            * Wildcard characters are: `*` for zero or more characters and `?` for a single character
        * Regular expression match - (`match`, `~~=`), (`not match`, `!~~=`)
    * Operators with sets
        * Test if value is one of the values in a set - `in (v1, v2)`, `not in (v1, v2)`
    
- Notes

    * Special field names that are reserved for filtering by predefined fields regardless of the actual source field names used to load the corresponding value: `level`, `message`, `caller` and `logger`.
    * To address a source field with one of these names instead of predefined fields, add a period before its name, i.e., `.level` will perform a match against the "level" source field.
    * To address a source field by its exact name, use a JSON-formatted string, i.e. `-q '".level" = info'`.
    * To specify special characters in field values, also use a JSON-formatted string, i.e. 
        ```
        $ hl my-service.log -q 'message contain "Error:\nSomething unexpected happened"'
        ```


### Filtering by time range

- Command

    ```
    $ hl example.log --since 'Jun 19 11:22:33' --until yesterday
    ```
    Displays only messages that occurred after Jun 19 11:22:33 UTC of the current year (or the previous year if the current date is less than Jun 19 11:22:33) and before yesterday midnight.

- Command

    ```
    $ hl example.log --since -3d
    ```
    Displays only messages from the past 72 hours.

- Command

    ```
    $ hl example.log --until '2021-06-01 18:00:00' --local
    ```
    Displays only messages that occurred before 6 PM local time on June 1, 2021, and shows timestamps in local time.


### Hiding or revealing selected fields

- Command

    ```
    $ hl example.log --hide provider
    ```
    Hides field `provider`.


- Command

    ```
    $ hl example.log --hide '*' --hide '!provider'
    ```
    Hides all fields except `provider`.


- Command

    ```
    $ hl example.log -h headers -h body -h '!headers.content-type'
    ```
    Hides fields `headers` and `body` but shows a single sub-field `content-type` inside field `headers`.


### Sorting messages chronologically

- Command

    ```
    $ hl -s *.log
    ```
    Displays log messages from all log files in the current directory sorted in chronological order.


### Sorting messages chronologically with following the changes

- Command

    ```
    $ hl --sync-interval-ms 500 -F <(kubectl logs -l app=my-app-1 -f) <(kubectl logs -l app=my-app-2 -f)
    ```
    Runs without a pager in follow mode by merging messages from the outputs of these 2 commands and sorting them chronologically within a custom 500ms interval.

- Command

    ```
    $ hl -F --tail 100 app1.log app2.log app3.log
    ```
    Runs without a pager in follow mode, following the changes in three log files in the current directory and sorting them chronologically at a default interval of 100ms.
    Preloads 100 lines from the end of each file before filtering.



### Configuration files

- Configuration file is automatically loaded if found in a predefined platform-specific location.

    | OS      | Location                                      |
    | ------- | --------------------------------------------- | 
    | macOS   | ~/.config/hl/config.yaml                      |
    | Linux   | ~/.config/hl/config.yaml                      |
    | Windows | %USERPROFILE%\AppData\Roaming\hl\config.yaml  |

- The path to the configuration file can be overridden using the HL_CONFIG environment variable.

- All parameters in the configuration file are optional and can be omitted. In this case, default values are used.

#### Default configuration file

- [config.yaml](etc/defaults/config.yaml)


### Environment variables

- Many parameters that are defined in command line arguments and configuration files can also be specified by environment variables.

#### Precedence of configuration sources (from lowest priority to highest priority)
* Configuration file
* Environment variables
* Command-line arguments

#### Examples
* `HL_TIME_FORMAT='%y-%m-%d %T.%3N'` overrides the time format specified in the configuration file.
* `HL_TIME_ZONE=Europe/Berlin` overrides the time zone specified in the configuration file.
* `HL_CONCURRENCY=4` overrides the concurrency limit specified in the configuration file.
* `HL_PAGING=never` specifies the default value for the paging option, but it can be overridden by command line arguments.


### Themes

#### Stock themes
- [themes](etc/defaults/themes/)

#### Selecting current theme
* Using `theme` value in the configuration file.
* Using environment variable, i.e. `HL_THEME=classic`, overrides the value specified in configuration file.
* Using command-line argument, i.e. `--theme classic`, overrides all other values.

#### Selecting themes with preview
To select themes with preview [fzf](https://github.com/junegunn/fzf) tool can be used like this:
```bash
hl --list-themes | fzf --preview-window="top,80%" --preview="head -n 100 example.log | hl -c --theme {}"
```

#### Custom themes
- Custom themes are automatically loaded when found in a predefined platform-specific location.

    | OS      | Location                                       |
    | ------- | ---------------------------------------------- | 
    | macOS   | ~/.config/hl/themes/*.yaml                     |
    | Linux   | ~/.config/hl/themes/*.yaml                     |
    | Windows | %USERPROFILE%\AppData\Roaming\hl\themes\*.yaml |

- Format description
  - Section `elements` contains styles for predefined elements.
  - Section `levels` contains optional overrides for styles defined in `elements` sections per logging level, which are [`debug`, `info`, `warning`, `error`].
  - Each element style contains optional `background`, `foreground` and `modes` parameters.
  - Example
    ```yaml
    elements:
        <element>:
            foreground: <color>
            background: <color>
            modes: [<mode>, <mode>, ...]
    levels:
        <level>:
            <element>:
                foreground: <color>
                background: <color>
                modes: [<mode>, <mode>, ...]
    ```
  - Color format is one of
    - Keyword `default` specifies default color defined by the terminal.
    - ASCII basic color name, one of
      - `black`
      - `red`
      - `green`
      - `yellow`
      - `blue`
      - `magenta`
      - `cyan`
      - `white`
      - `bright-black`
      - `bright-red`
      - `bright-green`
      - `bright-yellow`
      - `bright-blue`
      - `bright-magenta`
      - `bright-cyan`
      - `bright-white`
    - 256-color palette code, from `0` to `255`.
    - RGB color in hex web color format, i.e. `#FFFF00` for bright yellow color.
  - Modes is a list of additional styles, each of them is one of
    - `bold`
    - `faint`
    - `italic`
    - `underline`
    - `slow-blink`
    - `rapid-blink`
    - `reverse`
    - `conceal`
    - `crossed-out`


### Used terminal color schemes

#### iTerm2
* [One Dark Neo](https://gist.github.com/pamburus/0ad130f2af9ab03a97f2a9f7b4f18c68/746ca7103726d43b767f2111799d3cb5ec08adbb)
* Built-in "Light Background" color scheme

#### Alacritty
* [One Dark Neo](https://gist.github.com/pamburus/e27ebf60aa17d126f5c879f06112edd6/a1e66d34a65b883f1cb8ec28820cc0c53233e3aa#file-alacritty-yml-L904)
  * Note: It is recommended to use `draw_bold_text_with_bright_colors: true` setting
* [Light](https://gist.github.com/pamburus/e27ebf60aa17d126f5c879f06112edd6/a1e66d34a65b883f1cb8ec28820cc0c53233e3aa#file-alacritty-yml-L875)
  * Note: It is recommended to use `draw_bold_text_with_bright_colors: false` setting


### Complete set of options and flags

```
JSON and logfmt log converter to human readable representation

Usage: hl [OPTIONS] [FILE]...

Arguments:
  [FILE]...  Files to process

Options:
      --config <FILE>                    Configuration file path [env: HL_CONFIG=] [default: ~/.config/hl/config.yaml]
  -s, --sort                             Sort messages chronologically
  -F, --follow                           Follow input streams and sort messages chronologically during time frame set by --sync-interval-ms option
      --tail <N>                         Number of last messages to preload from each file in --follow mode [default: 10]
      --sync-interval-ms <MILLISECONDS>  Synchronization interval for live streaming mode enabled by --follow option [default: 100]
      --paging <WHEN>                    Control pager usage (HL_PAGER or PAGER) [env: HL_PAGING=] [default: auto] [possible values: auto, always, never]
  -P                                     Handful alias for --paging=never, overrides --paging option
      --help                             Print help
  -V, --version                          Print version

Filtering Options:
  -l, --level <LEVEL>    Filter messages by level [env: HL_LEVEL=]
      --since <TIME>     Filter messages by timestamp >= <TIME> (--time-zone and --local options are honored)
      --until <TIME>     Filter messages by timestamp <= <TIME> (--time-zone and --local options are honored)
  -f, --filter <FILTER>  Filter messages by field values [k=v, k~=v, k~~=v, 'k!=v', 'k!~=v', 'k!~~=v'] where ~ does substring match and ~~ does regular expression match
  -q, --query <QUERY>    Filter using query, accepts expressions from --filter and supports '(', ')', 'and', 'or', 'not', 'in', 'contain', 'like', '<', '>', '<=', '>=', etc

Output Options:
      --color [<WHEN>]        Color output control [env: HL_COLOR=] [default: auto] [possible values: auto, always, never]
  -c                          Handful alias for --color=always, overrides --color option
      --theme <THEME>         Color theme [env: HL_THEME=] [default: universal]
  -r, --raw                   Output raw source messages instead of formatted messages, which can be useful for applying filters and saving results in their original format
      --no-raw                Disable raw source messages output, overrides --raw option
      --raw-fields            Output field values as is, without unescaping or prettifying
  -h, --hide <KEY>            Hide or reveal fields with the specified keys, prefix with ! to reveal, specify '!*' to reveal all
      --flatten <WHEN>        Whether to flatten objects [env: HL_FLATTEN=] [default: always] [possible values: never, always]
  -t, --time-format <FORMAT>  Time format, see https://man7.org/linux/man-pages/man1/date.1.html [env: HL_TIME_FORMAT=] [default: "%b %d %T.%3N"]
  -Z, --time-zone <TZ>        Time zone name, see column "TZ identifier" at https://en.wikipedia.org/wiki/List_of_tz_database_time_zones [env: HL_TIME_ZONE=] [default: UTC]
  -L, --local                 Use local time zone, overrides --time-zone option
      --no-local              Disable local time zone, overrides --local option
  -e, --hide-empty-fields     Hide empty fields, applies for null, string, object and array fields only [env: HL_HIDE_EMPTY_FIELDS=]
  -E, --show-empty-fields     Show empty fields, overrides --hide-empty-fields option [env: HL_SHOW_EMPTY_FIELDS=]
      --input-info <VARIANT>  Show input number and/or input filename before each message [default: auto] [possible values: auto, none, full, compact, minimal]
  -o, --output <FILE>         Output file

Input Options:
      --input-format <FORMAT>       Input format [env: HL_INPUT_FORMAT=] [default: auto] [possible values: auto, json, logfmt]
      --unix-timestamp-unit <UNIT>  Unix timestamp unit [env: HL_UNIX_TIMESTAMP_UNIT=] [default: auto] [possible values: auto, s, ms, us, ns]
      --allow-prefix                Allow non-JSON prefixes before JSON messages [env: HL_ALLOW_PREFIX=]
      --delimiter <DELIMITER>       Log message delimiter, [NUL, CR, LF, CRLF] or any custom string

Advanced Options:
      --interrupt-ignore-count <N>  Number of interrupts to ignore, i.e. Ctrl-C (SIGINT) [env: HL_INTERRUPT_IGNORE_COUNT=] [default: 3]
      --buffer-size <SIZE>          Buffer size [env: HL_BUFFER_SIZE=] [default: "256 KiB"]
      --max-message-size <SIZE>     Maximum message size [env: HL_MAX_MESSAGE_SIZE=] [default: "64 MiB"]
  -C, --concurrency <N>             Number of processing threads [env: HL_CONCURRENCY=]
      --shell-completions <SHELL>   Print shell auto-completion script and exit [possible values: bash, elvish, fish, powershell, zsh]
      --list-themes                 Print available themes and exit
      --dump-index                  Print debug index metadata (in --sort mode) and exit
      --debug                       Print debug error messages that can help with troubleshooting
```

## Performance

![performance chart](doc/performance-chart.svg)

* MacBook Pro (16-inch, 2021)
    * **CPU**:   Apple M1 Max CPU
    * **OS**:    macOS Sonoma 14.4.1
    * **Data**:  ~ **2.3 GiB** log file, **6 000 000** lines
        * [hl](https://github.com/pamburus/hl) **v0.28.0** ~ *1.4 seconds*
            ```
            $ time hl example.log -c -o /dev/null
            hl example.log -c -o /dev/null  11.74s user 0.53s system 885% cpu 1.386 total
            ```
        * [hlogf](https://github.com/ssgreg/hlogf) **v1.4.1** ~ *8.5 seconds*
            ```
            $ time hlogf example.log --color always >/dev/null
            hlogf example.log --color always > /dev/null  6.93s user 1.79s system 99% cpu 8.757 total
            ```
        * [humanlog](https://github.com/humanlogio/humanlog) **v0.7.6** ~ *77 seconds*
            ```
            $ time humanlog <example.log --color always >/dev/null
            humanlog> reading stdin...
            humanlog --color always < example.log > /dev/null  80.02s user 4.71s system 109% cpu 1:17.11 total
            ```
        * [fblog](https://github.com/brocode/fblog) **v4.9.0** ~ *36 seconds*
            ```
            $ time fblog example.log >/dev/null
            fblog example.log > /dev/null  32.48s user 2.03s system 97% cpu 35.526 total
            ```

        * [fblog](https://github.com/brocode/fblog) with `-d` flag **v4.9.0** ~ *148 seconds*
            ```
            $ time fblog -d example.log >/dev/null
            fblog -d example.log > /dev/null  132.12s user 14.39s system 99% cpu 2:27.61 total
            ```


[ci-img]: https://github.com/pamburus/hl/actions/workflows/ci.yml/badge.svg
[ci]: https://github.com/pamburus/hl/actions/workflows/ci.yml
[cov-img]: https://codecov.io/gh/pamburus/hl/graph/badge.svg?token=464MN13408
[cov]: https://codecov.io/gh/pamburus/hl