use super::{Color, ModeSet};

// ---

/// A fully resolved style with concrete values.
///
/// This is the output type after resolving [`RawStyle`] (which may contain
/// role references and mode diffs). All values are concrete:
/// - `modes` contains the final mode operations to apply
/// - `foreground` and `background` are final computed colors
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Style {
    pub modes: ModeSet,
    pub foreground: Option<Color>,
    pub background: Option<Color>,
}

impl Style {
    pub const fn new() -> Self {
        Self {
            modes: ModeSet::new(),
            foreground: None,
            background: None,
        }
    }

    pub fn modes(self, modes: ModeSet) -> Self {
        Self { modes, ..self }
    }

    pub fn foreground(self, foreground: Option<Color>) -> Self {
        Self { foreground, ..self }
    }

    pub fn background(self, background: Option<Color>) -> Self {
        Self { background, ..self }
    }
}
