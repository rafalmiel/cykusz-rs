#![allow(dead_code)]

use super::reg::verb;
use crate::arch::mm::{PhysAddr, VirtAddr};
use crate::drivers::audio::hda::{reg, Address};
use crate::kernel::mm::virt::PageFlags;
use crate::kernel::mm::{allocate_order, map_to_flags};
use bit_field::BitField;
use tock_registers::interfaces::{Readable, Writeable};

struct Corb {
    reg: &'static mut reg::Corb,
    buf: VirtAddr,
    count: usize,
}

impl Corb {
    fn new(reg_addr: VirtAddr) -> Corb {
        Corb {
            reg: unsafe { reg_addr.read_mut::<reg::Corb>() },
            buf: VirtAddr(0),
            count: 0,
        }
    }

    fn setup(&mut self, buffer: PhysAddr) {
        self.stop();

        self.buf = buffer.to_virt();

        let size_cap = self.reg.size.size_cap();

        assert_ne!(0, size_cap);

        for (bit, corb_count, val) in [
            (2, 256, reg::CorbSize::CORBSIZE::Value::Entries256),
            (1, 16, reg::CorbSize::CORBSIZE::Value::Entries16),
            (0, 2, reg::CorbSize::CORBSIZE::Value::Entries2),
        ] {
            if size_cap.get_bit(bit) {
                self.count = corb_count;

                self.reg.size.set_size(val);

                self.set_address(buffer);

                self.reset_read_pointer();
                self.set_write_pointer(0);

                break;
            }
        }
    }

    fn set_write_pointer(&self, val: u16) {
        self.reg.wp.set_wp(val);
    }

    fn reset_read_pointer(&self) {
        self.reg.rp.set_is_rp_reset(true);
    }

    fn set_address(&self, buffer: PhysAddr) {
        self.reg.lowbase.set(buffer.0.get_bits(0..32) as u32);
        self.reg.upbase.set(buffer.0.get_bits(32..64) as u32);
    }

    fn stop(&self) {
        while self.reg.ctl.is_run() {
            self.reg.ctl.set_is_run(false);
        }
    }

    fn start(&self) {
        dbgln!(audio_v, "start corb");
        self.reg.ctl.set_is_run(true);

        while !self.reg.ctl.is_run() {}
    }

    fn next_write_pointer(&self) -> usize {
        (self.reg.wp.wp() + 1) as usize % self.count
    }

    fn wait_for_dma_not_busy(&self) {
        while self.reg.wp.wp() != self.reg.rp.rp() {}
    }

    fn send_cmd(&self, verb: u32) {
        dbgln!(audio_v, "send_cmd: {}", verb);
        self.wait_for_dma_not_busy();

        let wp = self.next_write_pointer();

        unsafe {
            dbgln!(audio_v, "store cmd buf: {}, wp: {}", self.buf, wp);
            (self.buf + 4 * wp).store(verb);
        }

        self.set_write_pointer(wp as u16);

        dbgln!(audio_v, "cmd sent");
    }
}

struct Rirb {
    reg: &'static mut reg::Rirb,
    buf: VirtAddr,
    rp: usize,
    count: usize,
}

impl Rirb {
    fn new(reg_addr: VirtAddr) -> Rirb {
        Rirb {
            reg: unsafe { reg_addr.read_mut::<reg::Rirb>() },
            buf: VirtAddr(0),
            rp: 0,
            count: 0,
        }
    }

    fn setup(&mut self, buffer: PhysAddr) {
        self.stop();

        self.buf = buffer.to_virt();

        let size_cap = self.reg.size.size_cap();

        assert_ne!(0, size_cap);

        for (bit, count, val) in [
            (2, 256, reg::RirbSize::RIRBSIZE::Value::Entries256),
            (1, 16, reg::RirbSize::RIRBSIZE::Value::Entries16),
            (0, 2, reg::RirbSize::RIRBSIZE::Value::Entries2),
        ] {
            if size_cap.get_bit(bit) {
                self.reg.size.set_size(val);

                self.count = count;

                dbgln!(audio, "Entry count: {}", count);

                self.set_address(buffer);

                self.reset_write_pointer();
                self.set_read_pointer(0);

                self.reg.intcnt.set_int_cnt(u8::MAX as u16);

                break;
            }
        }
    }

    fn reset_write_pointer(&self) {
        self.reg.wp.set_is_wp_reset(true);
    }

    fn set_write_pointer(&self, rp: u16) {
        self.reg.wp.set_wp(rp);
    }

    fn set_read_pointer(&mut self, rp: usize) {
        self.rp = rp;
    }

    fn set_address(&self, buffer: PhysAddr) {
        self.reg.lowbase.set(buffer.0.get_bits(0..32) as u32);
        self.reg.upbase.set(buffer.0.get_bits(32..64) as u32);
    }

