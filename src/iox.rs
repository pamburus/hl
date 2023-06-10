// std imports
use std::io::{Read, Result};

// ---

pub trait ReadFill {
    fn read_fill(&mut self, buf: &mut [u8]) -> Result<usize>;
}

impl<T: Read + ?Sized> ReadFill for T {
    fn read_fill(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut i = 0;
        while i < buf.len() {
            let n = self.read(&mut buf[i..])?;
            if n == 0 {
                break;
            }
            i += n;
        }
        Ok(i)
    }
}
