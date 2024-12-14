// stdlib imports
use std::{
    fs,
    io::{self, Read, Seek, Write},
    path::PathBuf,
};

// third-party imports
#[cfg(test)]
use mockall::mock;

// ---

pub trait FileSystem {
    type Metadata;

    fn canonicalize(&self, path: &PathBuf) -> io::Result<PathBuf>;
    fn metadata(&self, path: &PathBuf) -> io::Result<Self::Metadata>;
    fn exists(&self, path: &PathBuf) -> io::Result<bool>;
    fn open(&self, path: &PathBuf) -> io::Result<Box<dyn ReadOnlyFile + Send + Sync>>;
    fn create(&self, path: &PathBuf) -> io::Result<Box<dyn File + Send + Sync>>;
}

#[cfg(test)]
mock! {
    pub FileSystem<M> {}

    impl<M> FileSystem for FileSystem<M> {
        type Metadata = M;

        fn canonicalize(&self, path: &PathBuf) -> io::Result<PathBuf>;
        fn metadata(&self, path: &PathBuf) -> io::Result<M>;
        fn exists(&self, path: &PathBuf) -> io::Result<bool>;
        fn open(&self, path: &PathBuf) -> io::Result<Box<dyn ReadOnlyFile + Send + Sync>>;
        fn create(&self, path: &PathBuf) -> io::Result<Box<dyn File + Send + Sync>>;
    }
}

// ---

pub trait Meta {
    fn metadata(&self) -> io::Result<fs::Metadata>;
}

impl Meta for fs::File {
    fn metadata(&self) -> io::Result<fs::Metadata> {
        self.metadata()
    }
}

// ---

pub trait ReadOnlyFile: Read + Seek + Meta {}

impl<T: Read + Seek + Meta> ReadOnlyFile for T {}

#[cfg(test)]
mock! {
    pub ReadOnlyFile {}

    impl Read for ReadOnlyFile {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    }

    impl Seek for ReadOnlyFile {
        fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64>;
    }

    impl Meta for ReadOnlyFile {
        fn metadata(&self) -> io::Result<fs::Metadata>;
    }
}

// ---

pub trait File: ReadOnlyFile + Write {}

impl<T: ReadOnlyFile + Write> File for T {}

#[cfg(test)]
mock! {
    pub File {}

    impl Read for File {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    }

    impl Seek for File {
        fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64>;
    }

    impl Meta for File {
        fn metadata(&self) -> io::Result<fs::Metadata>;
    }

    impl Write for File {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize>;
        fn flush(&mut self) -> io::Result<()>;
    }
}

// ---

#[derive(Default)]
pub struct RealFileSystem;

impl FileSystem for RealFileSystem {
    type Metadata = fs::Metadata;

    #[inline]
    fn canonicalize(&self, path: &PathBuf) -> io::Result<PathBuf> {
        fs::canonicalize(path)
    }

    #[inline]
    fn metadata(&self, path: &PathBuf) -> io::Result<fs::Metadata> {
        fs::metadata(path)
    }

    #[inline]
    fn exists(&self, path: &PathBuf) -> io::Result<bool> {
        fs::exists(path)
    }

    #[inline]
    fn open(&self, path: &PathBuf) -> io::Result<Box<dyn ReadOnlyFile + Send + Sync>> {
        Ok(Box::new(fs::File::open(path)?))
    }

    #[inline]
    fn create(&self, path: &PathBuf) -> io::Result<Box<dyn File + Send + Sync>> {
        Ok(Box::new(fs::File::create(path)?))
    }
}
