use indexmap::{Equivalent, IndexMap};
use std::{
    hash::{Hash, Hasher},
    str::FromStr,
};
use wildflower::{Pattern, WILDCARD_MANY_CHAR, WILDCARD_SINGLE_CHAR};

// ---

pub trait KeyNormalize {
    fn normalize(c: char) -> char;
}

// ---

#[derive(Default, Clone)]
pub struct NoNormalizing {}

impl KeyNormalize for NoNormalizing {
    #[inline]
    fn normalize(byte: char) -> char {
        byte
    }
}

// ---

#[derive(Default, Clone)]
pub struct DefaultNormalizing {}

impl KeyNormalize for DefaultNormalizing {
    #[inline]
    fn normalize(c: char) -> char {
        if c == '_' {
            '-'
        } else if c < 128 as char {
            c.to_ascii_lowercase()
        } else {
            c
        }
    }
}

// ---

#[derive(PartialEq, Eq, Clone, Ord, PartialOrd, Copy, Debug)]
pub enum IncludeExcludeSetting {
    Unspecified,
    Include,
    Exclude,
}

impl IncludeExcludeSetting {
    pub fn apply(&self, other: Self) -> Self {
        match other {
            Self::Unspecified => *self,
            Self::Include => other,
            Self::Exclude => other,
        }
    }
}

impl Default for IncludeExcludeSetting {
    fn default() -> Self {
        Self::Unspecified
    }
}

// ---

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct MatchOptions<N: KeyNormalize> {
    pub delimiter: u8,
    _marker: std::marker::PhantomData<N>,
}

impl<N: KeyNormalize + Default> Default for MatchOptions<N> {
    fn default() -> Self {
        Self {
            delimiter: b'.',
            _marker: std::marker::PhantomData,
        }
    }
}

// ---

#[derive(Default)]
pub struct IncludeExcludeKeyFilter<N: KeyNormalize> {
    children: IndexMap<NormalizedKey<N>, IncludeExcludeKeyFilter<N>>,
    patterns: Vec<(Pattern<String>, IncludeExcludeKeyFilter<N>)>,
    options: MatchOptions<N>,
    setting: IncludeExcludeSetting,
}

impl<N: KeyNormalize + Clone> IncludeExcludeKeyFilter<N> {
    pub fn new(options: MatchOptions<N>) -> Self {
        if options.delimiter >= 128 {
            panic!("delimiter must be an ASCII character");
        }

        Self {
            children: IndexMap::new(),
            patterns: Vec::new(),
            options,
            setting: IncludeExcludeSetting::default(),
        }
    }

    pub fn entry<'a>(&'a mut self, key: &'a str) -> &'a mut IncludeExcludeKeyFilter<N> {
        let (head, tail) = self.split(key);

        if Self::is_pattern(&head) {
            return self.add_pattern(head.to_normalized(), tail);
        }

        let child = self
            .children
            .entry(head.to_normalized())
            .or_insert(Self::new(self.options.clone()));
        match tail {
            None => child,
            Some(tail) => child.entry(tail),
        }
    }

    pub fn get<'a>(&'a self, key: &str) -> Option<&'a IncludeExcludeKeyFilter<N>> {
        if self.leaf() {
            return None;
        }

        let (head, tail) = self.split(key);

        let found = |child: &'a Self| match tail {
            None => Some(child),
            Some(tail) => child.get(tail),
        };

        if let Some(child) = self.children.get(&head) {
            return found(child);
        }

        if self.patterns.len() != 0 {
            let head = head.to_optimized_string();
            for (pattern, child) in self.patterns.iter().rev() {
                if pattern.matches(head.as_ref()) {
                    return found(child);
                }
            }
        }

        None
    }

    pub fn include(&mut self) -> &mut Self {
        self.setting = IncludeExcludeSetting::Include;
        self
    }

    pub fn included(mut self) -> Self {
        self.setting = IncludeExcludeSetting::Include;
        self
    }

    pub fn exclude(&mut self) -> &mut Self {
        self.setting = IncludeExcludeSetting::Exclude;
        self
    }

    pub fn excluded(mut self) -> Self {
        self.setting = IncludeExcludeSetting::Exclude;
        self
    }

    pub fn setting(&self) -> IncludeExcludeSetting {
        self.setting.clone()
    }

    pub fn leaf(&self) -> bool {
        self.children.len() == 0 && self.patterns.len() == 0
    }

    fn split<'a>(&self, key: &'a str) -> (Key<&'a str, N>, Option<&'a str>) {
        let mut parts = key.splitn(2, self.options.delimiter as char);
        let head = parts.next().unwrap();
        let tail = parts.next();
        (Key::new(head), tail)
    }

    fn is_pattern(key: &Key<&str, N>) -> bool {
        key.inner().contains(WILDCARD_MANY_CHAR) || key.inner().contains(WILDCARD_SINGLE_CHAR)
    }

    fn add_pattern<'a>(
        &'a mut self,
        key: NormalizedKey<N>,
        tail: Option<&'a str>,
    ) -> &'a mut IncludeExcludeKeyFilter<N> {
        let pattern = Pattern::new(key.into_inner());
        self.children.retain(|k, _| !pattern.matches(k.as_str()));
        let item = match self.patterns.iter().position(|(p, _)| p == &pattern) {
            Some(i) => &mut self.patterns[i].1,
            None => {
                self.patterns.push((pattern, Self::new(self.options.clone())));
                &mut self.patterns.last_mut().unwrap().1
            }
        };
        match tail {
            None => item,
            Some(tail) => item.entry(tail),
        }
    }
}

