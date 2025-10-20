use std::str::FromStr;

use super::*;

#[test]
fn test_tz_offset() {
    let local = TzOffset::Local(FixedOffset::east_opt(3600).unwrap());
    let fixed = TzOffset::Fixed(FixedOffset::east_opt(2 * 3600).unwrap());
    let utc = TzOffset::IANA(
        chrono_tz::UTC.offset_from_utc_datetime(&NaiveDateTime::from_str("2024-01-01T00:00:00").unwrap()),
    );

    assert_eq!(local.tz_id(), "(Local)");
    assert_eq!(local.abbreviation(), Some("(L)"));
    assert_eq!(fixed.tz_id(), "(Fixed)");
    assert_eq!(fixed.abbreviation(), Some("(F)"));
    assert_eq!(utc.tz_id(), "UTC");
    assert_eq!(utc.abbreviation(), Some("UTC"));
}
