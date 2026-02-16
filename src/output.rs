use std::io::Write;

use clap::ValueEnum;
use serde::Deserialize;

pub type OutputStream = Box<dyn Write + Send + Sync>;

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputDelimiter {
    Newline,
    Nul,
}
