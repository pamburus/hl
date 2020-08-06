use std::cmp::{max, min, PartialOrd};

use chrono::{DateTime, Datelike, Timelike, Utc};

use crate::fmtx;

use fmtx::{aligned, Push};

pub enum Item {
    Char(u8),             // character
    Century(u8),          // pad
    Year(u8),             // pad
    YearShort(u8),        // pad
    MonthNumeric(u8),     // pad
    MonthShort,           // -
    MonthLong(u8),        // pad
    Day(u8),              // pad
    WeekdayNumeric(bool), // true - starting from sunday, 0..6 | false - starting from monday, 1..7
    WeekdayShort,         // -
    WeekdayLong(u8),      // pad
    YearDay(u8),          // pad
    YearDay0(u8),         // pad
    IsoWeek(u8),          // pad
    IsoYear(u8),          // pad
    Hour(u8),             // pad
    Hour12(u8),           // pad
    AmPm(bool),           // upper
    Minute,               // -
    Second,               // -
    Fraction(u8),         // precision
    UnixTimestamp,        // -
    TimezoneNumeric,      // -
    TimezoneName,         // -
}

#[derive(Clone)]
pub struct StrftimeFormat<'a> {
    spec: &'a [u8],
    jump: &'static [u8],
}

impl<'a> StrftimeFormat<'a> {
    pub fn new(spec: &'a str) -> Self {
        Self::from_bytes(spec.as_bytes())
    }

    pub fn from_bytes(spec: &'a [u8]) -> Self {
        Self { spec, jump: b"" }
    }

