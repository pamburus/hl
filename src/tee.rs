// std imports
use std::io::{Read, Result, Write};

// ---

pub struct TeeReader<R: Read, W: Write> {
    reader: R,
    writer: W,
    processed: usize,
}

impl<R: Read, W: Write> TeeReader<R, W> {
    #[inline]
    pub fn new(reader: R, writer: W) -> TeeReader<R, W> {
        TeeReader {
            reader,
            writer,
            processed: 0,
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub fn into_reader(self) -> R {
        self.reader
    }

    #[inline]
    #[allow(dead_code)]
    pub fn into_writer(self) -> W {
        self.writer
    }

    #[inline]
    #[allow(dead_code)]
    pub fn into(self) -> (R, W) {
        (self.reader, self.writer)
    }

    #[inline]
    pub fn processed(&self) -> usize {
        self.processed
    }
}

impl<R: Read, W: Write> Read for TeeReader<R, W> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = self.reader.read(buf)?;
        self.writer.write_all(&buf[..n])?;
        self.processed += n;
        Ok(n)
    }
}
