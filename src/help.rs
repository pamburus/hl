// std imports
use std::{fmt, io};

// third-party imports
use owo_colors::OwoColorize;

pub struct Formatter<O> {
    width: Option<usize>,
    output: O,
}

impl<O> Formatter<O>
where
    O: io::Write,
{
    pub fn new(output: O) -> Self
    where
        O: io::IsTerminal,
    {
        let width = if output.is_terminal() {
            term_size::dimensions().map(|d| d.0)
        } else {
            None
        };

        Self { output, width }
    }

    #[allow(dead_code)]
    pub fn with_width(output: O, width: Option<usize>) -> Self {
        Self { output, width }
    }

    pub fn format_grouped_list<G, V, GI, I>(&mut self, groups: GI) -> io::Result<()>
    where
        GI: IntoIterator<Item = (G, I)>,
        I: IntoIterator<Item = V>,
        G: fmt::Display,
        V: AsRef<str>,
    {
        let Some(width) = self.width else {
            return self.format_raw_list(groups.into_iter().map(|x| x.1).flat_map(|x| x.into_iter()));
        };

        let out = &mut self.output;

        let mut groups = groups
            .into_iter()
            .map(|(g, items)| (g, items.into_iter().collect::<Vec<_>>()))
            .collect::<Vec<_>>();

        let max_len = groups
            .iter()
            .map(|x| x.1.iter().map(|x| x.as_ref().len()).max().unwrap_or(0))
            .max()
            .unwrap_or(0);

        let columns = width / (max_len + 4);

        for (group, items) in groups.iter_mut() {
            writeln!(out, "{}:", group.bold())?;

            let rows = items.len().div_ceil(columns);

            for row in 0..rows {
                for col in 0..columns {
                    if let Some(val) = items.get(row + col * rows) {
                        write!(out, "• {:width$}", val.as_ref(), width = max_len + 2)?;
                    }
                }
                writeln!(out)?;
            }
        }
        Ok(())
    }

    fn format_raw_list<I, V>(&mut self, items: I) -> io::Result<()>
    where
        I: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        for item in items {
            writeln!(&mut self.output, "{}", item.as_ref())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
