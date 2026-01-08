use std::fmt;
use std::mem::take;

use memchr::memmem::find_iter;

use crate::utf8::utf8_char_width;

/// A wildcard pattern for matching text strings.
///
/// Patterns are created from strings containing wildcard characters:
/// - `*` matches zero or more characters
/// - `?` matches exactly one UTF-8 character
/// - `\` escapes the next character (or is literal if at end of pattern)
///
/// # Examples
///
/// ```
/// use wildcard::Pattern;
///
/// let pattern = Pattern::new("*.txt");
/// assert!(pattern.matches("readme.txt"));
/// assert!(!pattern.matches("readme.md"));
///
/// let pattern = Pattern::new("test?.log");
/// assert!(pattern.matches("test1.log"));
/// assert!(!pattern.matches("test.log"));
/// ```
///
/// - Patterns can be displayed back to strings with proper escaping via the `Display` trait
#[derive(Debug, PartialEq, Clone, Default)]
pub struct Pattern {
    segments: Vec<Segment>,
}

impl Pattern {
    /// Creates a new pattern from a string.
    ///
    /// This function is infallible; all input strings are valid patterns.
    /// A trailing backslash without a following character is treated as a literal backslash.
    ///
    /// # Examples
    ///
    /// ```
    /// use wildcard::Pattern;
    ///
    /// let pattern = Pattern::new("hello*");
    /// assert!(pattern.matches("hello world"));
    ///
    /// // Escaped wildcards
    /// let pattern = Pattern::new(r"file\*.txt");
    /// assert!(pattern.matches("file*.txt"));
    /// assert!(!pattern.matches("file123.txt"));
    ///
    /// // Trailing backslash is literal
    /// let pattern = Pattern::new(r"path\");
    /// assert!(pattern.matches(r"path\"));
    /// ```
    pub fn new(raw: impl AsRef<str>) -> Self {
        Compiler::new().compile(raw.as_ref())
    }

    #[inline]
    /// Tests whether the pattern matches the given text.
    ///
    /// Returns `true` if the entire text matches the pattern, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use wildcard::Pattern;
    ///
    /// let pattern = Pattern::new("*.rs");
    /// assert!(pattern.matches("main.rs"));
    /// assert!(pattern.matches("lib.rs"));
    /// assert!(!pattern.matches("main.txt"));
    ///
    /// // UTF-8 character matching
    /// let pattern = Pattern::new("??");
    /// assert!(pattern.matches("ab"));
    /// assert!(pattern.matches("🦀🎉"));
    /// assert!(!pattern.matches("a"));
    ///
    /// // Complex patterns with backtracking
    /// let pattern = Pattern::new("*test*");
    /// assert!(pattern.matches("this is a test case"));
    /// assert!(pattern.matches("test"));
    /// assert!(!pattern.matches("no match here"));
    /// ```
    pub fn matches(&self, text: &str) -> bool {
        Self::partial_match(&self.segments, text)
    }

    #[inline]
    fn partial_match(mut segments: &[Segment], mut text: &str) -> bool {
        while let Some((segment, rest)) = segments.split_first() {
            for _ in 0..segment.wild.min {
                let Some(&b) = text.as_bytes().first() else {
                    return false;
                };
                text = &text[utf8_char_width(b)..];
            }

            if segment.wild.many {
                if segment.text.is_empty() {
                    return true;
                }
                for i in find_iter(text.as_bytes(), segment.text.as_bytes()) {
                    if Self::partial_match(rest, &text[i + segment.text.len()..]) {
                        return true;
                    }
                }
                return false;
            } else {
                if !text.starts_with(&segment.text) {
                    return false;
                }
                text = &text[segment.text.len()..];
            }

            segments = rest;
        }

        text.is_empty()
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for segment in &self.segments {
            for _ in 0..segment.wild.min {
                write!(f, "?")?;
            }

            if segment.wild.many {
                write!(f, "*")?;
            }

            for ch in segment.text.chars() {
                match ch {
                    '*' | '?' | '\\' => write!(f, "\\{}", ch)?,
                    _ => write!(f, "{}", ch)?,
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
struct Segment {
    wild: WildSpec,
    text: String,
}

#[derive(Debug, PartialEq, Clone, Copy, Default)]
struct WildSpec {
    many: bool,
    min: usize,
}

#[derive(Default)]
struct Compiler {
    segments: Vec<Segment>,
    next: Segment,
}

impl Compiler {
    fn new() -> Self {
        Self::default()
    }

    fn flush(&mut self) {
        if !self.next.text.is_empty() {
            self.segments.push(take(&mut self.next));
        }
    }

    fn compile(mut self, raw: &str) -> Pattern {
        let mut chars = raw.chars();
        while let Some(ch) = chars.next() {
            match ch {
                '*' => {
                    self.flush();
                    self.next.wild.many = true;
                }
                '?' => {
                    self.flush();
                    self.next.wild.min += 1;
                }
                '\\' => {
                    if let Some(escaped) = chars.next() {
                        self.next.text.push(escaped);
                    } else {
                        self.next.text.push('\\');
                    }
                }
                _ => {
                    self.next.text.push(ch);
                }
            }
        }

        self.flush();

        if self.next.wild.many || self.next.wild.min > 0 || self.segments.is_empty() {
            self.segments.push(self.next);
        }

        Pattern {
            segments: self.segments,
        }
    }
}

#[cfg(test)]
mod tests;