    fn pop(&mut self) -> Option<u8> {
        if self.jump.len() != 0 {
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

    fn jump(&mut self, jump: &'static [u8]) -> Option<Item> {
        self.jump = jump;
        self.next()
    }
}

impl<'a> Iterator for StrftimeFormat<'a> {
    type Item = Item;
    fn next(&mut self) -> Option<Self::Item> {
        match self.pop() {
            None => None,
            Some(b'%') => {
                let checkpoint = self.clone();
                let result = match self.pop() {
                    Some(b'%') => Some(Item::Char(b'%')),
                    Some(b'C') => Some(Item::Century(b'0')),
                    Some(b'Y') => Some(Item::Year(b'0')),
                    Some(b'y') => Some(Item::YearShort(b'0')),
                    Some(b'm') => Some(Item::MonthNumeric(b'0')),
                    Some(b'b') => Some(Item::MonthShort),
                    Some(b'B') => Some(Item::MonthLong(b' ')),
                    Some(b'd') => Some(Item::Day(b'0')),
                    Some(b'F') => self.jump(b"%Y-%m-%d"),
                    Some(b'u') => Some(Item::WeekdayNumeric(false)),
                    Some(b'w') => Some(Item::WeekdayNumeric(true)),
                    Some(b'a') => Some(Item::WeekdayShort),
                    Some(b'A') => Some(Item::WeekdayLong(b' ')),
                    Some(b'H') => Some(Item::Hour(b'0')),
                    Some(b'k') => Some(Item::Hour(b' ')),
                    Some(b'I') => Some(Item::Hour12(b'0')),
                    Some(b'l') => Some(Item::Hour12(b' ')),
                    Some(b'p') => Some(Item::AmPm(true)),
                    Some(b'P') => Some(Item::AmPm(false)),
                    Some(b'M') => Some(Item::Minute),
                    Some(b'S') => Some(Item::Second),
                    Some(b'R') => self.jump(b"%H:%M"),
                    Some(b'r') => self.jump(b"%I:%M:%S %p"),
                    Some(b'T') => self.jump(b"%H:%M:%S"),
                    Some(b'X') => self.jump(b"%H:%M:%S"),
                    Some(b'+') => self.jump(b"%Y-%m-%dT%H:%M:%S%fZ"),
                    Some(b'f') => Some(Item::Fraction(6)),
                    Some(b'3') => match self.pop() {
                        Some(b'f') => Some(Item::Fraction(3)),
                        _ => None,
                    },
                    Some(b'6') => match self.pop() {
                        Some(b'f') => Some(Item::Fraction(6)),
                        _ => None,
                    },
                    Some(b'9') => match self.pop() {
                        Some(b'f') => Some(Item::Fraction(9)),
                        _ => None,
                    },
                    Some(b's') => Some(Item::UnixTimestamp),
                    Some(b'c') => self.jump(b"%a %b %e %H:%M:%S %Y"),
                    Some(b'D') => self.jump(b"%m/%d/%y"),
                    Some(b'x') => self.jump(b"%m/%d/%y"),
                    Some(b'e') => Some(Item::Day(b' ')),
                    Some(b'h') => Some(Item::MonthShort),
                    Some(b't') => Some(Item::Char(b' ')),
                    Some(b'n') => Some(Item::Char(b' ')),
                    Some(b'G') => Some(Item::IsoYear(b'0')),
                    Some(b'V') => Some(Item::IsoWeek(b'0')),
                    Some(b'z') => Some(Item::TimezoneNumeric),
                    Some(b'Z') => Some(Item::TimezoneName),
                    _ => None,
                };
                match result {
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

pub fn format_date<'a, B, F>(buf: &mut B, dt: DateTime<Utc>, format: F)
where
    B: Push<u8>,
    F: Iterator<Item = Item>,
{
    let dt = dt.naive_utc();
    for item in format {
        match item {
            Item::Char(b) => {
                buf.push(b);
            }
            Item::Century(pad) => {
                format_int(buf, dt.year() / 100, 2, pad);
            }
            Item::Year(pad) => {
                format_int(buf, dt.year() % 10000, 4, pad);
            }
            Item::YearShort(pad) => {
                format_int(buf, dt.year() % 100, 2, pad);
            }
            Item::MonthNumeric(pad) => {
                format_int(buf, dt.month(), 2, pad);
            }
            Item::MonthShort => {
                buf.extend_from_slice(&MONTHS_SHORT[dt.month0() as usize].as_bytes());
            }
            Item::MonthLong(pad) => aligned(buf, MONTHS_LONG_MAX_LEN, pad, |mut buf| {
                buf.extend_from_slice(&MONTHS_LONG[dt.month0() as usize].as_bytes())
            }),
            Item::Day(pad) => {
                format_int(buf, dt.day(), 2, pad);
            }
            Item::WeekdayNumeric(from_sunday) => {
                if from_sunday {
                    format_int(buf, dt.weekday().number_from_monday(), 1, b' ');
                } else {
                    format_int(buf, dt.weekday().num_days_from_sunday(), 1, b' ');
                }
            }
            Item::WeekdayShort => buf.extend_from_slice(
                &WEEKDAYS_SHORT[dt.weekday().num_days_from_monday() as usize].as_bytes(),
            ),
            Item::WeekdayLong(pad) => aligned(buf, WEEKDAYS_LONG_MAX_LEN, pad, |mut buf| {
                buf.extend_from_slice(
                    &WEEKDAYS_LONG[dt.weekday().num_days_from_monday() as usize].as_bytes(),
                );
            }),
            Item::YearDay(pad) => {
                format_int(buf, dt.ordinal(), 3, pad);
            }
            Item::YearDay0(pad) => {
                format_int(buf, dt.ordinal0(), 3, pad);
            }
            Item::IsoWeek(pad) => {
                format_int(buf, dt.iso_week().week(), 2, pad);
            }
            Item::IsoYear(pad) => {
                format_int(buf, dt.iso_week().year(), 4, pad);
            }
            Item::Hour(pad) => {
                format_int(buf, dt.hour(), 2, pad);
            }
            Item::Hour12(pad) => {
                format_int(buf, dt.hour12().1, 2, pad);
            }
            Item::AmPm(upper) => {
                let am_pm = if upper { AM_PM_UPPER } else { AM_PM_LOWER };
                buf.extend_from_slice(am_pm[dt.hour12().0 as usize].as_bytes());
            }
            Item::Minute => {
                format_int(buf, dt.minute(), 2, b'0');
            }
            Item::Second => {
                format_int(buf, dt.second(), 2, b'0');
            }
            Item::Fraction(precision) => {
                let nsec = dt.nanosecond();
                match precision {
                    1 => format_int(buf, nsec / 100000000, 1, b'0'),
                    2 => format_int(buf, nsec / 10000000, 2, b'0'),
                    3 => format_int(buf, nsec / 1000000, 3, b'0'),
                    4 => format_int(buf, nsec / 100000, 4, b'0'),
                    5 => format_int(buf, nsec / 10000, 5, b'0'),
                    6 => format_int(buf, nsec / 1000, 6, b'0'),
                    7 => format_int(buf, nsec / 100, 7, b'0'),
                    8 => format_int(buf, nsec / 10, 8, b'0'),
                    _ => format_int(buf, nsec, 9, b'0'),
                }
            }
            Item::UnixTimestamp => {
                format_int(buf, dt.timestamp(), 10, b'0');
            }
            Item::TimezoneNumeric => {
                buf.extend_from_slice(b"+0000");
            }
            Item::TimezoneName => {
                buf.extend_from_slice(b"UTC");
            }
        }
    }
}

pub fn reformat_rfc3339_timestamp<'a, B, F>(buf: &mut B, ts: &'a str, format: F)
where
    B: Push<u8>,
    F: Iterator<Item = Item>,
{
    let mut dt_cache = None;
    let mut dt = || {
        if let Some(dt) = dt_cache {
            dt
        } else {
            let dt = DateTime::parse_from_rfc3339(ts).ok();
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

    for item in format {
        match item {
            Item::Char(b) => {
                buf.push(b);
            }
            Item::Century(pad) => {
                if pad == b'0' || ts[0] != b'0' {
                    buf.push(ts[0])
                } else {
                    buf.push(pad)
                }
                buf.push(ts[1])
            }
            Item::Year(pad) => {
                if pad == b'0' {
                    buf.extend_from_slice(&ts[0..4])
                } else {
                    let pos = (&ts[0..3])
                        .iter()
                        .map(|&b| b == b'0')
                        .position(|x| x == false);
                    for _ in 0..pos.unwrap_or_default() {
                        buf.push(pad)
                    }
                    for i in pos.unwrap_or_default()..4 {
                        buf.push(ts[i])
                    }
                }
            }
            Item::YearShort(pad) => {
                if pad == b'0' || ts[2] != b'0' {
                    buf.push(ts[2])
                } else {
                    buf.push(pad)
                }
                buf.push(ts[3])
            }
            Item::MonthNumeric(pad) => {
                if pad == b'0' || ts[5] != b'0' {
                    buf.push(ts[5])
                } else {
                    buf.push(pad)
                }
                buf.push(ts[6])
            }
            Item::MonthShort => {
                let month = month();
                buf.extend_from_slice(&MONTHS_SHORT[month].as_bytes());
            }
            Item::MonthLong(pad) => aligned(buf, MONTHS_LONG_MAX_LEN, pad, |mut buf| {
                let month = month();
                buf.extend_from_slice(&MONTHS_LONG[month].as_bytes())
            }),
            Item::Day(pad) => {
                if pad == b'0' || ts[8] != b'0' {
                    buf.push(ts[8])
                } else {
                    buf.push(pad)
                }
                buf.push(ts[9])
            }
            Item::WeekdayNumeric(from_sunday) => {
                if let Some(dt) = dt() {
                    if from_sunday {
                        format_int(buf, dt.weekday().number_from_monday(), 1, b' ');
                    } else {
                        format_int(buf, dt.weekday().num_days_from_sunday(), 1, b' ');
                    }
                } else {
                    buf.push(b'?')
                }
            }
            Item::WeekdayShort => {
                buf.extend_from_slice(if let Some(dt) = dt() {
                    &WEEKDAYS_SHORT[dt.weekday().num_days_from_monday() as usize].as_bytes()
                } else {
                    b"(?)"
                });
            }
            Item::WeekdayLong(pad) => {
                aligned(buf, WEEKDAYS_LONG_MAX_LEN, pad, |mut buf| {
                    buf.extend_from_slice(if let Some(dt) = dt() {
                        &WEEKDAYS_LONG[dt.weekday().num_days_from_monday() as usize].as_bytes()
                    } else {
                        b"(?)"
                    });
                });
            }
            Item::YearDay(pad) => {
                if let Some(dt) = dt() {
                    format_int(buf, dt.ordinal(), 3, pad)
                } else {
                    buf.extend_from_slice(b"(?)")
                }
            }
            Item::YearDay0(pad) => {
                if let Some(dt) = dt() {
                    format_int(buf, dt.ordinal0(), 3, pad)
                } else {
                    buf.extend_from_slice(b"(?)")
                }
            }
            Item::IsoWeek(pad) => {
                if let Some(dt) = dt() {
                    format_int(buf, dt.iso_week().week(), 2, pad)
                } else {
                    buf.extend_from_slice(b"??")
                }
            }
            Item::IsoYear(pad) => {
                if let Some(dt) = dt() {
                    format_int(buf, dt.iso_week().year(), 4, pad)
                } else {
                    buf.extend_from_slice(b"(??)")
                }
            }
            Item::Hour(pad) => {
                if pad == b'0' || ts[11] != b'0' {
                    buf.push(ts[11])
                } else {
                    buf.push(pad)
                }
                buf.push(ts[12])
            }
            Item::Hour12(pad) => {
                let hour = hour() / 2;
                let hour = if hour == 0 { 12 } else { hour };
                format_int(buf, hour, 2, pad);
            }
            Item::AmPm(upper) => {
                let hour = hour();
                let am_pm = if upper { AM_PM_UPPER } else { AM_PM_LOWER };
                buf.extend_from_slice(am_pm[(hour >= 12) as usize].as_bytes());
            }
            Item::Minute => {
                buf.push(ts[14]);
                buf.push(ts[15]);
            }
            Item::Second => {
                buf.push(ts[17]);
                buf.push(ts[18]);
            }
            Item::Fraction(precision) => {
                let nsec = if ts.len() > 20 {
                    if let Some(pos) = (&ts[20..]).iter().position(|&b| !b.is_ascii_digit()) {
                        &ts[20..20 + pos]
                    } else {
                        &ts[19..19]
                    }
                } else {
                    &ts[19..19]
                };
                let precision = precision as usize;
                if precision < nsec.len() {
                    buf.extend_from_slice(&nsec[..precision])
                } else {
                    buf.extend_from_slice(nsec);
                    for _ in nsec.len()..precision {
                        buf.push(b'0')
                    }
                }
            }
            Item::UnixTimestamp => {
                if let Some(dt) = dt() {
                    format_int(buf, dt.timestamp(), 10, b'0');
                }
            }
            Item::TimezoneNumeric => {
                if let Some(pos) = (&ts[20..]).iter().position(|&b| !b.is_ascii_digit()) {
                    let tz = &ts[20 + pos..];
                    if tz == b"Z" || tz == b"z" {
                        buf.extend_from_slice(b"+0000");
                    } else {
                        buf.extend_from_slice(tz);
                    }
                }
            }
            Item::TimezoneName => {
                if let Some(pos) = (&ts[20..]).iter().position(|&b| !b.is_ascii_digit()) {
                    let tz = &ts[20 + pos..];
                    if tz == b"Z"
                        || tz == b"z"
                        || ((tz[0] == b'+' || tz[0] == b'-') && &tz[1..] == b"0000")
                    {
                        buf.extend_from_slice(b"UTC");
                    } else {
                        buf.extend_from_slice(b"(?)");
                    }
                }
                buf.extend_from_slice(b"UTC");
            }
        }
    }
}

fn format_int<B, I>(buf: &mut B, value: I, width: usize, pad: u8)
where
    B: Push<u8>,
    I: itoa::Integer + PartialOrd + Default,
{
    let negative = value < I::default();
    let mut ibuf = itoa::Buffer::new();
    let mut b = ibuf.format(value).as_bytes();
    if b.len() <= width {
        let filler = if negative { b' ' } else { pad };
        for _ in b.len()..width {
            buf.push(filler)
        }
    } else {
        let offset = b.len() - width;
        b = &b[offset..];
        if negative {
            buf.push(b'-');
            b = &b[1..];
        }
    }
    buf.extend_from_slice(b);
}

const MONTHS_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

const MONTHS_LONG: [&str; 12] = [
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
];

const MONTHS_LONG_MAX_LEN: usize = 9;

const WEEKDAYS_SHORT: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

const WEEKDAYS_LONG: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

const WEEKDAYS_LONG_MAX_LEN: usize = 9;

const AM_PM_UPPER: [&str; 2] = ["AM", "PM"];
const AM_PM_LOWER: [&str; 2] = ["am", "pm"];
