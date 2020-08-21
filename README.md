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

![](doc/screenshot.png)

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
    $ hl example.log -f provider~=string
    ```
    Shows only messages with field `provider` containing sub-string `string`.


### Complete set of options and flags

```
hl 0.6.8
JSON log converter to human readable representation

USAGE:
    hl [FLAGS] [OPTIONS] [--] [FILE]...

FLAGS:
    -c                  Handful alias for --color=always, overrides --color option
    -h, --help          Prints help information
    -P                  Handful alias for --paging=never, overrides --paging option
    -r, --raw-fields    Disables decoding and prettifying field values
    -V, --version       Prints version information

OPTIONS:
        --buffer-size <buffer-size>                          Buffer size, kibibytes [default: 2048]
        --color <color>
            Color output options, one of { auto, always, never } [default: auto]

        --concurrency <concurrency>
            Number of processing threads. Zero means automatic selection [default: 0]

    -f, --filter <filter>...
            Filtering by field values in form <key>=<value> or <key>~=<value>

        --interrupt-ignore-count <interrupt-ignore-count>
            Number of interrupts to ignore, i.e. Ctrl-C (SIGINT) [default: 3]

    -l, --level <level>
            Filtering by level, valid values: ['d', 'i', 'w', 'e'] [default: d]

        --paging <paging>
            Output paging options, one of { auto, always, never } [default: auto]

        --theme <theme>
            Color theme, one of { auto, dark, dark24, light } [default: auto]

    -t, --time-format <time-format>
            Time format, see https://man7.org/linux/man-pages/man1/date.1.html [default: %b %d %T.%3N]


ARGS:
    <FILE>...    Files to process
```

### Current limitations

- Only UTC timezone is supported.

### Future features

- Optional sorting of all log messages
