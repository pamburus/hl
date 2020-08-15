use std::cmp::{max, min, PartialOrd};

use bitmask::bitmask;
use chrono::{DateTime, Datelike, NaiveDateTime, Timelike, Utc};

use crate::fmtx;

use fmtx::{Alignment, Push};

// ---

bitmask! {
    pub mask Flags: u8 where flags Flag {
        SpacePadding  = 0b00000001,
        ZeroPadding   = 0b00000010,
        NoPadding     = 0b00000011,
        UpperCase     = 0b00000100,
        LowerCase     = 0b00001000,
        FromZero      = 0b00010000,
        FromSunday    = 0b00100000,
    }
}

use Flag::*;

type Precision = u8;

// ---

#[derive(Clone)]
pub enum Item {
    Char(u8),
    Century(Flags),
    Year(Flags),
    YearShort(Flags),
    MonthNumeric(Flags),
    MonthShort(Flags),
    MonthLong(Flags),
    Day(Flags),
    WeekdayNumeric(Flags),
    WeekdayShort(Flags),
    WeekdayLong(Flags),
    YearDay(Flags),
    YearQuarter(Flags),
    IsoWeek(Flags),
    IsoYear(Flags),
    IsoYearShort(Flags),
    Hour(Flags),
    Hour12(Flags),
    AmPm(Flags),
    Minute(Flags),
    Second(Flags),
    Nanosecond((Flags, Precision)),
    UnixTimestamp(Flags),
    TimezoneNumeric(Flags),
    TimezoneName(Flags),
}

impl AsRef<Item> for Item {
    fn as_ref(&self) -> &Item {
        self
    }
}

// ---

pub type DateTimeFormat = Vec<Item>;

// ---

#[derive(Clone)]
pub struct LinuxDateFormat<'a> {
    spec: &'a [u8],
    jump: &'static [u8],
    pad_counter: u8,
    pad: u8,
    flags: Flags,
}

impl<'a> LinuxDateFormat<'a> {
    pub fn new<T: AsRef<[u8]> + ?Sized>(spec: &'a T) -> Self {
        Self {
            spec: spec.as_ref(),
            jump: b"",
            pad_counter: 0,
            pad: b' ',
            flags: Flags::none(),
        }
    }

    pub fn compile(&mut self) -> DateTimeFormat {
        self.collect()
    }

    #[inline]
    fn pop(&mut self) -> Option<u8> {
        if self.pad_counter != 0 {
            self.pad_counter -= 1;
            Some(self.pad)
        } else if self.jump.len() != 0 {
            let result = self.jump[0];
            self.jump = &self.jump[1..];
            Some(result)
        } else if self.spec.len() != 0 {
            let result = self.spec[0];
            self.spec = &self.spec[1..];
            Some(result)
        } else {
            None
        }
    }

    #[inline]
    fn jump(&mut self, jump: &'static [u8], jump_width: u8, width: u8) -> Option<Item> {
        self.jump = jump;
        if jump_width < width {
            self.pad = b' ';
            self.pad_counter = width - jump_width;
        }
        self.next()
    }

    #[inline]
    fn jump_pad(&mut self, jump: &'static [u8], pad: u8, width: u8) -> Option<Item> {
        self.jump = jump;
        self.pad = pad;
        self.pad_counter = width;
        self.next()
    }

