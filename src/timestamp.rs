// stdlib imports
use std::cell::OnceCell;

// third-party imports
use chrono::{DateTime, Duration, FixedOffset, NaiveDateTime};

// local imports
use crate::app::UnixTimestampUnit;

// ---

#[derive(Debug)]
pub struct Timestamp<'a> {
    raw: &'a str,
    parsed: OnceCell<Option<DateTime<FixedOffset>>>,
    unix_unit: Option<UnixTimestampUnit>,
}

impl<'a> Timestamp<'a> {
    pub fn new(value: &'a str) -> Self {
        Self {
            raw: value,
            parsed: OnceCell::new(),
            unix_unit: None,
        }
    }

    pub fn raw(&self) -> &'a str {
        self.raw
    }

    pub fn with_unix_unit(self, unit: Option<UnixTimestampUnit>) -> Self {
        Self {
            raw: self.raw,
            parsed: if unit == self.unix_unit {
                self.parsed
            } else {
                OnceCell::new()
            },
            unix_unit: unit,
        }
    }

    pub fn parsed(&self) -> &Option<DateTime<FixedOffset>> {
        self.parsed.get_or_init(|| self.reparse())
    }

    pub fn parse(&self) -> Option<DateTime<FixedOffset>> {
        *self.parsed()
    }

    fn reparse(&self) -> Option<DateTime<FixedOffset>> {
        if let Ok(ts) = self.raw.parse() {
            Some(ts)
        } else if let Some(nt) = guess_number_type(self.raw.as_bytes()) {
            let ts = match (nt, self.unix_unit) {
                (NumberType::Integer, unit) => self.raw.parse::<i64>().ok().and_then(|ts| {
                    let unit = unit.unwrap_or_else(|| UnixTimestampUnit::guess(ts));
                    match unit {
                        UnixTimestampUnit::Seconds => DateTime::from_timestamp(ts, 0),
                        UnixTimestampUnit::Milliseconds => DateTime::from_timestamp_millis(ts),
                        UnixTimestampUnit::Microseconds => DateTime::from_timestamp_micros(ts),
                        _ => Some(DateTime::from_timestamp_nanos(ts)),
                    }
                }),
                (NumberType::Float, unit) => self.raw.bytes().position(|b| b == b'.').and_then(|i| {
                    let whole = self.raw[..i].parse::<i64>().ok()?;
                    let fractional = self.raw[i..].parse::<f64>().ok()?;
                    let unit = unit.unwrap_or_else(|| UnixTimestampUnit::guess(whole));
                    match unit {
                        UnixTimestampUnit::Seconds => {
                            let ns = (fractional * 1e9).round() as u32;
                            let (whole, ns) = if whole < 0 && ns > 0 {
                                (whole - 1, 1_000_000_000 - ns)
                            } else {
                                (whole, ns)
                            };
                            DateTime::from_timestamp(whole, ns)
                        }
                        UnixTimestampUnit::Milliseconds => {
                            let ns = (fractional * 1e6).round() as i64;
                            let ns = if whole < 0 { -ns } else { ns };
                            DateTime::from_timestamp_millis(whole).map(|ts| ts + Duration::nanoseconds(ns))
                        }
                        UnixTimestampUnit::Microseconds => {
                            let ns = (fractional * 1e3).round() as i64;
                            let ns = if whole < 0 { -ns } else { ns };
                            DateTime::from_timestamp_micros(whole).map(|ts| ts + Duration::nanoseconds(ns))
                        }
                        _ => Some(DateTime::from_timestamp_nanos(whole)),
                    }
                }),
            };
            ts.map(|ts| ts.into())
        } else {
            NaiveDateTime::parse_from_str(self.raw, "%Y-%m-%d %H:%M:%S%.f")
                .ok()
                .map(|ts| ts.and_utc().into())
        }
    }

    pub fn as_rfc3339(&self) -> Option<rfc3339::Timestamp> {
        rfc3339::Timestamp::parse(self.raw)
    }

    pub fn unix_utc(&self) -> Option<(i64, u32)> {
        self.parsed()
            .and_then(|ts| Some((ts.timestamp(), ts.timestamp_subsec_nanos())))
    }
}

// ---

pub mod rfc3339 {
    use super::only_digits;

