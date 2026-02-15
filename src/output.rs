use std::io::Write;

pub type OutputStream = Box<dyn Write + Send + Sync>;
