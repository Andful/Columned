#![feature(coroutines)]
#![feature(stmt_expr_attributes)]

use columned::{Guard, GuardedSlice, GuardedSliceBuilder, Subscriber};
use std::mem::MaybeUninit;

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
    fn new(n: usize, subscriber: impl Subscriber<'a>) -> Self {
        let mut x = GuardedSliceBuilder::new_default(n);
        let mut y = GuardedSliceBuilder::new_default(n);
        let mut z = GuardedSliceBuilder::new_default(n);

        let mut vx = GuardedSliceBuilder::new_default(n);
        let mut vy = GuardedSliceBuilder::new_default(n);
        let mut vz = GuardedSliceBuilder::new_default(n);

        let mut mass = GuardedSliceBuilder::new_default(n);

        subscriber
            .subscribe(&mut x)
            .subscribe(&mut y)
            .subscribe(&mut z)
            .subscribe(&mut vx)
            .subscribe(&mut vy)
            .subscribe(&mut vz)
            .subscribe(&mut mass)
            .finish()
            .unwrap();

        Self {
            position: Vec3::new(
                x.build().into_slice(),
                y.build().into_slice(),
                z.build().into_slice(),
            ),
            velocity: Vec3::new(
                vx.build().into_slice(),
                vy.build().into_slice(),
                vz.build().into_slice(),
            ),
            mass: mass.build().into_slice(),
        }
    }
}

#[derive(Debug)]
struct Vec3<'a> {
    x: &'a [f32],
    y: &'a [f32],
    z: &'a [f32],
}

impl<'a> Vec3<'a> {
    fn new(x: &'a [f32], y: &'a [f32], z: &'a [f32]) -> Self {
        Self { x, y, z }
    }
}

fn main() {
    let mut guard = Guard::new();
    
    let bodies = Bodies::new(5, guard.subscriber());

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
