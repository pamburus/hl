use super::*;
use io::Read;

use assert_matches::assert_matches;
use itertools::Itertools;
use nonzero_ext::nonzero;

use crate::{
    index::IndexerSettings,
    vfs::{self, LocalFileSystem},
};

#[test]
fn test_input_reference() {
    let reference = InputReference::Stdin;
    assert_eq!(reference.description(), "<stdin>");
    assert_eq!(reference.path(), None);
    let input = reference.open().unwrap();
    assert_eq!(input.reference, reference);
    let reference = InputReference::File(InputPath::ephemeral(PathBuf::from("test.log")));
    assert_eq!(reference.description(), "file \u{1b}[33m\"test.log\"\u{1b}[0m");
    assert_eq!(reference.path(), Some(&PathBuf::from("test.log")));
}

#[test]
fn test_input_holder() {
    let reference = InputReference::File(InputPath::ephemeral(PathBuf::from("sample/test.log")));
    let holder = InputHolder::new(reference, None);
    let mut stream = holder.open().unwrap().stream;
    let mut buf = Vec::new();
    let n = stream.read_to_end(&mut buf).unwrap();
    assert!(matches!(stream, Stream::RandomAccess(_)));
    let stream = stream.as_sequential();
    let meta = stream.metadata().unwrap();
    assert!(meta.is_some());
    assert_matches!(n, 147 | 149);
    assert_eq!(buf.len(), n);
}

#[test]
fn test_input() {
    let input = Input::stdin().unwrap();
    assert!(matches!(input.stream, Stream::Sequential(_)));
    assert_eq!(input.reference.description(), "<stdin>");
    let input = Input::open(&PathBuf::from("sample/prometheus.log")).unwrap();
    assert!(matches!(input.stream, Stream::RandomAccess(_)));
    assert_eq!(
        input.reference.description(),
        "file \u{1b}[33m\"sample/prometheus.log\"\u{1b}[0m"
    );
}

#[test]
fn test_input_tail() {
    let input = Input::stdin().unwrap().tail(1, Delimiter::SmartNewLine).unwrap();
    assert!(matches!(input.stream, Stream::Sequential(_)));

    for &(filename, requested, expected) in &[
        ("sample/test.log", 1, 1),
        ("sample/test.log", 2, 2),
        ("sample/test.log", 3, 2),
        ("sample/prometheus.log", 2, 2),
    ] {
        let input = Input::open(&PathBuf::from(filename))
            .unwrap()
            .tail(requested, Delimiter::SmartNewLine)
            .unwrap();
        let mut buf = Vec::new();
        let n = input.stream.into_sequential().read_to_end(&mut buf).unwrap();
        assert!(n > 0);
        assert_eq!(buf.lines().count(), expected);
    }
}

#[test]
fn test_stream() {
    let stream = Stream::Sequential(Box::new(Cursor::new(b"test")));
    let stream = stream.verified().decoded().tagged(InputReference::Stdin);
    assert!(matches!(stream, Stream::Sequential(_)));
    let mut buf = Vec::new();
    let n = stream.into_sequential().read_to_end(&mut buf).unwrap();
    assert_eq!(n, 4);
    assert_eq!(buf, b"test");

    let stream = Stream::RandomAccess(Box::new(UnseekableReader(Cursor::new(b"test"))));
    let stream = stream.tagged(InputReference::Stdin).verified();
    assert!(matches!(stream, Stream::Sequential(_)));
    let mut buf = Vec::new();
    let n = stream.into_sequential().read_to_end(&mut buf).unwrap();
    assert_eq!(n, 4);
    assert_eq!(buf, b"test");

    let stream = Stream::RandomAccess(Box::new(UnseekableReader(Cursor::new(b"test"))));
    assert!(matches!(stream.metadata(), Ok(None)));
    let stream = stream.tagged(InputReference::Stdin).decoded();
    assert!(matches!(stream, Stream::Sequential(_)));
    assert!(matches!(stream.metadata(), Ok(None)));
    let mut buf = Vec::new();
    let n = stream.into_sequential().read_to_end(&mut buf).unwrap();
    assert_eq!(n, 4);
    assert_eq!(buf, b"test");

    // echo 't' | gzip -cf | xxd -p | sed 's/\(..\)/\\x\1/g'
    let data = b"\x1f\x8b\x08\x00\x03\x87\x55\x67\x00\x03\x2b\xe1\x02\x00\x13\x47\x5f\xea\x02\x00\x00\x00";
    let stream = Stream::RandomAccess(Box::new(Cursor::new(data).with_metadata(None)));
    let stream = stream.tagged(InputReference::Stdin).decoded();
    assert!(matches!(stream, Stream::Sequential(_)));
    let mut buf = Vec::new();
    let n = stream.into_sequential().read_to_end(&mut buf).unwrap();
    assert_eq!(n, 2);
    assert_eq!(buf, b"t\n");

    let stream = Stream::RandomAccess(Box::new(FailingReader));
    let stream = stream.tagged(InputReference::Stdin).decoded();
    assert!(matches!(stream, Stream::Sequential(_)));
    let mut buf = [0; 128];
    let result = stream.into_sequential().read(&mut buf);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::Other);
}

