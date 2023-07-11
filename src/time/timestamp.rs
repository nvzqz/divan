use std::time::Instant;

use crate::time::{FineDuration, Tsc};

/// A measurement timestamp.
#[derive(Clone, Copy)]
pub enum Timestamp {
    /// Time provided by the operating system.
    Os(Instant),

    /// [Timestamp counter](https://en.wikipedia.org/wiki/Time_Stamp_Counter).
    Tsc(Tsc),
}

impl Timestamp {
    pub fn duration_since(self, earlier: Self, tsc_frequency: u64) -> FineDuration {
        match (self, earlier) {
            (Self::Os(this), Self::Os(earlier)) => this.duration_since(earlier).into(),
            (Self::Tsc(this), Self::Tsc(earlier)) => this.duration_since(earlier, tsc_frequency),
            _ => unreachable!(),
        }
    }
}

/// A [`Timestamp`] where the variant is determined by an external source of
/// truth.
///
/// By making the variant external to this type, we produce more optimized code
/// by:
/// - Reusing the same condition variable
/// - Reducing the size of the timestamp variables
#[derive(Clone, Copy)]
pub union AnyTimestamp {
    /// [`Timestamp::Os`].
    pub os: Instant,

    /// [`Timestamp::Tsc`].
    pub tsc: Tsc,
}

impl AnyTimestamp {
    #[inline(always)]
    pub fn now(use_tsc: bool) -> Self {
        if use_tsc {
            Self { tsc: Tsc::now() }
        } else {
            Self { os: Instant::now() }
        }
    }

    #[inline(always)]
    pub unsafe fn into_timestamp(self, is_tsc: bool) -> Timestamp {
        if is_tsc {
            Timestamp::Tsc(self.tsc)
        } else {
            Timestamp::Os(self.os)
        }
    }
}
