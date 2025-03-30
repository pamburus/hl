// third-party imports
use enumset::{EnumSet, EnumSetType};

// modules
pub mod convert;

pub trait EnumSetExt<T: EnumSetType> {
    fn intersects(&self, other: Self) -> bool;
    fn includes(&self, other: Self) -> bool;
}

impl<T: EnumSetType> EnumSetExt<T> for EnumSet<T> {
    #[inline]
    fn intersects(&self, other: Self) -> bool {
        !self.intersection(other).is_empty()
    }

    #[inline]
    fn includes(&self, other: Self) -> bool {
        self.intersection(other) == other
    }
}
