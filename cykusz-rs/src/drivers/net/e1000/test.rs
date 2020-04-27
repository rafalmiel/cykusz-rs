pub use super::*;

pub fn test() {
    let mut data = device().data.lock_irq();

    data.send_test();
}

impl E1000Data {
    fn send_test(&mut self) {
        let a =
            &[0xffu8, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0x21, 0xcc, 0xc0, 0x6b, 0x9b, 0x08, 0x06, 0x00, 0x01,
              0x08  , 0x00, 0x06, 0x04, 0x00, 0x01, 0x00, 0x21, 0xcc, 0xc0, 0x6b, 0x9b, 0xc0, 0xa8, 0x01, 0x71,
              0xff  , 0xff, 0xff, 0xff, 0xff, 0xff, 0xc0, 0xa8, 0x01, 0x71];

        unsafe {
            a.as_ptr().copy_to(BUF, a.len());
        }

        self.tx_ring[self.tx_cur as usize].addr = unsafe {
            VirtAddr(BUF as usize).to_phys_pagewalk().unwrap().0 as u64
        };
        self.tx_ring[self.tx_cur as usize].length = 42;
        self.tx_ring[self.tx_cur as usize].cmd = 0b1011;
        self.tx_ring[self.tx_cur as usize].status = TStatus::default();

        let old_cur = self.tx_cur;
        self.tx_cur = (self.tx_cur + 1) % E1000_NUM_TX_DESCS as u32;

        self.addr.write(Regs::TxDescTail, self.tx_cur);

        let status = &self.tx_ring[self.tx_cur as usize].status as *const TStatus;

        unsafe {
            while status.read_volatile().bits() & 0xff == 0 {
                println!("Status: 0b{:b}", status.read_volatile().bits());
            }
        }

        unsafe {
            println!("Send Status: 0x{:x}", status.read_volatile().bits());
        }
    }

}

