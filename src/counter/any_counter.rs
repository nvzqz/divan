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
        let count = self.counter.count();

        let (val, suffix) = match self.counter.kind {
            KnownCounterKind::Bytes => match self.bytes_format {
                BytesFormat::Binary => bytes_throughput_binary(count, self.picos),
                BytesFormat::Decimal => bytes_throughput_decimal(count, self.picos),
            },
            KnownCounterKind::Chars => chars_throughput(count, self.picos),
            KnownCounterKind::Items => items_throughput(count, self.picos),
        };

        let sig_figs = f.precision().unwrap_or(4);

        let mut str = util::format_f64(val, sig_figs);
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

mod scale {
    // Stop at pebibyte because `f64` cannot represent exibyte exactly.
    pub const KIB: f64 = 1024.;
    pub const MIB: f64 = 1024u64.pow(2) as f64;
    pub const GIB: f64 = 1024u64.pow(3) as f64;
    pub const TIB: f64 = 1024u64.pow(4) as f64;
    pub const PIB: f64 = 1024u64.pow(5) as f64;

    // Stop at peta because bytes stops at pebibyte.
    pub const K: f64 = 1e3;
    pub const M: f64 = 1e6;
    pub const G: f64 = 1e9;
    pub const T: f64 = 1e12;
    pub const P: f64 = 1e15;
}

/// Returns bytes per second at the appropriate binary scale along with the
/// scale's suffix.
fn bytes_throughput_binary(bytes: MaxCountUInt, picos: f64) -> (f64, &'static str) {
    let bytes_per_sec = if bytes == 0 { 0. } else { bytes as f64 * (1e12 / picos) };

    let (scale, suffix) = if bytes_per_sec.is_infinite() || bytes_per_sec < scale::KIB {
        (1., " B/s")
    } else if bytes_per_sec < scale::MIB {
        (scale::KIB, " KiB/s")
    } else if bytes_per_sec < scale::GIB {
        (scale::MIB, " MiB/s")
    } else if bytes_per_sec < scale::TIB {
        (scale::GIB, " GiB/s")
    } else if bytes_per_sec < scale::PIB {
        (scale::TIB, " TiB/s")
    } else {
        (scale::PIB, " PiB/s")
    };

    (bytes_per_sec / scale, suffix)
}

/// Returns bytes per second at the appropriate decimal scale along with the
/// scale's suffix.
fn bytes_throughput_decimal(bytes: MaxCountUInt, picos: f64) -> (f64, &'static str) {
    let bytes_per_sec = if bytes == 0 { 0. } else { bytes as f64 * (1e12 / picos) };

    let (scale, suffix) = if bytes_per_sec.is_infinite() || bytes_per_sec < scale::K {
        (1., " B/s")
    } else if bytes_per_sec < scale::M {
        (scale::K, " KB/s")
    } else if bytes_per_sec < scale::G {
        (scale::M, " MB/s")
    } else if bytes_per_sec < scale::T {
        (scale::G, " GB/s")
    } else if bytes_per_sec < scale::P {
        (scale::T, " TB/s")
    } else {
        (scale::P, " PB/s")
    };

    (bytes_per_sec / scale, suffix)
}

/// Returns `char`s per second at the appropriate scale along with the scale's
/// suffix.
fn chars_throughput(chars: MaxCountUInt, picos: f64) -> (f64, &'static str) {
    let chars_per_sec = if chars == 0 { 0. } else { chars as f64 * (1e12 / picos) };

    let (scale, suffix) = if chars_per_sec.is_infinite() || chars_per_sec < scale::K {
        (1., " char/s")
    } else if chars_per_sec < scale::M {
        (scale::K, " Kchar/s")
    } else if chars_per_sec < scale::G {
        (scale::M, " Mchar/s")
    } else if chars_per_sec < scale::T {
        (scale::G, " Gchar/s")
    } else if chars_per_sec < scale::P {
        (scale::T, " Tchar/s")
    } else {
        (scale::P, " Pchar/s")
    };

    (chars_per_sec / scale, suffix)
}

/// Returns items per second at the appropriate scale along with the scale's
/// suffix.
fn items_throughput(items: MaxCountUInt, picos: f64) -> (f64, &'static str) {
    let items_per_sec = if items == 0 { 0. } else { items as f64 * (1e12 / picos) };

    let (scale, suffix) = if items_per_sec.is_infinite() || items_per_sec < scale::K {
        (1., " item/s")
    } else if items_per_sec < scale::M {
        (scale::K, " Kitem/s")
    } else if items_per_sec < scale::G {
        (scale::M, " Mitem/s")
    } else if items_per_sec < scale::T {
        (scale::G, " Gitem/s")
    } else if items_per_sec < scale::P {
        (scale::T, " Titem/s")
    } else {
        (scale::P, " Pitem/s")
    };

    (items_per_sec / scale, suffix)
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
