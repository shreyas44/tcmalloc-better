#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! A drop-in global allocator wrapper around the [TCMalloc](https://github.com/google/tcmalloc) allocator.
//! TCMalloc is a general-purpose, performance-oriented allocator built by Google.
//!
//! ## Usage
//! ```rust,ignore
//! use tcmalloc_better::TCMalloc;
//!
//! #[global_allocator]
//! static GLOBAL: TCMalloc = TCMalloc;
//!
//! fn main() {
//!     TCMalloc::process_background_actions_thread();
//!
//!     // Rest of main
//! }
//! ```
//!
//! ## Caveats
//!
//! This library is intended for use on Linux (x86_64, aarch64, powerpc64le).
//! For applications requiring support on a broader range of platforms,
//! consider using [malloc-best-effort](https://crates.io/crates/malloc-best-effort), which automatically selects
//! the best allocator based on the target platform.
//!
//! ## Feature flags
#![doc = document_features::document_features!()]

#[cfg(feature = "extension")]
#[cfg_attr(docsrs, doc(cfg(feature = "extension")))]
mod extension;

use core::alloc::{GlobalAlloc, Layout};
use libtcmalloc_sys::{
    TCMallocInternalDeleteSizedAligned, TCMallocInternalNewAlignedNothrowBridge,
};

/// A memory allocator that can be registered as the standard library’s default
/// through the `#[global_allocator]` attribute.
pub struct TCMalloc;

unsafe impl GlobalAlloc for TCMalloc {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { TCMallocInternalNewAlignedNothrowBridge(layout.size(), layout.align()) as *mut u8 }
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            TCMallocInternalDeleteSizedAligned(
                ptr as *mut core::ffi::c_void,
                layout.size(),
                layout.align(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_frees_allocated_memory() {
        unsafe {
            let layout = Layout::from_size_align(8, 16).unwrap();
            let alloc = TCMalloc;

            let ptr = alloc.alloc(layout);
            alloc.dealloc(ptr, layout);
        }
    }

    #[test]
    fn it_frees_allocated_big_memory() {
        unsafe {
            let layout = Layout::from_size_align(1 << 20, 32).unwrap();
            let alloc = TCMalloc;

            let ptr = alloc.alloc(layout);
            alloc.dealloc(ptr, layout);
        }
    }

    #[test]
    fn it_frees_zero_allocated_memory() {
        unsafe {
            let layout = Layout::from_size_align(8, 16).unwrap();
            let alloc = TCMalloc;

            let ptr = alloc.alloc_zeroed(layout);
            alloc.dealloc(ptr, layout);
        }
    }

    #[test]
    fn it_frees_zero_allocated_big_memory() {
        unsafe {
            let layout = Layout::from_size_align(1 << 20, 32).unwrap();
            let alloc = TCMalloc;

            let ptr = alloc.alloc_zeroed(layout);
            alloc.dealloc(ptr, layout);
        }
    }

    #[test]
    fn it_frees_reallocated_memory() {
        unsafe {
            let layout = Layout::from_size_align(8, 16).unwrap();
            let new_size = 16;
            let new_layout = Layout::from_size_align(new_size, layout.align()).unwrap();
            let alloc = TCMalloc;

            let ptr = alloc.alloc(layout);
            let ptr = alloc.realloc(ptr, layout, new_size);
            alloc.dealloc(ptr, new_layout);
        }
    }

    #[test]
    fn it_frees_reallocated_big_memory() {
        unsafe {
            let layout = Layout::from_size_align(1 << 20, 32).unwrap();
            let new_size = 2 << 20;
            let new_layout = Layout::from_size_align(new_size, layout.align()).unwrap();
            let alloc = TCMalloc;

            let ptr = alloc.alloc(layout);
            let ptr = alloc.realloc(ptr, layout, new_size);
            alloc.dealloc(ptr, new_layout);
        }
    }
}
