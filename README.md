# TCMalloc better
A Rust wrapper over Google's TCMalloc memory allocator

[![Latest Version]][crates.io] [![Documentation]][docs.rs]

A drop-in global allocator wrapper around the [TCMalloc](https://github.com/google/tcmalloc) allocator.
TCMalloc is a general-purpose, performance-oriented allocator built by Google.

## Usage

```rust
use tcmalloc_better::TCMalloc;

#[global_allocator]
static GLOBAL: TCMalloc = TCMalloc;
```

## Requirements

A __C++__ compiler is required for building [TCMalloc](https://github.com/google/tcmalloc) with cargo.

[crates.io]: https://crates.io/crates/tcmalloc-better
[Latest Version]: https://img.shields.io/crates/v/tcmalloc-better.svg
[Documentation]: https://docs.rs/tcmalloc-better/badge.svg
[docs.rs]: https://docs.rs/tcmalloc-better
