mod device;
mod reg;

use crate::drivers::pci::{register_pci_device, PciDeviceHandle, PciHeader};
use alloc::sync::Arc;
use spin::Once;

use crate::arch::mm::phys::{allocate_order, deallocate_order};
use crate::arch::raw::mm::{PhysAddr, VirtAddr};
use crate::drivers::block::ahci::device::AhciDevice;
use crate::kernel::mm::Frame;
use crate::kernel::sync::Spin;
use crate::kernel::utils::wait_queue::WaitQueue;
use alloc::vec::Vec;
use bitflags::_core::sync::atomic::{AtomicBool, AtomicUsize};

struct Ahci {
    dev: Spin<AhciDevice>,
}

fn ahci_handler() -> bool {
    println!("AHCI int");
    device().dev.lock_irq().handle_interrupt()
}

impl Ahci {
    pub fn new() -> Ahci {
        Ahci {
            dev: Spin::new(AhciDevice::new()),
        }
    }
}

impl PciDeviceHandle for Ahci {
    fn handles(&self, pci_vendor_id: u64, pci_dev_id: u64) -> bool {
        match (pci_vendor_id, pci_dev_id) {
            (0x8086, 0x2922) => true,
            _ => false,
        }
    }

    fn start(&self, pci_data: &PciHeader) -> bool {
        device().dev.lock().start(pci_data)
    }
}

static DEVICE: Once<Arc<Ahci>> = Once::new();

fn device() -> &'static Arc<Ahci> {
    DEVICE.get().unwrap()
}

fn init() {
    DEVICE.call_once(|| Arc::new(Ahci::new()));

    register_pci_device(device().clone());
}

module_init!(init);

pub struct DmaBuf {
    buf: PhysAddr,
    order: usize,
}

pub struct ReadRequest {
    sector: usize,
    count: usize,
    buf_vec: Vec<DmaBuf>,
    incomplete: AtomicUsize,
    wq: WaitQueue,
}

fn make_request(sector: usize, count: usize) -> ReadRequest {
    let mut size = count * 512;

    let mut dma = Vec::<DmaBuf>::new();

    while size > 0 {
        let order = if size > 0x1000 { 1 } else { 0 };

        dma.push(DmaBuf {
            buf: allocate_order(order).unwrap().address(),
            order,
        });

        size -= core::cmp::min(size, 0x2000);
    }

    ReadRequest {
        sector,
        count,
        buf_vec: dma,
        incomplete: AtomicUsize::new(0),
        wq: WaitQueue::new(),
    }
}

pub fn read(sector: usize, count: usize, buf: VirtAddr) {
    let req = Arc::new(make_request(sector, count));

    // post request and wait for completion.....
    device().dev.lock_irq().read(req.clone());

    req.wq
        .wait_for(|| req.incomplete.load(core::sync::atomic::Ordering::SeqCst) == 0);
    println!("read Complete");

    let mut off = 0;
    let mut rem = count * 512;

    let slice = unsafe { core::slice::from_raw_parts_mut(buf.0 as *mut u8, rem) };

    for buf in req.buf_vec.iter() {
        let cnt = core::cmp::min(rem, 0x2000);

        slice[off..off + cnt].copy_from_slice(unsafe {
            core::slice::from_raw_parts(buf.buf.to_mapped().0 as *const u8, cnt)
        });

        rem -= cnt;
        off += cnt;

        deallocate_order(&Frame::new(buf.buf), buf.order);
    }
}
