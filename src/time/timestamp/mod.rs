use std::time::Instant;

use crate::time::{FineDuration, Timer, TimerKind};

mod tsc;

pub use tsc::*;

/// A measurement timestamp.
#[derive(Clone, Copy)]
pub enum Timestamp {
    /// Time provided by the operating system.
    Os(Instant),

    /// [CPU timestamp counter](https://en.wikipedia.org/wiki/Time_Stamp_Counter).
    Tsc(TscTimestamp),
}

impl Timestamp {
    pub fn duration_since(self, earlier: Self, timer: Timer) -> FineDuration {
        match (self, earlier, timer) {
            (Self::Os(this), Self::Os(earlier), Timer::Os) => this.duration_since(earlier).into(),
            (Self::Tsc(this), Self::Tsc(earlier), Timer::Tsc { frequency }) => {
                this.duration_since(earlier, frequency.get())
            }
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
    pub tsc: TscTimestamp,
}

impl AnyTimestamp {
    #[inline(always)]
    pub fn now(timer_kind: TimerKind) -> Self {
        match timer_kind {
            TimerKind::Os => Self { os: Instant::now() },
            TimerKind::Tsc => Self { tsc: TscTimestamp::now() },
        }
    }

    #[inline(always)]
    pub unsafe fn into_timestamp(self, timer_kind: TimerKind) -> Timestamp {
        match timer_kind {
            TimerKind::Os => Timestamp::Os(self.os),
            TimerKind::Tsc => Timestamp::Tsc(self.tsc),
        }
    }
}
