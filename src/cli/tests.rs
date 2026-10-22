use super::*;
use clap::CommandFactory;
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

    let set: InputInfoSet = json::from_str(r#"["none","minimal"]"#).unwrap();
    assert_eq!(
        InputInfo::resolve(*set),
        enum_set!(InputInfo::None | InputInfo::Minimal)
    );

    let res = json::from_str::<InputInfoSet>(r#"12"#);
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

#[test]
fn test_flatten_option() {
    assert_eq!(
        FlattenOption::from(Some(settings::FlattenOption::Never)),
        FlattenOption::Never
    );
    assert_eq!(
        FlattenOption::from(Some(settings::FlattenOption::Always)),
        FlattenOption::Always
    );
    assert_eq!(
        FlattenOption::from(settings::FlattenOption::Never),
        FlattenOption::Never
    );
    assert_eq!(
        FlattenOption::from(settings::FlattenOption::Always),
        FlattenOption::Always
    );
    assert_eq!(
        Into::<settings::FlattenOption>::into(FlattenOption::Never),
        settings::FlattenOption::Never
    );
    assert_eq!(
        Into::<settings::FlattenOption>::into(FlattenOption::Always),
        settings::FlattenOption::Always
    );
}

#[test]
fn test_expansion_option() {
    assert_eq!(ExpansionOption::from(None), ExpansionOption::Auto);
    assert_eq!(
        ExpansionOption::from(Some(settings::ExpansionMode::Inline)),
        ExpansionOption::Inline
    );
    assert_eq!(
        ExpansionOption::from(Some(settings::ExpansionMode::Never)),
        ExpansionOption::Never
    );
    assert_eq!(
        ExpansionOption::from(Some(settings::ExpansionMode::Always)),
        ExpansionOption::Always
    );
    assert_eq!(
        ExpansionOption::from(settings::ExpansionMode::Inline),
        ExpansionOption::Inline
    );
    assert_eq!(
        ExpansionOption::from(settings::ExpansionMode::Never),
        ExpansionOption::Never
    );
    assert_eq!(
        ExpansionOption::from(settings::ExpansionMode::Always),
        ExpansionOption::Always
    );
    assert_eq!(
        Into::<settings::ExpansionMode>::into(ExpansionOption::Inline),
        settings::ExpansionMode::Inline
    );
    assert_eq!(
        Into::<settings::ExpansionMode>::into(ExpansionOption::Never),
        settings::ExpansionMode::Never
    );
    assert_eq!(
        Into::<settings::ExpansionMode>::into(ExpansionOption::Always),
        settings::ExpansionMode::Always
    );
}

fn parse(args: &[&str]) -> std::result::Result<Opt, clap::Error> {
    Opt::try_parse_from(std::iter::once("hl").chain(args.iter().copied()))
}

#[test]
fn test_output_format_option() {
    assert_eq!(parse(&[]).unwrap().output_format, OutputFormat::Pretty);
    assert_eq!(
        parse(&["--output-format", "pretty"]).unwrap().output_format,
        OutputFormat::Pretty
    );
    assert_eq!(
        parse(&["--output-format", "json"]).unwrap().output_format,
        OutputFormat::Json
    );
    assert_eq!(
        parse(&["--output-format=logfmt"]).unwrap().output_format,
        OutputFormat::Logfmt
    );
    assert!(parse(&["--output-format", "yaml"]).is_err());

    // A repeated option overrides the earlier one.
    assert_eq!(
        parse(&["--output-format", "json", "--output-format", "logfmt"])
            .unwrap()
            .output_format,
        OutputFormat::Logfmt
    );
}

#[test]
fn test_output_format_env() {
    // The environment variable is read by clap only when the option is absent,
    // so checking the binding is enough, without changing the process environment.
    let cmd = Opt::command();
    let arg = cmd.get_arguments().find(|arg| arg.get_id() == "output_format").unwrap();
    assert_eq!(arg.get_env(), Some(std::ffi::OsStr::new("HL_OUTPUT_FORMAT")));
}

#[test]
fn test_flatten_option_auto() {
    assert_eq!(parse(&["--flatten", "auto"]).unwrap().flatten, FlattenOption::Auto);
    assert_eq!(parse(&[]).unwrap().flatten, FlattenOption::Auto);
    assert_eq!(FlattenOption::from(None), FlattenOption::Auto);
    assert_eq!(FlattenOption::from(settings::FlattenOption::Auto), FlattenOption::Auto);
    assert_eq!(
        settings::FlattenOption::from(FlattenOption::Auto),
        settings::FlattenOption::Auto
    );
}
