#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! A Rust raw wrapper over Google's TCMalloc memory allocator
//!
//! ## Feature flags
#![doc = document_features::document_features!()]

#[cfg(feature = "extension")]
#[cfg_attr(docsrs, doc(cfg(feature = "extension")))]
mod extension;

#[cfg(feature = "extension")]
#[cfg_attr(docsrs, doc(cfg(feature = "extension")))]
pub use extension::*;

unsafe extern "C" {
    /// Allocate `size` bytes aligned by `alignment`.
    ///
    /// Return a pointer to the allocated memory or null if out of memory.
    ///
    /// Returns a unique pointer if called with `size` 0. But access to memory by this pointer
    /// is undefined behaviour.
    pub fn BridgeTCMallocInternalNewAlignedNothrow(
        size: libc::size_t,
        alignment: libc::size_t,
    ) -> *mut core::ffi::c_void;

    /// Free previously allocated memory.
    ///
    /// The pointer `ptr` must have been allocated before.
    ///
    /// The `alignment` and `size` must match the ones used to allocate `ptr`.
    pub fn TCMallocInternalDeleteSizedAligned(
        ptr: *mut core::ffi::c_void,
        size: libc::size_t,
        alignment: libc::size_t,
    );

    /// Free previously allocated memory.
    ///
    /// The pointer `ptr` must have been allocated before.
    ///
    /// The `alignment` must match the one used to allocate `ptr`.
    ///
    /// Performance is lower than [`TCMallocInternalDeleteSizedAligned`].
    pub fn TCMallocInternalDeleteAligned(ptr: *mut core::ffi::c_void, alignment: libc::size_t);

    /// Reallocate previously allocated memory.
    ///
    /// The pointer `old_ptr` must have been allocated before.
    ///
    /// The `alignment` must match the one used to allocate `old_ptr`.
    ///
    /// Returned pointer should freed with [`TCMallocInternalDeleteAligned`].
    pub fn BridgeReallocAligned(
        old_ptr: *mut core::ffi::c_void,
        new_size: libc::size_t,
        alignment: libc::size_t,
    ) -> *mut core::ffi::c_void;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_frees_memory_malloc() {
        let ptr = unsafe { BridgeTCMallocInternalNewAlignedNothrow(8, 16) } as *mut u8;
        unsafe { TCMallocInternalDeleteSizedAligned(ptr as *mut libc::c_void, 8, 16) };
    }
}
