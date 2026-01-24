//! A lightweight wildcard pattern matching library.
//!
//! This crate provides simple and efficient wildcard pattern matching with `*` (zero or more
//! characters) and `?` (exactly one UTF-8 character) wildcards.
//!
//! # Features
//!
//! - **Simple API**: Infallible pattern creation with `Pattern::new()`
//! - **UTF-8 aware**: The `?` wildcard matches exactly one UTF-8 character
//! - **Efficient matching**: Uses optimized substring search with backtracking
//! - **Escaped characters**: Support for escaping `*`, `?`, and `\` with backslash
//! - **Display trait**: Reconstruct pattern strings with proper escaping
//!
//! # Pattern Syntax
//!
//! - `*` - Matches zero or more characters
//! - `?` - Matches exactly one UTF-8 character
//! - `\*`, `\?`, `\\` - Escaped literal characters
//! - Any other character matches itself
//! - A trailing `\` without a following character is treated as a literal backslash
//!
//! # Examples
//!
//! ```
//! use wildcard::Pattern;
//!
//! let pattern = Pattern::new("*.txt");
//! assert!(pattern.matches("hello.txt"));
//! assert!(pattern.matches("foo.txt"));
//! assert!(!pattern.matches("hello.rs"));
//!
//! let pattern = Pattern::new("test?.log");
//! assert!(pattern.matches("test1.log"));
//! assert!(pattern.matches("test2.log"));
//! assert!(!pattern.matches("test.log"));
//! assert!(!pattern.matches("test12.log"));
//!
//! // Escaped wildcards
//! let pattern = Pattern::new(r"file\*.txt");
//! assert!(pattern.matches("file*.txt"));
//! assert!(!pattern.matches("file123.txt"));
//! ```
//!
//! # UTF-8 Handling
//!
//! The `?` wildcard matches exactly one UTF-8 character, not one byte:
//!
//! ```
//! use wildcard::Pattern;
//!
//! let pattern = Pattern::new("???");
//! assert!(pattern.matches("abc"));
//! assert!(pattern.matches("🦀🎉🌟")); // Three emoji = three characters
//! assert!(!pattern.matches("ab"));
//! ```
//!
//! # Pattern Display
//!
//! Patterns can be converted back to strings with proper escaping:
//!
//! ```
//! use wildcard::Pattern;
//!
//! let pattern = Pattern::new("hello*world");
//! assert_eq!(pattern.to_string(), "hello*world");
//!
//! let pattern = Pattern::new(r"file\*.txt");
//! assert_eq!(pattern.to_string(), r"file\*.txt");
//! ```

mod pattern;
mod utf8;

pub use pattern::*;

#[cfg(test)]
mod tests {}
