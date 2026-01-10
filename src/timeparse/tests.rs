// std imports
use chrono::{Datelike, FixedOffset, TimeZone, Timelike};

// third-party imports
use rstest::rstest;

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

#[rstest]
#[case("-1h", true, "Should parse '-1h' successfully")]
#[case("-30m", true, "Should parse '-30m' successfully")]
#[case("-1d", true, "Should parse '-1d' successfully")]
#[case("-5s", true, "Should parse '-5s' successfully")]
#[case("-invalid", false, "Should fail to parse '-invalid'")]
#[case("1h", false, "Should reject '1h' as it would be a future timestamp")]
fn test_relative_past_parsing(#[case] input: &str, #[case] should_succeed: bool, #[case] msg: &str) {
    let tz = Tz::FixedOffset(Utc.fix());
    let format = DateTimeFormat::new();

    let result = parse_time(input, &tz, &format);
    if should_succeed {
        assert!(result.is_ok(), "{}", msg);
    } else {
        assert!(result.is_err(), "{}", msg);
    }
}

#[rstest]
#[case("+1h", true, "Should parse '+1h' successfully")]
#[case("+30m", true, "Should parse '+30m' successfully")]
#[case("+1d", true, "Should parse '+1d' successfully")]
#[case("+5s", true, "Should parse '+5s' successfully")]
#[case("+invalid", false, "Should fail to parse '+invalid'")]
#[case("1h", false, "Should reject '1h' as it would be a future timestamp")]
fn test_relative_future_parsing(#[case] input: &str, #[case] should_succeed: bool, #[case] msg: &str) {
    let tz = Tz::FixedOffset(Utc.fix());
    let format = DateTimeFormat::new();

    let result = parse_time(input, &tz, &format);
    if should_succeed {
        assert!(result.is_ok(), "{}", msg);
    } else {
        assert!(result.is_err(), "{}", msg);
    }
}

#[rstest]
#[case("-", "Should fail to parse lone '-'")]
#[case("+", "Should fail to parse lone '+'")]
#[case("-0invalidunit", "Should fail to parse '-0invalidunit'")]
#[case("+0invalidunit", "Should fail to parse '+0invalidunit'")]
fn test_relative_time_boundary_cases(#[case] input: &str, #[case] msg: &str) {
    let tz = Tz::FixedOffset(Utc.fix());
    let format = DateTimeFormat::new();

    let result = parse_time(input, &tz, &format);
    assert!(result.is_err(), "{}", msg);
}

#[rstest]
#[case("1 hour ago", true, "Should parse '1 hour ago' successfully")]
#[case("yesterday", true, "Should parse 'yesterday' successfully")]
#[case("1h", false, "Should reject '1h' (ambiguous bare duration)")]
#[case("2d", false, "Should reject '2d' (ambiguous bare duration)")]
#[case("30m", false, "Should reject '30m' (ambiguous bare duration)")]
#[case("5s", false, "Should reject '5s' (ambiguous bare duration)")]
#[case("tomorrow", true, "Should parse 'tomorrow' successfully")]
#[case("2027", true, "Should parse '2027' successfully")]
fn test_chrono_english_past_timestamps(#[case] input: &str, #[case] should_succeed: bool, #[case] msg: &str) {
    let tz = Tz::FixedOffset(Utc.fix());
    let format = DateTimeFormat::new();

    let result = parse_time(input, &tz, &format);
    if should_succeed {
        assert!(result.is_ok(), "{}", msg);
    } else {
        assert!(result.is_err(), "{}", msg);
    }
}

#[rstest]
#[case("1 hour ago")]
#[case("30 minutes ago")]
#[case("2 days ago")]
#[case("1 week ago")]
fn test_chrono_english_ago_syntax(#[case] input: &str) {
    let tz = Tz::FixedOffset(Utc.fix());
    let format = DateTimeFormat::new();

    let result = parse_time(input, &tz, &format);
    assert!(result.is_ok(), "Should parse '{}' successfully", input);
}

#[rstest]
#[case("in 1 hour")]
#[case("in 2 days")]
fn test_chrono_english_in_syntax(#[case] input: &str) {
    let tz = Tz::FixedOffset(Utc.fix());
    let format = DateTimeFormat::new();

    // Test that "in X" future syntax does not work (chrono-english doesn't support it)
    let result = parse_time(input, &tz, &format);
    assert!(
        result.is_err(),
        "'{}' should fail (not supported by chrono-english)",
        input
    );
}

#[test]
fn test_chrono_english_now() {
    let tz = Tz::FixedOffset(Utc.fix());
    let format = DateTimeFormat::new();

    // Test that "now" works
    let result = parse_time("now", &tz, &format);
    assert!(result.is_ok(), "Should parse 'now' successfully");
}

#[rstest]
#[case("today")]
#[case("yesterday")]
fn test_workaround_today_yesterday_start_of_day(#[case] input: &str) {
    let tz = Tz::FixedOffset(Utc.fix());
    let format = DateTimeFormat::new();

    let result = parse_time(input, &tz, &format);
    assert!(result.is_ok(), "Should parse '{}' successfully", input);

    let dt = result.unwrap();
    // Verify it's at start of day (00:00:00)
    assert_eq!(dt.hour(), 0, "'{}' should be at hour 00", input);
    assert_eq!(dt.minute(), 0, "'{}' should be at minute 00", input);
    assert_eq!(dt.second(), 0, "'{}' should be at second 00", input);
}