#[test]
fn test_input_read_error() {
    let reference = InputReference::File(InputPath::ephemeral(PathBuf::from("test.log")));
    let input = Input::new(reference, Stream::Sequential(Box::new(FailingReader)));
    let mut buf = [0; 128];
    let result = input.stream.into_sequential().read(&mut buf);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::Other);
    assert!(err.to_string().contains("test.log"));
}

#[test]
fn test_input_hold_error_is_dir() {
    let reference = InputReference::File(InputPath::ephemeral(PathBuf::from(".")));
    let result = reference.hold();
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    assert!(err.to_string().contains("is a directory"));
}

#[test]
fn test_input_hold_error_not_found() {
    let filename = "AKBNIJGHERHBNMCKJABHSDJ";
    let reference = InputReference::File(InputPath::ephemeral(PathBuf::from(filename)));
    let result = reference.hold();
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert_eq!(err.kind(), io::ErrorKind::NotFound);
    assert!(err.to_string().contains(filename));
}

#[test]
fn test_input_gzip() {
    use std::io::Cursor;
    let data = Cursor::new(
        // echo 'test' | gzip -cf | xxd -p | sed 's/\(..\)/\\x\1/g'
        b"\x1f\x8b\x08\x00\x9e\xdd\x48\x67\x00\x03\x2b\x49\x2d\x2e\xe1\x02\x00\xc6\x35\xb9\x3b\x05\x00\x00\x00",
    );
    let mut stream = Stream::Sequential(Box::new(data)).verified().decoded();
    let mut buf = Vec::new();
    let result = stream.read_to_end(&mut buf);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 5);
    assert_eq!(buf, b"test\n");
}

#[test]
fn test_indexed_input_stdin() {
    let data = br#"{"ts":"2024-10-01T01:02:03Z","level":"info","msg":"some test message"}\n"#;
    let stream = Stream::RandomAccess(Box::new(Cursor::new(data)));
    let indexer = Indexer::<LocalFileSystem>::new(1, PathBuf::new(), IndexerSettings::with_fs(LocalFileSystem));
    let input = IndexedInput::from_stream(InputReference::Stdin, stream, Delimiter::default(), &indexer).unwrap();
    let mut blocks = input.into_blocks().collect_vec();
    assert_eq!(blocks.len(), 1);
    let block = blocks.drain(..).next().unwrap();
    assert_eq!(block.entries_valid(), 1);
    let mut lines = block.into_entries().unwrap().collect_vec();
    let line = lines.drain(..).next().unwrap();
    assert_eq!(line.bytes(), data);
}

