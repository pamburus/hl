//! Testing utilities for internal use.

/// Trait that provides a method to generate a sample instance.
///
/// Types implementing this trait can create representative examples
/// of themselves for testing purposes. The `sample()` method should return
/// a stable, deterministic instance with reasonable default values.
pub trait Sample {
    /// Returns a sample instance of the implementing type.
    ///
    /// This is useful for testing where a representative example of the type is needed.
    fn sample() -> Self;
}
