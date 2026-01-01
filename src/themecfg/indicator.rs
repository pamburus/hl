// relative imports
use super::Style;

#[derive(Clone, Debug, Default)]
pub struct IndicatorPack {
    pub sync: SyncIndicatorPack,
}

#[derive(Clone, Debug, Default)]
pub struct SyncIndicatorPack {
    pub synced: Indicator,
    pub failed: Indicator,
}

#[derive(Clone, Debug, Default)]
pub struct Indicator {
    pub outer: IndicatorStyle,
    pub inner: IndicatorStyle,
    pub text: String,
}

#[derive(Clone, Debug, Default)]
pub struct IndicatorStyle {
    pub prefix: String,
    pub suffix: String,
    pub style: Style,
}
