use crate::arch::idt::{add_shared_irq_handler, InterruptFn, SharedInterruptFn};
use crate::arch::int::{set_active_high, set_irq_dest, set_level_triggered};
use crate::arch::mm::{PhysAddr, VirtAddr};
use crate::kernel::mm::map_to_flags;
use crate::kernel::mm::virt::PageFlags;
use alloc::sync::Arc;
use alloc::vec::Vec;
use bit_field::BitField;

use crate::kernel::sync::Spin;

mod epci;
mod pci;

pub trait PciAccess: Sync {
    fn read(&self, seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, width: u32) -> u64;
    fn write(&self, seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, val: u64, width: u32);
}

pub trait PciDeviceHandle: Sync + Send {
    fn handles(&self, pci_vendor_id: u64, pci_dev_id: u64) -> bool;
    fn start(&self, pci_data: &PciHeader) -> bool;
}

#[derive(Copy, Clone)]
pub struct PciHeader0 {
    data: PciData,
}

#[derive(Copy, Clone)]
pub struct PciHeader1 {
    data: PciData,
}

#[derive(Copy, Clone)]
pub struct PciHeader2 {
    data: PciData,
}

#[derive(Copy, Clone)]
pub enum PciHeader {
    Unknown,
    Type0(PciHeader0),
    Type1(PciHeader1),
    Type2(PciHeader2),
}

#[derive(Copy, Clone)]
pub struct PciData {
    pub seg: u16,
    pub bus: u16,
    pub dev: u16,
    pub fun: u16,
}

#[allow(dead_code)]
impl PciHeader {
    pub fn new() -> PciHeader {
        PciHeader::Unknown
    }

    pub fn init(&mut self, seg: u16, bus: u16, dev: u16, fun: u16) -> bool {
        let data = PciData { seg, bus, dev, fun };

        if !data.is_valid() {
            return false;
        }

        match data.header_type() & 0b01111111 {
            0x0 => *self = PciHeader::Type0(PciHeader0 { data }),
            0x1 => *self = PciHeader::Type1(PciHeader1 { data }),
            0x2 => *self = PciHeader::Type2(PciHeader2 { data }),
            _ => {
                panic!("Invalid PCI Header");
            }
        }

        return true;
    }

    pub fn debug(&self) {
        self.hdr().debug();
    }

    pub fn hdr(&self) -> &PciData {
        match self {
            PciHeader::Type0(hdr) => {
                return &hdr.data;
            }
            PciHeader::Type1(hdr) => {
                return &hdr.data;
            }
            PciHeader::Type2(hdr) => {
                return &hdr.data;
            }
            _ => {
                panic!("Header not initialized");
            }
        }
    }

    fn try_hdr0(&self) -> Option<&PciHeader0> {
        if let PciHeader::Type0(hdr) = self {
            Some(hdr)
        } else {
            None
        }
    }