    #[inline]
    fn parse_item(&mut self) -> Option<Item> {
        let b = self.pop();
        let (flags, b) = self.parse_flags(b);
        let (width, b) = self.parse_width(b);
        let b = self.skip_modifier(b);
        self.flags = Flags::none();
        let with_padding = |default| {
            if flags.intersects(SpacePadding | ZeroPadding) {
                flags
            } else {
                flags | (default & (SpacePadding | ZeroPadding))
            }
        };
        let with_case = |default| {
            if flags.intersects(UpperCase | LowerCase) {
                flags
            } else {
                flags | (default & (UpperCase | LowerCase))
            }
        };
        let mut pad = |min_width, pad, zero, jump, value| {
            if width <= min_width || flags.contains(NoPadding) {
                value
            } else {
                self.flags = flags;
                let pad = if flags.contains(SpacePadding) {
                    b' '
                } else if flags.contains(ZeroPadding) {
                    zero
                } else {
                    pad
                };
                self.jump_pad(jump, pad, width - min_width)
            }
        };
        let precision = || min(width, 9);
        match b {
            Some(b'%') => pad(1, b' ', b' ', b"%%", Some(Item::Char(b'%'))),
            Some(b'a') => pad(3, b' ', b' ', b"%a", Some(Item::WeekdayShort(flags))),
            Some(b'A') => pad(9, b' ', b' ', b"%A", Some(Item::WeekdayLong(flags))),
            Some(b'b') => pad(3, b' ', b' ', b"%b", Some(Item::MonthShort(flags))),
            Some(b'B') => pad(3, b' ', b' ', b"%B", Some(Item::MonthLong(flags))),
            Some(b'c') => self.jump(b"%a %b %e %H:%M:%S %Y", 24, width),
            Some(b'C') => pad(2, b'0', b'0', b"%C", Some(Item::Century(flags))),
            Some(b'd') => pad(2, b'0', b'0', b"%d", Some(Item::Day(flags))),
            Some(b'D') => self.jump(b"%m/%d/%y", 8, width),
            Some(b'e') => pad(
                2,
                b' ',
                b'0',
                b"%e",
                Some(Item::Day(with_padding(SpacePadding))),
            ),
            Some(b'F') => self.jump(b"%Y-%m-%d", 10, width),
            Some(b'g') => pad(2, b'0', b'0', b"%g", Some(Item::IsoYearShort(flags))),
            Some(b'G') => pad(4, b'0', b'0', b"%G", Some(Item::IsoYear(flags))),
            Some(b'h') => pad(3, b' ', b' ', b"%h", Some(Item::MonthShort(flags))),
            Some(b'H') => pad(2, b'0', b'0', b"%H", Some(Item::Hour(flags))),
            Some(b'I') => pad(2, b'0', b'0', b"%I", Some(Item::Hour12(flags))),
            Some(b'j') => pad(3, b'0', b'0', b"%j", Some(Item::YearDay(flags))),
            Some(b'k') => pad(
                2,
                b' ',
                b'0',
                b"%k",
                Some(Item::Hour(with_padding(SpacePadding))),
            ),
            Some(b'l') => pad(
                2,
                b' ',
                b'0',
                b"%l",
                Some(Item::Hour12(with_padding(SpacePadding))),
            ),
            Some(b'm') => pad(2, b'0', b'0', b"%m", Some(Item::MonthNumeric(flags))),
            Some(b'M') => pad(2, b'0', b'0', b"%M", Some(Item::Minute(flags))),
            Some(b'n') => Some(Item::Char(b' ')),
            Some(b'N') => Some(Item::Nanosecond((flags, precision()))),
            Some(b'p') => pad(2, b' ', b' ', b"%p", Some(Item::AmPm(with_case(UpperCase)))),
            Some(b'P') => pad(2, b' ', b' ', b"%P", Some(Item::AmPm(with_case(LowerCase)))),
            Some(b'q') => pad(1, b'0', b'0', b"%q", Some(Item::YearQuarter(flags))),
            Some(b'r') => self.jump(b"%I:%M:%S %p", 11, width),
            Some(b'R') => self.jump(b"%H:%M", 5, width),
            Some(b's') => pad(
                2,
                b' ',
                b'0',
                b"%s",
                Some(Item::UnixTimestamp(with_padding(SpacePadding))),
            ),
            Some(b'S') => pad(2, b'0', b'0', b"%S", Some(Item::Second(flags))),
            Some(b't') => Some(Item::Char(b' ')),
            Some(b'T') => self.jump(b"%H:%M:%S", 8, width),
            Some(b'u') => pad(1, b'0', b'0', b"%u", Some(Item::WeekdayNumeric(flags))),
            Some(b'V') => pad(2, b'0', b'0', b"%V", Some(Item::IsoWeek(flags))),
            Some(b'w') => pad(
                1,
                b'0',
                b'0',
                b"%w",
                Some(Item::WeekdayNumeric(flags | FromZero | FromSunday)),
            ),
            Some(b'W') => pad(2, b'0', b'0', b"%W", Some(Item::IsoWeek(flags | FromZero))),
            Some(b'x') => self.jump(b"%m/%d/%y", 8, width),
            Some(b'X') => self.jump(b"%H:%M:%S", 8, width),
            Some(b'y') => pad(2, b'0', b'0', b"%y", Some(Item::YearShort(flags))),
            Some(b'Y') => pad(4, b'0', b'0', b"%Y", Some(Item::Year(flags))),
            Some(b'z') => pad(5, b' ', b'0', b"%z", Some(Item::TimezoneNumeric(flags))),
            Some(b'Z') => pad(3, b' ', b' ', b"%Z", Some(Item::TimezoneName(flags))),
            _ => None,
        }
    }

