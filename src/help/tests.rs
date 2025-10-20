use std::io::IsTerminal;

use super::*;

#[test]
fn test_formatter_new() {
    let formatter = Formatter::new(std::io::stdout());
    assert!(formatter.width.is_some() == std::io::stdout().is_terminal());
}

#[test]
fn test_format_grouped_list() {
    let mut buf = Vec::new();
    let mut f = Formatter::with_width(&mut buf, Some(80));

    f.format_grouped_list([
        ("Group 1", ["Item 1", "Item 2", "Item 3"]),
        ("Group 2", ["Item 4", "Item 5", "Item 6"]),
    ])
    .unwrap();

    let output = String::from_utf8(buf).unwrap();
    assert_eq!(
        output,
        "\u{1b}[1mGroup 1\u{1b}[0m:\n• Item 1  • Item 2  • Item 3  \n\u{1b}[1mGroup 2\u{1b}[0m:\n• Item 4  • Item 5  • Item 6  \n"
    );
}
