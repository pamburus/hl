// std imports
use std::fmt::Debug;

// ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Index(pub(super) usize);

// ---

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct OptIndex(usize);

impl OptIndex {
    #[inline]
    pub fn new(index: Option<Index>) -> Self {
        Self(index.map(|x| x.0).unwrap_or(usize::MAX))
    }

    #[inline]
    pub fn unfold(self) -> Option<Index> {
        if self.0 == usize::MAX {
            None
        } else {
            Some(Index(self.0))
        }
    }
}

impl Into<Option<Index>> for OptIndex {
    #[inline]
    fn into(self) -> Option<Index> {
        self.unfold()
    }
}

impl From<Option<Index>> for OptIndex {
    #[inline]
    fn from(index: Option<Index>) -> Self {
        Self::new(index)
    }
}

impl Debug for OptIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(index) = self.unfold() {
            f.debug_tuple("Some").field(&index).finish()
        } else {
            f.write_str("None")
        }
    }
}
