pub type DevId = u64;

pub fn makedev(major: DevId, minor: DevId) -> DevId {
    let mut dev = 0;
    dev |= (major & 0x00000fff) << 8;
    dev |= (major & 0xfffff000) << 32;
    dev |= (minor & 0x000000ff) << 0;
    dev |= (minor & 0xffffff00) << 12;

    dev
}

pub fn major(dev: DevId) -> DevId {
    let mut major = 0;
    major |= (dev & 0x00000000000fff00) >> 8;
    major |= (dev & 0xfffff00000000000) >> 32;

    major
}

pub fn minor(dev: DevId) -> DevId {
    let mut minor = 0;
    minor |= (dev & 0x00000000000000ff) >> 0;
    minor |= (dev & 0x00000ffffff00000) >> 12;

    minor
}
