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

///`Guard` manages and owns a contiguous allocation of memory.
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
    pub fn new() -> Self {
        Self::default()
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

/// Prepare an allocation of a slice, by specifying its size and
/// its initialization function.
/// The initialization function will be called upon the call of
/// [allocate], [allocate_in], [with_allocation] or [with_allocation_in].
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

trait Sealed {}

/// Trait used internally to facilitate implementation. An user should not, and should not be able to implement this trait.
#[allow(private_bounds)]
pub trait AllocateTuple: Sealed {
    #[allow(missing_docs)]
    const ALIGNMENTS: &'static [usize];
    #[allow(missing_docs)]
    const INDICES: &'static [usize];
    #[allow(missing_docs)]
    type AllocatedArraysType<'a>;
    #[allow(missing_docs)]
    fn allocate_in<'a, A: core::alloc::Allocator>(
        self,
        guard: &'a mut Guard<A>,
        alloc: A,
    ) -> Result<Self::AllocatedArraysType<'a>, ::core::alloc::AllocError>;
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
            impl <$([<T $t>], [<F $t>]: FnOnce(&mut [::core::mem::MaybeUninit<[<T $t>]>]),)*> Sealed for ($(Allocate<[<T $t>], [<F $t>] >,)*) {}

            impl <$([<T $t>]: 'static, [<F $t>]: FnOnce(&mut [::core::mem::MaybeUninit<[<T $t>]>]),)*> AllocateTuple for ($(Allocate<[<T $t>], [<F $t>] >,)*) {
                const ALIGNMENTS: &'static [usize] = &[$(::core::mem::align_of::<[<T $t>]>(),)*];
                const INDICES: &'static [usize] = &const_arg_sort::<{ len!($($t),*) }>(Self::ALIGNMENTS);
                type AllocatedArraysType<'a> = ($(&'a mut [[<T $t>]],)*);

                fn allocate_in<'a, A: ::core::alloc::Allocator>(self, guard: &'a mut Guard<A>, alloc: A) -> Result<Self::AllocatedArraysType<'a>, ::core::alloc::AllocError> {
                    const N: usize = len!($($t),*);

                    if let GuardState::Allocated { .. } = guard.state {
                        return Err(::core::alloc::AllocError);
                    }

                    if N == 0 {
                        return Ok(($(&mut [] as &mut [[<T $t>]],)*));
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
                    Ok(($(unsafe{::core::mem::transmute::<&mut [std::mem::MaybeUninit<[<T $t>]>], &mut [[<T $t>]]>([<uninit_ $t>])},)*))
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
    let mut guard = Guard::default();
    let arrays = arg.allocate_in(&mut guard, alloc)?;
    Ok(f(arrays))
}

///Allocate a single contiguous allocation to instantiate multiple slices. The allocation is done within the allocator `alloc`.
pub fn allocate_in<'a, ARG: AllocateTuple, A: ::core::alloc::Allocator>(
    guard: &'a mut Guard<A>,
    alloc: A,
    arg: ARG,
) -> Result<ARG::AllocatedArraysType<'a>, ::core::alloc::AllocError> {
    arg.allocate_in(guard, alloc)
}

///Allocate a single contiguous allocation to instantiate multiple slices.
pub fn allocate<'a, ARG: AllocateTuple>(
    guard: &'a mut Guard,
    arg: ARG,
) -> Result<ARG::AllocatedArraysType<'a>, ::core::alloc::AllocError> {
    allocate_in(guard, Global, arg)
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

    #[test]
    fn test_basic2() {
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

        let mut guard: Guard = Guard::default();

        let (xs, ys, sums) = allocate(&mut guard, (xs, ys, sums)).unwrap();

        for ((sum, x), y) in sums.iter_mut().zip(xs.iter()).zip(ys.iter()) {
            *sum = x + y;
        }

        for (i, sum) in sums.iter().enumerate() {
            assert_eq!(*sum, 2 * i as u64);
        }
    }

    struct Vec3<'a> {
        x: &'a mut [f32],
        y: &'a mut [f32],
        z: &'a mut [f32],
    }

    struct Bodies<'a> {
        //Position
        position: Vec3<'a>,
        //Velocity
        velocity: Vec3<'a>,
        //Mass
        mass: &'a mut [f32],
    }

    fn generate_n_bodies<'a>(n: usize, guard: &'a mut Guard) -> Bodies<'a> {
        let init_to_zero = |data: &mut [std::mem::MaybeUninit<f32>]| {
            data.iter_mut().for_each(|d| {
                d.write(0.0);
            })
        };

        let x = unsafe { Allocate::alloc(n, init_to_zero) };
        let y = unsafe { Allocate::alloc(n, init_to_zero) };
        let z = unsafe { Allocate::alloc(n, init_to_zero) };
        let vx = unsafe { Allocate::alloc(n, init_to_zero) };
        let vy = unsafe { Allocate::alloc(n, init_to_zero) };
        let vz = unsafe { Allocate::alloc(n, init_to_zero) };
        let mass = unsafe { Allocate::alloc(n, init_to_zero) };

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
        println!("x: {:?}", &bodies.position.x);
    }
}
