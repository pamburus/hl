# hl
Log viewer which translates JSON logs into pretty human-readable representation. It is a faster alternative to [humanlog](https://github.com/aybabtme/humanlog) and [hlogf](https://github.com/ssgreg/hlogf) with several additional features.

## Installation options

* Download latest release from [download page](https://github.com/pamburus/hl/releases/latest)

* Download and extract using `curl` and `tar` on Linux

    ```
    curl -sSfL https://github.com/pamburus/hl/releases/latest/download/hl-linux.tar.gz | tar xz
    ```

* Download and extract using `curl` and `tar` on macOS

    ```
    curl -sSfL https://github.com/pamburus/hl/releases/latest/download/hl-macos.tar.gz | tar xz
    ```

## Examples

### Screenshot

![screenshot](doc/screenshot.png)

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

    Shows only messages with error log level.

- Errors and warnings

    Command 

    ```
    $ hl -l w
    ```
    Shows only messages with warning and error log level.

- Errors, warnings and informational

    Command 

    ```
    $ hl -l i
    ```
    Shows all log messages except debug level messages.

### Using live log streaming

- Command

    ```
    $ tail -f example.log | hl -P
    ```
    Follows changes in example.log file and displays them immediately.
    Flag `-P` disables automatic using of pager in this case.


### Filtering by field values.

- Command

    ```
    $ hl example.log -f component=tsdb
    ```
    Shows only messages with field `component` having value `tsdb`.

- Command

    ```
    $ hl example.log -f component!=tsdb -f component!=uninteresting
    ```
    Shows only messages with field `component` having value other than `tsdb` or `uninteresting`.

- Command

    ```
    $ hl example.log -f provider~=string
    ```
    Shows only messages with field `provider` containing sub-string `string`.


### Hiding or showing selected fields.

- Command

    ```
    $ hl example.log --hide provider
    ```
    Hides field `provider`.


- Command

    ```
    $ hl example.log --show provider
    ```
    Hides all fields except `provider`.


- Command

    ```
    $ hl example.log -h headers -h body -H headers.content-type
    ```
    Hides fields `headers` and `body` but shows a single sub-field `content-type` inside field `headers`.


### Complete set of options and flags

```
hl 0.8.5
JSON log converter to human readable representation

USAGE:
    hl [FLAGS] [OPTIONS] [--] [FILE]...

FLAGS:
    -c                         Handful alias for --color=always, overrides --color option
        --help                 Prints help information
    -e, --hide-empty-fields    Hide empty fields, applies for null, string, object and array fields only
    -L, --local                Use local time zone, overrides --time-zone option
    -P                         Handful alias for --paging=never, overrides --paging option
    -r, --raw-fields           Disable unescaping and prettifying of field values
    -E, --show-empty-fields    Show empty fields, overrides --hide-empty-fields option
    -V, --version              Prints version information

OPTIONS:
        --buffer-size <buffer-size>                          Buffer size, kibibytes [default: 2048]
        --color <color>
            Color output options, one of { auto, always, never } [default: auto]

    -C, --concurrency <concurrency>                          Number of processing threads
    -f, --filter <filter>...
            Filtering by field values in one of forms <key>=<value>, <key>~=<value>, <key>!=<value>, <key>!~=<value>

    -h, --hide <hide>...                                     An exclude-list of keys
        --interrupt-ignore-count <interrupt-ignore-count>
            Number of interrupts to ignore, i.e. Ctrl-C (SIGINT) [default: 3]

    -l, --level <level>
            Filtering by level, valid values: ['d', 'i', 'w', 'e'] [default: d]

        --paging <paging>
            Output paging options, one of { auto, always, never } [default: auto]

    -H, --show <show>...                                     An include-list of keys
        --theme <theme>
            Color theme, one of { auto, dark, dark24, light } [default: dark]

    -t, --time-format <time-format>
            Time format, see https://man7.org/linux/man-pages/man1/date.1.html [default: %b %d %T.%3N]

    -Z, --time-zone <time-zone>
            Time zone name, see column "TZ database name" at
            https://en.wikipedia.org/wiki/List_of_tz_database_time_zones [default: UTC]

ARGS:
    <FILE>...    Files to process
```

## Performance

* MacBook Pro (16-inch, 2019)
    * CPU - 2,4 GHz 8-Core Intel Core i9
    * OS - macOS 10.15.6
    * Data - ~1GiB log file, 4.150.000 lines
        * hl v0.6.8 ~ 1 second
            ```
            $ time hl prom-m2.log -c >/dev/null
            hl prom-m2.log -c > /dev/null  12.41s user 0.64s system 1430% cpu 0.912 total
            ```
        * hlogf v1.4.1 ~ 10 seconds
            ```
            $ time hlogf prom-m2.log --color= >/dev/null
            hlogf prom-m2.log --color= > /dev/null  9.91s user 1.22s system 101% cpu 10.970 total
            ```
        * humanlog v0.4.1 ~ 60 seconds
            ```
            $ time humanlog <prom-m2.log >/dev/null
            humanlog> reading stdin...
            humanlog < prom-m2.log > /dev/null  58.55s user 4.89s system 107% cpu 58.931 total
            ```
        ![performance chart](doc/performance-chart.png)

## Future features

- Optional sorting of log messages by timestamp
