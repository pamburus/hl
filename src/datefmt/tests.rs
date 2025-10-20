use super::*;

use chrono_tz::UTC;

fn format(s: &str) -> DateTimeFormat {
    LinuxDateFormat::new(s).compile()
}

fn utc(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> DateTime<Tz> {
    Tz::IANA(UTC)
        .with_ymd_and_hms(year, month, day, hour, min, sec)
        .unwrap()
}

fn f(fmt: &str, dt: DateTime<Tz>) -> String {
    let mut buf = Vec::new();
    format_date(&mut buf, dt, format(fmt));
    String::from_utf8(buf).unwrap()
}

#[test]
fn test_default_formatter() {
    // Test the Default implementation for DateTimeFormatter
    let formatter = DateTimeFormatter::default();

    // Verify the formatter can format dates correctly
    let dt = utc(2023, 5, 15, 14, 30, 45).fixed_offset();
    let mut buf = Vec::new();
    formatter.format(&mut buf, dt);
    let formatted = String::from_utf8(buf).unwrap();

    // The default format is "%Y-%m-%d %H:%M:%S"
    assert_eq!(formatted, "2023-05-15 14:30:45");
}

#[test]
fn test_compile_offset() {
    assert_eq!(format("%:z"), vec![Item::TimeZoneOffset((Flags::empty(), 2))]);
}

#[test]
fn test_linux_date_format() {
    assert_eq!(
        f("%Y-%m-%d %H:%M:%S %z", utc(2020, 1, 1, 12, 0, 0)),
        "2020-01-01 12:00:00 +0000"
    );
    assert_eq!(
        f("%Y-%m-%d %H:%M:%S %::z", utc(2020, 1, 1, 12, 0, 0)),
        "2020-01-01 12:00:00 +00:00:00"
    );
    assert_eq!(
        f("%Y-%m-%d %H:%M:%S %Z", utc(2020, 1, 1, 12, 0, 0)),
        "2020-01-01 12:00:00 UTC"
    );
    assert_eq!(f("%s", utc(2020, 1, 1, 12, 0, 0)), "1577880000");
    assert_eq!(f("%e", utc(2020, 1, 1, 12, 0, 0)), " 1");
    assert_eq!(f("%3e", utc(2020, 1, 1, 12, 0, 0)), "  1");
    assert_eq!(f("%02e", utc(2020, 1, 1, 12, 0, 0)), "01");
    assert_eq!(f("%p", utc(2020, 1, 1, 12, 0, 0)), "PM");
    assert_eq!(f("%P", utc(2020, 1, 1, 12, 0, 0)), "pm");
    assert_eq!(f("%^P", utc(2020, 1, 1, 12, 0, 0)), "PM");
    assert_eq!(f("%#p", utc(2020, 1, 1, 12, 0, 0)), "pm");
}

#[test]
fn test_reformat_rfc3339() {
    use crate::timestamp::Timestamp;

    let tz = |secs| Tz::FixedOffset(FixedOffset::east_opt(secs).unwrap());
    let tsr = Timestamp::new("2020-06-27T00:48:30.466249792+00:00");
    let tsr = tsr.as_rfc3339().unwrap();

    let zones = &[0];
    let formats = &[("%y-%m-%d %T.%N"), ("%b %d %T.%N"), ("%Y-%m-%d %T.%N %:z")];

    for tzv in zones {
        for fmt in formats {
            let setup = || {
                let buf = Vec::<u8>::with_capacity(128);
                let format = LinuxDateFormat::new(fmt).compile();
                let formatter = DateTimeFormatter::new(format, tz(*tzv));
                (formatter, buf, tsr.clone())
            };
            let payload = |(formatter, mut buf, tsr): (DateTimeFormatter, Vec<u8>, rfc3339::Timestamp)| {
                formatter.reformat_rfc3339(&mut buf, tsr);
                buf.len()
            };
            assert!(payload(setup()) != 0);
        }
    }
}

#[test]
fn test_century_format_rfc3339() {
    // Test century format %C in reformat_rfc3339 to cover lines 575-576
    use crate::timestamp::Timestamp;

    let tz = |secs| Tz::FixedOffset(FixedOffset::east_opt(secs).unwrap());
    let tsr = Timestamp::new("2023-05-15T14:30:45+00:00");
    let tsr = tsr.as_rfc3339().unwrap();

    // Create formatter with century format
    let format = LinuxDateFormat::new("%C").compile();
    let formatter = DateTimeFormatter::new(format, tz(0));
    let mut buf = Vec::new();

    formatter.reformat_rfc3339(&mut buf, tsr);
    let result = String::from_utf8(buf).unwrap();
    assert_eq!(result, "20"); // 2023 -> century 20
}

#[test]
fn test_weekday_numeric_formats() {
    // Test different weekday numeric formats to cover various flag combinations
    // in weekday_numeric function (lines 848-849, 851)

    // Monday, May 15, 2023
    let monday = utc(2023, 5, 15, 14, 30, 45);

    // %u format: Monday=1 to Sunday=7 (FromZero=false, FromSunday=false)
    assert_eq!(f("%u", monday), "1");

    // %w format: Sunday=0 to Saturday=6 (FromZero=true, FromSunday=true)
    assert_eq!(f("%w", monday), "1");

    // Test Sunday to cover the FromSunday branch
    let sunday = utc(2023, 5, 14, 14, 30, 45);
    assert_eq!(f("%u", sunday), "7"); // Sunday=7 in %u format
    assert_eq!(f("%w", sunday), "0"); // Sunday=0 in %w format
}

#[test]
fn test_year_padding_flags_rfc3339() {
    // Test year formatting with padding flags in reformat_rfc3339 to cover lines 953-954, 956, 960-961
    use crate::timestamp::Timestamp;

    let tz = |secs| Tz::FixedOffset(FixedOffset::east_opt(secs).unwrap());
    let tsr = Timestamp::new("2023-05-15T14:30:45+00:00");
    let tsr = tsr.as_rfc3339().unwrap();

    // Test year format with NoPadding flag (%-Y)
    let format = LinuxDateFormat::new("%-Y").compile();
    let formatter = DateTimeFormatter::new(format, tz(0));
    let mut buf = Vec::new();
    formatter.reformat_rfc3339(&mut buf, tsr.clone());
    let result = String::from_utf8(buf).unwrap();
    assert_eq!(result, "2023"); // Should trigger NoPadding branch

    // Test year format with SpacePadding flag (%_Y)
    let format2 = LinuxDateFormat::new("%_Y").compile();
    let formatter2 = DateTimeFormatter::new(format2, tz(0));
    let mut buf2 = Vec::new();
    formatter2.reformat_rfc3339(&mut buf2, tsr);
    let result2 = String::from_utf8(buf2).unwrap();
    assert_eq!(result2, "2023"); // Should trigger SpacePadding branch
}
