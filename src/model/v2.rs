pub mod ast;
pub mod parse;
pub mod record;
pub mod value;

pub use record::{
    Record,
    filter::{FieldFilter, FieldFilterKey, Number, NumericOp, ValueMatchPolicy},
};

pub(crate) use record::filter::UnaryBoolOp;

pub use crate::level::Level;

pub mod compat {
    pub use super::{
        parse::{Parser, Settings as ParserSettings},
        record::{
            Record, RecordWithSource, RecordWithSourceConstructor,
            filter::{CombinedFilter as Filter, FieldFilterSet, Filter as RecordFilter},
        },
    };
    pub use crate::level::Level;

    pub type RecordFilterNone = super::record::filter::Pass;
}