    pub fn msi(&self) -> Option<Msi<'_>> {
        self.try_hdr0()?
            .capabilities_iter()
            .find(|cap| cap.id() == 0x5)
            .and_then(|cap| Some(Msi::<'_>::new(cap.data, cap.offset)))
    }

    pub fn msix(&self) -> Option<Msix<'_>> {
        self.try_hdr0()?
            .capabilities_iter()
            .find(|cap| cap.id() == 0x11)
            .and_then(|cap| Some(Msix::<'_>::new(self, cap.offset)))
    }

    pub fn enable_msi_interrupt(&self, fun: InterruptFn) -> Option<usize> {
        let msi = self.msi()?;

        let inum = crate::arch::idt::alloc_handler(fun)?;
        crate::arch::int::mask_int(inum as u8, false);

        msi.enable_interrupt(inum as u8, false, false, 0);
        msi.enable(true);

        Some(inum)
    }

    pub fn enable_pci_interrupt(&self, fun: SharedInterruptFn) -> Option<usize> {
        let data = self.hdr();

        let pin = data.interrupt_pin();

        let int =
            crate::drivers::acpi::get_irq_mapping(data.bus as u32, data.dev as u32, pin as u32 - 1);

        let p = int?;

        set_irq_dest(p as u8, p as u8 + 32);
        set_active_high(p as u8, false);
        set_level_triggered(p as u8, true);
        add_shared_irq_handler(p as usize + 32, fun);

        Some(p as usize)
    }
}

bitflags! {
    pub struct ProgInterface: u8 {
        const PRIMARY_PCI_NATIVE = 0b0000_0001;
        const PRIMARY_CAN_SWITCH = 0b0000_0010;
        const SECONDARY_PCI_NATIVE = 0b0000_0100;
        const SECONDARY_CAN_SWITCH = 0b0000_1000;
        const DMA_CAPABLE = 0b1000_0000;
    }
}

#[allow(dead_code)]
impl PciData {
    pub fn debug(&self) {
        println!(
            "[ PCI ] ({}, {}, {}) V: 0x{:x} D: 0x{:x} C: 0x{:x} SC: 0x{:x} p: {}, l: {} h: 0x{:x}",
            self.bus,
            self.dev,
            self.fun,
            self.vendor_id(),
            self.device_id(),
            self.class(),
            self.subclass(),
            self.interrupt_pin(),
            self.interrupt_line(),
            self.header_type()
        );
    }

    pub fn is_valid(&self) -> bool {
        self.header_type() != 0xff
    }

    fn read(&self, offset: u32, width: u32) -> u64 {
        read(self.seg, self.bus, self.dev, self.fun, offset, width)
    }

    fn write(&self, offset: u32, val: u64, width: u32) {
        write(
            self.seg,
            self.bus,
            self.dev,
            self.fun,
            offset,
            val,
            width as u32,
        )
    }

    pub fn vendor_id(&self) -> u16 {
        self.read(0x00, 16) as u16
    }

    pub fn device_id(&self) -> u16 {
        self.read(0x02, 16) as u16
    }

    pub fn command(&self) -> u16 {
        self.read(0x04, 16) as u16
    }

    pub fn write_command(&self, val: u16) {
        self.write(0x04, val as u64, 16)
    }

    pub fn enable_bus_mastering(&self) {
        self.write_command(0b111);
    }

    pub fn status(&self) -> u16 {
        self.read(0x06, 16) as u16
    }

    pub fn revision_id(&self) -> u8 {
        self.read(0x08, 8) as u8
    }

    pub fn prog_if(&self) -> ProgInterface {
        ProgInterface::from_bits_truncate(self.read(0x09, 8) as u8)
    }

    pub fn subclass(&self) -> u8 {
        self.read(0xA, 8) as u8
    }

    pub fn class(&self) -> u8 {
        self.read(0xB, 8) as u8
    }

    pub fn cacheline_size(&self) -> u8 {
        self.read(0xC, 8) as u8
    }

    pub fn latency_timer(&self) -> u8 {
        self.read(0xD, 8) as u8
    }

    pub fn header_type(&self) -> u8 {
        self.read(0xE, 8) as u8
    }

    pub fn bist(&self) -> u8 {
        self.read(0xF, 8) as u8
    }

    pub fn interrupt_pin(&self) -> u8 {
        self.read(0x3D, 8) as u8
    }

    pub fn interrupt_line(&self) -> u8 {
        self.read(0x3C, 8) as u8
    }

    pub fn write_interrupt_line(&self, val: u8) {
        self.write(0x3C, val as u64, 8)
    }
}

#[repr(transparent)]
pub struct BarAddress(u64);

impl BarAddress {
    pub fn new(pci_data: &PciData, offset: u32) -> BarAddress {
        let mut ba = BarAddress(pci_data.read(offset, 32));

        if ba.is64() {
            ba.0.set_bits(32..64, pci_data.read(offset + 4, 32));
        }

        ba
    }

    pub fn is64(&self) -> bool {
        self.0.get_bits(1..=2) == 0x2
    }

    pub fn is_io(&self) -> bool {
        self.0.get_bit(0)
    }

    pub fn is_prefetchable(&self) -> bool {
        assert!(!self.is_io());
        self.0.get_bit(3)
    }

    pub fn address(&self) -> PhysAddr {
        assert!(!self.is_io());
        let mut a = self.0;
        a.set_bits(0..4, 0);
        PhysAddr(a as usize)
    }

    pub fn address_map_virt(&self) -> VirtAddr {
        self.address_map_virt_num(1)
    }

    pub fn address_map_virt_num(&self, num_pages: usize) -> VirtAddr {
        let addr = self.address();

        let mut flags = PageFlags::WRITABLE;
        if self.is_prefetchable() {
            flags.insert(PageFlags::WRT_THROUGH);
        } else {
            flags.insert(PageFlags::NO_CACHE);
        }

        for p in 0..num_pages {
            let offset = 0x1000 * p;
            map_to_flags(addr.to_virt() + offset, addr + offset, flags);
        }

        addr.to_virt()
    }

    pub fn io_address(&self) -> u16 {
        assert!(self.is_io());
        let mut a = self.0;
        a.set_bits(0..2, 0);
        a.try_into().unwrap()
    }
}

#[allow(dead_code)]
impl PciHeader0 {
    pub fn base_address(&self, num: usize) -> BarAddress {
        assert!(num <= 5);

        BarAddress::new(&self.data, 0x10u32 + 4 * num as u32)
    }

    pub fn base_address0(&self) -> BarAddress {
        self.base_address(0)
    }

    pub fn base_address1(&self) -> BarAddress {
        self.base_address(1)
    }

    pub fn base_address2(&self) -> BarAddress {
        self.base_address(2)
    }

    pub fn base_address3(&self) -> BarAddress {
        self.base_address(3)
    }

    pub fn base_address4(&self) -> BarAddress {
        self.base_address(4)
    }

    pub fn base_address5(&self) -> BarAddress {
        self.base_address(5)
    }

    pub fn cardbus_cis_pointer(&self) -> u32 {
        self.data.read(0x28, 32) as u32
    }

    pub fn subsystem_vendor_id(&self) -> u16 {
        self.data.read(0x2C, 16) as u16
    }

    pub fn subsystem_id(&self) -> u16 {
        self.data.read(0x2E, 16) as u16
    }

    pub fn expansion_rom_base(&self) -> u32 {
        self.data.read(0x30, 32) as u32
    }

    pub fn capabilities_ptr(&self) -> u8 {
        self.data.read(0x34, 8) as u8
    }

    pub fn capabilities_iter(&self) -> CapabilityIter {
        CapabilityIter::new(&self.data, self.capabilities_ptr())
    }

    pub fn min_grant(&self) -> u8 {
        self.data.read(0x3E, 8) as u8
    }

    pub fn max_latency(&self) -> u8 {
        self.data.read(0x3F, 8) as u8
    }
}

pub struct CapabilityId<'a> {
    data: &'a PciData,
    offset: u32,
}

impl<'a> CapabilityId<'a> {
    pub fn id(&self) -> u8 {
        self.data.read(self.offset, 8) as u8
    }

