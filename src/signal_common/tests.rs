use super::*;

const SIGINT: i32 = 2;
const SIGTERM: i32 = 15;

const TIMEOUT: Duration = Duration::from_secs(1);
const SHORT: Duration = Duration::from_millis(100);
const LONG: Duration = Duration::from_secs(2);

#[test]
fn exit_code_follows_shell_convention() {
    assert_eq!(exit_code(SIGINT, SignalExitStatus::Conventional), 130);
    assert_eq!(exit_code(SIGTERM, SignalExitStatus::Conventional), 143);
}

#[test]
fn exit_code_is_zero_on_success() {
    assert_eq!(exit_code(SIGINT, SignalExitStatus::Success), 0);
    assert_eq!(exit_code(SIGTERM, SignalExitStatus::Success), 0);
}

#[test]
fn interrupt_counter_exits_on_first_hit_without_ignore() {
    let start = Instant::now();
    let mut counter = InterruptCounter::new(0, TIMEOUT, start);
    assert!(counter.hit(start + SHORT));
}

#[test]
fn interrupt_counter_exits_after_ignore_count_rapid_hits() {
    let start = Instant::now();
    let mut counter = InterruptCounter::new(3, TIMEOUT, start);
    assert!(!counter.hit(start + SHORT));
    assert!(!counter.hit(start + SHORT * 2));
    assert!(counter.hit(start + SHORT * 3));
}

#[test]
fn interrupt_counter_resets_after_timeout() {
    let start = Instant::now();
    let mut counter = InterruptCounter::new(2, TIMEOUT, start);
    assert!(!counter.hit(start + SHORT));
    assert!(!counter.hit(start + SHORT + LONG));
    assert!(!counter.hit(start + SHORT * 2 + LONG));
    assert!(counter.hit(start + SHORT * 3 + LONG));
}
