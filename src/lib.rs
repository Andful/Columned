#![feature(allocator_api)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

//Have to figure out how to handle no_std
use std::alloc::Global;

use pastey::paste;

enum GuardState<A: core::alloc::Allocator = Global> {
    Allocated {
        ptr: core::ptr::NonNull<u8>,
        alloc: A,
        layout: core::alloc::Layout,
    },
    Unallocated,
}

///`Guard` manages and owns a contiguous allocation of memory. A `Guard` should be used only once for [allocate], [allocate_in], otherwise the allocation will fail.
pub struct Guard<A: core::alloc::Allocator = Global> {
    state: GuardState<A>,
}

impl<A: core::alloc::Allocator> Default for Guard<A> {
    fn default() -> Self {
        Self {
            state: GuardState::Unallocated,
        }
    }
}

impl<A: core::alloc::Allocator> Guard<A> {
    /// Construct a new [Guard].
    pub const fn new() -> Self {
        Self {
            state: GuardState::Unallocated,
        }
    }
}

impl<A: core::alloc::Allocator> Drop for Guard<A> {
    fn drop(&mut self) {
        match &self.state {
            GuardState::Allocated { ptr, alloc, layout } => unsafe {
                alloc.deallocate(*ptr, *layout)
            },
            GuardState::Unallocated => (),
        }
    }
}

/// A slice, which memory is managed by a [Guard].
pub struct GuardedSlice<'a, T>(&'a mut [T]);

impl<'a, T> GuardedSlice<'a, T> {
    ///Return the underling slice. This will cause [Drop::drop] of `T` to not be called.
    pub fn forget(mut self) -> &'a mut [T] {
        core::mem::take(&mut self.0)
    }
}

impl<'a, T> GuardedSlice<'a, T>
where
    T: Copy,
{
    ///Return the underling slice. This is equivalent to [GuardedSlice::forget], but requires `T: Copy`. Therefore, it would lead to a compilation error if `Drop` is implemented for `T`.
    pub fn into_slice(self) -> &'a mut [T] {
        self.forget()
    }
}

impl<T> core::ops::Deref for GuardedSlice<'_, T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<T> core::ops::DerefMut for GuardedSlice<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<T> Drop for GuardedSlice<'_, T> {
    fn drop(&mut self) {
        if core::mem::needs_drop::<T>() {
            let to_drop = core::mem::take(&mut self.0);
            let to_drop = unsafe {
                core::mem::transmute::<&mut [T], &mut [core::mem::ManuallyDrop<T>]>(to_drop)
            };
            to_drop.iter_mut().for_each(|e| unsafe {
                core::mem::ManuallyDrop::drop(e);
            });
        }
    }
}

impl<T> core::fmt::Debug for GuardedSlice<'_, T>
where
    T: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self.0, f)
    }
}

/// Prepare an allocation of a slice, by specifying its size and
/// its initialization function.
/// The initialization function will be called upon the call of
/// [allocate], [allocate_in], [with_allocation] or [with_allocation_in].
pub struct PrepAlloc<T, F>
where
    F: FnOnce(&mut [::core::mem::MaybeUninit<T>]),
{
    n: usize,
    init: F,
    pd: ::core::marker::PhantomData<T>,
}

impl<T, F> PrepAlloc<T, F>
where
    F: FnOnce(&mut [::core::mem::MaybeUninit<T>]),
{
    ///Prepare an allocation of a slice, by specifying its size and
    /// its initialization function.
    /// # Safety
    /// the initialization function `init` must initialize every element of its argument.
    pub unsafe fn new(n: usize, init: F) -> Self {
        Self {
            n,
            init,
            pd: Default::default(),
        }
    }
}

trait Sealed {}

