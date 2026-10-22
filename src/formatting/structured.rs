//! Formatting of records in machine-readable output formats, such as JSON and logfmt.

mod json;
mod keyset;
mod logfmt;

#[cfg(test)]
mod proptests;
#[cfg(test)]
mod tests;
