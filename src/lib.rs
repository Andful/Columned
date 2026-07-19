#![feature(allocator_api)]
#![feature(slice_ptr_get)]
#![feature(ptr_cast_slice)]
#![feature(phantom_variance_markers)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod chain;
mod guard;
mod guarded_slice;
mod single_allocation;
mod subscriber;

pub use guard::Guard;
pub use guarded_slice::{GuardedSlice, GuardedSliceBuilder};
pub use single_allocation::SingleAllocation;
pub use subscriber::Subscriber;

#[cfg(test)]
mod tests {
    //Have to figure out how to handle no_std
    use core::mem::MaybeUninit;

    use crate::{Guard, GuardedSliceBuilder, Subscriber};

    #[test]
    fn test() {
        let mut guard = Guard::new();

        let subscriber = guard.subscriber();

        fn init_zero(n: &mut [MaybeUninit<impl Default>]) {
            n.iter_mut().for_each(|e| {
                e.write(Default::default());
            })
        }

        let mut x: GuardedSliceBuilder<u8> = GuardedSliceBuilder::new(10);
        let mut y: GuardedSliceBuilder<u32> = GuardedSliceBuilder::new(10);
        let mut z: GuardedSliceBuilder<u16> = GuardedSliceBuilder::new(10);

        subscriber
            .subscribe(&mut x)
            .subscribe(&mut y)
            .subscribe(&mut z)
            .finish()
            .unwrap();

        let x = unsafe { x.build(init_zero) };
        let y = unsafe { y.build(init_zero) };
        let z = unsafe { z.build(init_zero) };

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