/// Trait used internally to facilitate implementation. An user should not, and should not be able to implement this trait.
#[allow(private_bounds)]
pub trait PrepAllocTuple<'a>: Sealed {
    #[allow(missing_docs)]
    const ALIGNMENTS: &'static [usize];
    #[allow(missing_docs)]
    const INDICES: &'static [usize];
    #[allow(missing_docs)]
    type AllocatedArraysType: 'a;
    #[allow(missing_docs)]
    fn allocate_in<A: core::alloc::Allocator>(
        self,
        guard: &'a mut Guard<A>,
        alloc: A,
    ) -> Result<Self::AllocatedArraysType, ::core::alloc::AllocError>;
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
            impl <$([<T $t>], [<F $t>]: FnOnce(&mut [::core::mem::MaybeUninit<[<T $t>]>]),)*> Sealed for ($(PrepAlloc<[<T $t>], [<F $t>] >,)*) {}

            impl <'a, $([<T $t>]: 'a, [<F $t>]: FnOnce(&mut [::core::mem::MaybeUninit<[<T $t>]>]),)*> PrepAllocTuple<'a> for ($(PrepAlloc<[<T $t>], [<F $t>] >,)*) {
                const ALIGNMENTS: &'static [usize] = &[$(::core::mem::align_of::<[<T $t>]>(),)*];
                const INDICES: &'static [usize] = &const_arg_sort::<{ len!($($t),*) }>(Self::ALIGNMENTS);
                type AllocatedArraysType = ($(GuardedSlice<'a, [<T $t>]>,)*);

                fn allocate_in<A: ::core::alloc::Allocator>(self, guard: &'a mut Guard<A>, alloc: A) -> Result<Self::AllocatedArraysType, ::core::alloc::AllocError> {
                    const N: usize = len!($($t),*);

                    if let GuardState::Allocated { .. } = guard.state {
                        return Err(::core::alloc::AllocError);
                    }

                    if N == 0 {
                        return Ok(($(GuardedSlice::<[<T $t>]>(&mut []),)*));
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
                    guard.state = GuardState::Allocated {
                        ptr,
                        alloc,
                        layout,
                    };
                    Ok(($(GuardedSlice(unsafe{::core::mem::transmute::<&mut [::core::mem::MaybeUninit<[<T $t>]>], &mut [[<T $t>]]>([<uninit_ $t>])}),)*))
                }
            }
        }
    };
}

