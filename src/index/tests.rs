use schema::{FLAG_LEVEL_ERROR, FLAG_LEVEL_INFO, FLAG_LEVEL_TRACE};

use super::*;

use std::{path::Component, time::Duration};

use crate::vfs::{self, MockFileSystem};
use assert_matches::assert_matches;

#[test]
fn test_process_file_success() {
    use io::Cursor;
    let indexer = Indexer::new(
        1,
        PathBuf::from("/tmp/cache"),
        IndexerSettings {
            buffer_size: nonzero!(1024u32).into(),
            max_message_size: nonzero!(1024u32).into(),
            ..IndexerSettings::with_fs(MockFileSystem::<MockSourceMetadata>::new())
        },
    );
    let data = concat!(
        "ts=2023-12-04T10:01:07.091243+01:00 msg=msg1\n",
        "ts=2023-12-04T10:01:07.091252+01:00 msg=msg2\n",
        "ts=2023-12-04T10:01:07.091633+01:00 msg=msg3\n",
    );
    let mut input = Cursor::new(data);
    let mut output = Cursor::new(Vec::new());
    let index = indexer
        .process_file(
            &PathBuf::from("/tmp/test.log"),
            &Metadata {
                len: data.len() as u64,
                modified: (1714739340, 0),
            },
            &mut input,
            &mut output,
            None,
        )
        .unwrap();
    assert_ne!(output.into_inner().len(), 0);
    assert_eq!(index.source.size, data.len() as u64);
    assert_eq!(index.source.path, "/tmp/test.log");
    assert_eq!(index.source.modified, (1714739340, 0));
    assert_eq!(index.source.stat.entries_valid, 3);
    assert_eq!(index.source.stat.entries_invalid, 0);
    assert_eq!(index.source.stat.flags, schema::FLAG_HAS_TIMESTAMPS);
    assert_eq!(
        index.source.stat.ts_min_max,
        Some((
            Timestamp::from((1701680467, 91243000)),
            Timestamp::from((1701680467, 91633000))
        ))
    );
    assert_eq!(index.source.blocks.len(), 1);
    assert_eq!(index.source.blocks[0].stat.entries_valid, 3);
    assert_eq!(index.source.blocks[0].stat.entries_invalid, 0);
    assert_eq!(index.source.blocks[0].stat.flags, schema::FLAG_HAS_TIMESTAMPS);
    assert_eq!(
        index.source.blocks[0].stat.ts_min_max,
        Some((
            Timestamp::from((1701680467, 91243000)),
            Timestamp::from((1701680467, 91633000))
        ))
    );
}

#[test]
fn test_process_file_error() {
    use io::Cursor;
    let fs = MockFileSystem::<MockSourceMetadata>::new();
    let indexer = Indexer::new(
        1,
        PathBuf::from("/tmp/cache"),
        IndexerSettings {
            buffer_size: nonzero!(1024u32).into(),
            max_message_size: nonzero!(1024u32).into(),
            ..IndexerSettings::with_fs(fs)
        },
    );
    let mut input = FailingReader;
    let mut output = Cursor::new(Vec::new());
    let result = indexer.process_file(
        &PathBuf::from("/tmp/test.log"),
        &Metadata {
            len: 135,
            modified: (1714739340, 0),
        },
        &mut input,
        &mut output,
        None,
    );
    assert!(result.is_err());
    assert_eq!(output.into_inner().len(), 0);
}

#[test]
fn test_indexer() {
    let fs = vfs::mem::FileSystem::new();

    let data = br#"ts=2024-01-02T03:04:05Z msg="some test message""#;
    let mut file = fs.create(&PathBuf::from("test.log")).unwrap();
    file.write_all(data).unwrap();

    let indexer = Indexer::new(1, PathBuf::from("/tmp/cache"), IndexerSettings::with_fs(fs));

    let index1 = indexer.index(&PathBuf::from("test.log")).unwrap();

    assert_eq!(index1.source.size, 47);
    assert_eq!(
        PathBuf::from(&index1.source.path).components().collect::<Vec<_>>(),
        vec![
            Component::RootDir,
            Component::Normal(std::ffi::OsStr::new("tmp")),
            Component::Normal(std::ffi::OsStr::new("test.log")),
        ],
    );
    assert_eq!(index1.source.stat.entries_valid, 1);
    assert_eq!(index1.source.stat.entries_invalid, 0);
    assert_eq!(index1.source.stat.flags, schema::FLAG_HAS_TIMESTAMPS);
    assert_eq!(index1.source.blocks.len(), 1);

    let index2 = indexer.index(&PathBuf::from("test.log")).unwrap();
    assert_eq!(index2.source.size, index1.source.size);
    assert_eq!(index2.source.modified, index1.source.modified);
}

