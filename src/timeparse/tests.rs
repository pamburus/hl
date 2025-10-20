// std imports
use chrono::{FixedOffset, TimeZone};

// local imports
use super::*;
use crate::datefmt::LinuxDateFormat;

fn ts(s: &str, tz: &Tz) -> DateTime<Tz> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(tz)
}

fn format(s: &str) -> DateTimeFormat {
    LinuxDateFormat::new(s).compile()
}

#[test]
fn test_use_custom_format_utc_t() {
    let tz = Tz::FixedOffset(Utc.fix());
    let ts = |s| ts(s, &tz);
    let format = format("%T");

    assert_eq!(
        use_custom_format("12:00:00", &format, &ts("2000-01-02T12:00:00Z"), &tz),
        Some(ts("2000-01-02T12:00:00Z"))
    );

    assert_eq!(
        use_custom_format("11:00:00", &format, &ts("2000-01-02T12:00:00Z"), &tz),
        Some(ts("2000-01-02T11:00:00Z"))
    );

    assert_eq!(
        use_custom_format("13:00:00", &format, &ts("2000-01-02T12:00:00Z"), &tz),
        Some(ts("2000-01-01T13:00:00Z"))
    );

    assert_eq!(
        use_custom_format("00:00:00", &format, &ts("2000-01-01T00:00:00Z"), &tz),
        Some(ts("2000-01-01T00:00:00Z"))
    );

    assert_eq!(
        use_custom_format("23:59:59", &format, &ts("2000-01-01T00:00:00Z"), &tz),
        Some(ts("1999-12-31T23:59:59Z"))
    );

    assert_eq!(
        use_custom_format("00:00:01", &format, &ts("2000-01-01T00:00:00Z"), &tz),
        Some(ts("1999-12-31T00:00:01Z"))
    );
}

#[test]
fn test_use_custom_format_dst_t() {
    let tz = Tz::IANA(chrono_tz::Europe::Belgrade);
    let ts = |s| ts(s, &tz);
    let format = format("%T");

    assert_eq!(
        tz.offset_from_utc_datetime(&ts("2022-10-30T00:00:00Z").naive_utc())
            .fix(),
        FixedOffset::east_opt(7200).unwrap()
    );

    assert_eq!(
        tz.offset_from_utc_datetime(&ts("2022-10-30T01:00:00Z").naive_utc())
            .fix(),
        FixedOffset::east_opt(3600).unwrap()
    );

    assert_eq!(
        use_custom_format("00:00:00", &format, &ts("2022-10-30T04:00:00+01:00"), &tz),
        Some(ts("2022-10-30T00:00:00+02:00"))
    );

    assert_eq!(
        use_custom_format("01:00:00", &format, &ts("2022-10-30T04:00:00+01:00"), &tz),
        Some(ts("2022-10-30T01:00:00+02:00"))
    );

    assert_eq!(
        use_custom_format("02:00:00", &format, &ts("2022-10-30T04:00:00+01:00"), &tz),
        Some(ts("2022-10-30T02:00:00+01:00"))
    );

    assert_eq!(
        use_custom_format("03:00:00", &format, &ts("2022-10-30T04:00:00+01:00"), &tz),
        Some(ts("2022-10-30T03:00:00+01:00"))
    );
}

#[test]
fn test_use_custom_format_dst_offset() {
    let tz = Tz::IANA(chrono_tz::Europe::Belgrade);
    let ts = |s| ts(s, &tz);
    let format = format("%y-%m-%d %T.%3N %:z");

    assert_eq!(
        use_custom_format(
            "22-10-29 01:32:16.810 +01:00",
            &format,
            &ts("2022-10-30T14:00:00+01:00"),
            &tz
        ),
        Some(ts("2022-10-29T01:32:16.810+01:00"))
    );
}

#[test]
fn test_use_custom_format_dst_zone() {
    let tz = Tz::IANA(chrono_tz::Europe::Belgrade);
    let ts = |s| ts(s, &tz);
    let format = format("%y-%m-%d %T.%3N %Z");

    assert_eq!(
        use_custom_format("22-10-29 01:32:16.810", &format, &ts("2022-10-30T14:00:00+01:00"), &tz),
        Some(ts("2022-10-29T01:32:16.810+02:00"))
    );
}

#[test]
fn test_use_custom_format_unsupported() {
    let tz = Tz::FixedOffset(Utc.fix());
    let ts = |s| ts(s, &tz);
    let now = &ts("2022-10-30T14:00:00+01:00");

    assert_eq!(use_custom_format("1", &format("%q"), now, &tz), None);
    assert_eq!(use_custom_format("1", &format("%C"), now, &tz), None);
    assert_eq!(use_custom_format("1", &format("%w"), now, &tz), None);
    assert_eq!(use_custom_format("1", &format("%j"), now, &tz), None);
    assert_eq!(use_custom_format("1", &format("%V"), now, &tz), None);
    assert_eq!(use_custom_format("1", &format("%G"), now, &tz), None);
    assert_eq!(use_custom_format("1", &format("%g"), now, &tz), None);
    assert_eq!(use_custom_format("1", &format("%N"), now, &tz), None);
    assert_eq!(
        use_custom_format(
            "1",
            &vec![Item::WeekdayNumeric(Flags::only(Flag::FromSunday))],
            now,
            &tz
        ),
        None
    );
}

