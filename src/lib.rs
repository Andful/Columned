#![feature(allocator_api)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

//Have to figure out how to handle no_std
use std::alloc::Global;

use pastey::paste;

/// Prepare an allocation of a slice, by specifying its size and
/// its initialization function.
/// The initialization function will be called upon the call of
/// [with_allocation] or [with_allocation_in].
pub struct Allocate<T, F>
where
    F: FnOnce(&mut [::core::mem::MaybeUninit<T>]),
{
    n: usize,
    init: F,
    pd: ::core::marker::PhantomData<T>,
}

impl<T, F> Allocate<T, F>
where
    F: FnOnce(&mut [::core::mem::MaybeUninit<T>]),
{
    ///Prepare an allocation of a slice, by specifying its size and
    /// its initialization function.
    /// # Safety
    /// the initialization function `init` must initialize every element of its argument.
    pub unsafe fn alloc(n: usize, init: F) -> Self {
        Self {
            n,
            init,
            pd: Default::default(),
        }
    }
}

trait AllocateTuple {
    const ALIGNMENTS: &'static [usize];
    const INDICES: &'static [usize];
    type AllocatedArraysType<'a>;
    fn with_allocation_in<R>(
        self,
        alloc: impl ::core::alloc::Allocator,
        f: impl FnOnce(Self::AllocatedArraysType<'_>) -> R,
    ) -> Result<R, ::core::alloc::AllocError>;
}

macro_rules! one {
    ($x:ident) => {
        1
    };
}

macro_rules! len {
    ($($xs:ident),*) => ($(one!($xs)+)* 0);
}

macro_rules! null_mut {
    ($x:ident) => {
        ::core::ptr::null_mut()
    };
}

macro_rules! null_mut_array {
    ($($xs:ident),*) => ([$(null_mut!($xs),)*]);
}

macro_rules! impl_allocate_tuple {
    ($($t:ident),*) => {
        paste!{
            impl <$([<T $t>]: 'static, [<F $t>]: FnOnce(&mut [::core::mem::MaybeUninit<[<T $t>]>]),)*> AllocateTuple for ($(Allocate<[<T $t>], [<F $t>] >,)*) {
                const ALIGNMENTS: &'static [usize] = &[$(::core::mem::align_of::<[<T $t>]>(),)*];
                const INDICES: &'static [usize] = &const_arg_sort::<{ len!($($t),*) }>(Self::ALIGNMENTS);
                type AllocatedArraysType<'a> = ($(&'a mut [[<T $t>]],)*);

                fn with_allocation_in<R>(self, alloc: impl ::core::alloc::Allocator, f: impl FnOnce(Self::AllocatedArraysType<'_>) -> R) -> Result<R, ::core::alloc::AllocError> {
                    const N: usize = len!($($t),*);

                    if N == 0 {
                        return Ok(f(($(&mut [] as &mut [[<T $t>]],)*)));
                    }

                    let align = Self::ALIGNMENTS[Self::INDICES[0]];
                    let ($([<val_ $t>],)*) = self;
                    let sizes: [usize; N] = [$(::core::mem::size_of::<[<T $t>]>()*[<val_ $t>].n,)*];
                    let size: usize = sizes.iter().sum();

                    let layout = ::core::alloc::Layout::from_size_align(size, align).unwrap();
                    let ptr = alloc.allocate(layout)?.cast();

                    let mut p = ptr.as_ptr();
                    let mut pointers: [*mut u8; N]  = null_mut_array!($($t),*);

                    #[allow(clippy::reversed_empty_ranges)]
                    for i in 0..N {
                        pointers[Self::INDICES[i]] = p;
                        p = p.wrapping_add(sizes[Self::INDICES[i]]);
                    }

                    let [$([<ptr_ $t>],)*] = pointers;
                    let ($([<uninit_ $t>],)*) = ($(unsafe { ::core::slice::from_raw_parts_mut([<ptr_ $t>] as *mut ::core::mem::MaybeUninit<[<T $t>]>, [<val_ $t>].n) },)*);

                    $(
                        ([<val_ $t>].init)([<uninit_ $t>]);
                    )*
                    let result = f(($(unsafe{::core::mem::transmute::<&mut [std::mem::MaybeUninit<[<T $t>]>], &mut [[<T $t>]]>([<uninit_ $t>])},)*));
                    unsafe {
                        alloc.deallocate(ptr, layout);
                    }
                    Ok(result)
                }
            }
        }
    };
}

