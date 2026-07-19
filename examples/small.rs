use columned::{Guard, GuardedSlice, GuardedSliceBuilder, Subscriber};

#[derive(Debug)]
struct Vec3<'a> {
    x: GuardedSlice<'a, f32>,
    y: GuardedSlice<'a, f32>,
    z: GuardedSlice<'a, f32>,
}

fn doodle(subscriber: impl Subscriber<'a>) -> impl Subscriber<'a> {}

trait Builder<'a> {
    fn build(&mut self, s: impl Subscriber<'a>);
}

impl<'a> Vec3<'a> {
    fn new(subscriber: impl Subscriber<'a>, builder: &mut impl Builder<'a>) -> Self {
        let mut x = GuardedSliceBuilder::<f32>::new_default(100);
        let mut y = GuardedSliceBuilder::<f32>::new_default(100);
        let mut z = GuardedSliceBuilder::<f32>::new_default(100);

        let subscriber = subscriber
            .subscribe(&mut x)
            .subscribe(&mut y)
            .subscribe(&mut z);

        builder.build(subscriber);

        // Continue

        Self {
            x: x.build(),
            y: y.build(),
            z: z.build(),
        }
    }
}

fn main() {
    let mut guard = Guard::new();

    let mut v = Vec3Builder::new(100);

    v.subscribe(guard.subscriber()).finish().unwrap();

    let v = v.build();
    //drop(guard); // would cause a compile error

    println!("{:?}", v);

    // use bodies here ...
}