    fn stop(&self) {
        self.reg.ctl.set_is_dma_en(false);
    }

    fn start(&self) {
        dbgln!(audio_v, "start rirb");
        self.reg.ctl.set_is_dma_en(true);
    }

    fn wait_for_response(&mut self) {
        while self.reg.wp.wp() == self.rp as u16 {
            dbgln!(audio_v, "reg wp: {}, rp: {}", self.reg.wp.wp(), self.rp);
        }
    }

    fn next_read_pointer(&self) -> usize {
        (self.rp + 1) % self.count
    }

    fn read_response(&mut self) -> u64 {
        dbgln!(audio_v, "reg wp: {}, rp: {}", self.reg.wp.wp(), self.rp);
        // Wait until we get the response
        self.wait_for_response();

        // Calculate read pointer
        let rp = self.next_read_pointer();

        // Read the result
        let result = unsafe { (self.buf + rp * 8).read::<u64>() };

        dbgln!(audio_v, "read response: {}", result);

        // Update write and read pointers
        self.set_write_pointer(rp as u16);
        self.set_read_pointer(rp);

        result
    }
}

struct Immediate {
    reg: &'static mut reg::Immediate,
}

impl Immediate {
    fn new(reg_addr: VirtAddr) -> Immediate {
        Immediate {
            reg: unsafe { reg_addr.read_mut::<reg::Immediate>() },
        }
    }

    fn cmd(&self, verb: u32) -> u64 {
        // Wait until not busy
        while self.reg.status.is_icb() {}

        // Write verb to the output
        self.reg.output.set(verb);

        // Send the command
        self.reg.status.set_is_icb(true);

        // Wait until result valid
        while !self.reg.status.is_irv() {}

        // Clear the Immediate Result Valid bit
        self.reg.status.set_is_irv(true);

        // Read the response
        let mut response = self.reg.input.get() as u64;
        response.set_bits(32..64, self.reg.input.get() as u64);

        response
    }
}

#[repr(C)]
pub struct Command {
    corb: Corb,
    rirb: Rirb,
    imm: Immediate,
    use_imm: bool,
}

impl Command {
    pub fn new(base_addr: VirtAddr, use_imm: bool) -> Command {
        Command {
            corb: Corb::new(base_addr + 0x40),
            rirb: Rirb::new(base_addr + 0x50),
            imm: Immediate::new(base_addr + 0x60),
            use_imm,
        }
    }

    pub fn setup(&mut self) {
        let cmd_mem = allocate_order(0)
            .expect("Failed to allocate corb mem")
            .address();
        map_to_flags(
            cmd_mem.to_virt(),
            cmd_mem,
            PageFlags::NO_CACHE | PageFlags::WRITABLE,
        );

        self.corb.setup(cmd_mem);
        self.rirb.setup(cmd_mem + 2048);

        if !self.use_imm {
            self.start_corb();
        }
    }

    fn immediate_command(&mut self, verb: u32) -> u64 {
        self.imm.cmd(verb)
    }

    fn do_cmd(&mut self, verb: u32) -> u64 {
        if self.use_imm {
            self.immediate_command(verb)
        } else {
            dbgln!(audio_cmd, "Sending verb: {:#X}", verb);
            self.corb.send_cmd(verb);
            dbgln!(audio_cmd, "Reading response");
            let a = self.rirb.read_response();
            dbgln!(audio_cmd, "Response read: {:#X}", a);
            a
        }
    }
    pub fn invoke<P: verb::NodeCommandConstData>(&mut self, address: Address) -> P::Output {
        let mut verb = 0u32;
        verb.set_bits(28..=31, address.codec())
            .set_bits(20..=27, address.node())
            .set_bits(P::PAYLOAD_SIZE..=19, P::COMMAND)
            .set_bits(0..P::PAYLOAD_SIZE, P::DATA);

        self.do_cmd(verb).into()
    }

    pub fn invoke_data<P: verb::NodeCommand>(
        &mut self,
        address: Address,
        data: P::Data,
    ) -> P::Output {
        let data = data.into();
        dbgln!(
            audio_cmd,
            "Invoke data: {:?} payload size: {} cmd: {:#X} data: {:#X}",
            address,
            P::PAYLOAD_SIZE,
            P::COMMAND,
            data
        );
        let mut verb = 0u32;
        verb.set_bits(28..=31, address.codec())
            .set_bits(20..=27, address.node())
            .set_bits(P::PAYLOAD_SIZE..=19, P::COMMAND)
            .set_bits(0..P::PAYLOAD_SIZE, data);

        dbgln!(audio_cmd, "Sending verb: {:#X}", verb);

        self.do_cmd(verb).into()
    }

    fn start_corb(&self) {
        self.corb.start();
        self.rirb.start();
    }

    fn stop_corb(&self) {
        self.corb.stop();
        self.rirb.stop();
    }
}