const fn const_arg_sort<const N: usize>(arr: &[usize]) -> [usize; N] {
    assert!(arr.len() == N);
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
pub fn with_allocation<ARG: for<'a> PrepAllocTuple<'a>, R>(
    arg: ARG,
    f: impl FnOnce(<ARG as PrepAllocTuple<'_>>::AllocatedArraysType) -> R,
) -> Result<R, ::core::alloc::AllocError> {
    with_allocation_in(Global, arg, f)
}

/// Allocate a single contiguous allocation to instantiate multiple slices.
/// The slices will be used to call `f`. The allocation is done within the allocator `alloc`.
pub fn with_allocation_in<ARG: for<'a> PrepAllocTuple<'a>, R>(
    alloc: impl ::core::alloc::Allocator,
    arg: ARG,
    f: impl FnOnce(<ARG as PrepAllocTuple<'_>>::AllocatedArraysType) -> R,
) -> Result<R, ::core::alloc::AllocError> {
    let mut guard = Guard::default();
    let arrays = arg.allocate_in(&mut guard, alloc)?;
    Ok(f(arrays))
}

///Allocate a single contiguous allocation to instantiate multiple slices. The allocation is done within the allocator `alloc`.
pub fn allocate_in<'a, ARG: PrepAllocTuple<'a>, A: ::core::alloc::Allocator>(
    guard: &'a mut Guard<A>,
    alloc: A,
    arg: ARG,
) -> Result<ARG::AllocatedArraysType, ::core::alloc::AllocError> {
    arg.allocate_in(guard, alloc)
}

///Allocate a single contiguous allocation to instantiate multiple slices.
pub fn allocate<'a, ARG: PrepAllocTuple<'a>>(
    guard: &'a mut Guard,
    arg: ARG,
) -> Result<ARG::AllocatedArraysType, ::core::alloc::AllocError> {
    allocate_in(guard, Global, arg)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_basic() {
        let xs: PrepAlloc<u64, _> = unsafe {
            PrepAlloc::new(10, |xs| {
                for (i, x) in xs.iter_mut().enumerate() {
                    x.write(i as u64);
                }
            })
        };
        let ys: PrepAlloc<u64, _> = unsafe {
            PrepAlloc::new(10, |ys| {
                for (i, y) in ys.iter_mut().enumerate() {
                    y.write(i as u64);
                }
            })
        };
        let sums: PrepAlloc<u64, _> = unsafe {
            PrepAlloc::new(10, |sums| {
                for sum in sums.iter_mut() {
                    sum.write(0);
                }
            })
        };

        with_allocation((xs, ys, sums), |(xs, ys, mut sums)| {
            for ((sum, x), y) in sums.iter_mut().zip(xs.iter()).zip(ys.iter()) {
                *sum = x + y;
            }

            for (i, sum) in sums.iter().enumerate() {
                assert_eq!(*sum, 2 * i as u64);
            }
        })
        .unwrap();
    }

    #[test]
    fn test_basic2() {
        let xs: PrepAlloc<u64, _> = unsafe {
            PrepAlloc::new(10, |xs| {
                for (i, x) in xs.iter_mut().enumerate() {
                    x.write(i as u64);
                }
            })
        };
        let ys: PrepAlloc<u64, _> = unsafe {
            PrepAlloc::new(10, |ys| {
                for (i, y) in ys.iter_mut().enumerate() {
                    y.write(i as u64);
                }
            })
        };
        let sums: PrepAlloc<u64, _> = unsafe {
            PrepAlloc::new(10, |sums| {
                for sum in sums.iter_mut() {
                    sum.write(0);
                }
            })
        };

        let mut guard: Guard = Guard::default();

        let (xs, ys, mut sums) = allocate(&mut guard, (xs, ys, sums)).unwrap();

        for ((sum, x), y) in sums.iter_mut().zip(xs.iter()).zip(ys.iter()) {
            *sum = x + y;
        }

        for (i, sum) in sums.iter().enumerate() {
            assert_eq!(*sum, 2 * i as u64);
        }
    }

    struct Vec3<'a> {
        x: GuardedSlice<'a, f32>,
        y: GuardedSlice<'a, f32>,
        z: GuardedSlice<'a, f32>,
    }

    struct Bodies<'a> {
        //Position
        position: Vec3<'a>,
        //Velocity
        velocity: Vec3<'a>,
        //Mass
        mass: GuardedSlice<'a, f32>,
    }

    fn generate_n_bodies<'a>(n: usize, guard: &'a mut Guard) -> Bodies<'a> {
        let init_to_zero = |data: &mut [std::mem::MaybeUninit<f32>]| {
            data.iter_mut().for_each(|d| {
                d.write(0.0);
            })
        };

        let x = unsafe { PrepAlloc::new(n, init_to_zero) };
        let y = unsafe { PrepAlloc::new(n, init_to_zero) };
        let z = unsafe { PrepAlloc::new(n, init_to_zero) };
        let vx = unsafe { PrepAlloc::new(n, init_to_zero) };
        let vy = unsafe { PrepAlloc::new(n, init_to_zero) };
        let vz = unsafe { PrepAlloc::new(n, init_to_zero) };
        let mass = unsafe { PrepAlloc::new(n, init_to_zero) };

        let (x, y, z, vx, vy, vz, mass) = allocate(guard, (x, y, z, vx, vy, vz, mass)).unwrap();

        Bodies {
            position: Vec3 { x, y, z },
            velocity: Vec3 {
                x: vx,
                y: vy,
                z: vz,
            },
            mass,
        }
    }

    #[test]
    fn test3() {
        let mut guard = Guard::new();
        let bodies = generate_n_bodies(100, &mut guard);
        //drop(guard);
        println!("x: {:?}", &*bodies.position.x);
    }
}
