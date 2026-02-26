#![feature(iter_map_windows)]
#![feature(allocator_api)]
#![warn(missing_docs)]

//! A single, contiguous, allocation for multiple arrays, of type `Column<T>`.
//! Meant to allocate multiple arrays, or so called `Column<T>` that live the same lifetimes.
//! The lifetimes of a `Column<T>`, and its backing memory, is tied to a `Columned`.
//! Therefore, the user must guarantee that `Columned` outlives any `Column<T>` which it allocated for.
//! `Column<T>` originating from a single allocation may have different sizes.  
//! This crate facilitates the implementation of columnar data structures.
//!
//! # Example
//!
//! ```
//! use columned::*;
//!
//! fn main() {
//!     let _columned; // Ensure this outlives the other variables.
//!
//!     let mut xs: Column<f64> = Default::default();
//!     let mut ys: Column<f64> = Default::default();
//!     let mut sums: Column<f64> = Default::default();
//!
//!     _columned = unsafe {
//!         Columned::new([
//!             xs.alloc(10),
//!             ys.alloc(10),
//!             sums.alloc(10)
//!         ])
//!     };
//!
//!     for (i, x) in xs.maybe_uninit().iter_mut().enumerate() {
//!         x.write(i as f64);
//!     }
//!
//!     for (i, y) in ys.maybe_uninit().iter_mut().enumerate() {
//!         y.write(i as f64);
//!     }
//!
//!     for sum in sums.maybe_uninit().iter_mut() {
//!         sum.write(0.0);
//!     }
//!
//!     for ((sum,x),y) in sums.iter_mut().zip(xs.iter()).zip(ys.iter()) {
//!         *sum = x + y;
//!     }
//!
//!     println!("{:?}", sums);
//! }
//! ```
//!
//! # Working Principle
//!
//! `Columned` manages a contiguous allocation of memory. Each `Coulmn` have a pointer to the contiguous allocation. The following figure illustrates the working principle.
//!
//! ```text
//!        Columned
//!        +--------+--------+
//!        | 0x0123 |   ...  |
//!        +--------+--------+
//!         ptr
//!          |
//!          V
//! Heap   +---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+
//!        |           0.1 |           3.2 |     5 |     7 |    20 |     6 |
//!        +---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+
//!          ^                               ^
//!          |                               |
//!         ptr      len                    ptr      len
//!        +--------+--------+             +--------+--------+
//!        | 0x0123 |      2 |             | 0x012b |      4 |
//!        +--------+--------+             +--------+--------+
//!        Column<f32>                     Column<u16>
//! ```
//! This also means that the user has to ensure that `Columned`
//! outlives the `Columns` that uses its managed memory.

use std::{
    alloc::{Allocator, Global, Layout},
    fmt::Debug,
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

/// A `Columned` instance, and its lifetime, corresponds to the allocation of a contiguous chunk of memory.
/// An allocation starts when [Columned::new] (or [Columned::new_in]) is called, and finished when the object is dropped.
pub struct Columned<A: Allocator = Global> {
    alloc: A,
    ptr: NonNull<u8>,
    layout: Layout,
    #[cfg(feature = "asserts")]
    deallocated: std::sync::Arc<std::sync::OnceLock<()>>,
}

impl Columned {
    /// # Safety
    /// This is highly unsafe, due to the number of invariants. These invariants are checked at runtime with the feature `asserts`.
    /// These invariants are:
    /// * The elements of `columns` must be sorted from the element of greatest alignment to the lowest.
    /// * After this call, each passed `Column<T>`, and their elements, must be initialized with the function [Column::maybe_uninit].
    ///   The elements of a `Column<T>` must be initialized before `Column<T>` is treated like an array or is dropped.
    /// * The resulting `Columned` must outlive the passed `Column<T>`.
    pub unsafe fn new<const N: usize>(columns: [ColumnAlloc<'_>; N]) -> Self {
        unsafe { Self::new_in(columns, Global) }
    }

    /// # Safety
    /// This is highly unsafe, due to the number of invariants. These invariants are checked at runtime with the feature `asserts`.
    /// These invariants are:
    /// * The elements of `columns` must be sorted from the element of greatest alignment to the lowest.
    /// * After this call, each passed `Column<T>`, and their elements, must be initialized with the function [Column::maybe_uninit].
    ///   The elements of a `Column<T>` must be initialized before `Column<T>` is treated like an array or is dropped.
    /// * The resulting `Columned` must outlive the passed `Column<T>`.
    pub unsafe fn new_in<const N: usize, A: Allocator>(
        mut columns: [ColumnAlloc<'_>; N],
        alloc: A,
    ) -> Columned<A> {
        if columns.is_empty() {
            return Columned {
                alloc,
                ptr: NonNull::dangling(),
                layout: Layout::new::<()>(),
                #[cfg(feature = "asserts")]
                deallocated: std::sync::Arc::new(std::sync::OnceLock::new()),
            };
        }
        #[cfg(feature = "asserts")]
        for (i, cols) in columns.windows(2).enumerate() {
            assert!(
                cols[0].align >= cols[1].align,
                "columns should be ordered by alignment, but align(columns[{}]) < align(columns[{}])",
                i,
                i + 1
            )
        }

        let align = columns[0].align;
        let size = columns.iter().map(|e| e.size * e.requested_len).sum();

        let layout = Layout::from_size_align(size, align).unwrap();
        let ptr: NonNull<u8> = alloc.allocate(layout).unwrap().cast();
        let mut p = ptr.as_ptr();
        #[cfg(feature = "asserts")]
        let deallocated = std::sync::Arc::new(std::sync::OnceLock::new());
        for e in columns.iter_mut() {
            *(e.ptr) = p as *mut ();
            *e.len = e.requested_len;
            p = p.wrapping_add(e.size * e.requested_len);

            #[cfg(feature = "asserts")]
            {
                *e.deallocated = deallocated.clone();
                *e.init = false;
            }
        }
        #[cfg(not(feature = "asserts"))]
        {
            Columned { alloc, ptr, layout }
        }
        #[cfg(feature = "asserts")]
        Columned {
            alloc,
            ptr,
            layout,
            deallocated,
        }
    }
}

impl<A> Drop for Columned<A>
where
    A: Allocator,
{
    fn drop(&mut self) {
        #[cfg(feature = "asserts")]
        {
            self.deallocated.get_or_init(|| ());
        }

        unsafe { self.alloc.deallocate(self.ptr, self.layout) };
    }
}

/// Intermediate representation to allocate memory for `Column`.
pub struct ColumnAlloc<'a> {
    size: usize,
    align: usize,
    ptr: &'a mut *mut (),
    len: &'a mut usize,
    requested_len: usize,
    #[cfg(feature = "asserts")]
    deallocated: &'a mut std::sync::Arc<std::sync::OnceLock<()>>,
    #[cfg(feature = "asserts")]
    init: &'a mut bool,
}

/// Array like structure.
pub struct Column<E>
where
    E: Sized,
{
    ptr: *mut (),
    len: usize,
    pd: PhantomData<E>,

    #[cfg(feature = "asserts")]
    deallocated: std::sync::Arc<std::sync::OnceLock<()>>,
    #[cfg(feature = "asserts")]
    init: bool,
}

impl<E> Debug for Column<E>
where
    E: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self.deref(), f)
    }
}

