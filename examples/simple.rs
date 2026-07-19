use columned::{Guard, GuardedSliceBuilder};

fn main() {
    let mut guard = Guard::new();

    let subscriber = guard.subscriber();

    let mut x: GuardedSliceBuilder<u8, _> = GuardedSliceBuilder::new_default(10);
    let mut y: GuardedSliceBuilder<u32, _> = GuardedSliceBuilder::new_default(10);
    let mut z: GuardedSliceBuilder<u16, _> = GuardedSliceBuilder::new_default(10);

    subscriber
        .subscribe(&mut x)
        .subscribe(&mut y)
        .subscribe(&mut z)
        .finish()
        .unwrap();

    let x = x.build();
    let y = y.build();
    let z = z.build();

    println!("guard allocation: {:?}", guard.as_ptr_range());

    println!(
        "guarded slices allocations: {:?} {:?} {:?}",
        y.as_ptr_range(),
        z.as_ptr_range(),
        x.as_ptr_range()
    );

    println!("x={:?} y={:?} z={:?}", x, y, z)
}