    #[derive(Clone, Eq, PartialEq, Debug)]
    pub struct Timestamp<'a> {
        v: &'a [u8],
        d: usize,
    }

    impl<'a> Timestamp<'a> {
        #[inline]
        pub fn as_bytes(&self) -> &'a [u8] {
            self.v
        }

        #[inline]
        pub fn as_str(&self) -> &'a str {
            return unsafe { std::str::from_utf8_unchecked(self.v) };
        }

        #[inline]
        pub fn date(&self) -> Date {
            Date { v: &self.v[0..10] }
        }

        #[inline]
        pub fn time(&self) -> Time {
            Time { v: &self.v[11..19] }
        }

        #[inline]
        pub fn fraction(&self) -> Fraction {
            Fraction { v: &self.v[19..self.d] }
        }

        #[inline]
        pub fn timezone(&self) -> Timezone {
            Timezone { v: &self.v[self.d..] }
        }

        #[inline]
        pub fn parse(v: &'a str) -> Option<Self> {
            let v = v.as_bytes();
            if v.len() < 20 || (v[10] != b'T' && v[10] != b't' && v[10] != b' ') {
                return None;
            }
            // parse date & time
            let _ = Date::parse(&v[0..10])?;
            let _ = Time::parse(&v[11..19])?;
            // parse fraction
            let d = if v[19] == b'.' {
                if v.len() < 22 || !v[20].is_ascii_digit() {
                    None
                } else if let Some(pos) = v[20..].iter().position(|x| !x.is_ascii_digit()) {
                    Some(20 + pos)
                } else {
                    Some(v.len())
                }
            } else {
                Some(19)
            }?;
            // parse timezone
            let _ = Timezone::parse(&v[d..])?;
            // return result
            Some(Self { v, d })
        }
    }

    // ---

    #[derive(Clone, Eq, PartialEq, Debug)]
    pub struct Date<'a> {
        v: &'a [u8],
    }

    impl<'a> Date<'a> {
        #[inline]
        pub fn as_bytes(&self) -> &'a [u8] {
            self.v
        }

        #[inline]
        pub fn as_str(&self) -> &'a str {
            return unsafe { std::str::from_utf8_unchecked(self.v) };
        }

        #[inline]
        pub fn parse<T: AsRef<[u8]> + ?Sized>(v: &'a T) -> Option<Self> {
            let v = v.as_ref();
            if v.len() != 10 || v[4] != b'-' || v[7] != b'-' {
                return None;
            }

            let _ = Number::parse(&v[0..4])?;
            let _ = Number::parse(&v[5..7])?;
            let _ = Number::parse(&v[8..10])?;

            Some(Self { v })
        }

        #[inline]
        pub fn year(&self) -> Number<'a> {
            Number { v: &self.v[0..4] }
        }

        #[inline]
        pub fn month(&self) -> Number<'a> {
            Number { v: &self.v[5..7] }
        }

        #[inline]
        pub fn day(&self) -> Number<'a> {
            Number { v: &self.v[8..10] }
        }
    }

    // ---

    #[derive(Clone, Eq, PartialEq, Debug)]
    pub struct Time<'a> {
        v: &'a [u8],
    }

    impl<'a> Time<'a> {
        #[inline]
        pub fn as_bytes(&self) -> &'a [u8] {
            self.v
        }

        #[inline]
        pub fn as_str(&self) -> &'a str {
            return unsafe { std::str::from_utf8_unchecked(self.v) };
        }

        #[inline]
        pub fn parse<T: AsRef<[u8]> + ?Sized>(v: &'a T) -> Option<Self> {
            let v = v.as_ref();
            if v.len() != 8 || v[2] != b':' || v[5] != b':' {
                return None;
            }

            let _ = Number::parse(&v[0..2])?;
            let _ = Number::parse(&v[3..5])?;
            let _ = Number::parse(&v[6..8])?;

            Some(Self { v })
        }

        #[inline]
        pub fn hour(&self) -> Number<'a> {
            Number { v: &self.v[0..2] }
        }

        #[inline]
        pub fn minute(&self) -> Number<'a> {
            Number { v: &self.v[3..5] }
        }

        #[inline]
        pub fn second(&self) -> Number<'a> {
            Number { v: &self.v[6..8] }
        }
    }

    // ---

    #[derive(Clone, Eq, PartialEq, Debug)]
    pub struct Timezone<'a> {
        v: &'a [u8],
    }

    impl<'a> Timezone<'a> {
        #[inline]
        pub fn as_bytes(&self) -> &'a [u8] {
            self.v
        }

        #[inline]
        pub fn as_str(&self) -> &'a str {
            return unsafe { std::str::from_utf8_unchecked(self.v) };
        }

        #[inline]
        pub fn parse<T: AsRef<[u8]> + ?Sized>(v: &'a T) -> Option<Self> {
            let v = v.as_ref();
            let v = match v.len() {
                1 => match v[0] {
                    b'Z' | b'z' => Some(v),
                    _ => None,
                },
                6 => match v[0] {
                    b'+' | b'-' => {
                        if v[3] == b':' {
                            let _ = Number::parse(&v[1..3])?;
                            let _ = Number::parse(&v[4..5])?;
                            Some(v)
                        } else {
                            None
                        }
                    }
                    _ => None,
                },
                _ => None,
            }?;
            Some(Self { v })
        }

        #[inline]
        pub fn is_utc(&self) -> bool {
            if self.v == b"z" || self.v == b"Z" {
                true
            } else {
                if let (Some(hour), Some(minute)) = (self.hour(), self.minute()) {
                    hour.as_str() == "00" && minute.as_str() == "00"
                } else {
                    false
                }
            }
        }

        #[inline]
        pub fn sign(&self) -> Option<u8> {
            if self.v.len() > 1 { Some(self.v[0]) } else { None }
        }

        #[inline]
        pub fn hour(&self) -> Option<Number<'a>> {
            if self.v.len() > 1 {
                Some(Number { v: &self.v[1..3] })
            } else {
                None
            }
        }

        #[inline]
        pub fn minute(&self) -> Option<Number<'a>> {
            if self.v.len() > 1 {
                Some(Number { v: &self.v[4..6] })
            } else {
                None
            }
        }
    }

    // ---

    #[derive(Clone, Eq, PartialEq, Debug)]
    pub struct Fraction<'a> {
        v: &'a [u8],
    }

    impl<'a> Fraction<'a> {
        #[inline]
        pub fn as_bytes(&self) -> &'a [u8] {
            self.v
        }

        #[inline]
        pub fn as_str(&self) -> &'a str {
            return unsafe { std::str::from_utf8_unchecked(self.v) };
        }

        #[inline]
        pub fn parse<T: AsRef<[u8]> + ?Sized>(v: &'a T) -> Option<Self> {
            let v = v.as_ref();
            let v = if v.len() == 0 {
                Some(v)
            } else {
                if v[0] == b'.' {
                    if v.len() >= 2 && only_digits(&v[1..]) {
                        Some(v)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }?;
            Some(Self { v })
        }
    }

    // ---

    #[derive(Clone, Eq, PartialEq, Debug)]
    pub struct Number<'a> {
        v: &'a [u8],
    }

    impl<'a> Number<'a> {
        #[inline]
        pub fn as_bytes(&self) -> &'a [u8] {
            self.v
        }

        #[inline]
        pub fn as_str(&self) -> &'a str {
            return unsafe { std::str::from_utf8_unchecked(self.v) };
        }

        #[inline]
        pub fn parse<T: AsRef<[u8]> + ?Sized>(v: &'a T) -> Option<Self> {
            let v = v.as_ref();
            if v.len() == 0 {
                None
            } else if only_digits(v) {
                Some(Self { v })
            } else {
                None
            }
        }

        #[inline]
        pub fn value(&self) -> u32 {
            match self.v.len() {
                2 => (self.v[1] - b'0') as u32 + (self.v[0] - b'0') as u32 * 10,
                4 => {
                    (self.v[3] - b'0') as u32
                        + (self.v[2] - b'0') as u32 * 10
                        + (self.v[1] - b'0') as u32 * 100
                        + (self.v[0] - b'0') as u32 * 1000
                }
                _ => {
                    let mut m = 1;
                    let mut r = 0;
                    for i in (0..self.v.len()).rev() {
                        r += (self.v[i] - b'0') as u32 * m;
                        m *= 10;
                    }
                    r
                }
            }
        }
    }
}

