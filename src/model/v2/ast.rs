// std imports
use std::{ops::Range, sync::Arc};

// workspace imports
use encstr::EncodedString;
use flat_tree::tree::{self};

// ---

const PREALLOCATED_CAPACITY: usize = 128;

pub type Span = Range<usize>;

pub mod error {
    pub use super::Span;
    pub type Error = (&'static str, Span);
    pub type Result<T> = std::result::Result<T, Error>;
}

pub use error::Result;

pub type Container = log_ast::ast::Container;
pub type Segment = log_ast::model::Segment<Arc<str>>;

// ---

pub trait BuildAttachment: tree::BuildAttachment {}
impl<A: tree::BuildAttachment> BuildAttachment for A {}

pub use flat_tree::{Index, OptIndex};
pub use tree::{AttachmentChild, AttachmentParent, AttachmentValue};

// ---

pub type Node<'c> = log_ast::ast::Node<'c>;
pub type Value = log_ast::ast::Value;
pub type Scalar = log_ast::ast::Scalar;
pub type Composite = log_ast::ast::Composite;
pub type String<'s> = EncodedString<'s>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder() {
        let mut container = Container::new();
        let root = container.metaroot();
        root.add_scalar(Scalar::Bool(true))
            .add_composite(Composite::Array, |b| (b.add_scalar(Scalar::Bool(false)), Ok(())))
            .1
            .unwrap();
        assert_eq!(container.roots().len(), 2);
    }

    #[test]
    fn test_builder_attach() {
        let mut container = Container::new();
        let root = container.metaroot();
        let attachment = root
            .add_scalar(Scalar::Bool(true))
            .attach("attachment")
            .add_composite(Composite::Array, |b| {
                let (b, attachment) = b.detach();
                assert_eq!(attachment, "attachment");
                (b.add_scalar(Scalar::Bool(false)).attach("another attachment"), Ok(()))
            })
            .0
            .detach()
            .1;
        assert_eq!(container.roots().len(), 2);
        assert_eq!(attachment, "another attachment");
    }
}
