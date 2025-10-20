use super::*;
use enumset::enum_set;
use std::str::FromStr;

type InputInfoSet = EnumSet<InputInfo>;

#[test]
fn test_input_info() {
    let set = InputInfoSet::all();
    assert_eq!(
        *set,
        enum_set!(InputInfo::Auto | InputInfo::None | InputInfo::Minimal | InputInfo::Compact | InputInfo::Full)
    );
    assert_eq!(set.to_string(), "auto,none,minimal,compact,full");

    let set = InputInfoSet::from_str("auto,none").unwrap();
    assert_eq!(InputInfo::resolve(*set), enum_set!(InputInfo::None));

    let set = InputInfoSet::empty();
    assert_eq!(
        InputInfo::resolve(*set),
        enum_set!(InputInfo::None | InputInfo::Minimal | InputInfo::Compact | InputInfo::Full)
    );

    let set = InputInfoSet::from_str("auto,none,invalid");
    assert!(set.is_err());

    let set = InputInfoSet::from_str("auto").unwrap();
    assert_eq!(
        InputInfo::resolve(*set),
        enum_set!(InputInfo::None | InputInfo::Minimal | InputInfo::Compact | InputInfo::Full)
    );

    let set = InputInfoSet::from_str("auto,none").unwrap();
    assert_eq!(InputInfo::resolve(*set), enum_set!(InputInfo::None));

    let set: InputInfoSet = serde_json::from_str(r#"["none","minimal"]"#).unwrap();
    assert_eq!(
        InputInfo::resolve(*set),
        enum_set!(InputInfo::None | InputInfo::Minimal)
    );

    let res = serde_json::from_str::<InputInfoSet>(r#"12"#);
    assert!(res.is_err());
}

#[test]
fn test_ascii_option() {
    // Test conversion from AsciiOption to AsciiModeOpt
    assert_eq!(AsciiModeOpt::from(AsciiOption::Auto), AsciiModeOpt::Auto);
    assert_eq!(AsciiModeOpt::from(AsciiOption::Never), AsciiModeOpt::Never);
    assert_eq!(AsciiModeOpt::from(AsciiOption::Always), AsciiModeOpt::Always);

    // Verify all options are covered for AsciiOption to AsciiModeOpt
    let options = [AsciiOption::Auto, AsciiOption::Never, AsciiOption::Always];
    for opt in &options {
        match opt {
            AsciiOption::Auto => assert_eq!(AsciiModeOpt::from(*opt), AsciiModeOpt::Auto),
            AsciiOption::Never => assert_eq!(AsciiModeOpt::from(*opt), AsciiModeOpt::Never),
            AsciiOption::Always => assert_eq!(AsciiModeOpt::from(*opt), AsciiModeOpt::Always),
        }
    }

    // Test conversion from AsciiModeOpt to AsciiOption
    assert_eq!(AsciiOption::from(AsciiModeOpt::Auto), AsciiOption::Auto);
    assert_eq!(AsciiOption::from(AsciiModeOpt::Never), AsciiOption::Never);
    assert_eq!(AsciiOption::from(AsciiModeOpt::Always), AsciiOption::Always);

    // Verify all options are covered for AsciiModeOpt to AsciiOption
    let mode_options = [AsciiModeOpt::Auto, AsciiModeOpt::Never, AsciiModeOpt::Always];
    for mode_opt in &mode_options {
        match mode_opt {
            AsciiModeOpt::Auto => assert_eq!(AsciiOption::from(*mode_opt), AsciiOption::Auto),
            AsciiModeOpt::Never => assert_eq!(AsciiOption::from(*mode_opt), AsciiOption::Never),
            AsciiModeOpt::Always => assert_eq!(AsciiOption::from(*mode_opt), AsciiOption::Always),
        }
    }
}
