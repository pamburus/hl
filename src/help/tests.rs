use std::io::IsTerminal;

use super::*;

#[test]
fn test_formatter_new() {
    let output = std::io::stdout();
    let is_terminal = output.is_terminal();
    let formatter = Formatter::new(output);
    if !is_terminal {
        assert!(formatter.width.is_none());
    }
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

#[test]
fn test_format_trait_grouped_list() {
    let mut buf = Vec::new();
    let mut f = Formatter::with_width(&mut buf, Some(80));

    Format::format_grouped_list(
        &mut f,
        [("Group 1", ["Item 1", "Item 2"]), ("Group 2", ["Item 3", "Item 4"])],
    )
    .unwrap();

    let output = String::from_utf8(buf).unwrap();
    assert_eq!(
        output,
        "\u{1b}[1mGroup 1\u{1b}[0m:\n• Item 1  • Item 2  \n\u{1b}[1mGroup 2\u{1b}[0m:\n• Item 3  • Item 4  \n"
    );
}

#[test]
fn test_format_trait_raw_list() {
    let mut buf = Vec::new();
    let mut f = Formatter::with_width(&mut buf, Some(80));

    Format::format_raw_list(&mut f, ["Item 1", "Item 2", "Item 3"]).unwrap();

    let output = String::from_utf8(buf).unwrap();
    assert_eq!(output, "Item 1\nItem 2\nItem 3\n");
}
