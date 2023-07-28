use std::num::NonZeroU64;

use crate::time::{TscTimestamp, TscUnavailable};

/// Measures time.
#[derive(Clone, Copy, Default)]
pub enum Timer {
    /// Operating system timer.
    #[default]
    Os,

    /// CPU timestamp counter.
    Tsc {
        /// [`TscTimestamp::frequency`].
        frequency: NonZeroU64,
    },
}

impl Timer {
    /// Returns all available timers.
    #[cfg(test)]
    pub fn available() -> Vec<Self> {
        let mut timers = vec![Self::Os];

        if let Ok(tsc) = Self::get_tsc() {
            timers.push(tsc);
        }

        timers
    }

    /// Attempts to get the CPU timestamp counter.
    #[inline]
    pub fn get_tsc() -> Result<Self, TscUnavailable> {
        Ok(Self::Tsc { frequency: TscTimestamp::frequency()? })
    }

    #[inline]
    pub fn kind(self) -> TimerKind {
        match self {
            Self::Os => TimerKind::Os,
            Self::Tsc { .. } => TimerKind::Tsc,
        }
    }
}

/// [`Timer`] kind.
#[derive(Clone, Copy, Default)]
pub enum TimerKind {
    /// Operating system timer.
    #[default]
    Os,

    /// CPU timestamp counter.
    Tsc,
}