#[test]
fn test_indexed_input_file_random_access() {
    let fs = Arc::new(vfs::mem::FileSystem::new());

    for _ in 0..2 {
        let path = PathBuf::from("sample/test.log");
        let indexer = Indexer::new(
            1,
            PathBuf::from("."),
            IndexerSettings {
                buffer_size: nonzero!(64u32).into(),
                ..IndexerSettings::with_fs(fs.clone())
            },
        );
        let input = IndexedInput::open(&path, &indexer, Delimiter::SmartNewLine).unwrap();
        let mut blocks = input.into_blocks().sorted().collect_vec();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].entries_valid(), 1);
        assert_matches!(blocks[0].size(), 74 | 75);
        assert_eq!(blocks[1].entries_valid(), 1);
        assert_matches!(blocks[1].size(), 73 | 74);
        let lines = blocks.pop().unwrap().into_entries().unwrap().collect_vec();
        assert_eq!(lines.len(), 1);
        assert_matches!(lines[0].len(), 73 | 74);
        let lines = blocks.pop().unwrap().into_entries().unwrap().collect_vec();
        assert_eq!(lines.len(), 1);
        assert_matches!(lines[0].len(), 74 | 75);
    }
}

#[test]
fn test_indexed_input_sequential_access() {
    let fs = Arc::new(vfs::mem::FileSystem::new());

    for _ in 0..2 {
        let path = PathBuf::from("sample/test.log");
        let indexer = Indexer::new(
            1,
            PathBuf::from("."),
            IndexerSettings {
                buffer_size: nonzero!(64u32).into(),
                ..IndexerSettings::with_fs(fs.clone())
            },
        );
        let reference = InputReference::File(InputPath::resolve_with_fs(path.clone(), &fs).unwrap());
        let stream = Stream::Sequential(Box::new(File::open(&path).unwrap()));
        let input = IndexedInput::from_stream(reference, stream, Delimiter::default(), &indexer).unwrap();
        let mut blocks = input.into_blocks().sorted().collect_vec();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].entries_valid(), 1);
        assert_matches!(blocks[0].size(), 74 | 75);
        assert_eq!(blocks[1].entries_valid(), 1);
        assert_matches!(blocks[1].size(), 73 | 74);
        let lines = blocks.pop().unwrap().into_entries().unwrap().collect_vec();
        assert_eq!(lines.len(), 1);
        assert_matches!(lines[0].len(), 73 | 74);
        let lines = blocks.pop().unwrap().into_entries().unwrap().collect_vec();
        assert_eq!(lines.len(), 1);
        assert_matches!(lines[0].len(), 74 | 75);
    }
}

// ---

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("read error"))
    }
}

impl Seek for FailingReader {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        match from {
            SeekFrom::Start(0) => Ok(0),
            SeekFrom::Current(0) => Ok(0),
            SeekFrom::End(0) => Ok(0),
            _ => Err(io::Error::other("seek error")),
        }
    }
}

impl Meta for FailingReader {
    fn metadata(&self) -> std::io::Result<Option<Metadata>> {
        Ok(None)
    }
}

// ---

struct UnseekableReader<R>(R);

impl<R: Read> Read for UnseekableReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl<R> Seek for UnseekableReader<R> {
    fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
        Err(io::Error::other("seek error"))
    }
}

impl<R> Meta for UnseekableReader<R> {
    fn metadata(&self) -> io::Result<Option<Metadata>> {
        Ok(None)
    }
}

#[test]
fn test_failing_reader_seek_error() {
    use std::io::SeekFrom;

    let mut reader = FailingReader;

    // These should succeed (zero seeks)
    assert!(reader.seek(SeekFrom::Start(0)).is_ok());
    assert!(reader.stream_position().is_ok());
    assert!(reader.seek(SeekFrom::End(0)).is_ok());

    // This should fail (non-zero seek)
    let result = reader.seek(SeekFrom::Start(10));
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "seek error");
}

#[test]
fn test_input_tail_with_json_delimiter() {
    use std::io::Cursor;

    let data = b"{\"a\":1}\n{\"b\":2}\n{\"c\":3}\n{\"d\":4}\n{\"e\":5}";
    let stream = Stream::RandomAccess(Box::new(Cursor::new(data)));
    let reference = InputReference::Stdin;
    let input = Input::new(reference, stream);

    // Request last 2 entries with JSON delimiter
    let input = input.tail(2, Delimiter::Json).unwrap();
    let mut buf = Vec::new();
    input.stream.into_sequential().read_to_end(&mut buf).unwrap();

    // Should get last 2 JSON objects
    assert_eq!(buf, b"{\"d\":4}\n{\"e\":5}");
}

