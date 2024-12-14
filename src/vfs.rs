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
    type Metadata: 'static;

    fn canonicalize(&self, path: &PathBuf) -> io::Result<PathBuf>;
    fn metadata(&self, path: &PathBuf) -> io::Result<Self::Metadata>;
    fn exists(&self, path: &PathBuf) -> io::Result<bool>;
    fn open(&self, path: &PathBuf) -> io::Result<Box<dyn ReadOnlyFile<Metadata = Self::Metadata> + Send + Sync>>;
    fn create(&self, path: &PathBuf) -> io::Result<Box<dyn File<Metadata = Self::Metadata> + Send + Sync>>;
}

#[cfg(test)]
mock! {
    pub FileSystem<M> {}

    impl<M:'static> FileSystem for FileSystem<M> {
        type Metadata = M;

        fn canonicalize(&self, path: &PathBuf) -> io::Result<PathBuf>;
        fn metadata(&self, path: &PathBuf) -> io::Result<M>;
        fn exists(&self, path: &PathBuf) -> io::Result<bool>;
        fn open(&self, path: &PathBuf) -> io::Result<Box<dyn ReadOnlyFile<Metadata=M> + Send + Sync>>;
        fn create(&self, path: &PathBuf) -> io::Result<Box<dyn File<Metadata=M> + Send + Sync>>;
    }
}

// ---

pub trait Meta {
    type Metadata;

    fn metadata(&self) -> io::Result<Self::Metadata>;
}

impl Meta for fs::File {
    type Metadata = fs::Metadata;

    fn metadata(&self) -> io::Result<Self::Metadata> {
        self.metadata()
    }
}

// ---

pub trait ReadOnlyFile: Read + Seek + Meta {}

impl<T: Read + Seek + Meta> ReadOnlyFile for T {}

#[cfg(test)]
mock! {
    pub ReadOnlyFile<M> {}

    impl<M> Read for ReadOnlyFile<M> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    }

    impl<M> Seek for ReadOnlyFile<M> {
        fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64>;
    }

    impl<M> Meta for ReadOnlyFile<M> {
        type Metadata = M;

        fn metadata(&self) -> io::Result<M>;
    }
}

// ---

pub trait File: ReadOnlyFile + Write {}

impl<T: ReadOnlyFile + Write> File for T {}

#[cfg(test)]
mock! {
    pub File<M> {}

    impl<M> Read for File<M> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    }

    impl<M> Seek for File<M> {
        fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64>;
    }

    impl<M> Meta for File<M> {
        type Metadata = M;

        fn metadata(&self) -> io::Result<M>;
    }

    impl<M> Write for File<M> {
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
    fn open(&self, path: &PathBuf) -> io::Result<Box<dyn ReadOnlyFile<Metadata = Self::Metadata> + Send + Sync>> {
        Ok(Box::new(fs::File::open(path)?))
    }

    #[inline]
    fn create(&self, path: &PathBuf) -> io::Result<Box<dyn File<Metadata = Self::Metadata> + Send + Sync>> {
        Ok(Box::new(fs::File::create(path)?))
    }
}
