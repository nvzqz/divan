use crate::counter::{AnyCounter, KnownCounterKind};

/// Prevents `Counter` from being implemented externally.
///
/// Items exist on this trait rather than `Counter` so that they are impossible
/// to access externally.
pub trait Sealed {
    const COUNTER_KIND: KnownCounterKind;

    fn into_any_counter(self) -> AnyCounter;
}
