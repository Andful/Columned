use columned::{Guard, GuardedSliceBuilder};

fn main() {
    let mut guard = Guard::new();

    let subscriber = guard.subscriber();

    let mut x: GuardedSliceBuilder<u8> = GuardedSliceBuilder::new(10);
    let mut y: GuardedSliceBuilder<u32> = GuardedSliceBuilder::new(10);
    let mut z: GuardedSliceBuilder<u16> = GuardedSliceBuilder::new(10);

    subscriber
        .subscribe(&mut x)
        .subscribe(&mut y)
        .subscribe(&mut z)
        .allocate()
        .unwrap();

    let x = unsafe {
        x.build(|e| {
            e.iter_mut().for_each(|e| {
                e.write(Default::default());
            })
        })
    };
    let y = unsafe {
        y.build(|e| {
            e.iter_mut().for_each(|e| {
                e.write(Default::default());
            })
        })
    };
    let z = unsafe {
        z.build(|e| {
            e.iter_mut().for_each(|e| {
                e.write(Default::default());
            })
        })
    };

    //println!("guard allocation: {:?}", guard.as_ptr_range());

    println!(
        "guarded slices allocations: {:?} {:?} {:?}",
        y.as_ptr_range(),
        z.as_ptr_range(),
        x.as_ptr_range()
    );

    println!("x={:?} y={:?} z={:?}", x, y, z)
}
