#![feature(allocator_api)]

use columned::{Guard, GuardedBuilder, Subscriber};
use std::alloc::AllocError;

// The structure-of-array
#[derive(Debug, Default)]
struct Bodies<'a> {
    //Position
    position: Vec3<'a>,
    //Velocity
    velocity: Vec3<'a>,
    //Mass
    mass: &'a [f32],
}

impl<'a> Bodies<'a> {
    fn new(
        n: usize,
        subscriber: Subscriber<'a, '_>,
        f: impl FnOnce(Subscriber<'a, '_>) -> Result<(), AllocError>,
    ) -> Result<Bodies<'a>, AllocError> {
        let mut bodies = Bodies::default();

        bodies.position = Vec3::new(n, subscriber, |subscriber| {
            bodies.velocity = Vec3::new(n, subscriber, |subscriber| {
                let mut mass = GuardedBuilder::new_slice(n);
                f(subscriber.subscribe(&mut mass))?;
                bodies.mass = mass.build_default().into_mut();
                Ok(())
            })?;
            Ok(())
        })?;

        Ok(bodies)
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
        let mut x = GuardedBuilder::new_slice(n);
        let mut y = GuardedBuilder::new_slice(n);
        let mut z = GuardedBuilder::new_slice(n);

        let subscriber = subscriber
            .subscribe(&mut x)
            .subscribe(&mut y)
            .subscribe(&mut z);

        f(subscriber)?;

        Ok(Vec3 {
            x: x.build_default().into_mut(),
            y: y.build_default().into_mut(),
            z: z.build_default().into_mut(),
        })
    }
}

fn main() {
    let mut guard = Guard::new();

    let bodies = Bodies::new(5, guard.subscriber(), |subscriber| subscriber.allocate()).unwrap();

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
