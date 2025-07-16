use crate::TCMalloc;
use libtcmalloc_sys::{NeedsProcessBackgroundActions, ProcessBackgroundActions};
#[cfg(feature = "std")]
use std::thread;

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "extension")]
#[cfg_attr(docsrs, doc(cfg(feature = "extension")))]
impl TCMalloc {
    /// Return true if `process_background_actions` should be called on this platform.
    #[inline]
    pub fn needs_process_background_actions() -> bool {
        unsafe { NeedsProcessBackgroundActions() }
    }

    /// Runs housekeeping actions for the allocator off of the main allocation path.
    ///
    /// Should be run in the background thread. May return or may not return.
    /// Use `process_background_actions_thread()` if possible.
    #[inline]
    pub fn process_background_actions() {
        unsafe { ProcessBackgroundActions() };
    }

    /// Runs housekeeping actions for the allocator in the background thread.
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    pub fn process_background_actions_thread() -> Option<thread::JoinHandle<()>> {
        if Self::needs_process_background_actions() {
            Some(thread::spawn(|| {
                Self::process_background_actions();
            }))
        } else {
            None
        }
    }

    /// Sets the maximum cache size per CPU cache. This is a per-core limit.
    #[inline]
    pub fn set_max_per_cpu_cache_size(value: i32) {
        unsafe { libtcmalloc_sys::TCMalloc_Internal_SetMaxPerCpuCacheSize(value) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn test_needs_process_background_actions() {
        assert!(!TCMalloc::needs_process_background_actions());
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_process_background_actions() {
        TCMalloc::process_background_actions_thread();
    }
}
