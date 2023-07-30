use std::{
    alloc::*,
    cell::Cell,
    ptr::{self, NonNull},
    sync::atomic::{AtomicPtr, Ordering::*},
};

use crate::{
    stats::StatsSet,
    util::{self, LocalCount, SharedCount},
};

// Use `AllocProfiler` when running crate-internal tests. This enables us to
// test it for undefined behavior with Miri.
#[cfg(test)]
#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

/// Initializes the current thread's allocation information object.
///
/// Note that despite the work done here, we still initialize thread-local alloc
/// info in the `GlobalAlloc` implementation for `Alloc` because benchmarks can
/// allocate their own threads.
pub(crate) fn init_current_thread_info() {
    // Allocate the info object using the `System` allocator since that's
    // convenient in this context. We may want to use the true global allocator
    // instead.
    AllocProfiler::system().current_thread_info().make_reusable();
}

/// Measures [`GlobalAlloc`] memory usage.
///
/// # Examples
///
/// The default usage is to create a
/// [`#[global_allocator]`](macro@global_allocator) that wraps the [`System`]
/// allocator with [`AllocProfiler::system()`]:
///
/// ```
/// use std::collections::*;
/// use divan::AllocProfiler;
///
/// #[global_allocator]
/// static ALLOC: AllocProfiler = AllocProfiler::system();
///
/// fn main() {
///     divan::main();
/// }
///
/// #[divan::bench(types = [
///     Vec<i32>,
///     LinkedList<i32>,
///     HashSet<i32>,
/// ])]
/// fn from_iter<T>() -> T
/// where
///     T: FromIterator<i32>,
/// {
///     (0..100).collect()
/// }
///
/// #[divan::bench(types = [
///     Vec<i32>,
///     LinkedList<i32>,
///     HashSet<i32>,
/// ])]
/// fn drop<T>(bencher: divan::Bencher)
/// where
///     T: FromIterator<i32>,
/// {
///     bencher
///         .with_inputs(|| (0..100).collect::<T>())
///         .bench_values(std::mem::drop);
/// }
/// ```
///
/// Wrap other [`GlobalAlloc`] implementations like
/// [`mimalloc`](https://docs.rs/mimalloc) with [`AllocProfiler::new()`]:
///
/// ```
/// use divan::AllocProfiler;
/// use mimalloc::MiMalloc;
///
/// # #[cfg(not(miri))]
/// #[global_allocator]
/// static ALLOC: AllocProfiler<MiMalloc> = AllocProfiler::new(MiMalloc);
/// ```
///
/// See [`string`](https://github.com/nvzqz/divan/blob/main/examples/benches/string.rs)
/// and [`collections`](https://github.com/nvzqz/divan/blob/main/examples/benches/collections.rs)
/// benchmarks for more examples.
///
/// # Implementation
///
/// Allocation information is recorded in thread-local storage to prevent
/// contention when benchmarks involve multiple threads, either internally or
/// through options like [`threads`](macro@crate::bench#threads). This is
/// currently achieved with the portable [`thread_local!`] macro. To reduce
/// Divan's footprint, the implementation in the future will take advantage of
/// faster platform-specific approaches, such as
/// [`pthread_getspecific`](https://pubs.opengroup.org/onlinepubs/9699919799/functions/pthread_getspecific.html)
/// on macOS.
#[derive(Debug, Default)]
pub struct AllocProfiler<Alloc = System> {
    alloc: Alloc,
}

unsafe impl<A: GlobalAlloc> GlobalAlloc for AllocProfiler<A> {
    // NOTE: Within `alloc` we can't access `REUSE_THREAD_INFO` because
    // thread-locals with `Drop` crash:
    // https://github.com/rust-lang/rust/issues/116390.
    //
    // To get around this, we initialize the drop handle at two locations:
    // 1. Within `sync_threads` immediately before the sample loop. This covers
    //    all benchmarks that don't spawn their own threads, so most of them.
    // 2. Within `dealloc` since it will likely be called before thread
    //    termination.

    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Tally allocation count.
        self.current_thread_info().tally(AllocOp::Alloc, layout.size());

