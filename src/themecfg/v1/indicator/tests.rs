use super::super::super::{MergeFlags, MergeWithOptions, RawStyle};
use super::{Indicator, IndicatorPack, IndicatorStyle, SyncIndicatorPack};

#[test]
fn test_indicator_pack_merge() {
    let mut base = IndicatorPack::<RawStyle>::default();
    let mut other = IndicatorPack::<RawStyle>::default();

    other.sync.synced.text = "✓".to_string();
    other.sync.failed.text = "✗".to_string();

    base.merge(other, MergeFlags::default());
    assert_eq!(base.sync.synced.text, "✓");
    assert_eq!(base.sync.failed.text, "✗");
}

#[test]
fn test_indicator_style_merge_empty() {
    let mut base = IndicatorStyle::<RawStyle>::default();
    let other = IndicatorStyle::<RawStyle> {
        prefix: "[".to_string(),
        suffix: "]".to_string(),
        ..Default::default()
    };

    base.merge(other, MergeFlags::default());
    assert_eq!(base.prefix, "[");
    assert_eq!(base.suffix, "]");
}

#[test]
fn test_sync_indicator_pack_merge() {
    let mut base = SyncIndicatorPack::<RawStyle>::default();
    let mut other = SyncIndicatorPack::<RawStyle>::default();

    other.synced.text = "✓".to_string();
    other.failed.text = "✗".to_string();

    base.merge(other, MergeFlags::default());
    assert_eq!(base.synced.text, "✓");
    assert_eq!(base.failed.text, "✗");
}

#[test]
fn test_indicator_merge_empty_text() {
    let mut base = Indicator::<RawStyle> {
        text: "original".to_string(),
        ..Default::default()
    };

    let other = Indicator::<RawStyle> {
        text: "".to_string(),
        ..Default::default()
    };

    base.merge(other, MergeFlags::default());
    assert_eq!(base.text, "original");
}

#[test]
fn test_indicator_pack_merged() {
    let base = IndicatorPack::<RawStyle>::default();
    let mut other = IndicatorPack::<RawStyle>::default();
    other.sync.synced.text = "✓".to_string();

    let merged = base.merged(other, MergeFlags::default());
    assert_eq!(merged.sync.synced.text, "✓");
}

#[test]
fn test_sync_indicator_pack_merged() {
    let base = SyncIndicatorPack::<RawStyle>::default();
    let mut other = SyncIndicatorPack::<RawStyle>::default();
    other.synced.text = "✓".to_string();

    let merged = base.merged(other, MergeFlags::default());
    assert_eq!(merged.synced.text, "✓");
}

#[test]
fn test_indicator_text_merge() {
    let base = Indicator::<RawStyle>::default();
    let other = Indicator::<RawStyle> {
        text: "test".to_string(),
        ..Default::default()
    };

    let merged = base.merged(other, MergeFlags::default());
    assert_eq!(merged.text, "test");
}

#[test]
fn test_indicator_style_defaults() {
    let style = IndicatorStyle::<RawStyle>::default();
    let other = IndicatorStyle::<RawStyle> {
        prefix: "[".to_string(),
        suffix: "]".to_string(),
        ..Default::default()
    };

    let merged = style.merged(other, MergeFlags::default());
    assert_eq!(merged.prefix, "[");
    assert_eq!(merged.suffix, "]");
}
