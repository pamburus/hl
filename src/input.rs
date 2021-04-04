// std imports
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

// third-part imports
use ansi_term::Colour;
use async_compression::futures::bufread::GzipDecoder;
use async_std::{
    fs::File,
    io::{self, empty, BufReader, Error, Read, ReadExt, Result},
    stream::Stream,
};
use futures_core::ready;
use pin_project_lite::pin_project;

// ---

pub type InputStream = Box<dyn Read + Send + Sync + Unpin>;

// ---

pub async fn open(path: &PathBuf) -> Result<Input> {
    let name = format!("file '{}'", Colour::Yellow.paint(path.to_string_lossy()),);

    let f = File::open(path)
        .await
        .map_err(|e| Error::new(e.kind(), format!("failed to open {}: {}", name, e)))?;

    let stream: InputStream = match path.extension().map(|x| x.to_str()) {
        Some(Some("gz")) => Box::new(GzipDecoder::new(BufReader::new(f))),
        _ => Box::new(f),
    };

    Ok(Input::new(name, stream))
}

// ---

pin_project! {
    pub struct Input {
        pub name: String,
        #[pin]
        pub stream: InputStream,
    }
}

impl Input {
    pub fn new(name: String, stream: InputStream) -> Self {
        Self { name, stream }
    }
}

// ---

pin_project! {
    pub struct ConcatReader<I> {
        iter: I,
        #[pin]
        item: Option<Input>,
    }
}

impl<I> ConcatReader<I> {
    pub fn new(iter: I) -> Self {
        Self { iter, item: None }
    }
}

impl<I> Read for ConcatReader<I>
where
    I: Iterator<Item = Result<Input>>,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        let mut this = self.project();
        loop {
            if this.item.is_none() {
                match this.iter.next() {
                    None => {
                        return Poll::Ready(Ok(0));
                    }
                    Some(result) => {
                        *this.item = Some(result?);
                    }
                };
            };
            let mut item = this.item.as_mut().as_pin_mut().unwrap();
            let n = ready!(Pin::new(item.stream.as_mut()).poll_read(cx, buf)).map_err(|e| {
                Error::new(e.kind(), format!("failed to read {}: {}", item.name, e))
            })?;
            if n != 0 {
                return Poll::Ready(Ok(n));
            }
            *this.item = None;
        }
    }

    /*
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
    */
}
