#![feature(allocator_api)]
#![feature(slice_ptr_get)]
#![feature(ptr_cast_slice)]
#![feature(phantom_variance_markers)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod chain;
mod guard;
mod guarded_slice;
mod once_allocator;
mod subscriber;

pub use guard::Guard;
pub use guarded_slice::{GuardedSlice, GuardedSliceBuilder};
pub use once_allocator::SingleAllocation;
pub use subscriber::Subscriber;

#[cfg(test)]
mod tests {
    //Have to figure out how to handle no_std
    use core::mem::MaybeUninit;

    use crate::{Guard, GuardedSliceBuilder};

    #[test]
    fn test() {
        let mut guard = Guard::new();

        let subscriber = guard.subscriber();

        fn init_zero(n: &mut [MaybeUninit<impl Default>]) {
            n.iter_mut().for_each(|e| {
                e.write(Default::default());
            })
        }

        let mut x: GuardedSliceBuilder<u8, _> = unsafe { GuardedSliceBuilder::new(10, init_zero) };
        let mut y: GuardedSliceBuilder<u32, _> = unsafe { GuardedSliceBuilder::new(10, init_zero) };
        let mut z: GuardedSliceBuilder<u16, _> = unsafe { GuardedSliceBuilder::new(10, init_zero) };

        subscriber
            .subscribe(&mut x)
            .subscribe(&mut y)
            .subscribe(&mut z)
            .finish()
            .unwrap();

        let x = x.build();
        let y = y.build();
        let z = z.build();

        println!("1: {:?}", guard.as_ptr_range());

        println!(
            "ptrs2: {:?} {:?} {:?}",
            y.as_ptr_range(),
            z.as_ptr_range(),
            x.as_ptr_range()
        );

        println!("{:?} {:?} {:?}", x, y, z)
    }
}
