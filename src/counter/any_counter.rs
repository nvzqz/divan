use std::fmt;

use crate::{counter::MaxCountUInt, time::FineDuration, util};

/// Type-erased `Counter`.
///
/// This does not implement `Copy` because in the future it will contain
/// user-defined counters.
#[derive(Clone)]
pub enum AnyCounter {
    Bytes(MaxCountUInt),
    Items(MaxCountUInt),
}

impl AnyCounter {
    pub(crate) fn display_throughput(&self, duration: FineDuration) -> DisplayThroughput {
        DisplayThroughput { counter: self, picos: duration.picos as f64 }
    }
}

pub(crate) struct DisplayThroughput<'a> {
    counter: &'a AnyCounter,
    picos: f64,
}

impl fmt::Debug for DisplayThroughput<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for DisplayThroughput<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (val, suffix) = match *self.counter {
            AnyCounter::Bytes(bytes) => bytes_throughput(bytes, self.picos),
            AnyCounter::Items(items) => items_throughput(items, self.picos),
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

/// Returns bytes per second at the appropriate scale along with the scale's
/// suffix.
fn bytes_throughput(bytes: MaxCountUInt, picos: f64) -> (f64, &'static str) {
    // Stop at pebibyte because `f64` cannot represent exibyte exactly.
    const KIB: f64 = 1024.;
    const MIB: f64 = 1024u64.pow(2) as f64;
    const GIB: f64 = 1024u64.pow(3) as f64;
    const TIB: f64 = 1024u64.pow(4) as f64;
    const PIB: f64 = 1024u64.pow(5) as f64;

    let bytes_per_sec = if bytes == 0 { 0. } else { bytes as f64 * (1e12 / picos) };

    let (scale, suffix) = if bytes_per_sec.is_infinite() || bytes_per_sec < KIB {
        (1., " B/s")
    } else if bytes_per_sec < MIB {
        (KIB, " KiB/s")
    } else if bytes_per_sec < GIB {
        (MIB, " MiB/s")
    } else if bytes_per_sec < TIB {
        (GIB, " GiB/s")
    } else if bytes_per_sec < PIB {
        (TIB, " TiB/s")
    } else {
        (PIB, " PiB/s")
    };

    (bytes_per_sec / scale, suffix)
}

/// Returns items per second at the appropriate scale along with the scale's
/// suffix.
fn items_throughput(items: MaxCountUInt, picos: f64) -> (f64, &'static str) {
    // Stop at peta because bytes stops at pebibyte.
    const K: f64 = 1e3;
    const M: f64 = 1e6;
    const G: f64 = 1e9;
    const T: f64 = 1e12;
    const P: f64 = 1e15;

    let items_per_sec = if items == 0 { 0. } else { items as f64 * (1e12 / picos) };

    let (scale, suffix) = if items_per_sec.is_infinite() || items_per_sec < K {
        (1., " item/s")
    } else if items_per_sec < M {
        (K, " Kitem/s")
    } else if items_per_sec < G {
        (M, " Mitem/s")
    } else if items_per_sec < T {
        (G, " Gitem/s")
    } else if items_per_sec < P {
        (T, " Titem/s")
    } else {
        (P, " Pitem/s")
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
            fn test(bytes: MaxCountUInt, picos: u128, expected: &str) {
                assert_eq!(
                    AnyCounter::Bytes(bytes).display_throughput(FineDuration { picos }).to_string(),
                    expected
                );
            }

            test(1, 0, "inf B/s");
            test(MaxCountUInt::MAX, 0, "inf B/s");

            test(0, 0, "0 B/s");
            test(0, 1, "0 B/s");
            test(0, u128::MAX, "0 B/s");
        }

        #[test]
        fn items() {
            #[track_caller]
            fn test(items: MaxCountUInt, picos: u128, expected: &str) {
                assert_eq!(
                    AnyCounter::Items(items).display_throughput(FineDuration { picos }).to_string(),
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