    pub fn next(&self) -> u8 {
        self.data.read(self.offset + 1, 8) as u8
    }
}

pub struct CapabilityIter<'a> {
    data: &'a PciData,
    cur_ptr: u8,
}

impl<'a> CapabilityIter<'a> {
    pub fn new(data: &'a PciData, init_ptr: u8) -> CapabilityIter<'a> {
        CapabilityIter::<'a> {
            data,
            cur_ptr: init_ptr,
        }
    }
}

impl<'a> Iterator for CapabilityIter<'a> {
    type Item = CapabilityId<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur_ptr != 0 {
            let ptr = self.cur_ptr;

            let cap_id = CapabilityId::<'a> {
                data: self.data,
                offset: ptr as u32,
            };

            self.cur_ptr = cap_id.next();

            return Some(cap_id);
        }

        None
    }
}

pub struct Msi<'a> {
    data: &'a PciData,
    offset: u32,
    is64: bool,
}

#[allow(unused)]
impl<'a> Msi<'a> {
    pub fn new(data: &'a PciData, offset: u32) -> Msi<'a> {
        let mut msi = Msi::<'a> {
            data,
            offset,
            is64: false,
        };

        msi.is64 = msi.control().get_bit(7);

        msi
    }

    fn control(&self) -> u16 {
        self.data.read(self.offset + 2, 16) as u16
    }

