use super::*;
use log_format_json::JsonFormat;

#[test]
fn test_segment() {
    let buf = r#"{"a":10}"#;
    let segment = Segment::parse(buf, &mut JsonFormat).unwrap().unwrap();
    assert_eq!(segment.source().len(), 8);
    assert_eq!(segment.entries().len(), 1);
    let entires = segment.entries().into_iter().collect::<Vec<_>>();
    assert_eq!(entires.len(), 1);
    let fields = entires[0].into_iter().collect::<Vec<_>>();
    assert_eq!(fields.len(), 1);
    let field = &fields[0];
    assert_eq!(field.key().source(), "a");
    match field.value() {
        Value::Number(number) => {
            assert_eq!(number.source(), "10");
        }
        _ => panic!("unexpected value: {:?}", field.value()),
    }
}
