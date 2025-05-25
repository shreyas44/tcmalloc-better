unsafe extern "C" {
    /// Return true if `ProcessBackgroundActions` should be called on this platform.
    pub fn NeedsProcessBackgroundActions() -> bool;

    /// Runs housekeeping actions for the allocator off of the main allocation path.
    ///
    /// Should be run in the background thread. May return or may not return.
    pub fn ProcessBackgroundActions();
}
