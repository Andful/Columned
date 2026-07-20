#![feature(coroutines)]
#![feature(stmt_expr_attributes)]
#![feature(allocator_api)]

use columned::{Guard, GuardedSlice, GuardedSliceBuilder, Subscriber};
use std::{
    alloc::{AllocError, Allocator},
    default,
    mem::{MaybeUninit, take},
};

// The structure-of-array
#[derive(Debug)]
struct Bodies<'a> {
    //Position
    position: Vec3<'a>,
    //Velocity
    velocity: Vec3<'a>,
    //Mass
    mass: &'a [f32],
}

impl<'a> Bodies<'a> {
    fn new(n: usize, subscriber: Subscriber<'a, '_>) -> Result<Bodies<'a>, AllocError> {
        let mut velocity = Vec3::<'a>::default();
        let mut mass = GuardedSliceBuilder::new(n);

        let position = Vec3::new(n, subscriber.subscribe(&mut mass), |subscriber| {
            velocity = Vec3::new(n, subscriber, |subscriber| subscriber.allocate())?;
            Ok(())
        })?;

        Ok(Bodies {
            position,
            velocity,
            mass: mass.build_default().into_slice(),
        })
    }
}

#[derive(Debug, Default)]
struct Vec3<'a> {
    x: &'a [f32],
    y: &'a [f32],
    z: &'a [f32],
}

impl<'a> Vec3<'a> {
    fn new(
        n: usize,
        subscriber: Subscriber<'a, '_>,
        f: impl FnOnce(Subscriber<'a, '_>) -> Result<(), AllocError>,
    ) -> Result<Vec3<'a>, AllocError> {
        let mut x = GuardedSliceBuilder::new(n);
        let mut y = GuardedSliceBuilder::new(n);
        let mut z = GuardedSliceBuilder::new(n);

        let subscriber = subscriber
            .subscribe(&mut x)
            .subscribe(&mut y)
            .subscribe(&mut z);

        f(subscriber)?;

        Ok(Vec3 {
            x: x.build_default().into_slice(),
            y: y.build_default().into_slice(),
            z: z.build_default().into_slice(),
        })
    }
}

fn main() {
    let mut guard = Guard::new();

    let bodies = Bodies::new(5, guard.subscriber()).unwrap();

    println!("{:#?}", bodies);

    println!(
        "Slices Memory:\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n",
        bodies.position.x.as_ptr_range(),
        bodies.position.y.as_ptr_range(),
        bodies.position.z.as_ptr_range(),
        bodies.velocity.x.as_ptr_range(),
        bodies.velocity.y.as_ptr_range(),
        bodies.velocity.z.as_ptr_range(),
        bodies.mass.as_ptr_range()
    );

    println!("Guard's Memory:\n{:?}", guard.as_ptr_range());

    // use bodies here ...
}