    fn set_control(&self, val: u16) {
        self.data.write(self.offset + 2, val as u64, 16);
    }

    pub fn is64(&self) -> bool {
        self.is64
    }

    pub fn multi_msg_capable(&self) -> u8 {
        let v = self.control();
        v.get_bits(1..=3) as u8
    }

    pub fn is_enabled(&self) -> bool {
        self.control().get_bit(0)
    }

    pub fn enable(&self, enabled: bool) {
        let mut v = self.control();
        v.set_bit(0, enabled);

        self.set_control(v);
    }

    pub fn address(&self) -> u64 {
        let mut addr: u64 = self.data.read(self.offset + 4, 32) as u64;

        if self.is64() {
            addr.set_bits(32..64, self.data.read(self.offset + 8, 32) as u64);
        }

        addr
    }

    pub fn set_address(&self, addr: u64) {
        self.data
            .write(self.offset + 4, addr.get_bits(0..32) as u64, 32);
        if self.is64() {
            self.data
                .write(self.offset + 8, addr.get_bits(32..64) as u64, 32);
        }
    }

    fn data_offset(&self) -> u32 {
        self.offset + if self.is64() { 0xC } else { 0x8 }
    }

    pub fn data(&self) -> u64 {
        self.data.read(self.data_offset(), 16) as u64
    }

    pub fn set_data(&self, data: u16) {
        self.data.write(self.data_offset(), data as u64, 16);
    }

    pub fn enable_interrupt(&self, vector: u8, level: bool, active_low: bool, target_proc: u32) {
        let (addr, data) =
            crate::arch::int::msi::get_addr_data(vector, level, active_low, target_proc);

        self.set_address(addr.val());
        self.set_data(data.val());
    }
}

#[allow(unused)]
pub struct Msix<'a> {
    header: &'a PciHeader,
    data: &'a PciData,
    offset: u32,
}

#[allow(unused)]
impl<'a> Msix<'a> {
    pub fn new(header: &'a PciHeader, offset: u32) -> Msix<'a> {
        Msix::<'a> {
            header,
            data: header.hdr(),
            offset,
        }
    }

    fn control(&self) -> u16 {
        self.data.read(self.offset + 2, 16) as u16
    }

    fn set_control(&self, val: u16) {
        self.data.write(self.offset + 2, val as u64, 16);
    }

    pub fn table_size(&self) -> usize {
        let ctrl = self.control();
        ctrl.get_bits(0..=10) as usize + 1
    }

    pub fn bir(&self) -> usize {
        self.data.read(self.offset + 4, 32).get_bits(0..=2) as usize
    }

    pub fn table_offset(&self) -> usize {
        *self.data.read(self.offset + 4, 32).set_bits(0..=2, 0) as usize
    }

    pub fn enable(&self, e: bool) {
        let mut ctrl = self.control();
        ctrl.set_bit(15, e);

        self.set_control(ctrl);
    }

    fn table_bar(&self) -> BarAddress {
        self.header.try_hdr0().unwrap().base_address(self.bir())
    }

    pub fn table_address(&self) -> PhysAddr {
        let bar = self.table_bar();

        return bar.address() + self.table_offset();
    }

    pub fn map_table_address(&self) -> VirtAddr {
        let addr = self.table_bar().address_map_virt();

        return addr + self.table_offset();
    }

    pub fn map_table(&self) -> MsixTable {
        MsixTable::new(self.map_table_address())
    }

