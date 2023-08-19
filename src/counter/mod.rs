//! Count values processed in each iteration to measure throughput.
//!
//! # Examples
//!
//! The following example measures throughput of converting
//! [`&[i32]`](prim@slice) into [`Vec<i32>`](Vec) by providing [`Bytes`] via
//! [`Bencher::counter`](crate::Bencher::counter):
//!
//! ```
//! use divan::counter::Bytes;
//!
//! #[divan::bench]
//! fn slice_into_vec(bencher: divan::Bencher) {
//!     let ints: &[i32] = &[
//!         // ...
//!     ];
//!
//!     let bytes = Bytes::size_of_val(ints);
//!
//!     bencher
//!         .counter(bytes)
//!         .bench(|| -> Vec<i32> {
//!             divan::black_box(ints).into()
//!         });
//! }
//! ```

use std::any::Any;

mod any_counter;
mod collection;
mod into_counter;
mod sealed;
mod uint;

pub(crate) use self::{
    any_counter::{AnyCounter, KnownCounterKind},
    collection::{CounterCollection, CounterSet},
    sealed::Sealed,
    uint::{CountUInt, MaxCountUInt},
};
pub use into_counter::IntoCounter;

/// Counts the number of values processed in each iteration of a benchmarked
/// function.
///
/// This is used via:
/// - [`#[divan::bench(counter = ...)]`](macro@crate::bench#counter)
/// - [`#[divan::bench_group(counter = ...)]`](macro@crate::bench_group#counter)
/// - [`Bencher::counter`](crate::Bencher::counter)
/// - [`Bencher::input_counter`](crate::Bencher::input_counter)
#[doc(alias = "throughput")]
pub trait Counter: Sized + Any + Sealed {}

/// Process N bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bytes<N>(
    /// The number of bytes processed.
    pub N,
);

/// Process N items.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Items<N>(
    /// The number of items processed.
    pub N,
);

impl<N: CountUInt> Sealed for Bytes<N> {
    const COUNTER_KIND: KnownCounterKind = KnownCounterKind::Bytes;

    #[inline]
    fn into_any_counter(self) -> AnyCounter {
        AnyCounter::Bytes(self.0.into_max_uint())
    }
}

impl<N: CountUInt> Sealed for Items<N> {
    const COUNTER_KIND: KnownCounterKind = KnownCounterKind::Items;

    #[inline]
    fn into_any_counter(self) -> AnyCounter {
        AnyCounter::Items(self.0.into_max_uint())
    }
}

impl<N: CountUInt> Counter for Bytes<N> {}

impl<N: CountUInt> Counter for Items<N> {}

impl Bytes<usize> {
    /// Counts the size of a type with [`std::mem::size_of`].
    #[inline]
    pub const fn size_of<T>() -> Self {
        Self(std::mem::size_of::<T>())
    }

    /// Counts the size of a value with [`std::mem::size_of_val`].
    #[inline]
    pub fn size_of_val<T: ?Sized>(val: &T) -> Self {
        // TODO: Make const, https://github.com/rust-lang/rust/issues/46571
        Self(std::mem::size_of_val(val))
    }
}

/// The numerical base for [`Bytes`] in benchmark outputs.
///
/// See [`Divan::bytes_format`](crate::Divan::bytes_format) for more info.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum BytesFormat {
    /// Powers of 1024, starting with KiB (kibibyte). This is the default.
    #[default]
    Binary,

    /// Powers of 1000, starting with KB (kilobyte).
    Decimal,
}

/// Private `BytesFormat` that prevents leaking trait implementations we don't
/// want to publicly commit to.
#[derive(Clone, Copy)]
pub(crate) struct PrivBytesFormat(pub BytesFormat);

impl clap::ValueEnum for PrivBytesFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self(BytesFormat::Binary), Self(BytesFormat::Decimal)]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        let name = match self.0 {
            BytesFormat::Binary => "binary",
            BytesFormat::Decimal => "decimal",
        };
        Some(clap::builder::PossibleValue::new(name))
    }
}