#[test]
fn test_input_tail_with_auto_delimiter() {
    use std::io::Cursor;

    // Multi-line JSON with continuation lines (closing braces)
    let data = b"{\"a\":1,\n  \"nested\":true\n}\n{\"b\":2\n}";
    let stream = Stream::RandomAccess(Box::new(Cursor::new(data)));
    let reference = InputReference::Stdin;
    let input = Input::new(reference, stream);

    // Request last 1 entry with Auto delimiter
    let input = input.tail(1, Delimiter::Auto).unwrap();
    let mut buf = Vec::new();
    input.stream.into_sequential().read_to_end(&mut buf).unwrap();

    // Should get last JSON object (multi-line entry)
    assert_eq!(buf, b"{\"b\":2\n}");
}

#[test]
fn test_input_tail_with_byte_delimiter() {
    use std::io::Cursor;

    let data = b"line1\nline2\nline3\nline4\nline5";
    let stream = Stream::RandomAccess(Box::new(Cursor::new(data)));
    let reference = InputReference::Stdin;
    let input = Input::new(reference, stream);

    // Request last 2 entries with LF delimiter
    let input = input.tail(2, Delimiter::Byte(b'\n')).unwrap();
    let mut buf = Vec::new();
    input.stream.into_sequential().read_to_end(&mut buf).unwrap();

    assert_eq!(buf, b"line4\nline5");
}

#[test]
fn test_input_tail_ending_with_delimiter() {
    use std::io::Cursor;

    let data = b"line1\nline2\nline3\n";
    let stream = Stream::RandomAccess(Box::new(Cursor::new(data)));
    let reference = InputReference::Stdin;
    let input = Input::new(reference, stream);

    // Request last 1 entry
    let input = input.tail(1, Delimiter::SmartNewLine).unwrap();
    let mut buf = Vec::new();
    input.stream.into_sequential().read_to_end(&mut buf).unwrap();

    // Should skip trailing delimiter and get last line
    assert_eq!(buf, b"line3\n");
}

#[test]
fn test_input_tail_json_with_spaces() {
    use std::io::Cursor;

    let data = b"{\"a\":1}\n  \n{\"b\":2}\n\t\n{\"c\":3}";
    let stream = Stream::RandomAccess(Box::new(Cursor::new(data)));
    let reference = InputReference::Stdin;
    let input = Input::new(reference, stream);

    // Request last 1 entry with JSON delimiter
    let input = input.tail(1, Delimiter::Json).unwrap();
    let mut buf = Vec::new();
    input.stream.into_sequential().read_to_end(&mut buf).unwrap();

    assert_eq!(buf, b"{\"c\":3}");
}

#[test]
fn test_input_tail_more_than_available() {
    use std::io::Cursor;

    let data = b"line1\nline2";
    let stream = Stream::RandomAccess(Box::new(Cursor::new(data)));
    let reference = InputReference::Stdin;
    let input = Input::new(reference, stream);

    // Request more entries than available
    let input = input.tail(10, Delimiter::SmartNewLine).unwrap();
    let mut buf = Vec::new();
    input.stream.into_sequential().read_to_end(&mut buf).unwrap();

    // Should get all data
    assert_eq!(buf, b"line1\nline2");
}

#[test]
fn test_input_tail_with_crlf() {
    use std::io::Cursor;

    let data = b"line1\r\nline2\r\nline3\r\n";
    let stream = Stream::RandomAccess(Box::new(Cursor::new(data)));
    let reference = InputReference::Stdin;
    let input = Input::new(reference, stream);

    // Request last 2 entries
    let input = input.tail(2, Delimiter::SmartNewLine).unwrap();
    let mut buf = Vec::new();
    input.stream.into_sequential().read_to_end(&mut buf).unwrap();

    assert_eq!(buf, b"line2\r\nline3\r\n");
}

