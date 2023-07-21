use std::{fmt, time::Duration};

/// [Picosecond](https://en.wikipedia.org/wiki/Picosecond)-precise [`Duration`].
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct FineDuration {
    pub picos: u128,
}

impl From<Duration> for FineDuration {
    #[inline]
    fn from(duration: Duration) -> Self {
        Self {
            picos: duration
                .as_nanos()
                .checked_mul(1_000)
                .unwrap_or_else(|| panic!("{duration:?} is too large to fit in `FineDuration`")),
        }
    }
}

impl fmt::Display for FineDuration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let sig_figs = f.precision().unwrap_or(4);

        let picos = self.picos;
        let scale = TimeScale::from_picos(picos);

        let multiple: u128 = {
            let sig_figs = u32::try_from(sig_figs).unwrap_or(u32::MAX);
            10_u128.saturating_pow(sig_figs)
        };

        // TODO: Format without heap allocation.
        let mut str: String = match picos::DAY.checked_mul(multiple) {
            Some(int_day) if picos >= int_day => {
                // Format using integer representation to not lose precision.
                (picos / picos::DAY).to_string()
            }
            _ => {
                // Format using floating point representation.

                // Multiply to allow `sig_figs` digits of fractional precision.
                let val = (((picos * multiple) / scale.picos()) as f64) / multiple as f64;

                let int_digits = 1 + val.trunc().log10() as usize;
                let fract_digits = sig_figs.saturating_sub(int_digits);

                let mut str = val.to_string();

                if let Some(dot_index) = str.find('.') {
                    if fract_digits == 0 {
                        str.truncate(dot_index);
                    } else {
                        let fract_start = dot_index + 1;
                        let fract_end = fract_start + fract_digits;
                        let fract_range = fract_start..fract_end;

                        if let Some(fract_str) = str.get(fract_range) {
                            // Get the offset from the end before all 0s.
                            let pre_zero =
                                fract_str.bytes().rev().enumerate().find_map(|(i, b)| {
                                    if b != b'0' {
                                        Some(i)
                                    } else {
                                        None
                                    }
                                });

                            if let Some(pre_zero) = pre_zero {
                                str.truncate(fract_end - pre_zero);
                            } else {
                                str.truncate(dot_index);
                            }
                        }
                    }
                }

                str
            }
        };

        str.push_str(scale.suffix());

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

impl fmt::Debug for FineDuration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

mod picos {
    pub const NANOS: u128 = 1_000;
    pub const MICROS: u128 = 1_000 * NANOS;
    pub const MILLIS: u128 = 1_000 * MICROS;
    pub const SEC: u128 = 1_000 * MILLIS;
    pub const MIN: u128 = 60 * SEC;
    pub const HOUR: u128 = 60 * MIN;
    pub const DAY: u128 = 24 * HOUR;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum TimeScale {
    PicoSec,
    NanoSec,
    MicroSec,
    MilliSec,
    Sec,
    Min,
    Hour,
    Day,
}

impl TimeScale {
    #[cfg(test)]
    const ALL: &[Self] = &[
        Self::PicoSec,
        Self::NanoSec,
        Self::MicroSec,
        Self::MilliSec,
        Self::Sec,
        Self::Min,
        Self::Hour,
        Self::Day,
    ];

    /// Determines the scale of time for representing a number of picoseconds.
    fn from_picos(picos: u128) -> Self {
        use picos::*;

        if picos < NANOS {
            Self::PicoSec
        } else if picos < MICROS {
            Self::NanoSec
        } else if picos < MILLIS {
            Self::MicroSec
        } else if picos < SEC {
            Self::MilliSec
        } else if picos < MIN {
            Self::Sec
        } else if picos < HOUR {
            Self::Min
        } else if picos < DAY {
            Self::Hour
        } else {
            Self::Day
        }
    }

    /// Returns the number of picoseconds needed to reach this scale.
    fn picos(self) -> u128 {
        use picos::*;

        match self {
            Self::PicoSec => 1,
            Self::NanoSec => NANOS,
            Self::MicroSec => MICROS,
            Self::MilliSec => MILLIS,
            Self::Sec => SEC,
            Self::Min => MIN,
            Self::Hour => HOUR,
            Self::Day => DAY,
        }
    }

    /// Returns the unit suffix.
    fn suffix(self) -> &'static str {
        match self {
            Self::PicoSec => "ps",
            Self::NanoSec => "ns",
            Self::MicroSec => "µs",
            Self::MilliSec => "ms",
            Self::Sec => "s",
            Self::Min => "m",
            Self::Hour => "h",
            Self::Day => "d",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::zero_prefixed_literal)]
    mod fmt {
        use super::*;

