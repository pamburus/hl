// third-party imports
use chrono::{DateTime, Datelike, Duration, NaiveDateTime, Offset, Utc};
use humantime::parse_duration;

// workspace imports
use enumset_ext::EnumSetExt;

// local imports
use crate::datefmt::{DateTimeFormat, Flag, Flags, Item};
use crate::error::*;
use crate::timezone::Tz;

pub fn parse_time(s: &str, tz: &Tz, format: &DateTimeFormat) -> Result<DateTime<Tz>> {
    let s = s.trim();
    None.or_else(|| relative_past(s))
        .or_else(|| relative_future(s))
        .or_else(|| use_custom_format(s, format, &Utc::now().with_timezone(tz), tz))
        .or_else(|| rfc3339(s, tz))
        .or_else(|| rfc3339_weak(s, tz))
        .or_else(|| human(s, tz))
        .ok_or(Error::UnrecognizedTime(s.into()))
}

fn relative_past(s: &str) -> Option<DateTime<Tz>> {
    if let Some(stripped) = s.strip_prefix('-') {
        let d = parse_duration(stripped).ok()?;
        let ts = Utc::now() - Duration::from_std(d).ok()?;
        Some(ts.with_timezone(&ts.timezone().into()))
    } else {
        None
    }
}

fn relative_future(s: &str) -> Option<DateTime<Tz>> {
    if let Some(stripped) = s.strip_prefix('+') {
        let d = parse_duration(stripped).ok()?;
        let ts = Utc::now() + Duration::from_std(d).ok()?;
        Some(ts.with_timezone(&ts.timezone().into()))
    } else {
        None
    }
}

fn human(s: &str, tz: &Tz) -> Option<DateTime<Tz>> {
    htp::parse(s, Utc::now().with_timezone(tz)).ok()
}

fn rfc3339(s: &str, tz: &Tz) -> Option<DateTime<Tz>> {
    Some(DateTime::parse_from_rfc3339(s).ok()?.with_timezone(tz))
}

fn rfc3339_weak(s: &str, tz: &Tz) -> Option<DateTime<Tz>> {
    let time = DateTime::<Utc>::from(humantime::parse_rfc3339_weak(s).ok()?).with_timezone(tz);
    let fix1 = time.offset().fix().local_minus_utc();
    let time = time - Duration::try_seconds(fix1 as i64)?;
    let fix2 = time.offset().fix().local_minus_utc();
    let time = time - Duration::try_seconds((fix2 - fix1) as i64)?;
    Some(time)
}

fn use_custom_format(s: &str, format: &DateTimeFormat, now: &DateTime<Tz>, tz: &Tz) -> Option<DateTime<Tz>> {
    let unsupported = || None;
    let mut buf = Vec::new();
    let mut has_year = false;
    let mut has_month = false;
    let mut has_day = false;
    let mut has_ampm = false;
    let mut has_hour = false;
    let mut has_minute = false;
    let mut has_second = false;
    let mut has_offset = false;

    for item in format {
        match *item {
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
                add_format_item(&mut buf, b"m", flags)?;
                has_month = true;
            }
            Item::MonthShort(flags) => {
                add_format_item(&mut buf, b"b", flags)?;
                has_month = true;
            }
            Item::MonthLong(flags) => {
                add_format_item(&mut buf, b"B", flags)?;
                has_month = true;
            }
            Item::Day(flags) => {
                add_format_item(&mut buf, b"d", flags)?;
                has_day = true;
            }
            Item::WeekdayNumeric(flags) => {
                let item = if flags.includes(Flag::FromSunday | Flag::FromZero) {
                    b"w"
                } else if !flags.intersects(Flag::FromSunday | Flag::FromZero) {
                    b"u"
                } else {
                    return None;
                };
                add_format_item(&mut buf, item, flags & !(Flag::FromSunday | Flag::FromZero))?;
            }
            Item::WeekdayShort(flags) => {
                add_format_item(&mut buf, b"a", flags)?;
            }
            Item::WeekdayLong(flags) => {
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
                let item = if flags.contains(Flag::LowerCase) { b"p" } else { b"P" };
                add_format_item(&mut buf, item, Flags::empty())?;
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
                if buf.is_empty() || buf[buf.len() - 1] != b'.' {
                    unsupported();
                }
                buf.pop();
                add_format_item(&mut buf, b".f", Flags::empty())?;
            }
            Item::UnixTimestamp(flags) => {
                add_format_item(&mut buf, b"s", flags)?;
            }
            Item::TimeZoneOffset((flags, precision)) => {
                let format: &[u8] = match precision {
                    0 => b"z",
                    1 => b":z",
                    _ => b"::z",
                };
                add_format_item(&mut buf, format, flags)?;
                has_offset = true;
            }
            Item::TimeZoneName(_) => {}
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
    if !has_hour {
        if has_ampm {
            buf.extend_from_slice(b" %I");
            extra.extend_from_slice(b" 12");
            buf.extend_from_slice(b" %p");
            extra.extend_from_slice(b" %p");
        } else {
            buf.extend_from_slice(b" %H");
            extra.extend_from_slice(b" 00");
        }
    }
    if !has_minute {
        buf.extend_from_slice(b" %M");
        extra.extend_from_slice(b" 00");
    }
    if !has_second {
        buf.extend_from_slice(b" %S");
        extra.extend_from_slice(b" 00");
    }
    if !has_offset {
        buf.extend_from_slice(b" %:z");
        extra.extend_from_slice(b" %:z");
    }
    let now = now.with_timezone(tz);
    let f1 = std::str::from_utf8(&buf).ok()?;
    let f2 = std::str::from_utf8(&extra).ok()?;
    let s = format!("{}{}", s, now.format(f2));
    let result = DateTime::parse_from_str(&s, f1).ok()?.with_timezone(tz);
    let initial_offset = (if has_offset { result } else { now }).offset().fix();
    let result = smart_adjust(&result, &now, has_year, has_month, has_day).unwrap_or(result);
    let shift = initial_offset.local_minus_utc() - result.offset().fix().local_minus_utc();
    Some(result + Duration::try_seconds(shift as i64)?)
}

fn smart_adjust(
    result: &DateTime<Tz>,
    now: &DateTime<Tz>,
    has_year: bool,
    has_month: bool,
    has_day: bool,
) -> Option<DateTime<Tz>> {
    if result <= now {
        return None;
    }

    if !has_day {
        let pred = result.date_naive().pred_opt()?;
        let pred = NaiveDateTime::new(pred, result.time());
        let fixed = pred.and_local_timezone(result.timezone()).latest()?;
        if &fixed <= now {
            return Some(fixed);
        }
    }

    if !has_month {
        let month = result.month();
        let fixed = result
            .with_year(if month > 1 { result.year() } else { result.year() - 1 })?
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
    if flags.intersects(Flag::UpperCase | Flag::LowerCase | Flag::FromZero | Flag::FromSunday) {
        return None;
    }

    buf.push(b'%');
    if flags.contains(Flag::SpacePadding) {
        buf.push(b'_');
    } else if flags.contains(Flag::ZeroPadding) {
        buf.push(b'0');
    } else if flags.contains(Flag::NoPadding) {
        buf.push(b'-');
    }
    buf.extend_from_slice(item);

    Some(())
}

#[cfg(test)]
mod tests;
