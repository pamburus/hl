// std imports
use std::{cmp::Ordering, collections::HashSet, sync::Arc};

const MIN_RELEVANCE: f64 = 0.75;

#[derive(Debug, Clone, Default)]
pub struct Suggestions {
    candidates: Vec<(f64, Arc<str>)>,
    reg: HashSet<Arc<str>>,
}

impl Suggestions {
    pub fn new<T, I>(wanted: &str, variants: I) -> Self
    where
        T: Into<Arc<str>>,
        I: IntoIterator<Item = T>,
    {
        let mut candidates = Vec::<(f64, _)>::new();
        let mut reg = HashSet::new();

        for variant in variants {
            let variant = variant.into();
            if reg.contains(&*variant) {
                continue;
            }

            let relevance = strsim::jaro(wanted, &*variant);

            if relevance > MIN_RELEVANCE {
                let item = variant;
                let candidate = (relevance, item.clone());
                let pos = candidates
                    .binary_search_by(|candidate| {
                        if candidate.0 < relevance {
                            Ordering::Greater
                        } else {
                            Ordering::Less
                        }
                    })
                    .unwrap_or_else(|e| e);
                candidates.insert(pos, candidate);
                reg.insert(item);
            }
        }

        Self { candidates, reg }
    }

    pub fn none() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    pub fn iter(&self) -> SuggestionsIter {
        SuggestionsIter {
            iter: self.candidates.iter(),
        }
    }

    pub fn merge(self, other: Self) -> Self {
        let mut a = self.candidates.into_iter();
        let mut reg = self.reg;

        let mut b = other.candidates.into_iter();
        let mut br = other.reg;
        let mut merged = Vec::with_capacity(a.len() + b.len());

        let mut i = a.next();
        let mut j = b.next();
        loop {
            let (val, left) = match (i.take(), j.take()) {
                (None, None) => break,
                (Some(x), None) => (x, true),
                (None, Some(y)) => (y, false),
                (Some(x), Some(y)) => {
                    if x >= y {
                        j = Some(y);
                        (x, true)
                    } else {
                        i = Some(x);
                        (y, false)
                    }
                }
            };

            if left {
                merged.push(val);
                i = a.next();
            } else {
                if !reg.contains(&val.1) {
                    reg.insert(br.take(&val.1).unwrap_or_else(|| val.1.clone()));
                    merged.push(val);
                }
                j = b.next();
            }
        }

        Self {
            candidates: merged,
            reg,
        }
    }
}

impl<'a> IntoIterator for &'a Suggestions {
    type Item = &'a str;
    type IntoIter = SuggestionsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        SuggestionsIter {
            iter: self.candidates.iter(),
        }
    }
}

pub struct SuggestionsIter<'a> {
    iter: std::slice::Iter<'a, (f64, Arc<str>)>,
}

impl<'a> Iterator for SuggestionsIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(_, candidate)| candidate.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggestions() {
        let suggestions = Suggestions::new("hello", vec!["helo", "hello", "helo", "hola", "hallo"]);
        assert!(!suggestions.is_empty());

        let mut iter = suggestions.iter();
        assert_eq!(iter.next(), Some("hello"));
        assert_eq!(iter.next(), Some("helo"));
        assert_eq!(iter.next(), Some("hallo"));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_suggestions_extend() {
        let mut suggestions = Suggestions::new("hello", vec!["helo", "hell", "hola", "hallo"]);
        suggestions = suggestions.merge(Suggestions::new("hello", vec!["helo", "hola", "hallo", "hello"]));
        let mut iter = suggestions.iter();

        assert_eq!(iter.next(), Some("hello"));
        assert_eq!(iter.next(), Some("helo"));
        assert_eq!(iter.next(), Some("hell"));
        assert_eq!(iter.next(), Some("hallo"));
        assert_eq!(iter.next(), None);

        let mut suggestions = Suggestions::new("hello", vec!["helo", "hell", "hola", "hallo"]);
        suggestions = suggestions.merge(Suggestions::new("hello", vec!["hello"]));
        let mut iter = suggestions.iter();

        assert_eq!(iter.next(), Some("hello"));
        assert_eq!(iter.next(), Some("helo"));
        assert_eq!(iter.next(), Some("hell"));
        assert_eq!(iter.next(), Some("hallo"));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_suggestions_none() {
        let suggestions = Suggestions::none();
        assert!(suggestions.is_empty());
        let mut iter = (&suggestions).into_iter();

        assert_eq!(iter.next(), None);
    }
}