impl<E> Drop for Column<E> {
    fn drop(&mut self) {
        #[cfg(feature = "asserts")]
        {
            assert!(
                self.deallocated.get().is_none(),
                "Underlying memory of Column has been deallocated"
            );
            assert!(self.init, "Underlying memory not initialized");
        }
        for e in
            unsafe { std::slice::from_raw_parts_mut(self.ptr as *mut ManuallyDrop<E>, self.len) }
        {
            unsafe { ManuallyDrop::drop(e) };
        }
    }
}

impl<E> Default for Column<E>
where
    E: Sized,
{
    fn default() -> Self {
        Self {
            ptr: std::ptr::dangling_mut::<E>() as *mut (),
            len: 0,
            pd: Default::default(),
            #[cfg(feature = "asserts")]
            deallocated: std::sync::Arc::new(std::sync::OnceLock::new()),
            #[cfg(feature = "asserts")]
            init: true,
        }
    }
}

impl<E> Column<E>
where
    E: Sized,
{
    /// Constructs a new, empty Column<T>.
    pub fn new() -> Self {
        Self::default()
    }

    /// Function call to allocate memory. The parameter `len` dictates the length of the array after successful allocation.
    pub fn alloc(&mut self, len: usize) -> ColumnAlloc<'_> {
        ColumnAlloc {
            size: core::mem::size_of::<E>(),
            align: core::mem::align_of::<E>(),
            ptr: &mut self.ptr,
            len: &mut self.len,
            requested_len: len,
            #[cfg(feature = "asserts")]
            deallocated: &mut self.deallocated,
            #[cfg(feature = "asserts")]
            init: &mut self.init,
        }
    }

    /// Method used to initialized elements of the array. This method must be used, if the elements of the array are not initialized.
    pub fn maybe_uninit(&mut self) -> &mut [MaybeUninit<E>] {
        #[cfg(feature = "asserts")]
        {
            self.init = true;
        }
        unsafe { std::slice::from_raw_parts_mut(self.ptr as *mut MaybeUninit<E>, self.len) }
    }
}

impl<E> Deref for Column<E>
where
    E: Sized,
{
    type Target = [E];
    fn deref(&self) -> &Self::Target {
        #[cfg(feature = "asserts")]
        {
            assert!(
                self.deallocated.get().is_none(),
                "Underlying memory of Column has been deallocated"
            );
            assert!(self.init, "Underlying memory not initialized");
        }
        unsafe { std::slice::from_raw_parts(self.ptr as *const E, self.len) }
    }
}

impl<E> DerefMut for Column<E>
where
    E: Sized,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        #[cfg(feature = "asserts")]
        {
            assert!(
                self.deallocated.get().is_none(),
                "Underlying memory of Column has been deallocated"
            );
            assert!(self.init, "Underlying memory not initialized");
        }
        unsafe { std::slice::from_raw_parts_mut(self.ptr as *mut E, self.len) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tests() {
        let _columned;

        let mut x: Column<f64> = Default::default();
        let mut y: Column<f64> = Default::default();

        _columned = unsafe { Columned::new([x.alloc(10), y.alloc(10)]) };

        x.maybe_uninit().iter_mut().for_each(|e| {
            e.write(0.0);
        });
        y.maybe_uninit().iter_mut().for_each(|e| {
            e.write(0.0);
        });
    }
}
