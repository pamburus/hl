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
