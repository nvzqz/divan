use std::{cell::UnsafeCell, mem::MaybeUninit};

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
    #[inline]
    pub fn prepare(&mut self, sample_size: usize) {
        self.entries.clear();
        self.entries.reserve_exact(sample_size);

        // SAFETY: `DeferEntry` only contains `MaybeUninit` fields, so
        // `MaybeUninit<DeferEntry>` may be safely represented as `DeferEntry`.
        unsafe { self.entries.set_len(sample_size) }
    }

    /// Returns the sample's entries for iteration.
    ///
    /// The caller is expected to use the returned slice to initialize inputs
    /// for the sample loop.
    #[inline]
    pub fn entries(&self) -> &[DeferEntry<I, O>] {
        &self.entries
    }
}

/// Storage for a single iteration within a sample.
///
/// Input is stored before output to improve cache prefetching since iteration
/// progresses from low to high addresses.
///
/// # UnsafeCell
///
/// `UnsafeCell` is used to allow `output` to safely refer to `input`. Although
/// `output` itself is never aliased, it is also stored as `UnsafeCell` in order
/// to get mutable access through a shared `&DeferEntry`.
///
/// # Safety
///
/// All fields **must** be `MaybeUninit`. This allows us to safely set the
/// length of `Vec<DeferEntry>` within the allocated capacity.
#[repr(C)]
pub(crate) struct DeferEntry<I, O> {
    pub input: UnsafeCell<MaybeUninit<I>>,
    pub output: UnsafeCell<MaybeUninit<O>>,
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
        entry_ref.input = UnsafeCell::new(MaybeUninit::new(String::new()));
        entry_ref.output = UnsafeCell::new(MaybeUninit::new(String::new()));

        unsafe {
            let entry = entry.assume_init();
            assert_eq!(entry.input.into_inner().assume_init(), "");
            assert_eq!(entry.output.into_inner().assume_init(), "");
        }
    }

    /// Tests that accessing `DeferEntry.input` through an aliased reference in
    /// `DeferEntry.output` is safe due `input` being an `UnsafeCell`.
    #[test]
    fn access_aliased_input() {
        struct Output<'i> {
            input: &'i mut String,
        }

        impl Drop for Output<'_> {
            fn drop(&mut self) {
                assert_eq!(self.input, "hello");
                self.input.push_str(" world");
            }
        }

        let entry: MaybeUninit<DeferEntry<String, Output>> = MaybeUninit::uninit();
        let entry_ref = unsafe { entry.assume_init_ref() };

        // Loop to ensure previous iterations don't affect later uses of the
        // same entry slot.
        for _ in 0..5 {
            unsafe {
                let input_ptr = entry_ref.input.get().cast::<String>();
                let output_ptr = entry_ref.output.get().cast::<Output>();

                // Initialize input and output.
                input_ptr.write("hello".to_owned());
                output_ptr.write(Output { input: &mut *input_ptr });

                // Use and discard output.
                assert_eq!((*output_ptr).input, "hello");
                output_ptr.drop_in_place();
                assert_eq!(&*input_ptr, "hello world");

                // Discard input.
                input_ptr.drop_in_place();
            }
        }
    }
}
