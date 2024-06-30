#![allow(dead_code)]

use crate::arch::mm::{PhysAddr, VirtAddr};
use crate::drivers::audio::hda::reg;
use crate::drivers::audio::hda::reg::{
    BufferDescriptorListEntry, StreamControl, StreamFormat, StreamStatus, WrapLocal,
};
use bit_field::BitField;
use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::LocalRegisterCopy;

#[derive(Copy, Clone)]
pub struct Stream {
    reg: &'static reg::Stream,
    dpl: VirtAddr,
}

unsafe impl Sync for Stream {}
unsafe impl Send for Stream {}

pub struct SampleRate {
    base: StreamFormat::BASE::Value,
    mult: StreamFormat::MULT::Value,
    div: u16,
}

use crate::kernel::mm::virt::PageFlags;
use crate::kernel::mm::{allocate_order, deallocate_order, map_to_flags, Frame, PAGE_SIZE};
use StreamFormat::MULT::Value::*;
use {StreamFormat::BASE::Value::KHZ44, StreamFormat::BASE::Value::KHZ48};

pub const SR_8: SampleRate = SampleRate {
    base: KHZ48,
    mult: NONE,
    div: 6,
};
pub const SR_11_025: SampleRate = SampleRate {
    base: KHZ44,
    mult: NONE,
    div: 4,
};
pub const SR_16: SampleRate = SampleRate {
    base: KHZ48,
    mult: NONE,
    div: 3,
};
pub const SR_22_05: SampleRate = SampleRate {
    base: KHZ44,
    mult: NONE,
    div: 2,
};
pub const SR_32: SampleRate = SampleRate {
    base: KHZ48,
    mult: X2,
    div: 3,
};

pub const SR_44_1: SampleRate = SampleRate {
    base: KHZ44,
    mult: NONE,
    div: 1,
};
pub const SR_48: SampleRate = SampleRate {
    base: KHZ48,
    mult: NONE,
    div: 1,
};
pub const SR_88_1: SampleRate = SampleRate {
    base: KHZ44,
    mult: X2,
    div: 1,
};
pub const SR_96: SampleRate = SampleRate {
    base: KHZ48,
    mult: X2,
    div: 1,
};
pub const SR_176_4: SampleRate = SampleRate {
    base: KHZ44,
    mult: X4,
    div: 1,
};
pub const SR_192: SampleRate = SampleRate {
    base: KHZ48,
    mult: X4,
    div: 1,
};

impl Stream {
    pub fn new(reg: VirtAddr, dpl: VirtAddr) -> Stream {
        Stream {
            reg: unsafe { reg.read_ref() },
            dpl,
        }
    }

    pub fn status(&self) -> WrapLocal<u8, StreamStatus::Register> {
        self.reg.status.get_local()
    }

    pub fn set_status(&self, status: WrapLocal<u8, StreamStatus::Register>) {
        self.reg.status.set(status.get())
    }

    pub fn control(&self) -> WrapLocal<u32, StreamControl::Register> {
        let ctrl: u32 = *0u32
            .set_bits(0..8, self.reg.ctllow.get() as u32)
            .set_bits(16..24, self.reg.ctlhigh.get() as u32);
        WrapLocal(LocalRegisterCopy::new(ctrl))
    }

    pub fn set_control(&self, ctrl: WrapLocal<u32, StreamControl::Register>) {
        self.reg.ctllow.set(ctrl.get().get_bits(0..8) as u8);
        self.reg.ctlhigh.set(ctrl.get().get_bits(16..24) as u8);
    }

    pub fn set_pcm_format(
        &self,
        sample_rate: &SampleRate,
        bits: StreamFormat::BITS::Value,
        channels: u8,
    ) {
        let mut pcm = WrapLocal::<u16, StreamFormat::Register>::new();
        pcm.set_base(sample_rate.base);
        pcm.set_div(sample_rate.div - 1);
        pcm.set_mult(sample_rate.mult);
        pcm.set_bits(bits);
        pcm.set_channel((channels - 1) as u16);
        self.reg.fmt.set(pcm.get());
    }

    pub fn fifo_size(&self) -> u16 {
        self.reg.fifos.get()
    }

    pub fn cyclic_buffer_length(&self) -> u32 {
        self.reg.buflen.get()
    }

    pub fn set_cyclic_buffer_length(&self, length: u32) {
        self.reg.buflen.set(length)
    }

    pub fn run(&self) {
        let mut ctrl = self.control();
        ctrl.set_is_run(true);
        self.set_control(ctrl);
    }

    pub fn stop(&self) {
        let mut ctrl = self.control();
        ctrl.set_is_run(false);
        self.set_control(ctrl);
    }

    pub fn stream_number(&self) -> u8 {
        self.control().stream() as u8
    }

    pub fn set_stream_number(&self, nr: u8) {
        let mut ctrl = self.control();
        ctrl.set_stream(nr as u32);
        self.set_control(ctrl);
    }

    pub fn set_address(&self, addr: PhysAddr) {
        self.reg.bdpllowbase.set(addr.0 as u32);
        self.reg.bdplupbase.set(addr.0.get_bits(32..) as u32);
    }

    pub fn set_last_valid_index(&self, index: u16) {
        self.reg.lastvali.set(index);
    }

    pub fn link_position(&self) -> u32 {
        self.reg.linkpos.get()
    }

    pub fn dpl_link_position(&self) -> u32 {
        unsafe { self.dpl.read_volatile::<u32>() }
    }

