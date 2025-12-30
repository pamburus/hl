// std imports
use std::ops::{Add, AddAssign};

// third-party imports
use enumset::{EnumSet, EnumSetType};
use serde::{Deserialize, Deserializer, Serialize};

// ---

#[derive(Debug, Deserialize, Serialize, EnumSetType)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    Bold,
    Faint,
    Italic,
    Underline,
    SlowBlink,
    RapidBlink,
    Reverse,
    Conceal,
    CrossedOut,
}

pub type ModeSet = EnumSet<Mode>;

impl Add<ModeSetDiff> for ModeSet {
    type Output = ModeSet;

    fn add(mut self, rhs: ModeSetDiff) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign<ModeSetDiff> for ModeSet {
    fn add_assign(&mut self, rhs: ModeSetDiff) {
        *self = (*self | rhs.adds) - rhs.removes;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ModeSetDiff {
    pub adds: ModeSet,
    pub removes: ModeSet,
}

impl ModeSetDiff {
    pub const fn new() -> Self {
        Self {
            adds: ModeSet::new(),
            removes: ModeSet::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.adds.is_empty() && self.removes.is_empty()
    }

    pub fn reversed(mut self) -> Self {
        std::mem::swap(&mut self.adds, &mut self.removes);
        self
    }

    pub fn add(mut self, mode: impl Into<ModeSet>) -> Self {
        self += ModeSetDiff::from(mode.into());
        self
    }

    pub fn remove(mut self, mode: impl Into<ModeSet>) -> Self {
        self += ModeSetDiff::from(mode.into()).reversed();
        self
    }
}

impl Add<ModeSetDiff> for ModeSetDiff {
    type Output = ModeSetDiff;

    fn add(mut self, rhs: ModeSetDiff) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign<ModeSetDiff> for ModeSetDiff {
    fn add_assign(&mut self, rhs: ModeSetDiff) {
        let adds = (self.adds | rhs.adds) - rhs.removes;
        let removes = (self.removes | rhs.removes) - rhs.adds;

        self.adds = adds;
        self.removes = removes;
    }
}

impl<'de> Deserialize<'de> for ModeSetDiff {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let diffs = Vec::<ModeDiff>::deserialize(deserializer)?;
        let mut result = ModeSetDiff::new();

        for diff in diffs {
            match diff.action {
                ModeDiffAction::Add => result.adds.insert(diff.mode),
                ModeDiffAction::Remove => result.removes.insert(diff.mode),
            };
        }

        Ok(result)
    }
}

impl Serialize for ModeSetDiff {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut diffs = Vec::new();

        for mode in self.adds.iter() {
            diffs.push(ModeDiff::add(mode));
        }

        for mode in self.removes.iter() {
            diffs.push(ModeDiff::remove(mode));
        }

        diffs.serialize(serializer)
    }
}

impl From<ModeSet> for ModeSetDiff {
    fn from(modes: ModeSet) -> Self {
        Self {
            adds: modes,
            removes: ModeSet::new(),
        }
    }
}

impl From<Mode> for ModeSetDiff {
    fn from(mode: Mode) -> Self {
        Self {
            adds: mode.into(),
            removes: ModeSet::new(),
        }
    }
}

// ---

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModeDiff {
    pub action: ModeDiffAction,
    pub mode: Mode,
}

impl ModeDiff {
    pub fn add(mode: Mode) -> Self {
        Self {
            action: ModeDiffAction::Add,
            mode,
        }
    }

    pub fn remove(mode: Mode) -> Self {
        Self {
            action: ModeDiffAction::Remove,
            mode,
        }
    }
}

impl<'de> Deserialize<'de> for ModeDiff {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        if let Some(s) = s.strip_prefix('+') {
            let mode: Mode = serde_plain::from_str(s).map_err(serde::de::Error::custom)?;
            Ok(ModeDiff::add(mode))
        } else if let Some(s) = s.strip_prefix('-') {
            let mode: Mode = serde_plain::from_str(s).map_err(serde::de::Error::custom)?;
            Ok(ModeDiff::remove(mode))
        } else {
            let mode: Mode = serde_plain::from_str(&s).map_err(serde::de::Error::custom)?;
            Ok(ModeDiff::add(mode))
        }
    }
}

impl Serialize for ModeDiff {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let prefix = match self.action {
            ModeDiffAction::Add => "+",
            ModeDiffAction::Remove => "-",
        };
        let mode_str = serde_plain::to_string(&self.mode).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&format!("{}{}", prefix, mode_str))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeDiffAction {
    Add,
    Remove,
}
