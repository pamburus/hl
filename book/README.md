# hl User Guide Book

This directory contains the complete user guide for **hl**, a high-performance JSON and logfmt log viewer.

## About This Book

This book is built using [mdBook](https://rust-lang.github.io/mdBook/) and provides comprehensive documentation for hl users. Unlike developer documentation, this guide focuses entirely on using hl as an end-user tool.

## Structure

- **src/** - Markdown source files
  - **intro.md** - Introduction to hl
  - **installation.md** - Installation instructions for all platforms
  - **quick-start.md** - Quick start guide with common examples
  - **features/** - Detailed feature documentation
  - **customization/** - Configuration and theming guides
  - **examples/** - Real-world usage examples
  - **reference/** - Complete reference documentation
  - **help/** - Troubleshooting and FAQ
- **book.toml** - mdBook configuration

## Building the Book

### Prerequisites

Install mdBook:

```sh
cargo install mdbook
```

### Build and Serve Locally

Build and serve the book with live reload:

```sh
cd book
mdbook serve
```

Then open http://localhost:3000 in your browser.

### Build Static HTML

Generate static HTML output:

```sh
cd book
mdbook build
```

The output will be in `book/` directory.

### Clean Build Artifacts

```sh
cd book
mdbook clean
```

## Content Overview

### Getting Started
- Introduction to hl and its key features
- Platform-specific installation guides
- Quick start tutorial with basic commands

### Features
Comprehensive coverage of all hl features:
- Viewing logs (pager integration, streaming, compressed files)
- Filtering (by level, fields, time range, complex queries)
- Sorting and following live logs
- Output formatting options
- Input format handling

### Customization
- Configuration file setup
- Environment variables
- Theme system (stock themes, custom themes, overlays)

### Examples
Practical, real-world examples:
- Basic usage patterns
- Advanced filtering scenarios
- Query language examples
- Time-based filtering
- Field management
- Live monitoring workflows

### Reference
Complete technical reference:
- All command-line options
- Query syntax specification
- Time format specifications
- Performance optimization tips

### Help
- Common troubleshooting scenarios
- Frequently asked questions

## Contributing

When adding or updating content:

1. **Use consistent formatting**
   - Code blocks should use `sh` for shell commands
   - Include practical examples for each feature
   - Use tables for comparison or reference data

2. **Focus on users, not developers**
   - No internal implementation details
   - No source code references
   - Practical examples over theory

3. **Cross-reference related content**
   - Link to related pages
   - Use "Next Steps" sections
   - Provide "See also" references

4. **Keep examples realistic**
   - Use real-world log scenarios
   - Show complete commands
   - Explain the output or behavior

## Style Guidelines

- Use second person ("you") for instructions
- Present tense for describing features
- Commands in separate code blocks
- Consistent terminology throughout
- Hierarchical headings (H1 for page title, H2 for sections, etc.)

## Testing Changes

Before committing changes:

1. Build the book locally: `mdbook build`
2. Check for broken links
3. Verify code examples are accurate
4. Test on mobile viewport (mdBook is responsive)

## Deployment

This book can be deployed to:
- GitHub Pages
- Any static hosting service
- Documentation hosting platforms

The build output is pure static HTML/CSS/JS with no backend requirements.

## License

This documentation is part of the hl project and follows the same license (MIT).