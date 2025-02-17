use core::fmt::Display;

use super::Span;

pub trait Origin: Display {
    fn map(&self, span: Span) -> Self;
}
