#![feature(allocator_api)]
#![feature(slice_ptr_get)]
#![feature(ptr_cast_slice)]
#![feature(phantom_variance_markers)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod guard;
mod guarded_slice;
mod single_allocation;
mod subscriber;

pub use guard::Guard;
pub use guarded_slice::{Guarded, GuardedBuilder};
pub use single_allocation::SingleAllocation;
pub use subscriber::Subscriber;

#[cfg(test)]
mod tests {
    //Have to figure out how to handle no_std
    use core::mem::MaybeUninit;

    use crate::{Guard, GuardedBuilder};

    #[test]
    fn test() {
        let mut guard = Guard::new();

        let subscriber = guard.subscriber();

        fn init_zero(n: &mut [MaybeUninit<impl Default>]) {
            n.iter_mut().for_each(|e| {
                e.write(Default::default());
            })
        }

        let mut x: GuardedBuilder<[u8]> = GuardedBuilder::new_slice(10);
        let mut y: GuardedBuilder<[u32]> = GuardedBuilder::new_slice(10);
        let mut z: GuardedBuilder<[u16]> = GuardedBuilder::new_slice(10);

        subscriber
            .subscribe(&mut x)
            .subscribe(&mut y)
            .subscribe(&mut z)
            .allocate()
            .unwrap();

        let x = unsafe { x.build(init_zero) };
        let y = unsafe { y.build(init_zero) };
        let z = unsafe { z.build(init_zero) };

        println!(
            "ptrs2: {:?} {:?} {:?}",
            y.as_ptr_range(),
            z.as_ptr_range(),
            x.as_ptr_range()
        );

        println!("{:?} {:?} {:?}", x, y, z);

        //println!("1: {:?}", guard.as_ptr_range());
    }
}
