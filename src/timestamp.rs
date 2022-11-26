// third-party imports
use chrono::naive::NaiveDateTime;
use chrono::{DateTime, FixedOffset};

// ---

pub struct Timestamp<'a>(&'a str, Option<Option<DateTime<FixedOffset>>>);

impl<'a> Timestamp<'a> {
    pub fn new(value: &'a str, parsed: Option<Option<DateTime<FixedOffset>>>) -> Self {
        Self(value, parsed)
    }

    pub fn raw(&self) -> &'a str {
        self.0
    }

    pub fn parse(&self) -> Option<DateTime<FixedOffset>> {
        if let Some(parsed) = self.1 {
            return parsed;
        }

        if let Ok(ts) = self.0.parse::<i64>() {
            let (ts, nsec) = if ts < 100000000000 {
                (ts, 0)
            } else if ts < 100000000000000 {
                (ts / 1000, (ts % 1000) * 1000000)
            } else {
                (ts / 1000000, (ts % 1000000) * 1000)
            };
            let ts = NaiveDateTime::from_timestamp_opt(ts, nsec as u32)?;
            Some(DateTime::from_utc(ts, FixedOffset::east_opt(0)?))
        } else {
            DateTime::parse_from_rfc3339(self.0).ok()
        }
    }

    pub fn as_rfc3339(&self) -> Option<rfc3339::Timestamp> {
        rfc3339::Timestamp::parse(self.0)
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
            Fraction {
                v: &self.v[19..self.d],
            }
        }

        #[inline]
        pub fn timezone(&self) -> Timezone {
            Timezone {
                v: &self.v[self.d..],
            }
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
            if self.v.len() > 1 {
                Some(self.v[0])
            } else {
                None
            }
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
                Some(Number { v: &self.v[4..5] })
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
    b.iter()
        .map(|&b| b.is_ascii_digit())
        .position(|x| x == false)
        .is_none()
}

// ---

#[cfg(test)]
mod tests {
    use super::*;

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
}