        self.alloc.alloc(layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // Tally allocation count.
        self.current_thread_info().tally(AllocOp::Alloc, layout.size());

        self.alloc.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: `ptr` must come from this allocator, so we can assume
        // thread-local info has been initialized during allocation.
        let info = unsafe { CURRENT_THREAD_INFO.get().unwrap_unchecked() };

        // Tally reallocation count.
        let shrink = new_size < layout.size();
        info.tally(
            AllocOp::realloc(shrink),
            if shrink { layout.size() - new_size } else { new_size - layout.size() },
        );

        self.alloc.realloc(ptr, layout, new_size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` must come from this allocator, so we can assume
        // thread-local info has been initialized during allocation.
        let info = unsafe { CURRENT_THREAD_INFO.get().unwrap_unchecked() };

        // Enable info to be reused after thread termination.
        info.make_reusable();

        // Tally deallocation count.
        info.tally(AllocOp::Dealloc, layout.size());

        self.alloc.dealloc(ptr, layout)
    }
}

impl AllocProfiler {
    /// Profiles the [`System`] allocator.
    #[inline]
    pub const fn system() -> Self {
        Self::new(System)
    }
}

impl<A> AllocProfiler<A> {
    /// Profiles a [`GlobalAlloc`].
    #[inline]
    pub const fn new(alloc: A) -> Self {
        Self { alloc }
    }
}

impl<A: GlobalAlloc> AllocProfiler<A> {
    #[inline]
    fn current_thread_info(&self) -> &'static ThreadAllocInfo {
        CURRENT_THREAD_INFO.with(|local_info| match local_info.get() {
            Some(info) => info,

            // PERF: Don't provide `local_info` to the helper because doing so
            // generates more code here.
            None => self.current_thread_info_slow(),
        })
    }

    #[cold]
    #[inline(never)]
    fn current_thread_info_slow(&self) -> &'static ThreadAllocInfo {
        let info = 'info: {
            // Attempt to reuse a previously-allocated instance from a terminated
            // thread relinquishing its claim on this allocation.
            if let Some(info) = ALL_THREAD_INFO.pop_reuse_list() {
                break 'info info;
            }

            // Allocate a new instance since none are available for reuse.
            //
            // We do not report this allocation because it was not invoked by the
            // benchmarked code.
            unsafe {
                let info: *mut ThreadAllocInfo =
                    self.alloc.alloc_zeroed(Layout::new::<ThreadAllocInfo>()).cast();

                // Default to global instance on allocation failure.
                if info.is_null() {
                    break 'info &ALL_THREAD_INFO;
                }

                // This allocation is reused by setting `ALL_THREAD_INFO.reuse_next`
                // to `info` on thread termination via `REUSE_THREAD_INFO` drop.
                util::miri::leak(info);

                // Make `info` discoverable by pushing it onto the global
                // linked list as the new head.
                let mut current_head = ALL_THREAD_INFO.next.load(Acquire);
                loop {
                    // Prepare `current_head` to become second node in the list.
                    (*info).next = AtomicPtr::new(current_head);

                    // Replace head node with `info`.
                    match ALL_THREAD_INFO.next.compare_exchange(current_head, info, AcqRel, Acquire)
                    {
                        // Successfully set our list head.
                        Ok(_) => {
                            let info = &*info;
                            break 'info info;
                        }

                        // Other thread set their list head.
                        Err(their_head) => current_head = their_head,
                    }
                }
            }
        };

        CURRENT_THREAD_INFO.set(Some(info));

        info
    }
}

/// Thread-local allocation information.
///
/// This represents a tree consisting of two independent linked-lists:
/// - `next` stores all instances.
/// - `reuse_next` stores reusable instances from terminated threads.
pub(crate) struct ThreadAllocInfo {
    pub tallies: SharedAllocTallyMap,

    /// The next instance in a global linked list.
    ///
    /// `ALL_THREAD_INFO` is the head of a global linked list that is only ever
    /// pushed to access all instances that have ever been created in order to
    /// accumulate stats.
    ///
    /// This field is initialized at most once.
    next: AtomicPtr<ThreadAllocInfo>,

    /// The next available instance for reuse.
    ///
    /// `ALL_THREAD_INFO.reuse_count` is the head of a global linked list that
    /// can be popped to reuse previously-allocated instances.
    ///
    /// This may be set to 1 to indicate `REUSE_THREAD_INFO` has been
    /// initialized. See `ThreadAllocInfo::make_reusable`.
    reuse_next: AtomicPtr<ThreadAllocInfo>,
}

/// Reclaims `ThreadAllocInfo` allocations on `Drop`.
///
/// This avoids allocating new thread info and pushing it to
/// `ALL_THREAD_INFO.next`.
struct ThreadInfoReuseHandle {
    info: &'static ThreadAllocInfo,
}

impl Drop for ThreadInfoReuseHandle {
    #[inline]
    fn drop(&mut self) {
        let mut current_head = ALL_THREAD_INFO.reuse_next.load(Acquire);

        loop {
            // Prepare `current_head` to become second node in the list.
            self.info.reuse_next.store(current_head, Relaxed);

            // Replace head node with `our_head`.
            match ALL_THREAD_INFO.reuse_next.compare_exchange(
                current_head,
                self.info as *const _ as *mut _,
                AcqRel,
                Acquire,
            ) {
                // We updated `self.reuse_next`.
                Ok(_) => return,

                Err(their_head) => current_head = their_head,
            }
        }
    }
}

thread_local! {
    /// Instance specific to the current thread.
    static CURRENT_THREAD_INFO: Cell<Option<&'static ThreadAllocInfo>> = const { Cell::new(None) };
}

thread_local! {
    /// When a thread terminates, this will be dropped and the allocation will
    /// be reclaimed for reuse.
    static REUSE_THREAD_INFO: Cell<Option<ThreadInfoReuseHandle>> = const { Cell::new(None) };
}

/// Global instance for thread information.
///
/// This is used as:
/// - The start of the linked list of all info instances.
/// - The owner of the linked list of reusable info instances.
/// - A last-resort instance on allocation failure.
static ALL_THREAD_INFO: ThreadAllocInfo = ThreadAllocInfo {
    tallies: SharedAllocTallyMap::EMPTY,
    next: AtomicPtr::new(ptr::null_mut()),
    reuse_next: AtomicPtr::new(ptr::null_mut()),
};

impl ThreadAllocInfo {
    #[inline]
    pub fn all() -> impl Iterator<Item = &'static Self> {
        std::iter::successors(Some(&ALL_THREAD_INFO), |current| unsafe {
            current.next.load(Acquire).as_ref()
        })
    }

    /// Returns the current thread's allocation information if initialized.
    #[inline]
    pub fn try_current() -> Option<&'static Self> {
        CURRENT_THREAD_INFO.get()
    }

    /// Sets 0 to all values.
    pub fn clear(&self) {
        for value in &self.tallies.values {
            value.count.store(0, Relaxed);
            value.size.store(0, Relaxed);
        }
    }

    /// Tallies the total count and size of the allocation operation.
    #[inline]
    fn tally(&self, op: AllocOp, size: usize) {
        let tally = self.tallies.get(op);
        tally.count.fetch_add(1, Relaxed);
        tally.size.fetch_add(size as LocalCount, Relaxed);
    }

    /// Registers `self` with `REUSE_THREAD_INFO` so that it can be reused on
    /// thread termination via `Drop` of `ThreadInfoReuseHandle`.
    #[inline]
    fn make_reusable(&'static self) {
        // This is 1 from `ptr::from_exposed_addr`, but usable in stable.
        const IS_REUSABLE: *mut u8 = NonNull::dangling().as_ptr();

        // PERF: We check `reuse_next` pointer instead of always accessing
        // `REUSE_THREAD_INFO` because:
        // - We reduce code emitted in `Alloc::dealloc` by using a separate
        //   function for thread-local access and dtor initialization.
        // - Thread-local access always goes through a function call, whereas an
        //   atomic load is about as fast as a non-synchronized load.
        //
        // Relaxed loads are fine here because we're not accessing memory
        // through the pointer.
        if self.reuse_next.load(Relaxed).cast() != IS_REUSABLE {
            slow_impl(self);
        }

        #[cold]
        fn slow_impl(info: &'static ThreadAllocInfo) {
            // Although it is unlikely we fail to allocate and use the global
            // instance, skip it in case.
            if ptr::eq(info, &ALL_THREAD_INFO) {
                return;
            }

            // Initialize dtor for thread-local.
            //
            // If this is being called during `dealloc` on thread termination,
            // then the thread info will be leaked.
            _ = REUSE_THREAD_INFO.try_with(|local| local.set(Some(ThreadInfoReuseHandle { info })));

            // Mark as claimed for reuse.
            info.reuse_next.store(IS_REUSABLE.cast(), Relaxed);
        }
    }

    /// Pops the head element off of the `reuse_next` linked list.
    ///
    /// When a thread terminates, `REUSE_THREAD_INFO` will relinquish this
    /// thread's claim on its `ThreadAllocInfo` instance by pushing it onto the
    /// `ALL_THREAD_INFO.reuse_next` linked list. This method reclaims the
    /// instance for the current thread.
    #[inline]
    fn pop_reuse_list(&self) -> Option<&'static ThreadAllocInfo> {
        unsafe {
            let mut current_head = self.reuse_next.load(Relaxed);

            loop {
                let current = match current_head as usize {
                    0 | 1 => return None,
                    _ => &*current_head,
                };

                // Replace `current_head` with its next node.
                let our_head = current.reuse_next.load(Relaxed);
                match self.reuse_next.compare_exchange(current_head, our_head, AcqRel, Acquire) {
                    // Successfully set our list head.
                    Ok(_) => return Some(current),

                    // Other thread set their list head.
                    Err(their_head) => current_head = their_head,
                }
            }
        }
    }
}

/// Allocation numbers being accumulated.
///
/// This uses [`SharedCount`], which is `AtomicU64` (if available) or
/// `AtomicUsize`.
#[derive(Clone, Copy, Default)]
#[repr(C)]
pub(crate) struct AllocTally<Count> {
    /// The number of times this operation was performed.
    pub count: Count,

    /// The amount of memory this operation changed.
    pub size: Count,
}

pub(crate) type SharedAllocTally = AllocTally<SharedCount>;

pub(crate) type LocalAllocTally = AllocTally<LocalCount>;

pub(crate) type TotalAllocTally = AllocTally<u128>;

impl LocalAllocTally {
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.count == 0 && self.size == 0
    }
}

impl AllocTally<StatsSet<f64>> {
    pub fn is_zero(&self) -> bool {
        self.count.is_zero() && self.size.is_zero()
    }
}

impl<C> AllocTally<C> {
    #[inline]
    pub fn as_array(&self) -> &[C; 2] {
        // SAFETY: This is `#[repr(C)]`, so we can treat it as a contiguous
        // sequence of items.
        unsafe { &*(self as *const _ as *const _) }
    }
}

/// Allocation number categories.
///
/// Note that grow/shrink are first to improve code generation for `realloc`.
#[derive(Clone, Copy)]
pub(crate) enum AllocOp {
    Grow,
    Shrink,
    Alloc,
    Dealloc,
}

impl AllocOp {
    pub const ALL: [Self; 4] = {
        use AllocOp::*;

        // Use same order as declared so that it can be indexed as-is.
        [Grow, Shrink, Alloc, Dealloc]
    };

