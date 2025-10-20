use super::*;

#[test]
fn test_suggestions() {
    let suggestions = Suggestions::new("hello", vec!["helo", "hello", "helo", "hola", "hallo"]);
    assert!(!suggestions.is_empty());

    let mut iter = suggestions.iter();
    assert_eq!(iter.next(), Some("hello"));
    assert_eq!(iter.next(), Some("helo"));
    assert_eq!(iter.next(), Some("hallo"));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_suggestions_extend() {
    let mut suggestions = Suggestions::new("hello", vec!["helo", "hell", "hola", "hallo"]);
    suggestions = suggestions.merge(Suggestions::new("hello", vec!["helo", "hola", "hallo", "hello"]));
    let mut iter = suggestions.iter();

    assert_eq!(iter.next(), Some("hello"));
    assert_eq!(iter.next(), Some("helo"));
    assert_eq!(iter.next(), Some("hell"));
    assert_eq!(iter.next(), Some("hallo"));
    assert_eq!(iter.next(), None);

    let mut suggestions = Suggestions::new("hello", vec!["helo", "hell", "hola", "hallo"]);
    suggestions = suggestions.merge(Suggestions::new("hello", vec!["hello"]));
    let mut iter = suggestions.iter();

    assert_eq!(iter.next(), Some("hello"));
    assert_eq!(iter.next(), Some("helo"));
    assert_eq!(iter.next(), Some("hell"));
    assert_eq!(iter.next(), Some("hallo"));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_suggestions_none() {
    let suggestions = Suggestions::none();
    assert!(suggestions.is_empty());
    let mut iter = (&suggestions).into_iter();

    assert_eq!(iter.next(), None);
}