#[test]
fn test_use_custom_format_weekday() {
    let tz = Tz::FixedOffset(Utc.fix());
    let ts = |s| ts(s, &tz);
    let now = &ts("2022-10-31T14:00:00+00:00");

    assert_eq!(
        use_custom_format("1", &format("%w"), now, &tz),
        Some(ts("2022-10-31T00:00:00+00:00"))
    );
    assert_eq!(
        use_custom_format(" 1", &format("%_2w"), now, &tz),
        Some(ts("2022-10-31T00:00:00+00:00"))
    );
    assert_eq!(
        use_custom_format("01", &format("%02w"), now, &tz),
        Some(ts("2022-10-31T00:00:00+00:00"))
    );
    assert_eq!(
        use_custom_format("1 ", &format("%-2w"), now, &tz),
        Some(ts("2022-10-31T00:00:00+00:00"))
    );
    assert_eq!(use_custom_format("2", &format("%w"), now, &tz), None);

    assert_eq!(
        use_custom_format("1", &format("%u"), now, &tz),
        Some(ts("2022-10-31T00:00:00+00:00"))
    );

    assert_eq!(
        use_custom_format("1 pm", &format("%w %p"), now, &tz),
        Some(ts("2022-10-31T12:00:00+00:00"))
    );
}

#[test]
fn test_rfc3339_weak() {
    let tz = Tz::IANA(chrono_tz::Europe::Belgrade);
    let ts = |s| ts(s, &tz);

    assert_eq!(
        rfc3339_weak("2022-10-30 01:00:00", &tz),
        Some(ts("2022-10-30T01:00:00+02:00"))
    );

    assert_eq!(
        rfc3339_weak("2022-10-30 02:00:00", &tz),
        Some(ts("2022-10-30T02:00:00+01:00"))
    );

    assert_eq!(
        rfc3339_weak("2022-10-30 03:00:00", &tz),
        Some(ts("2022-10-30T03:00:00+01:00"))
    );
}

#[test]
fn test_relative_past_parsing() {
    let tz = Tz::FixedOffset(Utc.fix());
    let format = DateTimeFormat::new();

    // Test valid relative past times
    let result = parse_time("-1h", &tz, &format);
    assert!(result.is_ok(), "Should parse '-1h' successfully");

    let result = parse_time("-30m", &tz, &format);
    assert!(result.is_ok(), "Should parse '-30m' successfully");

    let result = parse_time("-1d", &tz, &format);
    assert!(result.is_ok(), "Should parse '-1d' successfully");

    let result = parse_time("-5s", &tz, &format);
    assert!(result.is_ok(), "Should parse '-5s' successfully");

    // Test invalid relative past times
    let result = parse_time("-invalid", &tz, &format);
    assert!(result.is_err(), "Should fail to parse '-invalid'");

    // Test non-relative time (should not use relative_past)
    let result = parse_time("1h", &tz, &format);
    assert!(result.is_err(), "Should not parse '1h' as relative past");
}

#[test]
fn test_relative_future_parsing() {
    let tz = Tz::FixedOffset(Utc.fix());
    let format = DateTimeFormat::new();

    // Test valid relative future times
    let result = parse_time("+1h", &tz, &format);
    assert!(result.is_ok(), "Should parse '+1h' successfully");

    let result = parse_time("+30m", &tz, &format);
    assert!(result.is_ok(), "Should parse '+30m' successfully");

    let result = parse_time("+1d", &tz, &format);
    assert!(result.is_ok(), "Should parse '+1d' successfully");

    let result = parse_time("+5s", &tz, &format);
    assert!(result.is_ok(), "Should parse '+5s' successfully");

    // Test invalid relative future times
    let result = parse_time("+invalid", &tz, &format);
    assert!(result.is_err(), "Should fail to parse '+invalid'");

    // Test non-relative time (should not use relative_future)
    let result = parse_time("1h", &tz, &format);
    assert!(result.is_err(), "Should not parse '1h' as relative future");
}

#[test]
fn test_relative_time_boundary_cases() {
    let tz = Tz::FixedOffset(Utc.fix());
    let format = DateTimeFormat::new();

    // Test edge cases that should not match relative parsing
    let result = parse_time("-", &tz, &format);
    assert!(result.is_err(), "Should fail to parse lone '-'");

    let result = parse_time("+", &tz, &format);
    assert!(result.is_err(), "Should fail to parse lone '+'");

    // Test combinations that should trigger relative parsing logic but fail
    let result = parse_time("-0invalidunit", &tz, &format);
    assert!(result.is_err(), "Should fail to parse '-0invalidunit'");

    let result = parse_time("+0invalidunit", &tz, &format);
    assert!(result.is_err(), "Should fail to parse '+0invalidunit'");
}