    #[inline]
    fn parse_flags(&mut self, mut b: Option<u8>) -> (Flags, Option<u8>) {
        let mut flags = self.flags;
        loop {
            match b {
                Some(b'-') => flags.set(NoPadding),
                Some(b'_') => {
                    flags.set(SpacePadding);
                    flags.unset(ZeroPadding);
                }
                Some(b'0') => {
                    flags.unset(SpacePadding);
                    flags.set(ZeroPadding);
                }
                Some(b'^') => flags.set(UpperCase),
                Some(b'#') => flags.set(LowerCase),
                _ => break,
            }
            b = self.pop()
        }
        (flags, b)
    }

    #[inline]
    fn parse_width(&mut self, mut b: Option<u8>) -> (u8, Option<u8>) {
        let mut width: u8 = 0;
        loop {
            match b {
                Some(d @ b'0'..=b'9') => width = width * 10 + (d - b'0'),
                _ => break,
            }
            b = self.pop()
        }
        (width, b)
    }

    #[inline]
    fn skip_modifier(&mut self, b: Option<u8>) -> Option<u8> {
        match b {
            Some(b'E') | Some(b'O') => self.pop(),
            _ => b,
        }
    }
}

impl<'a> Iterator for LinuxDateFormat<'a> {
    type Item = Item;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.pop() {
            None => None,
            Some(b'%') => {
                let checkpoint = self.clone();
                match self.parse_item() {
                    Some(item) => Some(item),
                    None => {
                        *self = checkpoint;
                        Some(Item::Char(b'%'))
                    }
                }
            }
            Some(b) => Some(Item::Char(b)),
        }
    }
}

impl<'a> From<LinuxDateFormat<'a>> for Vec<Item> {
    fn from(f: LinuxDateFormat<'a>) -> Self {
        f.collect()
    }
}

// ---

