use std::fs::File;
use std::io::{Error, Read, Result};
use std::path::PathBuf;

use ansi_term::Colour;

pub struct ConcatReader {
    files: Vec<PathBuf>,
    i: usize,
    f: Option<File>,
}

impl ConcatReader {
    pub fn new(files: Vec<PathBuf>) -> Self {
        Self {
            files,
            i: 0,
            f: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.files.len() == 0
    }
}

impl Read for ConcatReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        loop {
            if self.i >= self.files.len() {
                return Ok(0);
            }

            let filename = &self.files[self.i];
            if self.f.is_none() {
                self.f = Some(File::open(&filename).map_err(|e| {
                    Error::new(
                        e.kind(),
                        format!(
                            "failed to open file '{}': {}",
                            Colour::Yellow.paint(filename.to_string_lossy()),
                            e
                        ),
                    )
                })?);
            }

            let mut f = self.f.as_ref().unwrap();
            let n = f.read(buf).map_err(|e| {
                Error::new(
                    e.kind(),
                    format!(
                        "failed to read file '{}': {}",
                        Colour::Yellow.paint(filename.to_string_lossy()),
                        e
                    ),
                )
            })?;
            if n != 0 {
                return Ok(n);
            }
            self.f = None;
            self.i += 1;
        }
    }
}
