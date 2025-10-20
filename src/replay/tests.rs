use super::*;
use core::str;
use nonzero_ext::nonzero;
use std::{io::Cursor, num::NonZero};

fn dual(b: &[u8]) -> (&str, &[u8]) {
    (str::from_utf8(b).unwrap(), b)
}

#[test]
fn test_replay_buf() {
    let mut w = ReplayBufCreator::build().segment_size(nonzero!(4_usize)).done();
    w.write_all(b"Lorem ipsum dolor sit amet.").unwrap();
    w.flush().unwrap();
    w.flush().unwrap();
    let r = ReplayBuf::try_from(w).unwrap();

    assert_eq!(r.segment_size().get(), 4);
    assert_eq!(r.size(), 27);
    assert_eq!(r.segments().len(), 7);

    let mut buf = vec![0; 27];
    let n = r.segments()[0].decode(&mut buf).unwrap();
    buf.truncate(n);
    assert_eq!(n, 4);
    assert_eq!(dual(&buf), dual(b"Lore"));
}

#[test]
fn test_replay_buf_reader() {
    let data = b"Lorem ipsum dolor sit amet.";
    let mut creator = ReplayBufCreator::new();
    creator.write_all(data).unwrap();
    let buf = ReplayBuf::try_from(creator).unwrap();
    let mut r = ReplayBufReader::build(buf)
        .cache(MinimalCache::default())
        .position(6)
        .done();

    let pos = r.stream_position().unwrap();
    assert_eq!(pos, 6);
    let mut buf = vec![0; 11];
    r.read_exact(&mut buf).unwrap();
    assert_eq!(dual(&buf), dual(b"ipsum dolor"));

    let pos = r.seek(SeekFrom::End(-9)).unwrap();
    assert_eq!(pos, 18);
    let mut buf = vec![];
    r.read_to_end(&mut buf).unwrap();
    assert_eq!(dual(&buf), dual(b"sit amet."));
}

fn test_rewinding_reader<F: FnOnce(usize, &str) -> Box<dyn ReadSeek>>(f: F) {
    let mut r = f(4, "Lorem ipsum dolor sit amet.");

    let mut buf3 = vec![0; 3];
    assert_eq!(r.read(&mut buf3).unwrap(), 3);
    assert_eq!(dual(&buf3), dual("Lor".as_bytes()));

    let mut buf4 = vec![0; 4];
    assert_eq!(r.read(&mut buf4).unwrap(), 4);
    assert_eq!(dual(&buf4), dual("em i".as_bytes()));

    let mut buf6 = vec![0; 6];
    assert_eq!(r.read(&mut buf6).unwrap(), 6);
    assert_eq!(dual(&buf6), dual("psum d".as_bytes()));

    assert_eq!(r.seek(SeekFrom::Start(1)).unwrap(), 1);

    assert_eq!(r.read(&mut buf4).unwrap(), 4);
    assert_eq!(dual(&buf4), dual("orem".as_bytes()));

    assert_eq!(r.seek(SeekFrom::Current(7)).unwrap(), 12);

    let mut buf5 = vec![0; 5];
    assert_eq!(r.read(&mut buf5).unwrap(), 5);
    assert_eq!(dual(&buf5), dual("dolor".as_bytes()));

    assert_eq!(r.seek(SeekFrom::End(-5)).unwrap(), 22);

    assert_eq!(r.read(&mut buf4).unwrap(), 4);
    assert_eq!(dual(&buf4), dual("amet".as_bytes()));

    assert_eq!(r.read(&mut buf3).unwrap(), 1);
    assert_eq!(dual(&buf3[..1]), dual(".".as_bytes()));

    assert_eq!(r.read(&mut buf3).unwrap(), 0);
}

#[test]
fn test_rewinding_reader_default() {
    test_rewinding_reader(|block_size, data| {
        let data = data.as_bytes().to_vec();
        Box::new(
            RewindingReader::build(move || Ok(Cursor::new(data.clone())))
                .block_size(block_size.try_into().unwrap())
                .done()
                .unwrap(),
        )
    });
}

#[test]
fn test_rewinding_reader_new() {
    test_rewinding_reader(|block_size, data| {
        let data = data.as_bytes().to_vec();
        let mut r = RewindingReader::new(move || Ok(Cursor::new(data.clone()))).unwrap();
        r.block_size = NonZero::try_from(block_size as u64).unwrap();
        Box::new(r)
    });
}

#[test]
fn test_rewinding_reader_lru() {
    test_rewinding_reader(|block_size, data| {
        let data = data.as_bytes().to_vec();
        Box::new(
            RewindingReader::build(move || Ok(Cursor::new(data.clone())))
                .block_size(block_size.try_into().unwrap())
                .cache(LruCache::new(3))
                .done()
                .unwrap(),
        )
    });
}

#[test]
fn test_replay_seek_reader() {
    let data = b"Lorem ipsum dolor sit amet.";
    let s = |buf| str::from_utf8(buf).unwrap();
    let mut r = ReplaySeekReader::build(Cursor::new(data))
        .segment_size(nonzero!(4_usize))
        .done();

    let pos = r.seek(SeekFrom::Start(6)).unwrap();
    assert_eq!(pos, 6);
    let mut buf = vec![0; 5];
    r.read_exact(&mut buf).unwrap();
    assert_eq!(s(&buf), "ipsum");

    let pos = r.seek(SeekFrom::Current(7)).unwrap();
    assert_eq!(pos, 18);
    let mut buf = vec![0; 3];
    r.read_exact(&mut buf).unwrap();
    assert_eq!(s(&buf), "sit");

    let pos = r.seek(SeekFrom::Current(-9)).unwrap();
    assert_eq!(pos, 12);
    let mut buf = vec![0; 5];
    r.read_exact(&mut buf).unwrap();
    assert_eq!(s(&buf), "dolor");

    let pos = r.seek(SeekFrom::End(-5)).unwrap();
    assert_eq!(pos, 22);
    let mut buf = vec![];
    r.read_to_end(&mut buf).unwrap();
    assert_eq!(s(&buf), "amet.");

    let pos = r.seek(SeekFrom::End(0)).unwrap();
    assert_eq!(pos, 27);
    let mut buf = vec![];
    r.read_to_end(&mut buf).unwrap();
    assert_eq!(s(&buf), "");

    let mut r = ReplaySeekReader::new(Cursor::new(data));
    let pos = r.seek(SeekFrom::End(0)).unwrap();
    assert_eq!(pos, 27);

    let mut r = ReplaySeekReader::build(Cursor::new(data))
        .cache(MinimalCache::default())
        .done();
    let pos = r.seek(SeekFrom::End(-7)).unwrap();
    assert_eq!(pos, 20);
}
