use crate::TCMalloc;
use libtcmalloc_sys::{NeedsProcessBackgroundActions, ProcessBackgroundActions};

impl TCMalloc {
    /// Return true if ProcessBackgroundActions should be called on this platform.
    pub fn needs_process_background_actions() -> bool {
        unsafe { NeedsProcessBackgroundActions() }
    }

    /// Runs housekeeping actions for the allocator off of the main allocation path.
    ///
    /// Should be run in the background thread.
    pub fn process_background_actions() -> ! {
        unsafe { ProcessBackgroundActions() };
        unreachable!("ProcessBackgroundActions() should never return")
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
}
