use std::path::Path;

use nu_ansi_term::Color;

pub mod suggest;

pub use suggest::Suggestions;

pub trait Highlight {
    fn hl(&self) -> String;
}

impl<S: AsRef<str>> Highlight for S {
    fn hl(&self) -> String {
        HILITE.paint(format!("{:?}", self.as_ref())).to_string()
    }
}

impl Highlight for Path {
    fn hl(&self) -> String {
        self.to_string_lossy().hl()
    }
}

pub const HILITE: Color = Color::Yellow;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight() {
        assert_eq!(HILITE.paint("hello").to_string(), "\u{1b}[33mhello\u{1b}[0m");
        assert_eq!("hello".hl(), "\u{1b}[33m\"hello\"\u{1b}[0m");
        assert_eq!(Path::new("hello").hl(), "\u{1b}[33m\"hello\"\u{1b}[0m");
    }
}