    #[inline]
    pub fn realloc(shrink: bool) -> Self {
        // This generates the same code as `std::mem::transmute`.
        if shrink {
            Self::Shrink
        } else {
            Self::Grow
        }
    }

    #[inline]
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Grow => "grow:",
            Self::Shrink => "shrink:",
            Self::Alloc => "alloc:",
            Self::Dealloc => "dealloc:",
        }
    }
}

/// Values keyed by `AllocOp`.
#[derive(Clone, Copy, Default)]
pub(crate) struct AllocOpMap<T> {
    pub values: [T; 4],
}

pub(crate) type SharedAllocTallyMap = AllocOpMap<SharedAllocTally>;

pub(crate) type LocalAllocTallyMap = AllocOpMap<LocalAllocTally>;

pub(crate) type TotalAllocTallyMap = AllocOpMap<TotalAllocTally>;

impl SharedAllocTallyMap {
    /// A map with all values set to 0.
    #[allow(clippy::declare_interior_mutable_const)]
    pub const EMPTY: Self = {
        const ZERO: SharedAllocTally =
            SharedAllocTally { size: SharedCount::new(0), count: SharedCount::new(0) };

        Self { values: [ZERO; 4] }
    };

    pub fn load(&self) -> LocalAllocTallyMap {
        LocalAllocTallyMap {
            values: AllocOp::ALL.map(|op| {
                let value = &self.values[op as usize];
                AllocTally { count: value.count.load(Relaxed), size: value.size.load(Relaxed) }
            }),
        }
    }
}

impl LocalAllocTallyMap {
    /// Returns `true` if all tallies are 0.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.values.iter().all(LocalAllocTally::is_zero)
    }

    pub fn add_to_total(&self, total: &mut TotalAllocTallyMap) {
        for (i, value) in self.values.iter().enumerate() {
            total.values[i].count += value.count as u128;
            total.values[i].size += value.size as u128;
        }
    }
}

impl<T> AllocOpMap<T> {
    #[inline]
    pub const fn get(&self, op: AllocOp) -> &T {
        &self.values[op as usize]
    }
}
