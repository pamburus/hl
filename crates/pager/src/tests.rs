use super::*;

#[test]
fn pager_from_env_creates_instance() {
    let pager = Pager::from_env();
    assert!(matches!(pager.origin, CommandOrigin::FromEnv { app_env_var: None }));
}

#[test]
fn pager_env_var_sets_app_env_var() {
    let pager = Pager::from_env().lookup_var("HL_PAGER");
    assert!(matches!(pager.origin, CommandOrigin::FromEnv { app_env_var: Some(ref v) } if v == "HL_PAGER"));
}

#[test]
fn pager_custom_stores_command() {
    let pager = Pager::custom(["less", "-R"]);
    assert!(matches!(pager.origin, CommandOrigin::Custom { ref command } if command == &["less", "-R"]));
}

#[test]
fn pager_env_sets_env_var() {
    let pager = Pager::custom(["less"]).with_env_var("LESSCHARSET", "UTF-8");
    assert_eq!(pager.env.get("LESSCHARSET"), Some(&"UTF-8".to_string()));
}

#[test]
fn pager_envs_sets_multiple() {
    let pager = Pager::custom(["less"]).with_env([("A", "1"), ("B", "2")]);
    assert_eq!(pager.env.get("A"), Some(&"1".to_string()));
    assert_eq!(pager.env.get("B"), Some(&"2".to_string()));
}

#[test]
fn pager_custom_empty_command_start_returns_error() {
    let result = Pager::custom([] as [&str; 0]).start();
    assert!(matches!(result, Some(Err(ref e)) if e.kind() == io::ErrorKind::InvalidInput));
}

#[test]
fn pager_custom_nonexistent_binary_start_returns_error() {
    let result = Pager::custom(["/nonexistent/binary/hl_test_abc123"]).start();
    assert!(matches!(result, Some(Err(_))));
}

#[test]
fn pager_from_env_whitespace_only_pager_returns_none() {
    let result = Pager::from_env().with_env_provider(|_| Some(" ".to_string())).start();
    assert!(result.is_none());
}

#[cfg(unix)]
#[test]
fn pager_from_env_with_app_var_resolves_and_starts() {
    use std::io::Write;

    let result = Pager::from_env()
        .lookup_var("HL_PAGER")
        .with_env_provider(|v| if v == "HL_PAGER" { Some("cat".to_string()) } else { None })
        .start();
    let mut pager = result.expect("should return Some").expect("should start successfully");
    pager.flush().expect("flush should succeed");
    // drop closes stdin and waits for cat to exit
}

#[cfg(unix)]
#[test]
fn pager_process_dropped_without_explicit_wait() {
    let mut pager = Pager::custom(["cat"])
        .start()
        .expect("should return Some")
        .expect("should start successfully");
    let process = pager.detach_process().expect("detach should return Some");
    drop(pager); // closes stdin, signals EOF to cat
    drop(process); // PagerProcess::drop waits for cat to exit
}

#[cfg(unix)]
#[test]
fn pager_exit_result_signal_returns_none_for_normal_exit() {
    let mut pager = Pager::custom(["cat"])
        .start()
        .expect("should return Some")
        .expect("should start successfully");
    let mut process = pager.detach_process().expect("detach should return Some");
    drop(pager); // closes stdin
    let result = process.wait().expect("wait should succeed");
    assert_eq!(result.signal(), None);
}
