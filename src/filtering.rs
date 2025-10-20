use std::{collections::HashMap, hash::Hash};
use wildflower::{Pattern, WILDCARD_MANY_CHAR, WILDCARD_SINGLE_CHAR};

// ---

pub trait KeyNormalize: Clone {
    fn normalize(&self, byte: u8) -> u8;
}

// ---

#[derive(Default, Clone)]
#[allow(dead_code)]
pub struct NoNormalizing {}

impl KeyNormalize for NoNormalizing {
    #[inline(always)]
    fn normalize(&self, byte: u8) -> u8 {
        byte
    }
}

// ---

#[derive(Default, Clone)]
pub struct DefaultNormalizing {}

impl KeyNormalize for DefaultNormalizing {
    #[inline(always)]
    fn normalize(&self, byte: u8) -> u8 {
        if byte == b'_' { b'-' } else { byte.to_ascii_lowercase() }
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
    #[inline(always)]
    pub fn apply(&self, other: Self) -> Self {
        match other {
            Self::Unspecified => *self,
            Self::Include => other,
            Self::Exclude => other,
        }
    }
}

impl Default for IncludeExcludeSetting {
    #[inline(always)]
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
    #[inline(always)]
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
    fallback: Option<Box<IncludeExcludeKeyFilter<N>>>,
    options: MatchOptions<N>,
    setting: IncludeExcludeSetting,
}

impl<N: KeyNormalize> IncludeExcludeKeyFilter<N> {
    pub fn new(options: MatchOptions<N>) -> Self {
        Self {
            children: HashMap::new(),
            patterns: Vec::new(),
            fallback: None,
            options,
            setting: IncludeExcludeSetting::default(),
        }
    }

    pub fn entry<'a>(&'a mut self, key: &'a str) -> &'a mut IncludeExcludeKeyFilter<N> {
        let (head, tail) = self.split(key);

        if Self::is_pattern(&head) {
            return self.add_pattern(head, tail);
        }

        self.set_fallback(self.setting);
        let child = self.children.entry(head).or_insert(Self::new(self.options.clone()));
        match tail {
            None => child,
            Some(tail) => child.entry(tail),
        }
    }

    pub fn get<'a>(&'a self, key: &str) -> Option<&'a IncludeExcludeKeyFilter<N>> {
        if self.leaf() {
            return if self.setting == IncludeExcludeSetting::Unspecified {
                None
            } else {
                Some(self)
            };
        }

        let (head, tail) = self.split(key);

        let found = |child: &'a Self| match tail {
            None => Some(child),
            Some(tail) => child.get(tail),
        };

        if let Some(child) = self.children.get(&head) {
            return found(child);
        }

        if !self.patterns.is_empty() {
            let head = head.as_str();
            for (pattern, child) in self.patterns.iter().rev() {
                if pattern.matches(head) {
                    return found(child);
                }
            }
        }

        self.fallback.as_deref()
    }

    #[inline(always)]
    pub fn include(&mut self) -> &mut Self {
        self.reset(IncludeExcludeSetting::Include);
        self.update_fallback();
        self
    }

    #[inline(always)]
    pub fn included(mut self) -> Self {
        self.include();
        self
    }

    #[inline(always)]
    pub fn exclude(&mut self) -> &mut Self {
        self.reset(IncludeExcludeSetting::Exclude);
        self.update_fallback();
        self
    }

    #[inline(always)]
    pub fn excluded(mut self) -> Self {
        self.exclude();
        self
    }

    #[inline(always)]
    pub fn setting(&self) -> IncludeExcludeSetting {
        self.setting
    }

    #[inline(always)]
    pub fn leaf(&self) -> bool {
        self.children.is_empty() && self.patterns.is_empty()
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
                self.update_fallback();
                &mut self.patterns.last_mut().unwrap().1
            }
        };
        match tail {
            None => item,
            Some(tail) => item.entry(tail),
        }
    }

    fn update_fallback(&mut self) {
        if self.setting == IncludeExcludeSetting::Unspecified || self.leaf() {
            self.fallback = None;
        } else {
            self.set_fallback(self.setting);
        }
    }

    fn set_fallback(&mut self, setting: IncludeExcludeSetting) {
        if setting == IncludeExcludeSetting::Unspecified {
            self.fallback = None;
        } else {
            let mut fallback = self
                .fallback
                .take()
                .unwrap_or_else(|| Box::new(Self::new(self.options.clone())));
            fallback.setting = setting;
            self.fallback = Some(fallback);
        }
    }

    fn reset(&mut self, setting: IncludeExcludeSetting) {
        self.children.clear();
        self.patterns.clear();
        self.fallback = None;
        self.setting = setting;
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
    #[inline(always)]
    fn as_bytes(&self) -> &[u8] {
        match self {
            Key::Short(v) => v.as_ref(),
            Key::Long(v) => v.as_slice(),
        }
    }

    #[inline(always)]
    fn as_str(&self) -> &str {
        std::str::from_utf8(self.as_bytes()).unwrap()
    }
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ---

#[cfg(test)]
mod tests;