pub fn format_date<T, B, F>(buf: &mut B, dt: DateTime<Utc>, format: F)
where
    B: Push<u8>,
    T: AsRef<Item>,
    F: IntoIterator<Item = T>,
{
    let dt = dt.naive_utc();
    let mut f = Formatter::new(buf);
    for item in format {
        match *item.as_ref() {
            Item::Char(b) => {
                f.char(b);
            }
            Item::Century(flags) => {
                f.numeric(dt.year() / 100, 2, flags);
            }
            Item::Year(flags) => {
                f.numeric(dt.year() % 10000, 4, flags);
            }
            Item::YearShort(flags) => {
                f.numeric(dt.year() % 100, 2, flags);
            }
            Item::YearQuarter(flags) => {
                f.quarter(dt.month0() as usize, flags);
            }
            Item::MonthNumeric(flags) => {
                f.month_numeric(dt.month0() as usize, flags);
            }
            Item::MonthShort(flags) => {
                f.month_short(dt.month0() as usize, flags);
            }
            Item::MonthLong(flags) => {
                f.month_long(dt.month0() as usize, flags);
            }
            Item::Day(flags) => {
                f.numeric(dt.day(), 2, flags);
            }
            Item::WeekdayNumeric(flags) => {
                f.weekday_numeric(&dt, flags);
            }
            Item::WeekdayShort(flags) => {
                f.weekday_short(dt.weekday().num_days_from_monday() as usize, flags);
            }
            Item::WeekdayLong(flags) => {
                f.weekday_long(dt.weekday().num_days_from_monday() as usize, flags);
            }
            Item::YearDay(flags) => {
                f.year_day(&dt, flags);
            }
            Item::IsoWeek(flags) => {
                f.iso_week(&dt, flags);
            }
            Item::IsoYear(flags) => {
                f.numeric(dt.iso_week().year(), 4, flags);
            }
            Item::IsoYearShort(flags) => {
                f.numeric(dt.iso_week().year() % 100, 2, flags);
            }
            Item::Hour(flags) => {
                f.numeric(dt.hour(), 2, flags);
            }
            Item::Hour12(flags) => {
                f.numeric(dt.hour12().1, 2, flags);
            }
            Item::AmPm(flags) => {
                f.am_pm(dt.hour12().0 as usize, flags);
            }
            Item::Minute(flags) => {
                f.numeric(dt.minute(), 2, flags);
            }
            Item::Second(flags) => {
                f.numeric(dt.second(), 2, flags);
            }
            Item::Nanosecond((flags, precision)) => {
                let nsec = dt.nanosecond();
                let (value, width) = match precision {
                    1 => (nsec / 100000000, 1),
                    2 => (nsec / 10000000, 2),
                    3 => (nsec / 1000000, 3),
                    4 => (nsec / 100000, 4),
                    5 => (nsec / 10000, 5),
                    6 => (nsec / 1000, 6),
                    7 => (nsec / 100, 7),
                    8 => (nsec / 10, 8),
                    _ => (nsec, 9),
                };
                f.numeric(value, width, flags);
            }
            Item::UnixTimestamp(flags) => {
                f.numeric(dt.timestamp(), 10, flags);
            }
            Item::TimezoneNumeric(flags) => {
                f.numeric(0, 5, flags);
            }
            Item::TimezoneName(flags) => {
                let text = if flags.contains(LowerCase) {
                    b"utc"
                } else {
                    b"UTC"
                };
                f.text(text);
            }
        }
    }
}

// ---

