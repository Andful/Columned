use columned::{Guard, GuardedBuilder};

fn main() {
    let mut guard = Guard::new();

    {
        let subscriber = guard.subscriber();

        let mut x: GuardedBuilder<[u8]> = GuardedBuilder::new_slice(10);
        let mut y: GuardedBuilder<[u32]> = GuardedBuilder::new_slice(10);
        let mut z: GuardedBuilder<[u16]> = GuardedBuilder::new_slice(10);

        subscriber
            .subscribe(&mut x)
            .subscribe(&mut y)
            .subscribe(&mut z)
            .allocate()
            .unwrap();

        let x = x.build_default();
        let y = y.build_default();
        let z = z.build_default();

        //println!("guard allocation: {:?}", guard.as_ptr_range());

        println!(
            "guarded slices allocations: {:?} {:?} {:?}",
            x.as_ptr_range(),
            y.as_ptr_range(),
            z.as_ptr_range()
        );

        println!("x={:?} y={:?} z={:?}", x, y, z);
    }

    println!("{:?}", guard.as_ptr_range())
}
