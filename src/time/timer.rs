use std::num::NonZeroU64;

use crate::time::TscTimestamp;

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
    pub fn get_tsc() -> Option<Self> {
        if cfg!(miri) {
            // Miri does not support inline assembly.
            return None;
        }

        Some(Self::Tsc { frequency: NonZeroU64::new(TscTimestamp::frequency()?)? })
    }

    #[inline]
    pub fn is_tsc(self) -> bool {
        matches!(self, Self::Tsc { .. })
    }

    #[inline]
    pub fn tsc_frequency(self) -> Option<u64> {
        match self {
            Self::Os => None,
            Self::Tsc { frequency } => Some(frequency.get()),
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
