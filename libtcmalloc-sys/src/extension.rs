unsafe extern "C" {
    pub fn NeedsProcessBackgroundActions() -> bool;

    pub fn ProcessBackgroundActions();
}
