use std::{any::TypeId, fmt};

use crate::{
    counter::{Bytes, BytesFormat, Chars, IntoCounter, Items, MaxCountUInt},
    time::FineDuration,
    util,
};

/// Type-erased `Counter`.
///
/// This does not implement `Copy` because in the future it will contain
/// user-defined counters.
#[derive(Clone)]
pub(crate) struct AnyCounter {
    kind: KnownCounterKind,
    count: MaxCountUInt,
}

impl AnyCounter {
    #[inline]
    pub(crate) fn new<C: IntoCounter>(counter: C) -> Self {
        let counter = counter.into_counter();

        if let Some(bytes) = util::cast_ref::<Bytes>(&counter) {
            Self::bytes(bytes.count)
        } else if let Some(chars) = util::cast_ref::<Chars>(&counter) {
            Self::chars(chars.count)
        } else if let Some(items) = util::cast_ref::<Items>(&counter) {
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
        if id == TypeId::of::<Bytes>() {
            Self::Bytes
        } else if id == TypeId::of::<Chars>() {
            Self::Chars
        } else if id == TypeId::of::<Items>() {
            Self::Items
        } else {
            unreachable!()
        }
    }
}

pub(crate) struct DisplayThroughput<'a> {
    counter: &'a AnyCounter,
    picos: f64,
    bytes_format: BytesFormat,
}

impl fmt::Debug for DisplayThroughput<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for DisplayThroughput<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let picos = self.picos;
        let count = self.counter.count();
        let count_per_sec = if count == 0 { 0. } else { count as f64 * (1e12 / picos) };

        let (scales, suffixes) = match self.counter.kind {
            KnownCounterKind::Bytes => match self.bytes_format {
                BytesFormat::Binary => (scale::BINARY_SCALES, scale::BYTES_BINARY_SUFFIXES),
                BytesFormat::Decimal => (scale::DECIMAL_SCALES, scale::BYTES_DECIMAL_SUFFIXES),
            },
            KnownCounterKind::Chars => (scale::DECIMAL_SCALES, scale::CHARS_SUFFIXES),
            KnownCounterKind::Items => (scale::DECIMAL_SCALES, scale::ITEMS_SUFFIXES),
        };

        let (val, suffix) = scale_throughput(count_per_sec, scales, suffixes);

        let sig_figs = f.precision().unwrap_or(4);

        let mut str = util::format_f64(val, sig_figs);
        str.push(' ');
        str.push_str(suffix);

        // Fill up to specified width.
        if let Some(fill_len) = f.width().and_then(|width| width.checked_sub(str.len())) {
            match f.align() {
                None | Some(fmt::Alignment::Left) => {
                    str.extend(std::iter::repeat(f.fill()).take(fill_len));
                }
                _ => return Err(fmt::Error),
            }
        }

        f.write_str(&str)
    }
}

/// Returns throughput at the appropriate scale along with the scale's suffix.
fn scale_throughput(
    count_per_sec: f64,
    scales: &scale::Scales,
    suffixes: &scale::Suffixes,
) -> (f64, &'static str) {
    let (scale, suffix) = if count_per_sec.is_infinite() || count_per_sec < scales[0] {
        (1., suffixes[0])
    } else if count_per_sec < scales[1] {
        (scales[0], suffixes[1])
    } else if count_per_sec < scales[2] {
        (scales[1], suffixes[2])
    } else if count_per_sec < scales[3] {
        (scales[2], suffixes[3])
    } else if count_per_sec < scales[4] {
        (scales[3], suffixes[4])
    } else {
        (scales[4], suffixes[5])
    };

    (count_per_sec / scale, suffix)
}

mod scale {
    /// Scales increasing in powers of 1000 (decimal) or 1024 (binary).
    pub type Scales = [f64; 5];

    /// Throughput suffixes.
    pub type Suffixes = [&'static str; 6];

    // Stop at peta because bytes stops at pebibyte.
    pub const DECIMAL_SCALES: &Scales = &[1e3, 1e6, 1e9, 1e12, 1e15];

    // Stop at pebibyte because `f64` cannot represent exibyte exactly.
    pub const BINARY_SCALES: &Scales = &[
        1024., // KiB
        1024u64.pow(2) as f64,
        1024u64.pow(3) as f64,
        1024u64.pow(4) as f64,
        1024u64.pow(5) as f64, // PiB
    ];

    pub const BYTES_BINARY_SUFFIXES: &Suffixes =
        &["B/s", "KiB/s", "MiB/s", "GiB/s", "TiB/s", "PiB/s"];

    pub const BYTES_DECIMAL_SUFFIXES: &Suffixes = &["B/s", "KB/s", "MB/s", "GB/s", "TB/s", "PB/s"];

    pub const CHARS_SUFFIXES: &Suffixes =
        &["char/s", "Kchar/s", "Mchar/s", "Gchar/s", "Tchar/s", "Pchar/s"];

    pub const ITEMS_SUFFIXES: &Suffixes =
        &["item/s", "Kitem/s", "Mitem/s", "Gitem/s", "Titem/s", "Pitem/s"];
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
