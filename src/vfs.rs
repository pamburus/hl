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
    fn open(&self, path: &Path) -> io::Result<Box<dyn FileRead<Metadata = Self::Metadata> + Send + Sync>>;
    fn create(&self, path: &Path) -> io::Result<Box<dyn FileReadWrite<Metadata = Self::Metadata> + Send + Sync>>;
}

#[cfg(test)]
mock! {
    pub FileSystem<M> {}

    impl<M:'static> FileSystem for FileSystem<M> {
        type Metadata = M;

        fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;
        fn metadata(&self, path: &Path) -> io::Result<M>;
        fn exists(&self, path: &Path) -> io::Result<bool>;
        fn open(&self, path: &Path) -> io::Result<Box<dyn FileRead<Metadata=M> + Send + Sync>>;
        fn create(&self, path: &Path) -> io::Result<Box<dyn FileReadWrite<Metadata=M> + Send + Sync>>;
    }
}

macro_rules! delegate_fs_methods {
    () => {
        #[inline]
        fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
            (**self).canonicalize(path)
        }

        #[inline]
        fn metadata(&self, path: &Path) -> io::Result<Self::Metadata> {
            (**self).metadata(path)
        }

        #[inline]
        fn exists(&self, path: &Path) -> io::Result<bool> {
            (**self).exists(path)
        }

        #[inline]
        fn open(&self, path: &Path) -> io::Result<Box<dyn FileRead<Metadata = Self::Metadata> + Send + Sync>> {
            (**self).open(path)
        }

        #[inline]
        fn create(&self, path: &Path) -> io::Result<Box<dyn FileReadWrite<Metadata = Self::Metadata> + Send + Sync>> {
            (**self).create(path)
        }
    };
}

impl<T> FileSystem for &T
where
    T: FileSystem,
{
    type Metadata = T::Metadata;
    delegate_fs_methods!();
}

impl<T> FileSystem for std::sync::Arc<T>
where
    T: FileSystem,
{
    type Metadata = T::Metadata;
    delegate_fs_methods!();
}

// ---

pub trait Meta {
    type Metadata;

    fn metadata(&self) -> io::Result<Self::Metadata>;
}

impl Meta for fs::File {
    type Metadata = fs::Metadata;

    #[inline]
    fn metadata(&self) -> io::Result<Self::Metadata> {
        self.metadata()
    }
}

// ---

pub trait FileRead: Read + Seek + Meta {}

impl<T: Read + Seek + Meta> FileRead for T {}

#[cfg(test)]
mock! {
    pub FileRead<M> {}

    impl<M> Read for FileRead<M> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    }

    impl<M> Seek for FileRead<M> {
        fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64>;
    }

    impl<M> Meta for FileRead<M> {
        type Metadata = M;

        fn metadata(&self) -> io::Result<M>;
    }
}

// ---

pub trait FileReadWrite: FileRead + Write {}

impl<T: FileRead + Write> FileReadWrite for T {}

#[cfg(test)]
mock! {
    pub FileReadWrite<M> {}

    impl<M> Read for FileReadWrite<M> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    }

    impl<M> Seek for FileReadWrite<M> {
        fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64>;
    }

    impl<M> Meta for FileReadWrite<M> {
        type Metadata = M;

        fn metadata(&self) -> io::Result<M>;
    }

    impl<M> Write for FileReadWrite<M> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize>;
        fn flush(&mut self) -> io::Result<()>;
    }
}

// ---

#[derive(Default)]
pub struct LocalFileSystem;

impl FileSystem for LocalFileSystem {
    type Metadata = fs::Metadata;

    #[inline]
    #[cfg(not(target_os = "linux"))]
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        fs::canonicalize(path)
    }

    #[inline]
    #[cfg(target_os = "linux")]
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        use std::os::unix::fs::FileTypeExt;
        let meta = fs::metadata(path)?;
        if meta.file_type().is_fifo()
            || meta.file_type().is_socket()
            || meta.file_type().is_block_device()
            || meta.file_type().is_char_device()
        {
            return std::path::absolute(path);
        }
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
    fn open(&self, path: &Path) -> io::Result<Box<dyn FileRead<Metadata = Self::Metadata> + Send + Sync>> {
        Ok(Box::new(fs::File::open(path)?))
    }

    #[inline]
    fn create(&self, path: &Path) -> io::Result<Box<dyn FileReadWrite<Metadata = Self::Metadata> + Send + Sync>> {
        Ok(Box::new(fs::File::create(path)?))
    }
}