pub fn reformat_rfc3339_timestamp<'a, T, B, F>(buf: &mut B, ts: &'a str, format: F)
where
    T: AsRef<Item>,
    B: Push<u8>,
    F: IntoIterator<Item = T>,
{
    let mut dt_cache = None;
    let mut dt = || {
        if let Some(dt) = dt_cache {
            dt
        } else {
            let dt = DateTime::parse_from_rfc3339(ts)
                .ok()
                .map(|dt| dt.naive_utc());
            dt_cache = Some(dt);
            dt
        }
    };

    let ts = ts.as_bytes();

    let mut month_cache = None;
    let mut month = || {
        if let Some(month) = month_cache {
            month
        } else {
            let month = (ts[6] - b'0') + (ts[5] - b'0') * 10;
            let month = min(max(month, 1), 12) - 1;
            let month = month as usize;
            month_cache = Some(month);
            month
        }
    };

    let mut hour_cache = None;
    let mut hour = || {
        if let Some(hour) = hour_cache {
            hour
        } else {
            let hour = (ts[12] - b'0') + (ts[11] - b'0') * 10;
            let hour = min(hour, 23);
            hour_cache = Some(hour);
            hour
        }
    };

    let mut f = Formatter::new(buf);

    for item in format {
        match *item.as_ref() {
            Item::Char(b) => {
                f.char(b);
            }
            Item::Century(n) => {
                reformat_numeric_2(f.buf, n, ts[0], ts[1]);
            }
            Item::Year(n) => {
                reformat_numeric(f.buf, n, &ts[0..4]);
            }
            Item::YearShort(n) => {
                reformat_numeric_2(f.buf, n, ts[2], ts[3]);
            }
            Item::YearQuarter(flags) => {
                f.quarter(month(), flags);
            }
            Item::MonthNumeric(n) => {
                reformat_numeric_2(f.buf, n, ts[5], ts[6]);
            }
            Item::MonthShort(flags) => {
                f.month_short(month(), flags);
            }
            Item::MonthLong(flags) => {
                f.month_long(month(), flags);
            }
            Item::Day(n) => {
                reformat_numeric_2(f.buf, n, ts[8], ts[9]);
            }
            Item::WeekdayNumeric(flags) => {
                if let Some(dt) = dt() {
                    f.weekday_numeric(&dt, flags);
                } else {
                    f.char(b'?');
                };
            }
            Item::WeekdayShort(flags) => {
                if let Some(dt) = dt() {
                    f.weekday_short(dt.weekday().num_days_from_monday() as usize, flags);
                } else {
                    f.text(b"(?)");
                }
            }
            Item::WeekdayLong(flags) => {
                if let Some(dt) = dt() {
                    f.weekday_long(dt.weekday().num_days_from_monday() as usize, flags);
                } else {
                    let text = b"(?)";
                    if flags.contains(NoPadding) {
                        f.text(text);
                    } else {
                        align_text(f.buf, Some(Alignment::Right), MAX_WEEKDAY_LONG_LEN, text);
                    }
                }
            }
            Item::YearDay(flags) => {
                if let Some(dt) = dt() {
                    f.year_day(&dt, flags);
                } else {
                    f.text(b"(?)")
                }
            }
            Item::IsoWeek(flags) => {
                if let Some(dt) = dt() {
                    f.iso_week(&dt, flags);
                } else {
                    f.text(b"??");
                }
            }
            Item::IsoYear(flags) => {
                if let Some(dt) = dt() {
                    f.numeric(dt.iso_week().year(), 4, flags);
                } else {
                    f.text(b"(??)");
                }
            }
            Item::IsoYearShort(flags) => {
                if let Some(dt) = dt() {
                    f.numeric(dt.iso_week().year() % 100, 2, flags);
                } else {
                    f.text(b"??");
                }
            }
            Item::Hour(n) => {
                reformat_numeric_2(f.buf, n, ts[11], ts[12]);
            }
            Item::Hour12(n) => {
                let hour = hour() / 2;
                let hour = if hour == 0 { 12 } else { hour };
                f.numeric(hour, 2, n);
            }
            Item::AmPm(flags) => {
                let hour = hour();
                f.am_pm((hour >= 12) as usize, flags);
            }
            Item::Minute(n) => {
                reformat_numeric_2(f.buf, n, ts[14], ts[15]);
            }
            Item::Second(n) => {
                reformat_numeric_2(f.buf, n, ts[17], ts[18]);
            }
            Item::Nanosecond((_, width)) => {
                let nsec = if ts.len() > 20 {
                    if let Some(pos) = (&ts[20..]).iter().position(|&b| !b.is_ascii_digit()) {
                        &ts[20..20 + pos]
                    } else {
                        &ts[19..19]
                    }
                } else {
                    &ts[19..19]
                };
                let precision = if width == 0 { 9 } else { width as usize };
                if precision < nsec.len() {
                    f.text(&nsec[..precision])
                } else {
                    f.text(nsec);
                    for _ in nsec.len()..precision {
                        f.char(b'0')
                    }
                }
            }
            Item::UnixTimestamp(flags) => {
                if let Some(dt) = dt() {
                    f.numeric(dt.timestamp(), 10, flags);
                }
            }
            Item::TimezoneNumeric(_) => {
                if let Some(pos) = (&ts[20..]).iter().position(|&b| !b.is_ascii_digit()) {
                    let tz = &ts[20 + pos..];
                    if tz == b"Z" || tz == b"z" {
                        f.text(b"+0000");
                    } else {
                        f.text(tz);
                    }
                }
            }
            Item::TimezoneName(flags) => {
                let value = if let Some(pos) = (&ts[20..]).iter().position(|&b| !b.is_ascii_digit())
                {
                    let tz = &ts[20 + pos..];
                    if tz == b"Z"
                        || tz == b"z"
                        || ((tz[0] == b'+' || tz[0] == b'-') && &tz[1..] == b"0000")
                    {
                        if flags.contains(LowerCase) {
                            b"utc"
                        } else {
                            b"UTC"
                        }
                    } else {
                        b"(?)"
                    }
                } else {
                    if flags.contains(LowerCase) {
                        b"utc"
                    } else {
                        b"UTC"
                    }
                };
                f.text(value);
            }
        }
    }
}

