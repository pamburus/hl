use chrono::naive::NaiveDateTime;
use chrono::{DateTime, Utc};

pub struct Timestamp<'a>(&'a str);

impl<'a> Timestamp<'a> {
    pub fn new(value: &'a str) -> Self {
        Self(value)
    }

    pub fn raw(&self) -> &'a str {
        self.0
    }

    pub fn parse(&self) -> Option<DateTime<Utc>> {
        match self.0.parse::<i64>() {
            Ok(ts) => {
                let (ts, nsec) = if ts < 100000000000 {
                    (ts, 0)
                } else if ts < 100000000000000 {
                    (ts / 1000, (ts % 1000) * 1000000)
                } else {
                    (ts / 1000000, (ts % 1000000) * 1000)
                };
                let ts = NaiveDateTime::from_timestamp_opt(ts, nsec as u32)?;
                Some(DateTime::from_utc(ts, Utc))
            }
            _ => DateTime::parse_from_rfc3339(self.0).ok().map(|x| x.into()),
        }
    }

    pub fn is_rfc3339(&self) -> bool {
        let v = self.0.as_bytes();
        if v.len() < 19 {
            return false;
        }
        if v[4] != b'-' || v[7] != b'-' || v[10] != b'T' || v[13] != b':' || v[16] != b':' {
            return false;
        }
        if !only_digits(&v[0..4]) {
            return false;
        }
        if !only_digits(&v[5..7]) {
            return false;
        }
        if !only_digits(&v[8..10]) {
            return false;
        }
        if !only_digits(&v[11..13]) {
            return false;
        }
        if !only_digits(&v[14..16]) {
            return false;
        }
        if !only_digits(&v[17..19]) {
            return false;
        }
        true
    }

    pub fn split_rfc3339(&self) -> Option<(&'a [u8], &'a [u8])> {
        if !self.is_rfc3339() {
            return None;
        }
        let mut v = self.0.as_bytes();
        if v.len() > 23 {
            v = &v[..23]
        }
        for i in 19..23 {
            if v[i] != b'.' && !v[i].is_ascii_digit() {
                v = &v[..i];
                break;
            }
        }
        Some((&v[2..10], &v[11..]))
    }
}

fn only_digits(b: &[u8]) -> bool {
    b.iter()
        .map(|&b| b.is_ascii_digit())
        .position(|x| x == false)
        .is_none()
}