const fn const_arg_sort<const N: usize>(arr: &[usize]) -> [usize; N] {
    //parts taken from: https://www.reddit.com/r/rust/comments/qw18oa/comment/hl05kuj/
    let mut indices = [0usize; N];
    let mut i = 0;
    while i < indices.len() {
        indices[i] = i;
        i += 1;
    }
    loop {
        let mut swapped = false;
        let mut i = 1;
        while i < arr.len() {
            if arr[indices[i - 1]] < arr[indices[i]] {
                (indices[i - 1], indices[i]) = (indices[i], indices[i - 1]);
                swapped = true;
            }
            i += 1;
        }
        if !swapped {
            break;
        }
    }
    indices
}

impl_allocate_tuple!();
impl_allocate_tuple!(l1);
impl_allocate_tuple!(l1, l2);
impl_allocate_tuple!(l1, l2, l3);
impl_allocate_tuple!(l1, l2, l3, l4);
impl_allocate_tuple!(l1, l2, l3, l4, l5);
impl_allocate_tuple!(l1, l2, l3, l4, l5, l6);
impl_allocate_tuple!(l1, l2, l3, l4, l5, l6, l7);
impl_allocate_tuple!(l1, l2, l3, l4, l5, l6, l7, l8);
impl_allocate_tuple!(l1, l2, l3, l4, l5, l6, l7, l8, l9);
impl_allocate_tuple!(l1, l2, l3, l4, l5, l6, l7, l8, l9, l10);
impl_allocate_tuple!(l1, l2, l3, l4, l5, l6, l7, l8, l9, l10, l11);
impl_allocate_tuple!(l1, l2, l3, l4, l5, l6, l7, l8, l9, l10, l11, l12);
impl_allocate_tuple!(l1, l2, l3, l4, l5, l6, l7, l8, l9, l10, l11, l12, l13);
impl_allocate_tuple!(l1, l2, l3, l4, l5, l6, l7, l8, l9, l10, l11, l12, l13, l14);
impl_allocate_tuple!(
    l1, l2, l3, l4, l5, l6, l7, l8, l9, l10, l11, l12, l13, l14, l15
);
impl_allocate_tuple!(
    l1, l2, l3, l4, l5, l6, l7, l8, l9, l10, l11, l12, l13, l14, l15, l16
);

/// Allocate a single contiguous allocation to instantiate multiple slices.
/// The slices will be used to call `f`.
#[allow(private_bounds)]
pub fn with_allocation<ARG: AllocateTuple, R>(
    arg: ARG,
    f: impl FnOnce(ARG::AllocatedArraysType<'_>) -> R,
) -> Result<R, ::core::alloc::AllocError> {
    with_allocation_in(Global, arg, f)
}

/// Allocate a single contiguous allocation to instantiate multiple slices.
/// The slices will be used to call `f`. The allocation is done within the allocator `alloc`.
#[allow(private_bounds)]
pub fn with_allocation_in<ARG: AllocateTuple, R>(
    alloc: impl ::core::alloc::Allocator,
    arg: ARG,
    f: impl FnOnce(ARG::AllocatedArraysType<'_>) -> R,
) -> Result<R, ::core::alloc::AllocError> {
    arg.with_allocation_in(alloc, f)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_basic() {
        let xs: Allocate<u64, _> = unsafe {
            Allocate::alloc(10, |xs| {
                for (i, x) in xs.iter_mut().enumerate() {
                    x.write(i as u64);
                }
            })
        };
        let ys: Allocate<u64, _> = unsafe {
            Allocate::alloc(10, |ys| {
                for (i, y) in ys.iter_mut().enumerate() {
                    y.write(i as u64);
                }
            })
        };
        let sums: Allocate<u64, _> = unsafe {
            Allocate::alloc(10, |sums| {
                for sum in sums.iter_mut() {
                    sum.write(0);
                }
            })
        };

        with_allocation((xs, ys, sums), |(xs, ys, sums)| {
            for ((sum, x), y) in sums.iter_mut().zip(xs.iter()).zip(ys.iter()) {
                *sum = x + y;
            }

            for (i, sum) in sums.iter().enumerate() {
                assert_eq!(*sum, 2 * i as u64);
            }
        })
        .unwrap();
    }
}
