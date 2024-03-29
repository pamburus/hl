use std::{collections::HashMap, hash::Hash};
use wildflower::{Pattern, WILDCARD_MANY_CHAR, WILDCARD_SINGLE_CHAR};

// ---

pub trait KeyNormalize: Clone {
    fn normalize(&self, byte: u8) -> u8;
}

// ---

#[derive(Default, Clone)]
pub struct NoNormalizing {}

impl KeyNormalize for NoNormalizing {
    #[inline]
    fn normalize(&self, byte: u8) -> u8 {
        byte
    }
}

// ---

#[derive(Default, Clone)]
pub struct DefaultNormalizing {}

impl KeyNormalize for DefaultNormalizing {
    #[inline]
    fn normalize(&self, byte: u8) -> u8 {
        if byte == b'_' {
            b'-'
        } else {
            byte.to_ascii_lowercase()
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

#[derive(PartialEq, Eq, Clone)]
pub struct MatchOptions<N: KeyNormalize> {
    pub delimiter: u8,
    pub norm: N,
}

impl<N: KeyNormalize + Default> Default for MatchOptions<N> {
    fn default() -> Self {
        Self {
            delimiter: b'.',
            norm: N::default(),
        }
    }
}

// ---

#[derive(Default)]
pub struct IncludeExcludeKeyFilter<N: KeyNormalize> {
    children: HashMap<Key, IncludeExcludeKeyFilter<N>>,
    patterns: Vec<(Pattern<String>, IncludeExcludeKeyFilter<N>)>,
    options: MatchOptions<N>,
    setting: IncludeExcludeSetting,
}

impl<N: KeyNormalize> IncludeExcludeKeyFilter<N> {
    pub fn new(options: MatchOptions<N>) -> Self {
        Self {
            children: HashMap::new(),
            patterns: Vec::new(),
            options,
            setting: IncludeExcludeSetting::default(),
        }
    }

    pub fn entry<'a>(&'a mut self, key: &'a str) -> &'a mut IncludeExcludeKeyFilter<N> {
        let (head, tail) = self.split(key);

        if Self::is_pattern(&head) {
            return self.add_pattern(head, tail);
        }

        let child = self.children.entry(head).or_insert(Self::new(self.options.clone()));
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
            let head = head.as_str();
            for (pattern, child) in self.patterns.iter().rev() {
                if pattern.matches(head) {
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

    fn split<'a>(&self, key: &'a str) -> (Key, Option<&'a str>) {
        let bytes = key.as_bytes();
        let n = bytes.iter().take_while(|&&x| x != self.options.delimiter).count();
        let head = bytes[..n].iter().map(|&x| self.options.norm.normalize(x));
        let head = if n <= 64 {
            Key::Short(head.collect())
        } else {
            Key::Long(head.collect())
        };
        let tail = if n == key.len() { None } else { Some(&key[n + 1..]) };
        (head, tail)
    }

    fn is_pattern(key: &Key) -> bool {
        let b = key.as_bytes();
        b.contains(&(WILDCARD_MANY_CHAR as u8)) || b.contains(&(WILDCARD_SINGLE_CHAR as u8))
    }

    fn add_pattern<'a>(&'a mut self, key: Key, tail: Option<&'a str>) -> &'a mut IncludeExcludeKeyFilter<N> {
        let pattern = Pattern::new(key.to_string());
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

#[derive(PartialEq, Eq, Hash, Debug)]
enum Key {
    Short(heapless::Vec<u8, 64>),
    Long(Vec<u8>),
}

impl PartialEq<&str> for Key {
    fn eq(&self, other: &&str) -> bool {
        match self {
            Key::Short(v) => v == other.as_bytes(),
            Key::Long(v) => v.as_slice() == other.as_bytes(),
        }
    }
}

impl Key {
    fn as_bytes(&self) -> &[u8] {
        match self {
            Key::Short(v) => v.as_ref(),
            Key::Long(v) => v.as_slice(),
        }
    }

    fn as_str(&self) -> &str {
        std::str::from_utf8(self.as_bytes()).unwrap()
    }

    fn to_string(&self) -> String {
        self.as_str().to_string()
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
