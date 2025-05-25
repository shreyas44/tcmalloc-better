#![no_std]

#[cfg(feature = "extension")]
mod extension;

#[cfg(feature = "extension")]
pub use extension::*;

unsafe extern "C" {
    /// Allocate `size` bytes aligned by `align`.
    ///
    /// Return a pointer to the allocated memory or null if out of memory.
    ///
    /// Returns a unique pointer if called with `size` 0.
    pub fn TCMallocInternalAlignedAlloc(
        align: libc::size_t,
        size: libc::size_t,
    ) -> *mut core::ffi::c_void;

    /// Free previously allocated memory.
    ///
    /// The pointer `ptr` must have been allocated before (or be null).
    ///
    /// The `align` and `size` must match the ones used to allocate `ptr`.
    pub fn TCMallocInternalFreeAlignedSized(
        ptr: *mut core::ffi::c_void,
        align: libc::size_t,
        size: libc::size_t,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_frees_memory_malloc() {
        let ptr = unsafe { TCMallocInternalAlignedAlloc(8, 8) } as *mut u8;
        unsafe { TCMallocInternalFreeAlignedSized(ptr as *mut libc::c_void, 8, 8) };
    }
}
