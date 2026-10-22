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
    test("2020-08-21T07:20:48", None, 1597994448, 0, 0);
    test("2020-08-21T07:20:48.123", None, 1597994448, 123000000, 0);
    test("1597994448", None, 1597994448, 0, 0);
    test("1597994448123", None, 1597994448, 123000000, 0);
    test("1597994448123456", None, 1597994448, 123456000, 0);
    test("1597994448123456789", None, 1597994448, 123456789, 0);
    test("1597994448.123", None, 1597994448, 123000000, 0);
    test("1597994448.123456", None, 1597994448, 123456000, 0);
    test("1597994448.123456789", None, 1597994448, 123456789, 0);
    test("-1.123456789", None, -2, 1000000000 - 123456789, 0);
    // The integer part may be zero or left out, and the fraction may be left out.
    test("0.5", None, 0, 500000000, 0);
    test(".5", None, 0, 500000000, 0);
    test("+.5", None, 0, 500000000, 0);
    test("-0.5", None, -1, 500000000, 0);
    test("-.5", None, -1, 500000000, 0);
    test("5.", None, 5, 0, 0);
    test("-5.", None, -5, 0, 0);
    test("1597994448.", None, 1597994448, 0, 0);
    test("-0.5", Some(UnixTimestampUnit::Milliseconds), -1, 999500000, 0);
    test("-.5", Some(UnixTimestampUnit::Microseconds), -1, 999999500, 0);
    // A fraction that rounds up to a whole second carries into the next one.
    test("59.9999999999", None, 60, 0, 0);
    test("1597994448.9999999996", None, 1597994449, 0, 0);
    test("-0.9999999999", None, -1, 0, 0);
    // The integer part may be as large as any integer timestamp.
    test("-9223372036854775808.0", None, -9223372037, 145224192, 0);
    // Fractions of nanoseconds are rounded, as in other units.
    test("-1.7", Some(UnixTimestampUnit::Nanoseconds), -1, 999999998, 0);
    test("-.7", Some(UnixTimestampUnit::Nanoseconds), -1, 999999999, 0);
    test("1.5", Some(UnixTimestampUnit::Nanoseconds), 0, 2, 0);
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
fn test_parse_out_of_range() {
    let test = |s, unit| assert_eq!(Timestamp::new(s).with_unix_unit(Some(unit)).parse(), None, "{s}");
    test("-8334601228800000.5", UnixTimestampUnit::Milliseconds);
    test("-8334601228800000000.5", UnixTimestampUnit::Microseconds);
    let max = format!("{}.9999999999", chrono::DateTime::<chrono::Utc>::MAX_UTC.timestamp());
    test(&max, UnixTimestampUnit::Seconds);
}

