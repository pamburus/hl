// std imports
use std::{
    borrow::Cow,
    fmt::{self, Write as _},
    path::Path,
};

// third-party imports
use owo_colors::{OwoColorize, Style};

// local imports
pub mod suggest;

// re-exports
pub use suggest::Suggestions;

// ---

pub trait Highlight {
    type Output: fmt::Display;

    fn hl(self) -> Self::Output;
}

impl<'a, S> Highlight for &'a S
where
    S: fmt::Display,
{
    type Output = Highlighted<&'a S>;

    fn hl(self) -> Self::Output {
        Highlighted(self)
    }
}

impl<'a> Highlight for &'a Path {
    type Output = Highlighted<Converted<&'a Path>>;

    fn hl(self) -> Self::Output {
        Converted(self).hl()
    }
}

impl<'a, S> Highlight for &'a [S]
where
    S: fmt::Display,
{
    type Output = HighlightedSequence<&'a [S]>;

    fn hl(self) -> Self::Output {
        HighlightedSequence(self)
    }
}

impl<S, const N: usize> Highlight for [S; N]
where
    S: fmt::Display,
{
    type Output = HighlightedSequence<[S; N]>;

    fn hl(self) -> Self::Output {
        HighlightedSequence(self)
    }
}

// ---

pub trait HighlightQuoted {
    type Output: fmt::Display;

    fn hlq(self) -> Self::Output;
}

impl<'a, S> HighlightQuoted for &'a S
where
    S: fmt::Display,
{
    type Output = Highlighted<Quoted<&'a S>>;

    fn hlq(self) -> Self::Output {
        Quoted(self).hl()
    }
}

impl<'a> HighlightQuoted for &'a Path {
    type Output = Highlighted<Quoted<Converted<&'a Path>>>;

    fn hlq(self) -> Self::Output {
        Quoted(Converted(self)).hl()
    }
}

// ---

pub struct Highlighted<S>(S);

impl<S> fmt::Display for Highlighted<S>
where
    S: fmt::Display + Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.style(HIGHLIGHT))
    }
}

// ---

pub struct HighlightedSequence<S>(S);

impl<S> fmt::Display for HighlightedSequence<&[S]>
where
    S: fmt::Display + Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        for (i, item) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", item.style(HIGHLIGHT))?;
        }
        write!(f, "]")
    }
}

impl<S, const N: usize> fmt::Display for HighlightedSequence<[S; N]>
where
    S: fmt::Display + Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        HighlightedSequence(&self.0[..]).fmt(f)
    }
}

// ---

pub struct Quoted<S>(S);

impl<S> fmt::Display for Quoted<S>
where
    S: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut buf = String::new();
        write!(&mut buf, "{}", self.0)?;
        write!(f, "{:?}", buf)
    }
}

impl<S> Highlight for Quoted<S>
where
    S: fmt::Display,
{
    type Output = Highlighted<Quoted<S>>;

    fn hl(self) -> Self::Output {
        Highlighted(self)
    }
}

// ---

pub struct Converted<T>(T);

impl<T> fmt::Display for Converted<T>
where
    T: HighlightConvert + Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.clone().convert())
    }
}

impl<S> Highlight for Converted<S>
where
    S: HighlightConvert + Clone,
{
    type Output = Highlighted<Converted<S>>;

    fn hl(self) -> Self::Output {
        Highlighted(self)
    }
}

// ---

trait HighlightConvert {
    type Output: fmt::Display;

    fn convert(self) -> Self::Output;
}

impl<'a> HighlightConvert for &'a Path {
    type Output = Cow<'a, str>;

    fn convert(self) -> Self::Output {
        self.to_string_lossy()
    }
}

// ---

const HIGHLIGHT: Style = Style::new().yellow();

#[cfg(test)]
mod tests;
