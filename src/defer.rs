use std::mem::MaybeUninit;

/// Stores input and output values together to defer usage and drop during
/// benchmarking.
///
/// Inputs are stored are stored contiguously with outputs in memory. This
/// improves performance by:
/// - Removing the overhead of `zip` between two separate buffers.
/// - Improving cache locality and cache prefetching. Input is strategically
///   placed before output because iteration is from low to high addresses, so
///   doing this makes memory access patterns very predictable.
pub(crate) struct DeferStore<I, O> {
    // TODO: Reduce memory usage by not allocating space for outputs when they
    // do not need to be dropped.
    entries: Vec<DeferEntry<I, O>>,
}

impl<I, O> Default for DeferStore<I, O> {
    #[inline]
    fn default() -> Self {
        Self { entries: Vec::new() }
    }
}

impl<I, O> DeferStore<I, O> {
    /// Prepares storage for iterating over entries for a sample.
    ///
    /// The caller is expected to use the returned slice to initialize inputs
    /// for the sample loop.
    #[inline]
    pub fn prepare(&mut self, sample_size: usize) -> &mut [DeferEntry<I, O>] {
        self.entries.clear();
        self.entries.reserve_exact(sample_size);

        // SAFETY: `DeferEntry` only contains `MaybeUninit` fields, so
        // `MaybeUninit<DeferEntry>` may be safely represented as `DeferEntry`.
        unsafe { self.entries.set_len(sample_size) }

        &mut self.entries
    }

    /// Drops every `DeferEntry.output`.
    ///
    /// # Safety
    ///
    /// Outputs must have been initialized before this call.
    #[inline]
    pub unsafe fn drop_outputs(&mut self) {
        for entry in &mut self.entries {
            entry.output.assume_init_drop();
        }
        self.entries.clear();
    }
}

/// Storage for a single iteration within a sample.
///
/// Input is stored before output to improve cache prefetching since iteration
/// progresses from low to high addresses.
///
/// # Safety
///
/// All fields **must** be `MaybeUninit`. This allows us to safely set the
/// length of `Vec<DeferEntry>` within the allocated capacity.
#[repr(C)]
pub(crate) struct DeferEntry<I, O> {
    pub input: MaybeUninit<I>,
    pub output: MaybeUninit<O>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that accessing an uninitialized `DeferEntry` is safe due to all of
    /// its fields being `MaybeUninit`.
    #[test]
    fn access_uninit_entry() {
        let mut entry: MaybeUninit<DeferEntry<String, String>> = MaybeUninit::uninit();

        let entry_ref = unsafe { entry.assume_init_mut() };
        entry_ref.input = MaybeUninit::new(String::new());
        entry_ref.output = MaybeUninit::new(String::new());

        unsafe {
            let entry = entry.assume_init();
            assert_eq!(entry.input.assume_init(), "");
            assert_eq!(entry.output.assume_init(), "");
        }
    }
}
