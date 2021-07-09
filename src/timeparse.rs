// third-party imports
use chrono::{DateTime, Datelike, Duration, FixedOffset, TimeZone, Utc};
use humantime::parse_duration;

// local imports
use crate::datefmt::{DateTimeFormat, Flag, Flags, Item};
use crate::error::*;

pub fn parse_time(
    s: &str,
    tz: &FixedOffset,
    format: &DateTimeFormat,
) -> Result<DateTime<FixedOffset>> {
    let s = s.trim();
    None.or_else(|| relative_past(s))
        .or_else(|| relative_future(s))
        .or_else(|| use_custom_format(s, format, &Utc::now().with_timezone(tz), tz))
        .or_else(|| rfc3339_weak(s, tz))
        .or_else(|| human(s, tz))
        .ok_or(Error::UnrecognizedTime(s.into()))
}

fn relative_past(s: &str) -> Option<DateTime<FixedOffset>> {
    if s.starts_with('-') {
        let d = parse_duration(&s[1..]).ok()?;
        Some((Utc::now() - Duration::from_std(d).ok()?).into())
    } else {
        None
    }
}

fn relative_future(s: &str) -> Option<DateTime<FixedOffset>> {
    if s.starts_with('+') {
        let d = parse_duration(&s[1..]).ok()?;
        Some((Utc::now() + Duration::from_std(d).ok()?).into())
    } else {
        None
    }
}

fn human(s: &str, tz: &FixedOffset) -> Option<DateTime<FixedOffset>> {
    htp::parse(s, Utc::now().with_timezone(tz)).ok()
}

fn rfc3339_weak(s: &str, tz: &FixedOffset) -> Option<DateTime<FixedOffset>> {
    let offset = Duration::seconds(tz.utc_minus_local().into());
    let time = DateTime::<Utc>::from(humantime::parse_rfc3339_weak(s).ok()?) + offset;
    Some(time.into())
}

