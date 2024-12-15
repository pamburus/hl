// stdlib imports
use std::{
    fs,
    io::{self, Read, Seek, Write},
    path::{Path, PathBuf},
};

// third-party imports
#[cfg(test)]
use mockall::mock;

// ---

pub trait FileSystem {
    type Metadata: 'static;

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;
    fn metadata(&self, path: &Path) -> io::Result<Self::Metadata>;
    fn exists(&self, path: &Path) -> io::Result<bool>;
    fn open(&self, path: &Path) -> io::Result<Box<dyn ReadOnlyFile<Metadata = Self::Metadata> + Send + Sync>>;
    fn create(&self, path: &Path) -> io::Result<Box<dyn File<Metadata = Self::Metadata> + Send + Sync>>;
}

#[cfg(test)]
mock! {
    pub FileSystem<M> {}

    impl<M:'static> FileSystem for FileSystem<M> {
        type Metadata = M;

        fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;
        fn metadata(&self, path: &Path) -> io::Result<M>;
        fn exists(&self, path: &Path) -> io::Result<bool>;
        fn open(&self, path: &Path) -> io::Result<Box<dyn ReadOnlyFile<Metadata=M> + Send + Sync>>;
        fn create(&self, path: &Path) -> io::Result<Box<dyn File<Metadata=M> + Send + Sync>>;
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
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        fs::canonicalize(path)
    }

    #[inline]
    fn metadata(&self, path: &Path) -> io::Result<fs::Metadata> {
        fs::metadata(path)
    }

    #[inline]
    fn exists(&self, path: &Path) -> io::Result<bool> {
        fs::exists(path)
    }

    #[inline]
    fn open(&self, path: &Path) -> io::Result<Box<dyn ReadOnlyFile<Metadata = Self::Metadata> + Send + Sync>> {
        Ok(Box::new(fs::File::open(path)?))
    }

    #[inline]
    fn create(&self, path: &Path) -> io::Result<Box<dyn File<Metadata = Self::Metadata> + Send + Sync>> {
        Ok(Box::new(fs::File::create(path)?))
    }
}

// ---

#[cfg(test)]
pub mod mem {
    use super::{Meta, ReadOnlyFile};
    use std::{
        collections::HashMap,
        io::{self, Read, Seek, Write},
        path::{Path, PathBuf},
        sync::{Arc, RwLock},
        time::SystemTime,
    };

    // ---

    #[derive(Copy, Clone)]
    #[allow(dead_code)]
    pub struct Metadata {
        pub len: usize,
        pub created: SystemTime,
        pub modified: SystemTime,
    }

    #[derive(Copy, Clone)]
    struct InternalMetadata {
        pub created: SystemTime,
        pub modified: SystemTime,
    }

    impl InternalMetadata {
        fn new() -> Self {
            let now = SystemTime::now();

            Self {
                created: now,
                modified: now,
            }
        }
    }

    impl From<(usize, InternalMetadata)> for Metadata {
        fn from(meta: (usize, InternalMetadata)) -> Self {
            Self {
                len: meta.0,
                created: meta.1.created,
                modified: meta.1.modified,
            }
        }
    }

    struct File {
        data: Vec<u8>,
        meta: InternalMetadata,
    }

    impl File {
        fn new(data: Vec<u8>) -> Self {
            Self {
                data,
                meta: InternalMetadata::new(),
            }
        }

        fn metadata(&self) -> Metadata {
            (self.data.len(), self.meta).into()
        }
    }

    // ---

    struct FileCursor {
        file: Arc<RwLock<File>>,
        pos: usize,
    }

    impl FileCursor {
        fn new(file: Arc<RwLock<File>>) -> Self {
            Self { file, pos: 0 }
        }
    }

    impl Read for FileCursor {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let data = &self.file.read().unwrap().data;
            let len = buf.len().min(data.len() - self.pos);
            buf[..len].copy_from_slice(&data[self.pos..self.pos + len]);
            self.pos += len;
            Ok(len)
        }
    }

    impl Write for FileCursor {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let file = &mut self.file.write().unwrap();
            let data = &mut file.data;
            if self.pos + buf.len() > data.len() {
                data.resize(self.pos + buf.len(), 0);
            }
            data[self.pos..self.pos + buf.len()].copy_from_slice(buf);
            file.meta.modified = SystemTime::now();
            self.pos += buf.len();
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Seek for FileCursor {
        fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
            let new_pos = match pos {
                io::SeekFrom::Start(offset) => offset as usize,
                io::SeekFrom::Current(offset) => (self.pos as i64 + offset) as usize,
                io::SeekFrom::End(offset) => (self.file.read().unwrap().data.len() as i64 + offset) as usize,
            };
            self.pos = new_pos;
            Ok(new_pos as u64)
        }
    }

    impl Meta for FileCursor {
        type Metadata = Metadata;

        fn metadata(&self) -> io::Result<Self::Metadata> {
            Ok(self.file.read().unwrap().metadata())
        }
    }

    // ---

    #[derive(Default)]
    pub struct FileSystem {
        files: RwLock<HashMap<PathBuf, Arc<RwLock<File>>>>,
    }

    impl FileSystem {
        pub fn new() -> Self {
            FileSystem {
                files: RwLock::new(HashMap::new()),
            }
        }
    }

    impl super::FileSystem for FileSystem {
        type Metadata = Metadata;

        fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
            Ok(PathBuf::from("/tmp").join(path))
        }

        fn metadata(&self, path: &Path) -> io::Result<Self::Metadata> {
            let path = self.canonicalize(path)?;
            let files = self.files.read().unwrap();
            if let Some(file) = files.get(&path) {
                Ok(file.read().unwrap().metadata())
            } else {
                Err(io::Error::new(io::ErrorKind::NotFound, "file not found"))
            }
        }

        fn exists(&self, path: &Path) -> io::Result<bool> {
            let path = self.canonicalize(path)?;
            let files = self.files.read().unwrap();
            Ok(files.contains_key(&path))
        }

        fn open(&self, path: &Path) -> io::Result<Box<dyn ReadOnlyFile<Metadata = Self::Metadata> + Send + Sync>> {
            let path = self.canonicalize(path)?;
            let files = self.files.read().unwrap();
            if let Some(file) = files.get(&path) {
                Ok(Box::new(FileCursor::new(file.clone())))
            } else {
                Err(io::Error::new(io::ErrorKind::NotFound, "file not found"))
            }
        }

        fn create(&self, path: &Path) -> io::Result<Box<dyn super::File<Metadata = Self::Metadata> + Send + Sync>> {
            let path = self.canonicalize(path)?;
            let mut files = self.files.write().unwrap();
            if files.contains_key(&path) {
                return Err(io::Error::new(io::ErrorKind::AlreadyExists, "file already exists"));
            }
            let file = Arc::new(RwLock::new(File::new(Vec::new())));
            files.insert(path.clone(), file.clone());
            Ok(Box::new(FileCursor::new(file)))
        }
    }
}
