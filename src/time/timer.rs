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
