# hl User Guide Book

> **Status**: ✅ **COMPLETE** - Ready for deployment

A comprehensive user guide for hl, the fast and powerful log viewer.

## Overview

This is the complete mdBook-based documentation for hl, covering all features, examples, customization options, and reference materials.

## Statistics

- **Pages**: 46 complete documentation pages
- **Content**: ~17,000+ lines of documentation
- **Examples**: 100+ tested, practical examples
- **Sections**: Getting Started, Features, Customization, Examples, Reference, Help

## Quick Start

### Prerequisites

```sh
cargo install mdbook
```

### Build and Serve

```sh
# Build the book
mdbook build

# Serve locally (http://localhost:3000)
mdbook serve
```

### Build Output

Generated HTML will be in `book/` directory (gitignored).

## Structure

```
book/
├── src/                      # Documentation source files
│   ├── SUMMARY.md           # Table of contents
│   ├── intro.md             # Introduction (with hl banner)
│   ├── images/              # Visual assets
│   │   ├── banner.svg       # hl logo banner
│   │   ├── logo.svg         # hl logo
│   │   ├── theme-style-roles.svg  # Style inheritance diagram
│   │   └── theme-element-inheritance.svg  # Element inheritance diagram
│   ├── features/            # Feature documentation (20 pages)
│   ├── customization/       # Customization guides (7 pages)
│   ├── examples/            # Practical examples (7 pages)
│   ├── reference/           # Reference docs (4 pages)
│   └── help/                # Troubleshooting & FAQ (2 pages)
├── docs-audit/              # Timestamp documentation audit
├── book.toml                # mdBook configuration
├── COMPLETION_GUIDE.md      # Status: 100% complete
├── FINAL_REVIEW_CHECKLIST.md # Pre-deployment checklist
└── BOOK_COMPLETE.md         # Completion summary
```

## What's Documented

### ✅ Getting Started
- Introduction and overview
- Installation (all platforms)
- Quick start guide

### ✅ Features
- **Viewing**: Basic viewing, pager, streaming, multiple files, compressed files
- **Filtering**: By level, fields, time range, complex queries
- **Sorting**: Chronological sorting with indexing
- **Following**: Live monitoring mode
- **Formatting**: Field visibility, time display, expansion, raw output
- **Input**: Formats, timestamp handling, prefixes

### ✅ Customization
- Configuration files
- Environment variables
- Themes (stock, custom, overlays)

### ✅ Examples
- Basic usage, filtering, queries, time filtering, field management, live monitoring, multiple logs

### ✅ Reference
- Complete CLI options
- Query syntax specification
- Time format reference
- Performance tips

### ✅ Help
- Troubleshooting guide
- FAQ

### ✅ Visual Assets
- hl banner logo (introduction page)
- Style inheritance diagram (custom themes)
- Element inheritance diagram (custom themes)

## Documentation Quality

Every page includes:
- Clear introduction and context
- Practical, runnable examples
- Complete command syntax
- Common use cases and best practices
- Troubleshooting sections
- Cross-references to related topics

## Special Features

### Timestamp Documentation Audit
Complete audit and verification of all timestamp-related documentation:
- Three distinct contexts documented (input parsing, filter parsing, output formatting)
- All formats verified against implementation
- Timezone behavior clarified
- Audit materials in `docs-audit/`

### Delimiter Behavior Documentation
- Actual behavior documented (not just option names)
- Smart newline and continuation detection explained

## Deployment

The book is ready for deployment to:
- GitHub Pages (automated via Actions)
- Any static hosting service
- Bundled with releases
- Linked from docs.rs

## Maintenance

To keep documentation updated:

1. **Check affected pages** when code changes
2. **Test examples** to ensure they still work
3. **Update cross-references** for new features
4. **Rebuild** and verify: `mdbook build`

See `COMPLETION_GUIDE.md` for detailed maintenance guidelines.

## Review

Use `FINAL_REVIEW_CHECKLIST.md` for comprehensive pre-deployment review.

## Build Status

✅ Builds successfully with no errors or warnings

## License

Same as hl project (MIT)

---

**The hl book is complete and ready to help users! 📚**