// std imports
use std::sync::Arc;

// ---

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EnabledFormat {
    #[cfg(feature = "json")]
    Json,
    #[cfg(feature = "logfmt")]
    Logfmt,
}

// ---

pub type EnabledFormatList = Arc<Vec<EnabledFormat>>;

pub trait IntoEnabledFormatList {
    fn into_enabled_format_list(self) -> EnabledFormatList;
}

impl IntoEnabledFormatList for &[EnabledFormat] {
    #[inline]
    fn into_enabled_format_list(self) -> EnabledFormatList {
        Arc::new(self.to_vec())
    }
}

impl IntoEnabledFormatList for Vec<EnabledFormat> {
    #[inline]
    fn into_enabled_format_list(self) -> EnabledFormatList {
        Arc::new(self)
    }
}

impl IntoEnabledFormatList for EnabledFormat {
    #[inline]
    fn into_enabled_format_list(self) -> EnabledFormatList {
        Arc::new(vec![self])
    }
}

impl IntoEnabledFormatList for EnabledFormatList {
    #[inline]
    fn into_enabled_format_list(self) -> EnabledFormatList {
        self
    }
}