    pub fn table(&self) -> MsixTable {
        MsixTable::new(self.table_address().to_virt())
    }
}

pub struct MsixTable {
    addr: VirtAddr,
}

impl MsixTable {
    fn new(addr: VirtAddr) -> MsixTable {
        MsixTable { addr }
    }

    pub fn enable_interrupt(
        &self,
        num: usize,
        vector: u8,
        level: bool,
        active_low: bool,
        target_proc: u32,
    ) {
        let (addr, data) =
            crate::arch::int::msi::get_addr_data(vector, level, active_low, target_proc);

        let offset = self.addr + 16 * num;

        unsafe {
            offset.store_volatile(addr.val() as u64);
            (offset + 8).store_volatile(data.val() as u32);
            (offset + 12).store_volatile(0u32);
        }

    }

    pub fn alloc_interrupt(&self, num: usize, fun: InterruptFn) -> Option<usize> {
        let inum = crate::arch::idt::alloc_handler(fun)?;

        self.enable_interrupt(num, inum as u8, false, false, 0);

        crate::arch::int::mask_int(inum as u8, false);

        Some(inum)
    }
}

struct PciDevice {
    handle: Arc<dyn PciDeviceHandle>,
    #[allow(unused)]
    found: bool,
    data: PciHeader,
}

struct Pci {
    devices: Vec<PciDevice>,
}

impl Pci {
    const fn new() -> Pci {
        Pci {
            devices: Vec::new(),
        }
    }

    fn check_devices(&mut self, pci_data: &PciHeader) {
        let vendor_id = pci_data.hdr().vendor_id();
        let dev_id = pci_data.hdr().device_id();

        for dev in &mut self.devices {
            if dev.handle.handles(vendor_id as u64, dev_id as u64) {
                dev.found = true;

                dev.data = *pci_data;

                dev.handle.start(&dev.data);
            }
        }
    }

    fn check(&mut self, bus: u8, device: u8, function: u8) {
        let mut pci_data = PciHeader::new();
        let succeeded = pci_data.init(0, bus as u16, device as u16, function as u16);

        if succeeded {
            pci_data.debug();

            self.check_devices(&pci_data);
        }
    }

    fn read_u32(&self, bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
        DRIVER
            .lock()
            .unwrap()
            .read(0, bus as u16, slot as u16, func as u16, offset as u32, 32) as u32
    }

    pub fn init(&mut self) {
        for bus in 0..=255 {
            for device in 0..32 {
                self.check(bus, device, 0);
                let header = (self.read_u32(bus, device, 0, 0xc) >> 16) & 0xff;

                if header & 0x80 > 0 {
                    for f in 1..8 {
                        self.check(bus, device, f);
                    }
                }
            }
        }
    }
}

static DRIVER: Spin<Option<&'static dyn PciAccess>> = Spin::new(None);
static PCI: Spin<Pci> = Spin::new(Pci::new());

pub fn register_pci_driver(driver: &'static dyn PciAccess) {
    *DRIVER.lock() = Some(driver);
}

pub fn register_pci_device(device: Arc<dyn PciDeviceHandle>) {
    let mut driver = PCI.lock();

    driver.devices.push(PciDevice {
        handle: device,
        found: false,
        data: PciHeader::new(),
    });
}

pub fn init() {
    if !epci::init() {
        pci::init();
        println!("[ OK ] PCI Initialized");
    } else {
        println!("[ OK ] Express PCI Initialized");
    }
}

pub fn enumerate_pci() {
    let mut driver = PCI.lock();

    driver.init();
}

pub fn read(seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, width: u32) -> u64 {
    DRIVER.lock().unwrap().read(seg, bus, dev, fun, reg, width)
}

pub fn write(seg: u16, bus: u16, dev: u16, fun: u16, reg: u32, val: u64, width: u32) {
    DRIVER
        .lock()
        .unwrap()
        .write(seg, bus, dev, fun, reg, val, width);
}

platform_init!(init);
