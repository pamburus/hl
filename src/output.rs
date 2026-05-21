use std::io::Write;

#[cfg(feature = "native")]
use clap::ValueEnum;
use serde::Deserialize;

pub type OutputStream = Box<dyn Write + Send + Sync>;

#[cfg_attr(feature = "native", derive(ValueEnum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputDelimiter {
    Newline,
    Nul,
}