// ---

#[inline]
fn format_int<B, I>(buf: &mut B, value: I, width: usize, flags: Flags)
where
    B: Push<u8>,
    I: itoa::Integer + PartialOrd + Default,
{
    let pad = if flags.contains(NoPadding) {
        None
    } else if flags.contains(SpacePadding) {
        Some(b' ')
    } else {
        Some(b'0')
    };
    let mut ibuf = itoa::Buffer::new();
    let negative = value < I::default();
    let mut b = ibuf.format(value).as_bytes();
    if let Some(pad) = pad {
        if b.len() <= width {
            let pad = if negative { b' ' } else { pad };
            for _ in b.len()..width {
                buf.push(pad)
            }
        } else {
            let offset = b.len() - width;
            b = &b[offset..];
            if negative {
                buf.push(b'-');
                b = &b[1..];
            }
        }
    }
    buf.extend_from_slice(b);
}

// ---

struct Formatter<'a, B: Push<u8>> {
    buf: &'a mut B,
}

impl<'a, B: Push<u8>> Formatter<'a, B> {
    #[inline]
    fn new(buf: &'a mut B) -> Self {
        Self { buf }
    }

    #[inline]
    fn char(&mut self, b: u8) {
        self.buf.push(b);
    }

    #[inline]
    fn text(&mut self, text: &[u8]) {
        self.buf.extend_from_slice(text);
    }

    #[inline]
    fn am_pm(&mut self, index: usize, flags: Flags) {
        let text = AM_PM[case_index(flags)][index].as_bytes();
        self.buf.extend_from_slice(text);
    }

    #[inline]
    fn month_numeric(&mut self, index: usize, flags: Flags) {
        let month = if flags.contains(FromZero) {
            index
        } else {
            index + 1
        };
        self.numeric(month, 2, flags);
    }

    #[inline]
    fn month_short(&mut self, index: usize, flags: Flags) {
        let text = &MONTHS_SHORT[case_index(flags)][index].as_bytes();
        self.buf.extend_from_slice(text);
    }

    #[inline]
    fn month_long(&mut self, index: usize, flags: Flags) {
        let text = &MONTHS_LONG[case_index(flags)][index].as_bytes();
        if flags.contains(NoPadding) {
            self.buf.extend_from_slice(text)
        } else {
            align_text(self.buf, Some(Alignment::Right), MAX_MONTH_LONG_LEN, text);
        }
    }

    #[inline]
    fn weekday_short(&mut self, index: usize, flags: Flags) {
        let text = &WEEKDAYS_SHORT[case_index(flags)][index].as_bytes();
        self.buf.extend_from_slice(text);
    }

    #[inline]
    fn weekday_long(&mut self, index: usize, flags: Flags) {
        let text = &WEEKDAYS_LONG[case_index(flags)][index].as_bytes();
        if flags.contains(NoPadding) {
            self.buf.extend_from_slice(text)
        } else {
            align_text(self.buf, Some(Alignment::Right), MAX_WEEKDAY_LONG_LEN, text);
        }
    }

    #[inline]
    fn weekday_numeric(&mut self, dt: &NaiveDateTime, flags: Flags) {
        let value = if flags.contains(FromSunday) {
            if flags.contains(FromZero) {
                dt.weekday().num_days_from_sunday()
            } else {
                dt.weekday().number_from_sunday()
            }
        } else {
            if flags.contains(FromZero) {
                dt.weekday().num_days_from_monday()
            } else {
                dt.weekday().number_from_monday()
            }
        };
        self.numeric(value, 1, flags);
    }

    #[inline]
    fn quarter(&mut self, month0: usize, flags: Flags) {
        let value = month0 / 4;
        let value = if flags.contains(FromZero) {
            value
        } else {
            value + 1
        };
        self.numeric(value, 1, flags);
    }

