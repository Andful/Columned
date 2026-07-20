use columned::{Guard, GuardedSliceBuilder, SingleAllocation};
use std::io::BufRead;
use std::mem::MaybeUninit;

use rand::distr::Distribution;
use rand::distr::StandardUniform;

fn init_random<T>(_: usize) -> T
where
    StandardUniform: Distribution<T>,
{
    rand::random()
}

fn main() {
    let mut data = [MaybeUninit::<u64>::uninit(); 1 << 16];
    let allocator = SingleAllocation::new(&mut data);
    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();
    loop {
        println!("Input a number");
        let Ok(n): Result<usize, _> = lines.next().unwrap().unwrap().parse() else {
            break;
        };
        let mut guard = Guard::new_in(&allocator);

        let mut x = GuardedSliceBuilder::<f64>::new(n);
        let mut y = GuardedSliceBuilder::<f64>::new(n);
        let mut z = GuardedSliceBuilder::<MaybeUninit<f64>>::new(n);

        guard
            .subscriber()
            .subscribe(&mut x)
            .subscribe(&mut y)
            .subscribe(&mut z)
            .allocate()
            .unwrap();

        let x = x.build_from_fn(init_random);
        let y = y.build_from_fn(init_random);
        let mut z = z.build_uninit();

        for i in 0..x.len() {
            z[i].write(x[i] + y[i]);
        }

        let z = unsafe { z.assume_init() };

        for i in 0..x.len() {
            assert_eq!(z[i], x[i] + y[i]);
        }
    }
}
