use bit_field::BitField;

#[repr(transparent)]
pub struct Address(u64);

#[repr(transparent)]
pub struct Data(u16);

impl Address {
    pub fn new(target_proc: u32) -> Address {
        let mut addr = Address(0);
        addr.0.set_bits(20..32, 0xFEE);
        addr.0.set_bits(12..20, target_proc as u64);

        addr
    }

    pub fn val(&self) -> u64 {
        self.0
    }
}

impl Data {
    pub fn new(vector: u8, level_trigerred: bool, active_low: bool) -> Data {
        let mut data = Data(0);
        data.0.set_bits(0..8, vector as u16);
        data.0.set_bit(14, active_low);
        data.0.set_bit(15, level_trigerred);

        data
    }

    pub fn val(&self) -> u16 {
        self.0
    }
}

pub fn get_addr_data(
    vector: u8,
    level_triggered: bool,
    active_low: bool,
    target_proc: u32,
) -> (Address, Data) {
    (
        Address::new(target_proc),
        Data::new(vector, level_triggered, active_low),
    )
}