#[test]
fn test_timestamp() {
    let ts = Timestamp::from((1701680467, 91243000));
    assert_eq!(ts.sec, 1701680467);
    assert_eq!(ts.nsec, 91243000);

    let ts = ts + Duration::from_secs(1);
    assert_eq!(ts.sec, 1701680468);
    assert_eq!(ts.nsec, 91243000);

    let ts = ts + Duration::from_nanos(1_000);
    assert_eq!(ts.sec, 1701680468);
    assert_eq!(ts.nsec, 91244000);

    let ts = ts + Duration::from_nanos(1_000_000_000);
    assert_eq!(ts.sec, 1701680469);
    assert_eq!(ts.nsec, 91244000);

    let ts = ts - Duration::from_secs(1);
    assert_eq!(ts.sec, 1701680468);
    assert_eq!(ts.nsec, 91244000);

    let ts = ts - Duration::from_nanos(1_000);
    assert_eq!(ts.sec, 1701680468);
    assert_eq!(ts.nsec, 91243000);

    let ts = ts - Duration::from_nanos(900_000_000);
    assert_eq!(ts.sec, 1701680467);
    assert_eq!(ts.nsec, 191243000);

    let ts2 = Timestamp::from((1701680467, 91243000));
    let diff = ts - ts2;
    assert_eq!(diff.as_nanos(), 100_000_000);

    let ts2 = Timestamp::from((1701680466, 991243000));
    let diff = ts - ts2;
    assert_eq!(diff.as_nanos(), 200_000_000);

    let ts = chrono::DateTime::<chrono::Utc>::from_timestamp_nanos(1_701_680_466_991_243_000);
    let ts = Timestamp::from(ts);
    assert_eq!(ts.sec, 1701680466);
    assert_eq!(ts.nsec, 991243000);
}

#[test]
fn test_buffer_size() {
    let bs = BufferSize(nonzero!(1024u32));
    assert_eq!(bs.get(), 1024);
    assert_eq!(NonZeroU32::from(bs), nonzero!(1024u32));
}

#[test]
fn test_message_size() {
    let ms = MessageSize(nonzero!(1024u32));
    assert_eq!(ms.get(), 1024);
    assert_eq!(NonZeroU32::from(ms), nonzero!(1024u32));
}

#[test]
fn test_source_block() {
    let block = SourceBlock::new(
        0,
        4096,
        Stat {
            flags: FLAG_LEVEL_TRACE | FLAG_LEVEL_INFO,
            entries_valid: 128,
            entries_invalid: 5,
            ts_min_max: Some((
                Timestamp::from((1701680250, 91243000)),
                Timestamp::from((1701680467, 91633000)),
            )),
        },
        Chronology::default(),
        None,
    );
    assert_eq!(block.offset, 0);
    assert_eq!(block.size, 4096);
    assert_eq!(block.stat.flags, FLAG_LEVEL_TRACE | FLAG_LEVEL_INFO);
    assert!(block.match_level(Level::Trace));
    assert!(block.match_level(Level::Debug));
    assert!(block.match_level(Level::Info));
    assert!(!block.match_level(Level::Warning));
    assert!(!block.match_level(Level::Error));

    let mut other = SourceBlock::new(
        4096,
        4096,
        Stat {
            flags: FLAG_LEVEL_INFO | FLAG_LEVEL_ERROR,
            entries_valid: 64,
            entries_invalid: 2,
            ts_min_max: Some((
                Timestamp::from((1701680467, 91242000)),
                Timestamp::from((1701680467, 91633000)),
            )),
        },
        Chronology::default(),
        None,
    );
    assert!(block.overlaps_by_time(&other));

    other.stat.ts_min_max = Some((
        Timestamp::from((1701680467, 191633000)),
        Timestamp::from((1701680468, 491633000)),
    ));
    assert!(!block.overlaps_by_time(&other));

    other.stat.ts_min_max = None;
    assert!(!block.overlaps_by_time(&other));
}

#[test]
fn test_indexer_settings_default() {
    // Test that Default implementation works (calls with_fs with FS::default())
    let _settings = IndexerSettings::<MockFileSystem<MockSourceMetadata>>::default();

    // Just verify it doesn't panic and creates a valid settings instance
    // The Default implementation should call with_fs(FS::default())
}