fn use_custom_format(
    s: &str,
    format: &DateTimeFormat,
    now: &DateTime<FixedOffset>,
    tz: &FixedOffset,
) -> Option<DateTime<FixedOffset>> {
    let unsupported = || None;
    let mut buf = Vec::new();
    let mut has_year = false;
    let mut has_month = false;
    let mut has_day = false;
    let mut has_ampm = false;
    let mut has_hour = false;
    let mut has_minute = false;
    let mut has_second = false;

    for item in format {
        match *item.as_ref() {
            Item::Char(b) => {
                buf.push(b);
            }
            Item::Century(_) => {
                return unsupported();
            }
            Item::Year(flags) => {
                add_format_item(&mut buf, b"Y", flags)?;
                has_year = true;
            }
            Item::YearShort(flags) => {
                add_format_item(&mut buf, b"y", flags)?;
                has_year = true;
            }
            Item::YearQuarter(_) => {
                return unsupported();
            }
            Item::MonthNumeric(flags) => {
                if flags.intersects(Flag::FromZero) {
                    return unsupported();
                }
                add_format_item(&mut buf, b"m", flags)?;
                has_month = true;
            }
            Item::MonthShort(flags) => {
                if flags != Flags::none() {
                    return unsupported();
                }
                add_format_item(&mut buf, b"b", flags)?;
                has_month = true;
            }
            Item::MonthLong(flags) => {
                if flags != Flags::none() {
                    return unsupported();
                }
                add_format_item(&mut buf, b"B", flags)?;
                has_month = true;
            }
            Item::Day(flags) => {
                add_format_item(&mut buf, b"d", flags)?;
                has_day = true;
            }
            Item::WeekdayNumeric(flags) => {
                let item = if flags.contains(Flag::FromSunday | Flag::FromZero) {
                    b"w"
                } else if !flags.intersects(Flag::FromSunday | Flag::FromZero) {
                    b"u"
                } else {
                    return None;
                };
                add_format_item(&mut buf, item, flags & !(Flag::FromSunday | Flag::FromZero))?;
            }
            Item::WeekdayShort(flags) => {
                if flags != Flags::none() {
                    return unsupported();
                }
                add_format_item(&mut buf, b"a", flags)?;
            }
            Item::WeekdayLong(flags) => {
                if flags != Flags::none() {
                    return unsupported();
                }
                add_format_item(&mut buf, b"A", flags)?;
            }
            Item::YearDay(_) => {
                return unsupported();
            }
            Item::IsoWeek(_) => {
                return unsupported();
            }
            Item::IsoYear(_) => {
                return unsupported();
            }
            Item::IsoYearShort(_) => {
                return unsupported();
            }
            Item::Hour(flags) => {
                add_format_item(&mut buf, b"H", flags)?;
                has_hour = true;
                has_ampm = true;
            }
            Item::Hour12(flags) => {
                add_format_item(&mut buf, b"I", flags)?;
                has_hour = true;
            }
            Item::AmPm(flags) => {
                let item = if flags.contains(Flag::LowerCase) {
                    b"p"
                } else {
                    b"P"
                };
                add_format_item(&mut buf, item, Flags::none())?;
                has_ampm = true;
            }
            Item::Minute(flags) => {
                add_format_item(&mut buf, b"M", flags)?;
                has_minute = true;
            }
            Item::Second(flags) => {
                add_format_item(&mut buf, b"S", flags)?;
                has_second = true;
            }
            Item::Nanosecond((_, _)) => {
                if buf.len() == 0 || buf[buf.len() - 1] != b'.' {
                    unsupported();
                }
                buf.pop();
                add_format_item(&mut buf, b".f", Flags::none())?;
            }
            Item::UnixTimestamp(flags) => {
                add_format_item(&mut buf, b"s", flags)?;
            }
            Item::TimeZoneHour(_) => {
                return unsupported();
            }
            Item::TimeZoneMinute(_) => {
                return unsupported();
            }
            Item::TimeZoneSecond(_) => {
                return unsupported();
            }
            Item::TimeZoneName(_) => {
                return unsupported();
            }
        }
    }
    let mut extra = Vec::new();
    if !has_year {
        buf.extend_from_slice(b" %Y");
        extra.extend_from_slice(b" %Y");
    }
    if !has_month {
        buf.extend_from_slice(b" %m");
        extra.extend_from_slice(b" %m");
    }
    if !has_day {
        buf.extend_from_slice(b" %d");
        extra.extend_from_slice(b" %d");
    }
    if !has_ampm {
        buf.extend_from_slice(b" %p");
        extra.extend_from_slice(b" %p");
    }
    if !has_hour {
        buf.extend_from_slice(b" %H");
        extra.extend_from_slice(b" 00");
    }
    if !has_minute {
        buf.extend_from_slice(b" %M");
        extra.extend_from_slice(b" 00");
    }
    if !has_second {
        buf.extend_from_slice(b" %S");
        extra.extend_from_slice(b" 00");
    }
    let f1 = std::str::from_utf8(&buf).ok()?;
    let f2 = std::str::from_utf8(&extra).ok()?;
    let s = format!("{}{}", s, now.format(f2));
    let result = tz.datetime_from_str(&s, f1).ok()?;
    smart_adjust(result, now, has_year, has_month, has_day).or(Some(result))
}

fn smart_adjust(
    result: DateTime<FixedOffset>,
    now: &DateTime<FixedOffset>,
    has_year: bool,
    has_month: bool,
    has_day: bool,
) -> Option<DateTime<FixedOffset>> {
    if &result <= now {
        return None;
    }

    if !has_day {
        let pred = result.date().pred();
        let fixed = result
            .timezone()
            .ymd(pred.year(), pred.month(), pred.day())
            .and_time(result.time())?;
        if &fixed <= now {
            return Some(fixed);
        }
    }

    if !has_month {
        let month = result.month();
        let fixed = result
            .with_year(if month > 1 {
                result.year()
            } else {
                result.year() - 1
            })?
            .with_month(if month > 1 { month - 1 } else { 12 })?;
        if &fixed <= now {
            return Some(fixed);
        }
    }

    if !has_year {
        let fixed = result.with_year(result.year() - 1)?;
        if &fixed <= now {
            return Some(fixed);
        }
    }

    None
}

fn add_format_item(buf: &mut Vec<u8>, item: &[u8], flags: Flags) -> Option<()> {
    buf.push(b'%');
    if flags.intersects(Flag::SpacePadding) {
        buf.push(b'_');
    }
    if flags.intersects(Flag::ZeroPadding) {
        buf.push(b'0');
    }
    if flags.intersects(Flag::NoPadding) {
        buf.push(b'-');
    }
    buf.extend_from_slice(item);

    if flags.intersects(Flag::UpperCase | Flag::LowerCase | Flag::FromZero | Flag::FromSunday) {
        None
    } else {
        Some(())
    }
}
