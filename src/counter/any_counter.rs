use std::any::TypeId;

use crate::{
    counter::{BytesCount, BytesFormat, CharsCount, IntoCounter, ItemsCount, MaxCountUInt},
    time::FineDuration,
    util::{self, fmt::DisplayThroughput},
};

/// Type-erased `Counter`.
///
/// This does not implement `Copy` because in the future it will contain
/// user-defined counters.
#[derive(Clone)]
pub(crate) struct AnyCounter {
    pub kind: KnownCounterKind,
    count: MaxCountUInt,
}

impl AnyCounter {
    #[inline]
    pub(crate) fn new<C: IntoCounter>(counter: C) -> Self {
        let counter = counter.into_counter();

        if let Some(bytes) = util::cast_ref::<BytesCount>(&counter) {
            Self::bytes(bytes.count)
        } else if let Some(chars) = util::cast_ref::<CharsCount>(&counter) {
            Self::chars(chars.count)
        } else if let Some(items) = util::cast_ref::<ItemsCount>(&counter) {
            Self::items(items.count)
        } else {
            unreachable!()
        }
    }

    #[inline]
    pub(crate) fn known(kind: KnownCounterKind, count: MaxCountUInt) -> Self {
        Self { kind, count }
    }

    #[inline]
    pub(crate) fn bytes(count: MaxCountUInt) -> Self {
        Self::known(KnownCounterKind::Bytes, count)
    }

    #[inline]
    pub(crate) fn chars(count: MaxCountUInt) -> Self {
        Self::known(KnownCounterKind::Chars, count)
    }

    #[inline]
    pub(crate) fn items(count: MaxCountUInt) -> Self {
        Self::known(KnownCounterKind::Items, count)
    }

    pub(crate) fn display_throughput(
        &self,
        duration: FineDuration,
        bytes_format: BytesFormat,
    ) -> DisplayThroughput {
        DisplayThroughput { counter: self, picos: duration.picos as f64, bytes_format }
    }

    #[inline]
    pub(crate) fn count(&self) -> MaxCountUInt {
        self.count
    }

    #[inline]
    pub(crate) fn known_kind(&self) -> KnownCounterKind {
        self.kind
    }
}

/// Kind of `Counter` defined by this crate.
#[derive(Clone, Copy)]
pub(crate) enum KnownCounterKind {
    Bytes,
    Chars,
    Items,
}

impl KnownCounterKind {
    pub const COUNT: usize = 3;

    pub const ALL: [Self; Self::COUNT] = [Self::Bytes, Self::Chars, Self::Items];

    /// The maximum width for columns displaying counters.
    pub const MAX_COMMON_COLUMN_WIDTH: usize = "1.111 Kitem/s".len();

    #[inline]
    pub fn of<C: IntoCounter>() -> Self {
        let id = TypeId::of::<C::Counter>();
        if id == TypeId::of::<BytesCount>() {
            Self::Bytes
        } else if id == TypeId::of::<CharsCount>() {
            Self::Chars
        } else if id == TypeId::of::<ItemsCount>() {
            Self::Items
        } else {
            unreachable!()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod display_throughput {
        use super::*;

        #[test]
        fn bytes() {
            #[track_caller]
            fn test(
                bytes: MaxCountUInt,
                picos: u128,
                expected_binary: &str,
                expected_decimal: &str,
            ) {
                for (bytes_format, expected) in [
                    (BytesFormat::Binary, expected_binary),
                    (BytesFormat::Decimal, expected_decimal),
                ] {
                    assert_eq!(
                        AnyCounter::bytes(bytes)
                            .display_throughput(FineDuration { picos }, bytes_format)
                            .to_string(),
                        expected
                    );
                }
            }

            #[track_caller]
            fn test_all(bytes: MaxCountUInt, picos: u128, expected: &str) {
                test(bytes, picos, expected, expected);
            }

            test_all(1, 0, "inf B/s");
            test_all(MaxCountUInt::MAX, 0, "inf B/s");

            test_all(0, 0, "0 B/s");
            test_all(0, 1, "0 B/s");
            test_all(0, u128::MAX, "0 B/s");
        }

        #[test]
        fn chars() {
            #[track_caller]
            fn test(chars: MaxCountUInt, picos: u128, expected: &str) {
                assert_eq!(
                    AnyCounter::chars(chars)
                        .display_throughput(FineDuration { picos }, BytesFormat::default())
                        .to_string(),
                    expected
                );
            }

            test(1, 0, "inf char/s");
            test(MaxCountUInt::MAX, 0, "inf char/s");

            test(0, 0, "0 char/s");
            test(0, 1, "0 char/s");
            test(0, u128::MAX, "0 char/s");
        }

        #[test]
        fn items() {
            #[track_caller]
            fn test(items: MaxCountUInt, picos: u128, expected: &str) {
                assert_eq!(
                    AnyCounter::items(items)
                        .display_throughput(FineDuration { picos }, BytesFormat::default())
                        .to_string(),
                    expected
                );
            }

            test(1, 0, "inf item/s");
            test(MaxCountUInt::MAX, 0, "inf item/s");

            test(0, 0, "0 item/s");
            test(0, 1, "0 item/s");
            test(0, u128::MAX, "0 item/s");
        }
    }
}