// ---

fn only_digits(b: &[u8]) -> bool {
    b.iter().map(|&b| b.is_ascii_digit()).position(|x| x == false).is_none()
}

fn guess_number_type(b: &[u8]) -> Option<NumberType> {
    if b.len() == 0 {
        return None;
    }

    let b = if b[0] == b'-' || b[0] == b'+' { &b[1..] } else { b };
    let mut dots = 0;
    let mut check = |b| match b {
        b'.' => {
            dots += 1;
            dots <= 1
        }
        b'0'..=b'9' => true,
        _ => return false,
    };

    match (b.iter().map(|b| check(*b)).position(|x| x == false).is_none(), dots) {
        (true, 0) => Some(NumberType::Integer),
        (true, 1) => Some(NumberType::Float),
        _ => None,
    }
}

enum NumberType {
    Integer,
    Float,
}

// ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let test = |s, unit, unix_timestamp, nanos, tz| {
            let ts = Timestamp::new(s).with_unix_unit(unit).parse().unwrap();
            assert_eq!(ts.timestamp(), unix_timestamp);
            assert_eq!(ts.timezone().local_minus_utc(), tz);
            assert_eq!(ts.timestamp_subsec_nanos(), nanos);
        };
        test("2020-08-21 07:20:48", None, 1597994448, 0, 0);
        test("1597994448", None, 1597994448, 0, 0);
        test("1597994448123", None, 1597994448, 123000000, 0);
        test("1597994448123456", None, 1597994448, 123456000, 0);
        test("1597994448123456789", None, 1597994448, 123456789, 0);
        test("1597994448.123", None, 1597994448, 123000000, 0);
        test("1597994448.123456", None, 1597994448, 123456000, 0);
        test("1597994448.123456789", None, 1597994448, 123456789, 0);
        test("-1.123456789", None, -2, 1000000000 - 123456789, 0);
        test(
            "1597994448123.456789",
            Some(UnixTimestampUnit::Milliseconds),
            1597994448,
            123456789,
            0,
        );
        test(
            "1597994448123456.789",
            Some(UnixTimestampUnit::Microseconds),
            1597994448,
            123456789,
            0,
        );
        test(
            "1597994448123456789.0",
            Some(UnixTimestampUnit::Nanoseconds),
            1597994448,
            123456789,
            0,
        );
    }

    #[test]
    fn test_split_rfc3339() {
        use rfc3339::Timestamp;
        let test = |ts: Timestamp, date, time, fraction, timezone| {
            assert_eq!(ts.date().as_str(), date);
            assert_eq!(ts.time().as_str(), time);
            assert_eq!(ts.fraction().as_str(), fraction);
            assert_eq!(ts.timezone().as_str(), timezone);
        };
        test(
            Timestamp::parse("2020-08-21T07:20:48Z").unwrap(),
            "2020-08-21",
            "07:20:48",
            "",
            "Z",
        );
        test(
            Timestamp::parse("2020-08-21t07:20:48Z").unwrap(),
            "2020-08-21",
            "07:20:48",
            "",
            "Z",
        );
        test(
            Timestamp::parse("2020-08-21 07:20:48z").unwrap(),
            "2020-08-21",
            "07:20:48",
            "",
            "z",
        );
        test(
            Timestamp::parse("2020-08-21T07:20:48.092+03:00").unwrap(),
            "2020-08-21",
            "07:20:48",
            ".092",
            "+03:00",
        );
        test(
            Timestamp::parse("2020-08-21T07:20:48.092-03:00").unwrap(),
            "2020-08-21",
            "07:20:48",
            ".092",
            "-03:00",
        );
        test(
            Timestamp::parse("2020-08-21T07:20:48+03:00").unwrap(),
            "2020-08-21",
            "07:20:48",
            "",
            "+03:00",
        );
    }

    #[test]
    fn time_zone() {
        let test = |s, tz| {
            let ts = Timestamp::new(s).parse().unwrap();
            assert_eq!(ts.timezone().local_minus_utc(), tz);
        };
        let test_rfc3339 = |s, sign, hour, minute| {
            let ts = Timestamp::new(s);
            let ts = ts.as_rfc3339().unwrap();
            assert_eq!(ts.timezone().sign(), sign);
            assert_eq!(ts.timezone().hour().map(|x| x.value()), hour);
            assert_eq!(ts.timezone().minute().map(|x| x.value()), minute);
        };

        test("2020-08-21T07:20:48Z", 0);
        test("2020-08-21T07:20:48+03:00", 3 * 3600);
        test("2020-08-21T07:20:48-03:00", -3 * 3600);
        test("2020-08-21T07:20:48+00:00", 0);
        test_rfc3339("2020-08-21T07:20:48Z", None, None, None);
        test_rfc3339("2020-08-21T07:20:48+03:00", Some(b'+'), Some(3), Some(0));
        test_rfc3339("2020-08-21T07:20:48-03:00", Some(b'-'), Some(3), Some(0));
    }
}
