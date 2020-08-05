use std::fs::File;
use std::io::{Error, Read, Result};
use std::path::PathBuf;

use ansi_term::Colour;

pub struct Input {
    pub name: String,
    pub stream: Box<dyn Read + Send + Sync>,
}

pub struct ConcatReader<I> {
    iter: I,
    item: Option<Input>,
}

pub fn open(filename: &PathBuf) -> Result<Input> {
    let name = format!(
        "file '{}'",
        Colour::Yellow.paint(filename.to_string_lossy()),
    );

    let f = File::open(filename)
        .map_err(|e| Error::new(e.kind(), format!("failed to open {}: {}", name, e)))?;

    Ok(Input::new(name, Box::new(f)))
}

impl Input {
    pub fn new(name: String, stream: Box<dyn Read + Send + Sync>) -> Self {
        Self { name, stream }
    }
}

impl<I> ConcatReader<I>
where
    I: Iterator<Item = Result<Input>>,
{
    pub fn new(iter: I) -> Self {
        Self { iter, item: None }
    }
}

impl<I> Read for ConcatReader<I>
where
    I: Iterator<Item = Result<Input>>,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        loop {
            if self.item.is_none() {
                match self.iter.next() {
                    None => {
                        return Ok(0);
                    }
                    Some(result) => {
                        self.item = Some(result?);
                    }
                };
            }

            let input = self.item.as_mut().unwrap();
            let n = input.stream.read(buf).map_err(|e| {
                Error::new(e.kind(), format!("failed to read {}: {}", input.name, e))
            })?;
            if n != 0 {
                return Ok(n);
            }
            self.item = None;
        }
    }
}
