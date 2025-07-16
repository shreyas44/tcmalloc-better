unsafe extern "C" {
    /// Return true if `ProcessBackgroundActions` should be called on this platform.
    pub fn NeedsProcessBackgroundActions() -> bool;

    /// Runs housekeeping actions for the allocator off of the main allocation path.
    ///
    /// Should be run in the background thread. May return or may not return.
    pub fn ProcessBackgroundActions();

    /// Sets the maximum cache size per CPU cache. This is a per-core limit.
    pub fn TCMalloc_Internal_SetMaxPerCpuCacheSize(value: i32);
}