#[test]
fn test_input_tail_partial_match_handling() {
    use std::io::Cursor;

    // Data that might have partial delimiter at buffer boundary
    let data = b"line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8";
    let stream = Stream::RandomAccess(Box::new(Cursor::new(data)));
    let reference = InputReference::Stdin;
    let input = Input::new(reference, stream);

    let input = input.tail(3, Delimiter::SmartNewLine).unwrap();
    let mut buf = Vec::new();
    input.stream.into_sequential().read_to_end(&mut buf).unwrap();

    assert_eq!(buf, b"line6\nline7\nline8");
}

#[test]
fn test_block_entry_methods() {
    use crate::scanning::Delimiter;

    let data = br#"{"ts":"2024-10-01T01:02:03Z","level":"info","msg":"message 1"}
{"ts":"2024-10-01T01:02:04Z","level":"warn","msg":"message 2"}"#;
    let stream = Stream::RandomAccess(Box::new(Cursor::new(data)));
    let indexer = Indexer::<LocalFileSystem>::new(1, PathBuf::new(), IndexerSettings::with_fs(LocalFileSystem));
    let input = IndexedInput::from_stream(InputReference::Stdin, stream, Delimiter::SmartNewLine, &indexer).unwrap();

    let blocks = input.into_blocks().collect_vec();
    assert!(!blocks.is_empty());

    // Test entries_valid method - should have at least one valid entry across all blocks
    let total_valid: u64 = blocks.iter().map(|b| b.entries_valid()).sum();
    assert!(total_valid > 0);
}

#[test]
fn test_indexed_input_with_json_delimiter() {
    use std::io::Cursor;

    let data = b"{\"a\":1}\n{\"b\":2}\n{\"c\":3}";
    let stream = Stream::RandomAccess(Box::new(Cursor::new(data)));
    let indexer = Indexer::<LocalFileSystem>::new(1, PathBuf::new(), IndexerSettings::with_fs(LocalFileSystem));
    let input = IndexedInput::from_stream(InputReference::Stdin, stream, Delimiter::Json, &indexer).unwrap();

    let blocks = input.into_blocks().collect_vec();
    assert!(!blocks.is_empty());

    // Should successfully parse with JSON delimiter - verify we can iterate entries
    let total_entries: usize = blocks
        .into_iter()
        .map(|block| block.into_entries().unwrap().count())
        .sum();
    assert!(total_entries > 0);
}

#[test]
fn test_indexed_input_with_auto_delimiter() {
    use std::io::Cursor;

    // Multi-line entries with continuation characters
    let data = b"line1\n  continued\nline2\n}also continued\nline3";
    let stream = Stream::RandomAccess(Box::new(Cursor::new(data)));
    let indexer = Indexer::<LocalFileSystem>::new(1, PathBuf::new(), IndexerSettings::with_fs(LocalFileSystem));
    let input = IndexedInput::from_stream(InputReference::Stdin, stream, Delimiter::Auto, &indexer).unwrap();

    let blocks = input.into_blocks().collect_vec();
    assert!(!blocks.is_empty());

    // Should successfully parse with Auto delimiter - verify we can iterate entries
    let total_entries: usize = blocks
        .into_iter()
        .map(|block| block.into_entries().unwrap().count())
        .sum();
    assert!(total_entries > 0);
}

#[test]
fn test_block_entry_empty() {
    use std::io::Cursor;

    let data = b"\n\n";
    let stream = Stream::RandomAccess(Box::new(Cursor::new(data)));
    let indexer = Indexer::<LocalFileSystem>::new(1, PathBuf::new(), IndexerSettings::with_fs(LocalFileSystem));
    let input = IndexedInput::from_stream(InputReference::Stdin, stream, Delimiter::SmartNewLine, &indexer).unwrap();

    let blocks = input.into_blocks().collect_vec();
    assert!(!blocks.is_empty());

    // Should be able to iterate entries even for empty/invalid input - verify it doesn't panic
    for block in blocks {
        let _entries = block.into_entries().unwrap().collect_vec();
    }
}
