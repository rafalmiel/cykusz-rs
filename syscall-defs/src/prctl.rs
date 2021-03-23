#[repr(usize)]
pub enum PrctlCmd {
    Unknown = 0x0,
    ArchSetFs = 0x1002,
}

impl From<usize> for PrctlCmd {
    fn from(v: usize) -> Self {
        match v {
            0x1002 => PrctlCmd::ArchSetFs,
            _ => PrctlCmd::Unknown,
        }
    }
}