        #[track_caller]
        fn test(picos: u128, expected: &str) {
            let duration = FineDuration { picos };
            assert_eq!(duration.to_string(), expected);
            assert_eq!(format!("{duration:.4}"), expected);
            assert_eq!(format!("{duration:<0}"), expected);
        }

        macro_rules! assert_fmt_eq {
            ($input:literal, $expected:literal) => {
                assert_eq!(format!($input), format!($expected));
            };
        }

        #[test]
        fn precision() {
            for &scale in TimeScale::ALL {
                let base_duration = FineDuration { picos: scale.picos() };
                let incr_duration = FineDuration { picos: scale.picos() + 1 };

                let base_string = base_duration.to_string();

                // Integer format.
                assert_eq!(format!("{base_duration:.0}"), base_string);

                if scale == TimeScale::PicoSec {
                    assert_eq!(format!("{incr_duration:.0}"), "2ps");
                } else {
                    assert_eq!(format!("{incr_duration:.0}"), base_string);
                }
            }
        }

        #[test]
        fn fill() {
            for scale in TimeScale::ALL {
                let duration = FineDuration { picos: scale.picos() };
                let suffix = scale.suffix();
                let pad = " ".repeat(9 - suffix.len());

                assert_fmt_eq!("{duration:<2}", "1{suffix}");
                assert_fmt_eq!("{duration:<10}", "1{suffix}{pad}");
            }
        }

        #[test]
        fn pico_sec() {
            test(000, "0ps");

            test(001, "1ps");
            test(010, "10ps");
            test(100, "100ps");

            test(102, "102ps");
            test(120, "120ps");
            test(123, "123ps");
            test(012, "12ps");
        }

        #[test]
        fn nano_sec() {
            test(001_000, "1ns");
            test(010_000, "10ns");
            test(100_000, "100ns");

            test(100_002, "100ns");
            test(100_020, "100ns");
            test(100_200, "100.2ns");
            test(102_000, "102ns");
            test(120_000, "120ns");

            test(001_002, "1.002ns");
            test(001_023, "1.023ns");
            test(001_234, "1.234ns");
            test(001_230, "1.23ns");
            test(001_200, "1.2ns");
        }

        #[test]
        fn micro_sec() {
            test(001_000_000, "1µs");
            test(010_000_000, "10µs");
            test(100_000_000, "100µs");

            test(100_000_002, "100µs");
            test(100_000_020, "100µs");
            test(100_000_200, "100µs");
            test(100_002_000, "100µs");
            test(100_020_000, "100µs");
            test(100_200_000, "100.2µs");
            test(102_000_000, "102µs");

            test(120_000_000, "120µs");
            test(012_000_000, "12µs");
            test(001_200_000, "1.2µs");

            test(001_020_000, "1.02µs");
            test(001_002_000, "1.002µs");
            test(001_000_200, "1µs");
            test(001_000_020, "1µs");
            test(001_000_002, "1µs");

            test(001_230_000, "1.23µs");
            test(001_234_000, "1.234µs");
            test(001_234_500, "1.234µs");
            test(001_234_560, "1.234µs");
            test(001_234_567, "1.234µs");
        }

        #[test]
        fn milli_sec() {
            test(001_000_000_000, "1ms");
            test(010_000_000_000, "10ms");
            test(100_000_000_000, "100ms");
        }

        #[test]
        fn sec() {
            test(picos::SEC, "1s");
            test(picos::SEC * 10, "10s");
            test(picos::SEC * 59, "59s");

            test(picos::MILLIS * 59_999, "59.99s");
        }

        #[test]
        fn min() {
            test(picos::MIN, "1m");
            test(picos::MIN * 10, "10m");
            test(picos::MIN * 59, "59m");

            test(picos::MILLIS * 3_599_000, "59.98m");
            test(picos::MILLIS * 3_599_999, "59.99m");
            test(picos::HOUR - 1, "59.99m");
        }

        #[test]
        fn hour() {
            test(picos::HOUR, "1h");
            test(picos::HOUR * 10, "10h");
            test(picos::HOUR * 23, "23h");

            test(picos::MILLIS * 86_300_000, "23.97h");
            test(picos::MILLIS * 86_399_999, "23.99h");
            test(picos::DAY - 1, "23.99h");
        }

        #[test]
        fn day() {
            test(picos::DAY, "1d");

            test(picos::DAY + picos::DAY / 10, "1.1d");
            test(picos::DAY + picos::DAY / 100, "1.01d");
            test(picos::DAY + picos::DAY / 1000, "1.001d");

            test(picos::DAY * 000010, "10d");
            test(picos::DAY * 000100, "100d");
            test(picos::DAY * 001000, "1000d");
            test(picos::DAY * 010000, "10000d");
            test(picos::DAY * 100000, "100000d");

            test(u128::MAX / 1000, "3938453320844195178d");
            test(u128::MAX, "3938453320844195178974d");
        }
    }
}
