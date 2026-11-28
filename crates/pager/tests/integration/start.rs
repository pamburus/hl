//! Starting pager processes and waiting for them to exit.

use pager::Pager;

#[test]
fn pager_custom_nonexistent_binary_start_returns_error() {
    let result = Pager::custom(["/nonexistent/binary/hl_test_abc123"]).start();
    assert!(matches!(result, Some(Err(_))));
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
