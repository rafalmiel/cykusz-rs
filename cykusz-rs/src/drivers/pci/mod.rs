use crate::arch::idt::{add_shared_irq_handler, InterruptFn, SharedInterruptFn};
use crate::arch::int::{set_active_high, set_irq_dest, set_level_triggered};
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

    pub fn enable_msi_interrupt(&self, fun: InterruptFn) -> Option<usize> {
        let msi = self.try_hdr0()?.msi()?;

        let inum = crate::arch::idt::alloc_handler(fun)?;
        crate::arch::int::mask_int(inum as u8, false);

        msi.enable_interrupt(inum as u8, false, false, 0);
        msi.set_enabled(true);

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

#[allow(dead_code)]
impl PciHeader0 {
    pub fn base_address0(&self) -> u32 {
        self.data.read(0x10, 32) as u32
    }

    pub fn base_address1(&self) -> u32 {
        self.data.read(0x14, 32) as u32
    }

    pub fn base_address2(&self) -> u32 {
        self.data.read(0x18, 32) as u32
    }

    pub fn base_address3(&self) -> u32 {
        self.data.read(0x1C, 32) as u32
    }

    pub fn base_address4(&self) -> u32 {
        self.data.read(0x20, 32) as u32
    }

    pub fn base_address5(&self) -> u32 {
        self.data.read(0x24, 32) as u32
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

    pub fn msi(&self) -> Option<Msi<'_>> {
        self.capabilities_iter()
            .find(|cap| cap.id() == 0x5)
            .and_then(|cap| Some(Msi::<'_>::new(cap.data, cap.offset)))
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

    pub fn set_enabled(&self, enabled: bool) {
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

    pub fn enable_interrupt(&self, vector: u8, level: bool, active_low: bool, target_proc: u8) {
        let mut addr: u32 = 0;
        addr.set_bits(20..32, 0xFEE);
        addr.set_bits(12..20, target_proc as u32);

        let mut data: u32 = 0;
        data.set_bits(0..8, vector as u32);
        data.set_bit(14, active_low);
        data.set_bit(15, level);

        self.set_address(addr as u64);
        self.set_data(data as u16);
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