    pub fn set_interrupt_on_completion(&self, enable: bool) {
        let mut ctrl = self.control();
        ctrl.set_is_ioce(enable);
        self.set_control(ctrl);
    }

    pub fn is_buffer_complete(&self) -> bool {
        self.status().is_bcis()
    }

    pub fn clear_interrupts(&self) {
        let mut st = self.status();
        st.set_is_bcis(true); //Write 1 to clear
        st.set_is_dese(true);
        st.set_is_fifoe(true);
        self.set_status(st);
    }

    pub fn sample_size(&self) -> usize {
        use reg::StreamFormat::BITS::Value;
        let format = self.reg.fmt.get_local();
        let channel = format.channel() as usize;
        let bits = format.bits().unwrap();
        match bits {
            Value::BITS8 => 1 * (channel + 1),
            Value::BITS16 => 2 * (channel + 1),
            _ => 4 + (channel + 1),
        }
    }

    pub fn reg_address(&self) -> PhysAddr {
        VirtAddr(self.reg as *const _ as usize).to_phys()
    }
}

pub struct BufferDescriptorListTemplate<const ENTRIES: usize, const ENTRY_SIZE: usize> {
    list: &'static mut [BufferDescriptorListEntry],
    buffer_order: usize,
}

impl<const ENTRIES: usize, const ENTRY_SIZE: usize>
    BufferDescriptorListTemplate<ENTRIES, ENTRY_SIZE>
{
    pub fn new() -> Option<BufferDescriptorListTemplate<ENTRIES, ENTRY_SIZE>> {
        assert_eq!(ENTRY_SIZE % 128, 0);
        assert!(ENTRIES <= 256);

        let mem = allocate_order(0)?.address();

        map_to_flags(
            mem.to_virt(),
            mem,
            PageFlags::WRITABLE | PageFlags::NO_CACHE,
        );

        let size = ENTRIES * ENTRY_SIZE;

        let buffer_order = crate::arch::mm::phys::order_for_size(size)?;

        let buf_mem = allocate_order(buffer_order)?.address();

        dbgln!(
            audio,
            "buffer order: {} {:#x} - {:#x}",
            buffer_order,
            buf_mem.0,
            buf_mem.0 + size
        );

        for page in (buf_mem..(buf_mem + size)).step_by(PAGE_SIZE) {
            map_to_flags(
                page.to_virt(),
                page,
                PageFlags::WRITABLE | PageFlags::NO_CACHE,
            );
            unsafe {
                page.to_virt().as_slice_mut::<u8>(PAGE_SIZE).fill(0);
            }
        }

        let list = unsafe {
            mem.to_virt()
                .read_mut::<[BufferDescriptorListEntry; ENTRIES]>()
        };

        for (idx, entry) in list.iter().enumerate() {
            dbgln!(
                audio,
                "Set buffer list ptr {}: {:#X}",
                idx,
                buf_mem.0 + (idx * ENTRY_SIZE)
            );
            entry.address.set((buf_mem.0 + (idx * ENTRY_SIZE)) as u64);
            entry.length.set(ENTRY_SIZE as u32);
            entry.ioc.set_is_ioc(true);
        }

        Some(BufferDescriptorListTemplate { list, buffer_order })
    }

    pub fn list_address(&self) -> PhysAddr {
        VirtAddr(self.list.as_ptr() as usize).to_phys()
    }

    pub fn buffer_address(&self) -> VirtAddr {
        PhysAddr(self.list[0].address.get() as usize).to_virt()
    }

    pub const fn buffer_size(&self) -> usize {
        ENTRIES * ENTRY_SIZE
    }

    pub const fn block_size(&self) -> usize {
        ENTRY_SIZE
    }

    pub const fn entries(&self) -> usize {
        ENTRIES
    }
}

pub type BufferDescriptorList = BufferDescriptorListTemplate<32, 2048>;

impl<'a, const ENTRIES: usize, const ENTRY_SIZE: usize> Drop
    for BufferDescriptorListTemplate<ENTRIES, ENTRY_SIZE>
{
    fn drop(&mut self) {
        dbgln!(audio, "DROP Buff Desc List");
        if !self.list.is_empty() {
            deallocate_order(
                &Frame::new(self.buffer_address().to_phys()),
                self.buffer_order,
            );
        }
        deallocate_order(&Frame::new(self.list_address()), 0);
    }
}

pub struct OutputStream {
    stream: Stream,
    buffer: BufferDescriptorList,
}

impl<'a> OutputStream {
    pub fn new(stream: Stream) -> Option<OutputStream> {
        let buffer = BufferDescriptorList::new()?;

        stream.set_address(buffer.list_address());
        stream.set_cyclic_buffer_length(buffer.buffer_size() as u32);
        stream.set_last_valid_index(buffer.entries() as u16 - 1);
        stream.set_interrupt_on_completion(true);

        Some(OutputStream { stream, buffer })
    }

    pub fn run(&self) {
        self.stream.run();

        while !self.stream.control().is_run() {
            dbgln!(audio, "run awaiting");
        }
    }

    pub fn update(&self) {
        unsafe {
            self.buffer
                .buffer_address()
                .as_bytes_mut(self.buffer.buffer_size())
                .fill(1);
        }
    }

    pub fn stream(&self) -> &Stream {
        &self.stream
    }

    pub fn buffer(&self) -> &BufferDescriptorList {
        &self.buffer
    }
}