#[rstest]
#[case("friday")]
#[case("monday")]
#[case("tuesday")]
#[case("wednesday")]
#[case("thursday")]
#[case("saturday")]
#[case("sunday")]
#[case("fri")]
#[case("mon")]
#[case("tue")]
#[case("wed")]
#[case("thu")]
#[case("sat")]
#[case("sun")]
fn test_workaround_bare_weekday_means_last(#[case] input: &str) {
    let tz = Tz::FixedOffset(Utc.fix());
    let format = DateTimeFormat::new();

    let now = Utc::now().with_timezone(&tz);
    let result = parse_time(input, &tz, &format);

    assert!(result.is_ok(), "Should parse '{}' successfully", input);

    let dt = result.unwrap();
    // Bare weekday should refer to the past (last occurrence)
    assert!(
        dt <= now,
        "'{}' should be in the past, got {} vs now {}",
        input,
        dt,
        now
    );

    // Should be at start of day
    assert_eq!(dt.hour(), 0, "'{}' should be at hour 00", input);
    assert_eq!(dt.minute(), 0, "'{}' should be at minute 00", input);
    assert_eq!(dt.second(), 0, "'{}' should be at second 00", input);
}

#[rstest]
#[case("last friday")]
#[case("last monday")]
fn test_explicit_last_weekday_still_works(#[case] input: &str) {
    let tz = Tz::FixedOffset(Utc.fix());
    let format = DateTimeFormat::new();

    let now = Utc::now().with_timezone(&tz);
    let result = parse_time(input, &tz, &format);

    assert!(result.is_ok(), "Should parse '{}' successfully", input);

    let dt = result.unwrap();
    assert!(dt <= now, "'{}' should be in the past", input);
}

#[rstest]
#[case("january")]
#[case("february")]
#[case("march")]
#[case("april")]
#[case("may")]
#[case("june")]
#[case("july")]
#[case("august")]
#[case("september")]
#[case("october")]
#[case("november")]
#[case("december")]
#[case("jan")]
#[case("feb")]
#[case("mar")]
#[case("apr")]
#[case("jun")]
#[case("jul")]
#[case("aug")]
#[case("sep")]
#[case("oct")]
#[case("nov")]
#[case("dec")]
fn test_workaround_bare_month_means_last(#[case] input: &str) {
    let tz = Tz::FixedOffset(Utc.fix());
    let format = DateTimeFormat::new();

    let now = Utc::now().with_timezone(&tz);
    let result = parse_time(input, &tz, &format);

    assert!(result.is_ok(), "Should parse '{}' successfully", input);

    let dt = result.unwrap();
    // Bare month should refer to the past (last occurrence)
    assert!(
        dt <= now,
        "'{}' should be in the past, got {} vs now {}",
        input,
        dt,
        now
    );

    // Should be at 1st of the month at start of day
    assert_eq!(dt.day(), 1, "'{}' should be 1st of month", input);
    assert_eq!(dt.hour(), 0, "'{}' should be at hour 00", input);
    assert_eq!(dt.minute(), 0, "'{}' should be at minute 00", input);
    assert_eq!(dt.second(), 0, "'{}' should be at second 00", input);
}

#[rstest]
#[case("last april")]
#[case("last december")]
fn test_explicit_last_month_still_works(#[case] input: &str) {
    let tz = Tz::FixedOffset(Utc.fix());
    let format = DateTimeFormat::new();

    let now = Utc::now().with_timezone(&tz);
    let result = parse_time(input, &tz, &format);

    assert!(result.is_ok(), "Should parse '{}' successfully", input);

    let dt = result.unwrap();
    assert!(dt <= now, "'{}' should be in the past", input);
    assert_eq!(dt.day(), 1, "'{}' should be 1st of month", input);
}

#[rstest]
#[case("friday 19:00")]
#[case("monday 9:00")]
#[case("tuesday 14:30")]
fn test_workaround_weekday_with_time(#[case] input: &str) {
    let tz = Tz::FixedOffset(Utc.fix());
    let format = DateTimeFormat::new();

    let now = Utc::now().with_timezone(&tz);
    let result = parse_time(input, &tz, &format);

    assert!(result.is_ok(), "Should parse '{}' successfully", input);

    let dt = result.unwrap();
    // Should refer to the past (last occurrence of that weekday)
    assert!(
        dt <= now,
        "'{}' should be in the past, got {} vs now {}",
        input,
        dt,
        now
    );
}

#[rstest]
#[case("april 15")]
#[case("december 25")]
#[case("jan 1")]
fn test_workaround_month_with_day(#[case] input: &str) {
    let tz = Tz::FixedOffset(Utc.fix());
    let format = DateTimeFormat::new();

    let now = Utc::now().with_timezone(&tz);
    let result = parse_time(input, &tz, &format);

    assert!(result.is_ok(), "Should parse '{}' successfully", input);

    let dt = result.unwrap();
    // Should refer to the past (last occurrence of that month)
    assert!(
        dt <= now,
        "'{}' should be in the past, got {} vs now {}",
        input,
        dt,
        now
    );
}

#[rstest]
#[case("15 april")]
#[case("25 december")]
#[case("1 jan")]
#[case("15 apr")]
#[case("25 dec")]
fn test_workaround_day_month(#[case] input: &str) {
    let tz = Tz::FixedOffset(Utc.fix());
    let format = DateTimeFormat::new();

    let now = Utc::now().with_timezone(&tz);
    let result = parse_time(input, &tz, &format);

    assert!(result.is_ok(), "Should parse '{}' successfully", input);

    let dt = result.unwrap();
    // Should refer to the past (last occurrence of that month)
    assert!(
        dt <= now,
        "'{}' should be in the past, got {} vs now {}",
        input,
        dt,
        now
    );
}
