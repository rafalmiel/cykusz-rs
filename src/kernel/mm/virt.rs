bitflags! {
    pub struct PageFlags: usize {
        const WRITABLE      = 1 << 0;
        const USER          = 1 << 1;
        const NO_EXECUTE    = 1 << 2;
    }
}
