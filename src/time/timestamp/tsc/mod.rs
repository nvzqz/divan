use crate::time::FineDuration;

// TODO: x86_64 via `rdtsc`.

#[cfg(target_arch = "aarch64")]
#[path = "aarch64.rs"]
mod arch;

/// [CPU timestamp counter](https://en.wikipedia.org/wiki/Time_Stamp_Counter).
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct TscTimestamp {
    pub value: u64,
}

impl TscTimestamp {
    /// Reads the timestamp frequency.
    #[inline]
    pub fn frequency() -> Option<u64> {
        #[cfg(target_arch = "aarch64")]
        return Some(arch::frequency());

        #[allow(unreachable_code)]
        None
    }

    /// Reads the timestamp counter.
    #[inline(always)]
    pub fn now() -> Self {
        #[allow(unused)]
        let value = 0;

        #[cfg(target_arch = "aarch64")]
        let value = arch::timestamp();

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