// ---

#[cfg(test)]
pub mod mem {
    use super::{FileRead, Meta};

    use std::{
        collections::HashMap,
        io::{self, Read, Seek, Write},
        path::{Path, PathBuf},
        sync::{Arc, RwLock},
        time::SystemTime,
    };

    use clean_path::Clean;

    // ---

    #[derive(Copy, Clone, Debug)]
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
        #[inline]
        fn new() -> Self {
            let now = SystemTime::now();

            Self {
                created: now,
                modified: now,
            }
        }
    }

    impl From<(usize, InternalMetadata)> for Metadata {
        #[inline]
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
        #[inline]
        fn new(data: Vec<u8>) -> Self {
            Self {
                data,
                meta: InternalMetadata::new(),
            }
        }

        #[inline]
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
        #[inline]
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

        #[inline]
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

        #[inline]
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
            Ok(PathBuf::from("/tmp").join(path).clean())
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

        fn open(&self, path: &Path) -> io::Result<Box<dyn FileRead<Metadata = Self::Metadata> + Send + Sync>> {
            let path = self.canonicalize(path)?;
            let files = self.files.read().unwrap();
            if let Some(file) = files.get(&path) {
                Ok(Box::new(FileCursor::new(file.clone())))
            } else {
                Err(io::Error::new(io::ErrorKind::NotFound, "file not found"))
            }
        }

        fn create(
            &self,
            path: &Path,
        ) -> io::Result<Box<dyn super::FileReadWrite<Metadata = Self::Metadata> + Send + Sync>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem() {
        let fs = mem::FileSystem::new();
        let path = Path::new("file.txt");

        assert!(!fs.exists(path).unwrap());

        let mut file = fs.create(path).unwrap();
        file.write_all(b"hello world").unwrap();
        file.flush().unwrap();

        let res = fs.create(path);
        assert!(res.is_err());
        assert_eq!(res.err().map(|e| e.kind()), Some(io::ErrorKind::AlreadyExists));

        assert!(fs.exists(path).unwrap());

        let meta = fs.metadata(path).unwrap();
        assert_eq!(meta.len, 11);

        let res = fs.metadata(Path::new("nonexistent.txt"));
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err().kind(), io::ErrorKind::NotFound));

        let canonical_path = fs.canonicalize(path).unwrap();
        assert_eq!(canonical_path, PathBuf::from("/tmp/file.txt"));

        let mut file = fs.open(path).unwrap();
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();

        assert_eq!(buf, b"hello world");

        let meta = file.metadata().unwrap();
        assert_eq!(meta.len, 11);

        let res = fs.open(Path::new("nonexistent.txt"));
        assert!(res.is_err());
        assert_eq!(res.err().map(|e| e.kind()), Some(io::ErrorKind::NotFound));
    }

    #[test]
    fn test_filesystem_reference() {
        let fs = mem::FileSystem::new();
        let fs_ref = &fs;
        let path = Path::new("ref_test.txt");

        // Test all methods through reference
        assert!(!fs_ref.exists(path).unwrap());

        let mut file = fs_ref.create(path).unwrap();
        file.write_all(b"reference test").unwrap();
        file.flush().unwrap();

        assert!(fs_ref.exists(path).unwrap());

        let meta = fs_ref.metadata(path).unwrap();
        assert_eq!(meta.len, 14);

        let canonical_path = fs_ref.canonicalize(path).unwrap();
        assert_eq!(canonical_path, PathBuf::from("/tmp/ref_test.txt"));

        let mut file = fs_ref.open(path).unwrap();
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, b"reference test");
    }

    #[test]
    fn test_filesystem_arc() {
        let fs = std::sync::Arc::new(mem::FileSystem::new());
        let path = Path::new("arc_test.txt");

        // Test all methods through Arc
        assert!(!fs.exists(path).unwrap());

        let mut file = fs.create(path).unwrap();
        file.write_all(b"arc test").unwrap();
        file.flush().unwrap();

        assert!(fs.exists(path).unwrap());

        let meta = fs.metadata(path).unwrap();
        assert_eq!(meta.len, 8);

        let canonical_path = fs.canonicalize(path).unwrap();
        assert_eq!(canonical_path, PathBuf::from("/tmp/arc_test.txt"));

        let mut file = fs.open(path).unwrap();
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, b"arc test");
    }
}
