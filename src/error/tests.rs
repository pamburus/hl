use super::*;
use assert_matches::assert_matches;

struct TestAppInfo;
impl AppInfoProvider for TestAppInfo {}

#[derive(Default)]
struct CustomAppInfo {
    suggestion_arg: &'static str,
}

impl AppInfoProvider for CustomAppInfo {
    fn app_name(&self) -> Cow<'static, str> {
        "test".into()
    }

    fn usage_suggestion(&self, request: UsageRequest) -> Option<UsageResponse> {
        match request {
            UsageRequest::ListThemes => Some(("list-themes".into(), self.suggestion_arg.into())),
        }
    }
}

#[test]
fn test_log() {
    let err = Error::Io(std::io::Error::other("test"));
    let mut buf = Vec::new();
    err.log_to(&mut buf, &TestAppInfo).unwrap();
    assert_eq!(
        String::from_utf8(buf).unwrap(),
        "\u{1b}[1m\u{1b}[91merror:\u{1b}[39m\u{1b}[0m test\n"
    );
}

#[test]
fn test_tips() {
    let err = Error::Theme(themecfg::Error::ThemeNotFound {
        name: "test".to_string(),
        suggestions: Suggestions::new("test", vec!["test1", "test2"]),
    });
    assert_eq!(
        err.tips(&TestAppInfo).to_string(),
        "\u{1b}[1m\u{1b}[32m  tip:\u{1b}[39m\u{1b}[0m did you mean \u{1b}[33m\"test1\"\u{1b}[0m or \u{1b}[33m\"test2\"\u{1b}[0m?\n",
    );

    let mut buf = Vec::new();
    err.log_to(&mut buf, &TestAppInfo).unwrap();
    assert!(!buf.is_empty());

    let err = Error::Theme(themecfg::Error::ThemeNotFound {
        name: "test".to_string(),
        suggestions: Suggestions::none(),
    });

    assert_eq!(
        err.tips(&CustomAppInfo::default()).to_string(),
        "\u{1b}[1m\u{1b}[32m  tip:\u{1b}[39m\u{1b}[0m run \u{1b}[1mtest list-themes\u{1b}[0m to list available themes\n",
    );
}

#[test]
fn test_usage() {
    let app = CustomAppInfo::default();
    assert_eq!(
        app.usage_suggestion(UsageRequest::ListThemes),
        Some(("list-themes".into(), "".into()))
    );
    let app = CustomAppInfo {
        suggestion_arg: "<filter>",
    };
    assert_eq!(
        app.usage_suggestion(UsageRequest::ListThemes),
        Some(("list-themes".into(), "<filter>".into()))
    );
    assert_eq!(
        usage(&app, UsageRequest::ListThemes),
        Some("\u{1b}[1mtest list-themes\u{1b}[0m <filter>".into())
    );
}

#[test]
fn test_app_name() {
    assert!(!TestAppInfo.app_name().is_empty());
}

#[test]
fn test_from_config_error() {
    let config_err = ConfigError::Message("test config error".to_string());
    let err = Error::from(config_err);
    assert_matches!(err, Error::Config(boxed_err) if boxed_err.to_string().contains("test config error"));
}

#[test]
fn test_from_notify_error() {
    let notify_err = notify::Error::path_not_found();
    let err = Error::from(notify_err);
    assert_matches!(err, Error::NotifyError(boxed_err) if !boxed_err.to_string().is_empty());
}

#[test]
fn test_from_pest_error() {
    // Create a simple pest error for testing
    let pest_err = pest::error::Error::<crate::query::Rule>::new_from_pos(
        pest::error::ErrorVariant::ParsingError {
            positives: vec![],
            negatives: vec![],
        },
        pest::Position::from_start("test"),
    );
    let err = Error::from(pest_err);
    assert_matches!(err, Error::QueryParseError(_));
}
