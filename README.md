# Columned

[![Crates.io][crates-badge]][crates-url]
[![Docs.rs][docs-badge]][docs-url]
[![MIT licensed][mit-badge]][mit-url]

[crates-url]: https://crates.io/crates/columned
[crates-badge]: https://img.shields.io/crates/v/columned.svg
[docs-url]: https://docs.rs/columned
[docs-badge]: https://docs.rs/columned/badge.svg
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/Andful/Columned/blob/master/LICENSE
<!--[actions-badge]: https://github.com/tokio-rs/tokio/workflows/CI/badge.svg-->
<!--[actions-url]: https://github.com/tokio-rs/tokio/actions?query=workflow%3ACI+branch%3Amaster-->

A single, contiguous, allocation for multiple arrays, such as for, structure-of-arrays.  
Meant to allocate multiple arrays, that live the same lifetimes.
This reduces multiple allocations, to a single one. This may improve performance, 
as multiple memory allocations may need multiple, slow, system calls.
Further, this may alleviate memory fragmentation. This crate facilitates the implementation of
columnar/structure-of-arrays data structures.

# Working Principle

`Guard` manages a contiguous allocation of memory. Each slice has a pointer to this contiguous allocation. The following figure illustrates the working principle.

```text
       Guard
       +--------+--------+
       | 0x0123 |   ...  |
       +--------+--------+
        ptr
         |
         V
Heap   +---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+
       |           0.1 |           3.2 |     5 |     7 |    20 |     6 |
       +---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+
         ^                               ^
         |                               |
        ptr      len                    ptr      len
       +--------+--------+             +--------+--------+
       | 0x0123 |      2 |             | 0x012b |      4 |
       +--------+--------+             +--------+--------+
       GuardedSlice<f32>               GuardedSlice<u16>
```
The lifetimes of the type system will ensure that the `Guard` will outlive any slice.

# Examples
## Simple Example
```rust
use columned::{Guard, PrepAlloc, allocate};

fn main() {
    //Declare size and initialization of the slices.
    let xs: PrepAlloc<u64, _> = unsafe {
        PrepAlloc::new(10, |xs| {
            for (i, x) in xs.iter_mut().enumerate() {
                x.write(i as u64);
            }
        })
    };
    let ys: PrepAlloc<u64, _> = unsafe {
        PrepAlloc::new(10, |ys| {
            for (i, y) in ys.iter_mut().enumerate() {
                y.write(i as u64);
            }
        })
    };
    let sums: PrepAlloc<u64, _> = unsafe {
        PrepAlloc::new(10, |sums| {
            for sum in sums.iter_mut() {
                sum.write(0);
            }
        })
    };

    //Initialize a "Guard", which will manage the allocation.
    let mut guard: Guard = Guard::default();

    let (xs, ys, mut sums) = allocate(&mut guard, (xs, ys, sums)).unwrap();

    //drop(guard); // This would cause a compilation error

    for ((mut sum, x), y) in sums.iter_mut().zip(xs.iter()).zip(ys.iter()) {
        *sum = x + y;
    }

    for (i, sum) in sums.iter().enumerate() {
        assert_eq!(*sum, 2 * i as u64);
    }
}
```
## Structure of Array Example
```rust
use std::mem::MaybeUninit;
use columned::{Guard, PrepAlloc, GuardedSlice, allocate};

// The structure-of-array
struct Bodies<'a> {
    //Position
    position: Vec3<'a>,
    //Velocity
    velocity: Vec3<'a>,
    //Mass
    mass: GuardedSlice<'a, f32>,
}

struct Vec3<'a> {
    x: GuardedSlice<'a, f32>,
    y: GuardedSlice<'a, f32>,
    z: GuardedSlice<'a, f32>,
}

fn generate_n_bodies<'a>(n: usize, guard: &'a mut Guard) -> Bodies<'a> {
    let init_to_zero = |data: &mut [std::mem::MaybeUninit<f32>]| {
         data.iter_mut().for_each(|d| {
                d.write(0.0);
         })
    };

    let x = unsafe { PrepAlloc::new(n, init_to_zero) };
    let y = unsafe { PrepAlloc::new(n, init_to_zero) };
    let z = unsafe { PrepAlloc::new(n, init_to_zero) };
    let vx = unsafe { PrepAlloc::new(n, init_to_zero) };
    let vy = unsafe { PrepAlloc::new(n, init_to_zero) };
    let vz = unsafe { PrepAlloc::new(n, init_to_zero) };
    let mass = unsafe { PrepAlloc::new(n, init_to_zero) };

    let (x, y, z, vx, vy, vz, mass) = allocate(guard, (x, y, z, vx, vy, vz, mass)).unwrap();

    Bodies {
        position: Vec3 {
            x,
            y,
            z,
        },
        velocity: Vec3 {
            x: vx,
            y: vy,
            z: vz,
        },
        mass,
    }
}

fn main() {
    let mut guard = Guard::new();
    let bodies = generate_n_bodies(100, &mut guard);
    
    //drop(guard); // would cause a compile error
    
    // use bodies here ...
}
```