#[test]
fn test_indexer_with_json_delimiter() {
    use crate::scanning::Delimiter;

    let fs = vfs::mem::FileSystem::new();

    let data = br#"{
  "timestamp": "2024-01-01T00:00:02Z",
  "level": "error",
  "message": "third"
}
{
  "timestamp": "2024-01-01T00:00:00Z",
  "level": "info",
  "message": "first"
}
{
  "timestamp": "2024-01-01T00:00:01Z",
  "level": "warn",
  "message": "second"
}
"#;
    let mut file = fs.create(&PathBuf::from("test.log")).unwrap();
    file.write_all(data).unwrap();

    let indexer = Indexer::new(
        1,
        PathBuf::from("/tmp/cache"),
        IndexerSettings {
            buffer_size: nonzero!(4096u32).into(),
            delimiter: Delimiter::Json,
            ..IndexerSettings::with_fs(fs)
        },
    );

    let index = indexer.index(&PathBuf::from("test.log")).unwrap();

    eprintln!("Number of blocks: {}", index.source.blocks.len());
    for (i, block) in index.source.blocks.iter().enumerate() {
        eprintln!(
            "Block {}: offset={}, size={}, lines_valid={}, lines_invalid={}",
            i, block.offset, block.size, block.stat.entries_valid, block.stat.entries_invalid
        );
        eprintln!(
            "  chronology: bitmap.len={}, offsets.len={}, jumps={:?}",
            block.chronology.bitmap.len(),
            block.chronology.offsets.len(),
            block.chronology.jumps
        );
    }

    // Verify we found all 3 valid lines across all blocks
    assert_eq!(index.source.stat.entries_valid, 3);
    assert_eq!(index.source.stat.entries_invalid, 0);
}

// ---

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("read error"))
    }
}

#[test]
fn test_stat_default() {
    let stat1 = Stat::default();
    let stat2 = Stat::new();

    // Both should have the same initial state
    assert_eq!(stat1.flags, stat2.flags);
    assert_eq!(stat1.entries_valid, stat2.entries_valid);
    assert_eq!(stat1.entries_invalid, stat2.entries_invalid);
    assert_eq!(stat1.ts_min_max, stat2.ts_min_max);
}

#[test]
fn test_build_index_from_stream_file_creation_error() {
    use io::Cursor;

    // Create a mock filesystem that fails on file creation
    let mut fs = MockFileSystem::<MockSourceMetadata>::new();
    fs.expect_create()
        .returning(|_| Err(io::Error::new(io::ErrorKind::PermissionDenied, "Permission denied")));

    let indexer = Indexer::new(
        1,
        PathBuf::from("/tmp/cache"),
        IndexerSettings {
            buffer_size: nonzero!(1024u32).into(),
            max_message_size: nonzero!(1024u32).into(),
            ..IndexerSettings::with_fs(fs)
        },
    );

    let data = "ts=2023-12-04T10:01:07.091243+01:00 msg=test\n";
    let mut input = Cursor::new(data);
    let source_path = Path::new("/test/source.log");
    let index_path = Path::new("/test/source.log.idx");

    let mut mock_meta = MockSourceMetadata::new();
    mock_meta.expect_len().returning(|| 42);
    mock_meta.expect_modified().returning(|| Ok(UNIX_EPOCH));

    // Convert MockSourceMetadata to Metadata
    let metadata = Metadata::from(&mock_meta).unwrap();

    // This should trigger the FailedToOpenFileForWriting error
    let result = indexer.build_index_from_stream(&mut input, source_path, &metadata, index_path, None);

    assert_matches!(
        result,
        Err(Error::FailedToOpenFileForWriting { path, .. }) if path == index_path
    );
}

#[test]
fn test_source_metadata_is_empty() {
    // Test the default implementation of is_empty by using a concrete implementation
    use std::time::SystemTime;

    struct TestMetadata {
        len: u64,
    }

    impl SourceMetadata for TestMetadata {
        fn len(&self) -> u64 {
            self.len
        }

        fn modified(&self) -> io::Result<SystemTime> {
            Ok(SystemTime::UNIX_EPOCH)
        }
    }

    // Test when len() returns 0
    let meta_empty = TestMetadata { len: 0 };
    assert!(meta_empty.is_empty());
    // Also call modified() to cover that method
    assert!(meta_empty.modified().is_ok());

    // Test when len() returns non-zero
    let meta_nonempty = TestMetadata { len: 42 };
    assert!(!meta_nonempty.is_empty());
    // Also call modified() to cover that method
    assert!(meta_nonempty.modified().is_ok());
}
