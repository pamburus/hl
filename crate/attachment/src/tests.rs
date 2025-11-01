// ---
use super::*;

#[test]
fn test_attach() {
    use super::*;

    let a = WithAttachment::new(1);
    let (b, v) = a.attach(2).detach();
    assert_eq!(b, WithAttachment(1, NoAttachment));
    assert_eq!(v, 2);
}

#[test]
fn test_error_attach() {
    let f = || mk_ok(10).with_attachment(42u32);
    let (x, y) = f().unwrap().detach();
    assert_eq!(x, WithAttachment(10, NoAttachment));
    assert_eq!(y, 42);
}

fn mk_ok(val: usize) -> Result<usize, &'static str> {
    Ok(val)
}
