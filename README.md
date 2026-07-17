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

A single, contiguous, allocation for multiple arrays.
Meant to allocate multiple arrays, that live the same lifetimes.
This reduces multiple allocations, to a single one.
This crate may facilitates the implementation of columnar data structures.

## Example

```rust
use columned::{Allocate, with_allocation};

fn main() {
    let xs: Allocate<u64, _> = unsafe {
        Allocate::alloc(10, |xs| {
            for (i, x) in xs.iter_mut().enumerate() {
                x.write(i as u64);
            }
        })
    };
    let ys: Allocate<u64, _> = unsafe {
        Allocate::alloc(10, |ys| {
            for (i, y) in ys.iter_mut().enumerate() {
                y.write(i as u64);
            }
        })
    };
    let sums: Allocate<u64, _> = unsafe {
        Allocate::alloc(10, |sums| {
            for sum in sums.iter_mut() {
                sum.write(0);
            }
        })
    };

    with_allocation((xs, ys, sums), |(xs, ys, sums)| {
        for ((sum, x), y) in sums.iter_mut().zip(xs.iter()).zip(ys.iter()) {
            *sum = x + y;
        }

        for (i, sum) in sums.iter().enumerate() {
            assert_eq!(*sum, 2 * i as u64);
        }
    })
    .unwrap();
}
```

# Working Principle

`Columned` manages a contiguous allocation of memory. Each `Coulmn` have a pointer to this contiguous allocation. The following figure illustrates the working principle.

```text
       Columned
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
       Column<f32>                     Column<u16>
```
This also means that the user has to ensure that `Columned` outlives the `Columns` that uses its managed memory.