use crate::counter::AnyCounter;

/// Prevents `Counter` from being implemented externally.
///
/// The `into_any_counter` method exists on this trait rather than `Counter` so
/// that it is impossible to access externally.
pub trait Sealed {
    fn into_any_counter(self) -> AnyCounter;
}
