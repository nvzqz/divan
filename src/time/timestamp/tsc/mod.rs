use crate::time::FineDuration;

#[cfg(target_arch = "aarch64")]
#[path = "aarch64.rs"]
mod arch;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[path = "x86.rs"]
mod arch;

/// [CPU timestamp counter](https://en.wikipedia.org/wiki/Time_Stamp_Counter).
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct TscTimestamp {
    pub value: u64,
}

impl TscTimestamp {
    /// Gets the timestamp frequency.
    ///
    /// On AArch64, this simply reads `cntfrq_el0`. On x86, this measures the
    /// TSC frequency.
    #[inline]
    #[allow(unreachable_code)]
    pub fn frequency() -> Option<u64> {
        // Miri does not support inline assembly.
        #[cfg(miri)]
        return None;

        #[cfg(any(target_arch = "aarch64", target_arch = "x86", target_arch = "x86_64"))]
        return arch::frequency();

        None
    }

    /// Reads the timestamp counter.
    #[inline(always)]
    pub fn start() -> Self {
        #[allow(unused)]
        let value = 0;

        #[cfg(target_arch = "aarch64")]
        let value = arch::timestamp();

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        let value = arch::start_timestamp();

        Self { value }
    }

    /// Reads the timestamp counter.
    #[inline(always)]
    pub fn end() -> Self {
        #[allow(unused)]
        let value = 0;

        #[cfg(target_arch = "aarch64")]
        let value = arch::timestamp();

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        let value = arch::end_timestamp();

        Self { value }
    }

    pub fn duration_since(self, earlier: Self, frequency: u64) -> FineDuration {
        if earlier.value > self.value {
            return Default::default();
        }

        const PICOS: u128 = 1_000_000_000_000;

        let diff = self.value as u128 - earlier.value as u128;

        FineDuration { picos: (diff * PICOS) / frequency as u128 }
    }
}
