// std imports
use std::{hash::Hash, str, sync::LazyLock};

// third-party imports
use enum_map::Enum;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};

/// Element represents a log element that can be styled.
#[repr(u8)]
#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy, Ord, PartialOrd, Enum, Deserialize, Serialize, EnumIter)]
#[serde(rename_all = "kebab-case")]
pub enum Element {
    Input,
    InputNumber,
    InputNumberInner,
    InputName,
    InputNameInner,
    Time,
    Level,
    LevelInner,
    Logger,
    LoggerInner,
    Caller,
    CallerInner,
    Message,
    MessageDelimiter,
    Field,
    Key,
    Array,
    Object,
    String,
    Number,
    Boolean,
    BooleanTrue,
    BooleanFalse,
    Null,
    Ellipsis,
}

impl Element {
    /// Returns true if this is an "inner" element (nested inside another element).
    pub fn is_inner(&self) -> bool {
        self.outer().is_some()
    }

    /// Returns corresponding "outer" element for an "inner" element.
    pub fn outer(&self) -> Option<Self> {
        match self {
            Self::InputNumber => Some(Self::Input),
            Self::InputName => Some(Self::Input),
            Self::InputNumberInner => Some(Self::InputNumber),
            Self::InputNameInner => Some(Self::InputName),
            Self::LevelInner => Some(Self::Level),
            Self::LoggerInner => Some(Self::Logger),
            Self::CallerInner => Some(Self::Caller),
            _ => None,
        }
    }

    /// Returns a list of all outer-inner element pairs.
    pub fn nested() -> &'static [(Self, Self)] {
        static PAIRS: LazyLock<Vec<(Element, Element)>> = LazyLock::new(|| {
            Element::iter()
                .filter_map(|element| element.outer().map(|parent| (parent, element)))
                .collect()
        });
        &PAIRS
    }
}
