# TCMalloc better
A Rust wrapper over Google's TCMalloc memory allocator

[![Latest Version]][crates.io] [![Documentation]][docs.rs]

A drop-in global allocator wrapper around the [TCMalloc] allocator.
TCMalloc is a general-purpose, performance-oriented allocator built by Google.

## Comparison with other tcmalloc wrappers
Current TCMalloc wrappers rely on [gperftools](https://github.com/gperftools/gperftools), which has been in a detached state from the main development branch
 for over 10 years and lacks modern features such as per-CPU caching.

* [tcmalloc](https://crates.io/crates/tcmalloc) - Cons:
  * Outdated wrapper, which does not updates for years.
  * Depends on gperftools-2.7
* [tcmalloc2](https://crates.io/crates/tcmalloc2) - Cons:
  * Wrapper which can not build offline
  * Depends on gperftools-2.16

## Usage

```rust
use tcmalloc_better::TCMalloc;

#[global_allocator]
static GLOBAL: TCMalloc = TCMalloc;
```

## Requirements

A __C++__ compiler is required for building [TCMalloc] with cargo.

[crates.io]: https://crates.io/crates/tcmalloc-better
[Latest Version]: https://img.shields.io/crates/v/tcmalloc-better.svg
[Documentation]: https://docs.rs/tcmalloc-better/badge.svg
[docs.rs]: https://docs.rs/tcmalloc-better
[TCMalloc]: https://github.com/google/tcmalloc