    #[inline]
    fn year_day(&mut self, dt: &NaiveDateTime, flags: Flags) {
        let value = if flags.contains(FromZero) {
            dt.ordinal0()
        } else {
            dt.ordinal()
        };
        self.numeric(value, 3, flags);
    }

    #[inline]
    fn iso_week(&mut self, dt: &NaiveDateTime, flags: Flags) {
        let value = if flags.contains(FromZero) {
            dt.iso_week().week0()
        } else {
            dt.iso_week().week()
        };
        self.numeric(value, 2, flags);
    }

    #[inline]
    fn numeric<I>(&mut self, value: I, width: usize, flags: Flags)
    where
        B: Push<u8>,
        I: itoa::Integer + PartialOrd + Default,
    {
        format_int(self.buf, value, width, flags)
    }
}

// --

#[inline]
fn align_text<B: Push<u8>>(buf: &mut B, alignment: Option<Alignment>, width: usize, s: &[u8]) {
    let pad = b' ';
    match alignment {
        None => {
            buf.extend_from_slice(s);
        }
        Some(Alignment::Left) => {
            buf.extend_from_slice(s);
            for _ in s.len()..width {
                buf.push(pad);
            }
        }
        Some(Alignment::Right) => {
            for _ in s.len()..width {
                buf.push(pad);
            }
            buf.extend_from_slice(s);
        }
        Some(Alignment::Center) => {
            let n = (width - min(s.len(), width) + 1) / 2;
            for _ in 0..n {
                buf.push(pad);
            }
            buf.extend_from_slice(s);
            for _ in n + s.len()..width {
                buf.push(pad);
            }
        }
    }
}

// ---

#[inline]
fn reformat_numeric_2<B: Push<u8>>(buf: &mut B, flags: Flags, b0: u8, b1: u8) {
    if b0 == b'0' {
        if !flags.contains(NoPadding) {
            if flags.contains(SpacePadding) {
                buf.push(b' ')
            } else {
                buf.push(b'0')
            }
        }
    } else {
        buf.push(b0)
    }
    buf.push(b1)
}

// ---

#[inline]
fn reformat_numeric<B: Push<u8>>(buf: &mut B, flags: Flags, b: &[u8]) {
    if !flags.contains(NoPadding) && !flags.contains(SpacePadding) {
        buf.extend_from_slice(&b)
    } else {
        let pos = (&b[0..b.len() - 1])
            .iter()
            .map(|&b| b == b'0')
            .position(|x| x == false);
        if !flags.contains(NoPadding) {
            for _ in 0..pos.unwrap_or_default() {
                buf.push(b' ')
            }
        }
        for i in pos.unwrap_or_default()..4 {
            buf.push(b[i])
        }
    }
}

// ---

#[inline]
fn case_index(flags: Flags) -> usize {
    if flags.contains(UpperCase) {
        1
    } else if flags.contains(LowerCase) {
        2
    } else {
        0
    }
}

// ---

const MONTHS_SHORT: [[&str; 12]; 3] = [
    [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ],
    [
        "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
    ],
    [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ],
];

const MONTHS_LONG: [[&str; 12]; 3] = [
    [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ],
    [
        "JANUARY",
        "FEBRUARY",
        "MARCH",
        "APRIL",
        "MAY",
        "JUNE",
        "JULY",
        "AUGUST",
        "SEPTEMBER",
        "OCTOBER",
        "NOVEMBER",
        "DECEMBER",
    ],
    [
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ],
];

const MAX_MONTH_LONG_LEN: usize = 9;

const WEEKDAYS_SHORT: [[&str; 7]; 3] = [
    ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"],
    ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"],
    ["sun", "mon", "tue", "wed", "thu", "fri", "sat"],
];

const WEEKDAYS_LONG: [[&str; 7]; 3] = [
    [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ],
    [
        "MONDAY",
        "TUESDAY",
        "WEDNESDAY",
        "THURSDAY",
        "FRIDAY",
        "SATURDAY",
        "SUNDAY",
    ],
    [
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ],
];

const MAX_WEEKDAY_LONG_LEN: usize = 9;

const AM_PM: [[&str; 2]; 3] = [["AM", "PM"], ["AM", "PM"], ["am", "pm"]];