#[test]
fn test_parse_invalid_numbers() {
    for s in [".", "-.", "+", "-", "1.2.3", "--1", "1-"] {
        assert_eq!(Timestamp::new(s).parse(), None, "{s}");
    }
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

#[test]
fn test_timezone_is_utc() {
    use crate::timestamp::rfc3339::Timezone;

    // Test Z timezone
    let tz_z = Timezone::parse("Z").unwrap();
    assert!(tz_z.is_utc());

    // Test z timezone
    let tz_z_lower = Timezone::parse("z").unwrap();
    assert!(tz_z_lower.is_utc());

    // Test +00:00 timezone (should be UTC)
    let tz_plus_zero = Timezone::parse("+00:00").unwrap();
    assert!(tz_plus_zero.is_utc());

    // Test -00:00 timezone (should be UTC)
    let tz_minus_zero = Timezone::parse("-00:00").unwrap();
    assert!(tz_minus_zero.is_utc());

    // Test non-UTC timezone
    let tz_plus_three = Timezone::parse("+03:00").unwrap();
    assert!(!tz_plus_three.is_utc());

    // Test negative non-UTC timezone
    let tz_minus_five = Timezone::parse("-05:00").unwrap();
    assert!(!tz_minus_five.is_utc());
}

#[test]
fn test_fraction_parse() {
    use crate::timestamp::rfc3339::Fraction;

    // Test valid fractional seconds
    let frac_valid = Fraction::parse(".123").unwrap();
    assert_eq!(frac_valid.as_str(), ".123");

    // Test empty string (should be valid)
    let frac_empty = Fraction::parse("").unwrap();
    assert_eq!(frac_empty.as_str(), "");

    // Test invalid fractional seconds (starts with . but has non-digits)
    let frac_invalid1 = Fraction::parse(".abc");
    assert!(frac_invalid1.is_none());

    // Test invalid fractional seconds (starts with . but is too short)
    let frac_invalid2 = Fraction::parse(".");
    assert!(frac_invalid2.is_none());

    // Test invalid fractional seconds (doesn't start with .)
    let frac_invalid3 = Fraction::parse("123");
    assert!(frac_invalid3.is_none());
}

fn rfc3339_utc(s: &str, unit: Option<UnixTimestampUnit>) -> Option<String> {
    let mut buf = Vec::new();
    if Timestamp::new(s).with_unix_unit(unit).format_rfc3339_utc(&mut buf) {
        Some(String::from_utf8(buf).unwrap())
    } else {
        assert!(buf.is_empty(), "nothing is written on failure");
        None
    }
}

#[test]
fn test_format_rfc3339_utc_textual() {
    let test = |s, expected| assert_eq!(rfc3339_utc(s, None).as_deref(), Some(expected), "{s}");
    // Already in UTC with a `Z` suffix: copied verbatim.
    test("2026-07-06T10:00:00.123Z", "2026-07-06T10:00:00.123Z");
    test("2026-07-06T10:00:00Z", "2026-07-06T10:00:00Z");
    // Converted to UTC, keeping the source's fraction digits.
    test("2026-07-06T12:00:00.5+02:00", "2026-07-06T10:00:00.5Z");
    test("2026-07-06T10:00:00.10Z", "2026-07-06T10:00:00.10Z");
    test("2026-07-06T10:00:00.10+00:00", "2026-07-06T10:00:00.10Z");
    test("2026-07-06t10:00:00z", "2026-07-06T10:00:00Z");
    test("2026-07-06T12:00:00-02:30", "2026-07-06T14:30:00Z");
    // Zone-less values are interpreted as UTC.
    test("2026-07-06 10:00:00.123", "2026-07-06T10:00:00.123Z");
    test("2026-07-06T10:00:00", "2026-07-06T10:00:00Z");
    // At most 9 fraction digits.
    test("2026-07-06T10:00:00.1234567891Z", "2026-07-06T10:00:00.123456789Z");
}

#[test]
fn test_format_rfc3339_utc_unix() {
    let test = |s, unit, expected| assert_eq!(rfc3339_utc(s, unit).as_deref(), Some(expected), "{s}");
    // Integer values get the digits of their unit.
    test("1700000000", None, "2023-11-14T22:13:20Z");
    test("1700000000000", None, "2023-11-14T22:13:20.000Z");
    test("1700000000123456", None, "2023-11-14T22:13:20.123456Z");
    test("1700000000123456789", None, "2023-11-14T22:13:20.123456789Z");
    // Fractional values add their fraction digits to the unit's.
    test("1700000000.25", None, "2023-11-14T22:13:20.25Z");
    test("1700000000000.5", None, "2023-11-14T22:13:20.0005Z");
    test("1700000000.1234567891", None, "2023-11-14T22:13:20.123456789Z");
    // An explicit unit is honored.
    test(
        "1700000000",
        Some(UnixTimestampUnit::Milliseconds),
        "1970-01-20T16:13:20.000Z",
    );
    test("1700000000", Some(UnixTimestampUnit::Seconds), "2023-11-14T22:13:20Z");
    test(
        "1700000000000000",
        Some(UnixTimestampUnit::Nanoseconds),
        "1970-01-20T16:13:20.000000000Z",
    );
}

#[test]
fn test_format_rfc3339_utc_failure() {
    assert_eq!(rfc3339_utc("yesterday", None), None);
    assert_eq!(rfc3339_utc("-1e300", None), None);
    // Years that RFC 3339 can't represent.
    assert_eq!(
        rfc3339_utc("-62167219200", Some(UnixTimestampUnit::Seconds)).as_deref(),
        Some("0000-01-01T00:00:00Z")
    );
    assert_eq!(rfc3339_utc("-62167219201", Some(UnixTimestampUnit::Seconds)), None);
    assert_eq!(rfc3339_utc("253402300800", Some(UnixTimestampUnit::Seconds)), None);
}
