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


[API Docs](https://docs.rs/tokio/latest/columned)

A single, contiguous, allocation for multiple arrays, of type `Column<T>`.
Meant to allocate multiple arrays, or so called `Column<T>` that live the same lifetimes.
The lifetimes of a `Column<T>`, and its backing memory, is tied to a `Columned`.
Therefore, the user must guarantee that `Columned` outlives any `Column<T>` which it allocated for.
`Column<T>` originating from a single allocation may have different sizes.  
This crate facilitates the implementation of columnar data structures.

## Example

```rust
use columned::*;

fn main() {
    let _columned; // Ensure this outlives the other variables.

    let mut xs: Column<f64> = Default::default();
    let mut ys: Column<f64> = Default::default();
    let mut sums: Column<f64> = Default::default();

    _columned = unsafe {
        Columned::new([
            xs.alloc(10),
            ys.alloc(10),
            sums.alloc(10)
        ])
    };

    for (i, x) in xs.maybe_uninit().iter_mut().enumerate() {
        x.write(i as f64);
    }

    for (i, y) in ys.maybe_uninit().iter_mut().enumerate() {
        y.write(i as f64);
    }

    for sum in sums.maybe_uninit().iter_mut() {
        sum.write(0.0);
    }

    for ((sum,x),y) in sums.iter_mut().zip(xs.iter()).zip(ys.iter()) {
        *sum = x + y;
    }

    println!("{:?}", sums);
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