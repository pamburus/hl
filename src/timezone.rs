use chrono::{FixedOffset, Local, LocalResult, NaiveDate, NaiveDateTime, Offset, TimeZone, Utc};
use chrono_tz::OffsetName;

use std::{convert::From, fmt};

// ---

#[derive(Clone, Debug, Copy)]
pub enum TzOffset {
    Local(FixedOffset),
    Fixed(FixedOffset),
    IANA(<chrono_tz::Tz as TimeZone>::Offset),
}

impl OffsetName for TzOffset {
    fn tz_id(&self) -> &str {
        match self {
            Self::Local(_) => "(Local)",
            Self::Fixed(_) => "(Fixed)",
            Self::IANA(offset) => offset.tz_id(),
        }
    }

    fn abbreviation(&self) -> &str {
        match self {
            Self::Local(_) => "(L)",
            Self::Fixed(_) => "(F)",
            Self::IANA(offset) => offset.abbreviation(),
        }
    }
}

impl Offset for TzOffset {
    fn fix(&self) -> FixedOffset {
        match self {
            Self::Local(offset) => offset.fix(),
            Self::Fixed(offset) => offset.fix(),
            Self::IANA(offset) => offset.fix(),
        }
    }
}

impl fmt::Display for TzOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local(offset) => write!(f, "{}", offset),
            Self::Fixed(offset) => write!(f, "{}", offset),
            Self::IANA(offset) => write!(f, "{}", offset),
        }
    }
}

impl From<FixedOffset> for TzOffset {
    fn from(offset: FixedOffset) -> Self {
        TzOffset::Fixed(offset)
    }
}

// ---

#[derive(Clone, Debug, Copy)]
pub enum Tz {
    Local,
    FixedOffset(FixedOffset),
    IANA(chrono_tz::Tz),
}

impl Tz {
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Local => None,
            Self::FixedOffset(_) => None,
            Self::IANA(tz) => Some(tz.name()),
        }
    }

    pub fn is_utc(&self) -> bool {
        match self {
            Self::Local => false,
            Self::FixedOffset(offset) => offset.local_minus_utc() == 0,
            Self::IANA(tz) => *tz == chrono_tz::UTC,
        }
    }
}

impl TimeZone for Tz {
    type Offset = TzOffset;

    /// Reconstructs the time zone from the offset.
    fn from_offset(offset: &Self::Offset) -> Self {
        match offset {
            Self::Offset::Local(_) => Self::Local,
            Self::Offset::Fixed(offset) => Self::FixedOffset(FixedOffset::from_offset(offset)),
            Self::Offset::IANA(offset) => Self::IANA(chrono_tz::Tz::from_offset(offset)),
        }
    }

    /// Creates the offset(s) for given local `NaiveDate` if possible.
    fn offset_from_local_date(&self, local: &NaiveDate) -> LocalResult<Self::Offset> {
        match self {
            Self::Local => Local.offset_from_local_date(local).map(Self::Offset::Local),
            Self::FixedOffset(tz) => tz.offset_from_local_date(local).map(Self::Offset::Fixed),
            Self::IANA(tz) => tz.offset_from_local_date(local).map(Self::Offset::IANA),
        }
    }

    /// Creates the offset(s) for given local `NaiveDateTime` if possible.
    fn offset_from_local_datetime(&self, local: &NaiveDateTime) -> LocalResult<Self::Offset> {
        match self {
            Self::Local => Local
                .offset_from_local_datetime(local)
                .map(Self::Offset::Local),
            Self::FixedOffset(tz) => tz
                .offset_from_local_datetime(local)
                .map(Self::Offset::Fixed),
            Self::IANA(tz) => tz.offset_from_local_datetime(local).map(Self::Offset::IANA),
        }
    }

    /// Creates the offset for given UTC `NaiveDate`. This cannot fail.
    fn offset_from_utc_date(&self, utc: &NaiveDate) -> Self::Offset {
        match self {
            Self::Local => Self::Offset::Local(Local.offset_from_utc_date(utc)),
            Self::FixedOffset(tz) => Self::Offset::Fixed(tz.offset_from_utc_date(utc)),
            Self::IANA(tz) => Self::Offset::IANA(tz.offset_from_utc_date(utc)),
        }
    }

    /// Creates the offset for given UTC `NaiveDateTime`. This cannot fail.
    fn offset_from_utc_datetime(&self, utc: &NaiveDateTime) -> Self::Offset {
        match self {
            Self::Local => Self::Offset::Local(Local.offset_from_utc_datetime(utc)),
            Self::FixedOffset(tz) => Self::Offset::Fixed(tz.offset_from_utc_datetime(utc)),
            Self::IANA(tz) => Self::Offset::IANA(tz.offset_from_utc_datetime(utc)),
        }
    }
}

impl From<Local> for Tz {
    fn from(_: Local) -> Self {
        Tz::Local
    }
}

impl From<Utc> for Tz {
    fn from(tz: Utc) -> Self {
        Tz::FixedOffset(tz.fix())
    }
}

impl From<FixedOffset> for Tz {
    fn from(tz: FixedOffset) -> Self {
        Tz::FixedOffset(tz)
    }
}

impl From<chrono_tz::Tz> for Tz {
    fn from(tz: chrono_tz::Tz) -> Self {
        Tz::IANA(tz)
    }
}

impl fmt::Display for Tz {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local => write!(f, "(L)"),
            Self::FixedOffset(tz) => write!(f, "{}", tz.fix()),
            Self::IANA(tz) => write!(f, "{}", tz.name()),
        }
    }
}
