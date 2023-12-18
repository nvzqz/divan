use std::{
    alloc::*,
    ptr,
    sync::atomic::{AtomicPtr, Ordering::*},
};

use cfg_if::cfg_if;

use crate::{
    stats::StatsSet,
    util::{self, CachePadded, LocalCount, SharedCount},
};

#[cfg(target_os = "macos")]
use crate::util::sync::PThreadKey;

#[cfg(not(target_os = "macos"))]
use std::cell::Cell;

// Use `AllocProfiler` when running crate-internal tests. This enables us to
// test it for undefined behavior with Miri.
#[cfg(test)]
#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

/// Makes the current thread's `ThreadAllocInfo` instance reusable.
///
/// This is a no-op on macOS because we instead use the `pthread_key_create`
/// destructor.
#[inline]
pub(crate) fn init_current_thread_info() {
    // If `AllocProfiler` is the global allocator, it will have initialized the
    // current thread's `ThreadAllocInfo` because we have already allocated by
    // this point.
    #[cfg(not(target_os = "macos"))]
    if let Some(info) = ThreadAllocInfo::try_current() {
        info.reuse_on_thread_dtor();
    }
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
/// Collecting allocation information happens at any point during which Divan is
/// also measuring the time. As a result, counting allocations affects timing.
///
/// To reduce Divan's footprint during benchmarking:
/// - Allocation information is recorded in thread-local storage to prevent
///   contention when benchmarks involve multiple threads, either through
///   options like [`threads`](macro@crate::bench#threads) or internally
///   spawning their own threads.
/// - It does not check for overflow and assumes it will not happen. This is
///   subject to change in the future.
/// - Fast thread-local storage access is assembly-optimized on macOS.
///
/// Allocation information is the only data Divan records outside of timing, and
/// thus it also has the only code that affects timing. Recording of alloc info
/// takes place in 3 steps:
/// 1. Load the thread-local slot for allocation information.
///
///    On macOS, this is via the
///    [`gs`](https://github.com/nvzqz/divan/blob/v0.1.6/src/util/sync.rs#L34)/[`tpidrro_el0`](https://github.com/nvzqz/divan/blob/v0.1.6/src/util/sync.rs#L47)
///    registers. Although this is not guaranteed as stable ABI, in practice
///    many programs assume these registers store thread-local data.
///    [`thread_local!`] is used on all other platforms.
///
/// 2. Perform a [`fetch_add`](SharedCount::fetch_add) for allocation operation
///    invocation count.
///
/// 3. Perform a [`fetch_add`](SharedCount::fetch_add) for allocation operation
///    bytes count (a.k.a. size).
///
/// Allocation information is recorded in thread-local storage to prevent
/// atomics contention when benchmarks involve multiple threads, through options
/// like [`threads`](macro@crate::bench#threads) or internally spawning their
/// own threads.
///
/// This is currently achieved with:
/// - [`thread_local!`] on most platforms
/// - [`pthread_getspecific`](https://pubs.opengroup.org/onlinepubs/9699919799/functions/pthread_getspecific.html)
///   via registers on macOS, as mentioned earlier
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
        // `ptr` must come from this allocator, so we can assume thread-local
        // info has been initialized during allocation.
        let info = ThreadAllocInfo::current_or_global();

        // Tally reallocation count.
        let shrink = new_size < layout.size();
        info.tally(
            AllocOp::realloc(shrink),
            if shrink { layout.size() - new_size } else { new_size - layout.size() },
        );

        self.alloc.realloc(ptr, layout, new_size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // `ptr` must come from this allocator, so we can assume thread-local
        // info has been initialized during allocation.
        let info = ThreadAllocInfo::current_or_global();

        // Enable info to be reused after thread termination.
        #[cfg(not(target_os = "macos"))]
        info.reuse_on_thread_dtor();

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
        match ThreadAllocInfo::try_current() {
            Some(info) => info,

            // PERF: Don't provide `local_info` to the helper because doing so
            // generates more code here.
            None => self.current_thread_info_slow(),
        }
    }

    #[cold]
    #[inline(never)]
    fn current_thread_info_slow(&self) -> &'static ThreadAllocInfo {
        let info = 'info: {
            // Attempt to reuse a previously-allocated instance from a terminated
            // thread relinquishing its claim on this allocation.
            if let Some(info) = ALLOC_META.thread_info_head.pop_reuse_list() {
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
                    break 'info &ALLOC_META.thread_info_head;
                }

                // This allocation is reused by setting `ALL_THREAD_INFO.reuse_next`
                // to `info` on thread termination via `REUSE_THREAD_INFO` drop.
                util::miri::leak(info);

                // Make `info` discoverable by pushing it onto the global
                // linked list as the new head.
                let mut current_head = ALLOC_META.thread_info_head.next.load(Relaxed);
                loop {
                    // Prepare `current_head` to become second node in the list.
                    (*info).next = AtomicPtr::new(current_head);

                    // Replace head node with `info`.
                    match ALLOC_META.thread_info_head.next.compare_exchange(
                        current_head,
                        info,
                        AcqRel,
                        Acquire,
                    ) {
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

        info.set_as_current();
        info
    }
}

/// Thread-local allocation information.
///
/// This represents a tree consisting of two independent linked-lists:
/// - `next` stores all instances.
/// - `reuse_next` stores reusable instances from terminated threads.
#[repr(C)]
pub(crate) struct ThreadAllocInfo {
    pub tallies: CachePadded<SharedAllocTallyMap>,

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
    /// initialized. See `ThreadAllocInfo::reuse_on_thread_dtor`.
    reuse_next: AtomicPtr<ThreadAllocInfo>,
}

/// Reclaims `ThreadAllocInfo` allocations on `Drop`.
///
/// This avoids allocating new thread info and pushing it to
/// `ALL_THREAD_INFO.next`.
#[cfg(not(target_os = "macos"))]
struct ThreadInfoReuseHandle {
    info: &'static ThreadAllocInfo,
}

#[cfg(not(target_os = "macos"))]
impl Drop for ThreadInfoReuseHandle {
    #[inline]
    fn drop(&mut self) {
        self.info.reuse()
    }
}

#[cfg(not(target_os = "macos"))]
thread_local! {
    /// Instance specific to the current thread.
    ///
    /// On macOS, we use `ALLOC_META.pthread_key` instead.
    static CURRENT_THREAD_INFO: Cell<Option<&'static ThreadAllocInfo>> = const { Cell::new(None) };
}

#[cfg(not(target_os = "macos"))]
thread_local! {
    /// When a thread terminates, this will be dropped and the allocation will
    /// be reclaimed for reuse.
    ///
    /// On macOS, we use the `pthread_key_create` destructor.
    static REUSE_THREAD_INFO: Cell<Option<ThreadInfoReuseHandle>> = const { Cell::new(None) };
}

#[repr(C)]
struct ThreadAllocMeta {
    #[cfg(target_os = "macos")]
    pthread_key: CachePadded<PThreadKey<ThreadAllocInfo>>,

    /// This is used as:
    /// - The start of the linked list of all info instances.
    /// - The owner of the linked list of reusable info instances.
    /// - A last-resort info instance on allocation failure.
    thread_info_head: ThreadAllocInfo,
}

/// Global instance for allocation metadata.
static ALLOC_META: ThreadAllocMeta = ThreadAllocMeta {
    #[cfg(target_os = "macos")]
    pthread_key: CachePadded(PThreadKey::new()),

    thread_info_head: ThreadAllocInfo {
        tallies: CachePadded(SharedAllocTallyMap::EMPTY),
        next: AtomicPtr::new(ptr::null_mut()),
        reuse_next: AtomicPtr::new(ptr::null_mut()),
    },
};

impl ThreadAllocInfo {
    #[inline]
    pub fn all() -> impl Iterator<Item = &'static Self> {
        std::iter::successors(Some(&ALLOC_META.thread_info_head), |current| unsafe {
            current.next.load(Acquire).as_ref()
        })
    }

    /// Assigns `self` as the current thread's info.
    ///
    /// On macOS, each thread races to assign `ALLOC_META.pthread_key` via
    /// `pthread_key_create`.
    #[inline]
    pub fn set_as_current(&'static self) {
        cfg_if! {
            if #[cfg(target_os = "macos")] {
                // Assign `self` and later push it onto the reuse linked list
                // when the thread terminates.
                ALLOC_META.pthread_key.0.set(self, ThreadAllocInfo::reuse);
            } else {
                CURRENT_THREAD_INFO.set(Some(self));
            }
        }
    }

    /// Returns the current thread's allocation information if initialized.
    #[inline]
    pub fn try_current() -> Option<&'static Self> {
        cfg_if! {
            if #[cfg(target_os = "macos")] {
                ALLOC_META.pthread_key.0.get()
            } else {
                CURRENT_THREAD_INFO.get()
            }
        }
    }

    /// Returns the current thread's allocation information if initialized, or
    /// the global instance.
    #[inline]
    pub fn current_or_global() -> &'static Self {
        Self::try_current().unwrap_or(&ALLOC_META.thread_info_head)
    }

    /// Sets 0 to all values.
    pub fn clear(&self) {
        for value in &self.tallies.0.values {
            value.count.store(0, Relaxed);
            value.size.store(0, Relaxed);
        }
    }

    /// Tallies the total count and size of the allocation operation.
    #[inline]
    fn tally(&self, op: AllocOp, size: usize) {
        self.tally_n(op, 1, size);
    }

    /// Tallies the total count and size of the allocation operation.
    #[inline]
    fn tally_n(&self, op: AllocOp, count: usize, size: usize) {
        let tally = self.tallies.0.get(op);
        tally.count.fetch_add(count as LocalCount, Relaxed);
        tally.size.fetch_add(size as LocalCount, Relaxed);
    }

    /// Pushes `self` to the start of the reuse linked list.
    fn reuse(&'static self) {
        let mut current_head = ALLOC_META.thread_info_head.reuse_next.load(Relaxed);

        loop {
            // Prepare `current_head` to become second node in the list.
            self.reuse_next.store(current_head, Relaxed);

            // Replace head node with `self`.
            match ALLOC_META.thread_info_head.reuse_next.compare_exchange(
                current_head,
                self as *const _ as *mut _,
                AcqRel,
                Acquire,
            ) {
                // We updated `self.reuse_next`.
                Ok(_) => return,

                Err(their_head) => current_head = their_head,
            }
        }
    }

    /// Registers `self` with `REUSE_THREAD_INFO` so that it can be reused on
    /// thread termination via `Drop` of `ThreadInfoReuseHandle`.
    #[inline]
    #[cfg(not(target_os = "macos"))]
    fn reuse_on_thread_dtor(&'static self) {
        // This is 1 from `ptr::from_exposed_addr`, but usable in stable.
        const IS_REUSABLE: *mut u8 = ptr::NonNull::dangling().as_ptr();

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
            if ptr::eq(info, &ALLOC_META.thread_info_head) {
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

#[cfg(feature = "internal_benches")]
mod thread_info {
    use super::*;

    // We want the approach to scale well with thread count.
    const THREADS: &[usize] = &[0, 1, 2, 4, 16];

    #[crate::bench(crate = crate, threads = THREADS)]
    fn tally_alloc(bencher: crate::Bencher) {
        // Using 0 simulates tallying without affecting benchmark reporting.
        let count = crate::black_box(0);
        let size = crate::black_box(0);

        bencher.bench(|| {
            AllocProfiler::system().current_thread_info().tally_n(AllocOp::Alloc, count, size)
        })
    }

    #[crate::bench_group(crate = crate, threads = THREADS)]
    mod current {
        use super::*;

        #[crate::bench(crate = crate)]
        fn init() -> &'static ThreadAllocInfo {
            AllocProfiler::system().current_thread_info()
        }

        #[crate::bench(crate = crate)]
        fn r#try() -> Option<&'static ThreadAllocInfo> {
            ThreadAllocInfo::try_current()
        }
    }
}
