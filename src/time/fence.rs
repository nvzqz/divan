use std::sync::atomic;

/// Prevents other operations from affecting timing measurements.
#[inline(always)]
pub fn full_fence() {
    atomic::fence(atomic::Ordering::SeqCst);
}

/// Prevents the compiler from reordering operations.
#[inline(always)]
pub fn compiler_fence() {
    atomic::compiler_fence(atomic::Ordering::SeqCst);
}