// ---

struct NormalizedKey<N> {
    value: String,
    _marker: std::marker::PhantomData<N>,
}

impl<N> NormalizedKey<N> {
    fn into_inner(self) -> String {
        self.value
    }

    fn as_str(&self) -> &str {
        &self.value
    }
}

impl<N> PartialEq for NormalizedKey<N>
where
    N: KeyNormalize,
{
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<S, N> PartialEq<Key<S, N>> for NormalizedKey<N>
where
    S: AsRef<str>,
    N: KeyNormalize,
{
    fn eq(&self, other: &Key<S, N>) -> bool {
        self.value
            .chars()
            .zip(other.value.as_ref().chars())
            .all(|(a, b)| a == N::normalize(b))
    }
}

impl<N> Eq for NormalizedKey<N> where N: KeyNormalize {}

impl<N> std::hash::Hash for NormalizedKey<N>
where
    N: KeyNormalize,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

// ---

struct Key<S, N> {
    value: S,
    _marker: std::marker::PhantomData<N>,
}

impl<S, N> Key<S, N>
where
    S: AsRef<str>,
    N: KeyNormalize + Clone,
{
    fn new(value: S) -> Self {
        Self {
            value,
            _marker: std::marker::PhantomData,
        }
    }

    fn inner(&self) -> &S {
        &self.value
    }

    fn to_string(&self) -> String
    where
        S: AsRef<str>,
        N: KeyNormalize,
    {
        self.value.as_ref().chars().map(|c| N::normalize(c)).collect()
    }

    fn to_optimized_string(&self) -> OptimizedString
    where
        S: AsRef<str>,
        N: KeyNormalize,
    {
        if self.value.as_ref().len() <= 64 {
            let mut s = heapless::String::new();
            for c in self.value.as_ref().chars() {
                if !s.push(N::normalize(c)).is_ok() {
                    return OptimizedString::Long(self.to_string());
                }
            }
            OptimizedString::Short(s)
        } else {
            OptimizedString::Long(self.to_string())
        }
    }

    fn to_normalized(&self) -> NormalizedKey<N>
    where
        S: AsRef<str>,
        N: KeyNormalize,
    {
        NormalizedKey {
            value: self.to_string(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S1, S2, N> PartialEq<Key<S1, N>> for Key<S2, N>
where
    S1: AsRef<str>,
    S2: AsRef<str>,
    N: KeyNormalize,
{
    fn eq(&self, other: &Key<S1, N>) -> bool {
        self.value
            .as_ref()
            .chars()
            .zip(other.value.as_ref().chars())
            .all(|(a, b)| N::normalize(a) == N::normalize(b))
    }
}

impl<S, N> Eq for Key<S, N>
where
    S: AsRef<str>,
    N: KeyNormalize,
{
}

impl<S, N> Equivalent<NormalizedKey<N>> for Key<S, N>
where
    N: KeyNormalize,
    S: AsRef<str>,
{
    fn equivalent(&self, other: &NormalizedKey<N>) -> bool {
        *other == *self
    }
}

impl<S, N> Hash for Key<S, N>
where
    S: AsRef<str>,
    N: KeyNormalize,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        for c in self.value.as_ref().chars() {
            N::normalize(c).hash(state)
        }
    }
}

// ---

enum OptimizedString {
    Short(heapless::String<64>),
    Long(String),
}

impl AsRef<str> for OptimizedString {
    fn as_ref(&self) -> &str {
        match self {
            OptimizedString::Short(s) => s.as_str(),
            OptimizedString::Long(s) => s.as_str(),
        }
    }
}

impl From<&str> for OptimizedString {
    fn from(s: &str) -> Self {
        if s.len() <= 64 {
            OptimizedString::Short(heapless::String::from_str(s).unwrap())
        } else {
            OptimizedString::Long(s.into())
        }
    }
}

// ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter() {
        let mut filter = IncludeExcludeKeyFilter::new(MatchOptions::<DefaultNormalizing>::default());
        filter.entry("a").exclude();
        filter.entry("a.b").include();

        let x = filter.get("x");
        assert!(x.is_none(), "x should be none");

        let a = filter.get("a").unwrap();
        assert_eq!(a.setting(), IncludeExcludeSetting::Exclude);

        let b = a.get("b").unwrap();
        assert_eq!(b.setting(), IncludeExcludeSetting::Include);
    }
}
