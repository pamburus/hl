# Introduction

![hl banner](images/banner.svg)

Welcome to `hl`, a high-performance command-line tool for viewing and analyzing structured logs. Built with efficiency in mind, `hl` transforms JSON and logfmt formatted log data into clean, colorized, human-readable output—making it easy for developers, system administrators, DevOps engineers, and SREs to make sense of their logs.

## Key Features

- **High Performance**: Process gigabytes of logs in seconds with efficient parsing and indexing
- **Smart Filtering**: Filter logs by level, field values, time ranges, or complex queries
- **Chronological Sorting**: Sort entries across multiple log files by timestamp
- **Live Streaming**: Follow live logs with real-time updates and automatic sorting
- **Customizable Themes**: Choose from built-in themes or create your own
- **Automatic Paging**: Seamlessly integrates with your preferred pager (like `less`)
- **Compressed File Support**: Read gzip, bzip2, xz, and zstd compressed files directly
- **Field Management**: Hide, reveal, or expand fields to focus on what matters
- **Time Zone Flexibility**: Display timestamps in UTC, local time, or any timezone

## Who Should Use `hl`?

`hl` is perfect for anyone who works with structured logs:

- **Developers** debugging applications and services
- **DevOps Engineers** monitoring distributed systems
- **System Administrators** analyzing server logs
- **Site Reliability Engineers** investigating incidents
- **QA Engineers** reviewing test logs

## Why Choose `hl`?

Traditional log viewing tools often struggle with large files or lack the flexibility needed for modern structured logs. `hl` addresses these challenges by:

1. **Speed**: Processes logs at approximately 2 GiB/s during initial indexing, handles hundreds of gigabytes across hundreds of files
2. **Flexibility**: Powerful query language for complex filtering scenarios
3. **Usability**: Intuitive command-line interface with sensible defaults
4. **Reliability**: Written in Rust with stable libraries for rock-solid performance—no crashes, no surprises
5. **Beauty**: Customizable themes make logs easier to read and understand

## How It Works

`hl` reads structured log data (JSON or logfmt format), parses it, applies your filters and formatting preferences, and outputs a human-friendly representation. It can work with:

- Local log files (plain or compressed)
- Standard input from pipes
- Live streaming data
- Multiple files simultaneously

The tool automatically detects the log format and adjusts its processing accordingly, making it easy to use with different log sources.

## Getting Help

This guide covers everything from basic installation to advanced filtering techniques. If you're new to `hl`, start with the [Installation](./installation.md) and [Quick Start](./quick-start.md) sections. For specific features, browse the Features section in the table of contents.

If you encounter issues or have questions not covered in this guide, you can:

- Check the [Troubleshooting](./help/troubleshooting.md) section
- Review the [FAQ](./help/faq.md)
- Visit the [GitHub repository](https://github.com/pamburus/hl) to file an issue

Let's get started!
