use arch::raw::descriptor as dsc;
use arch::raw::gdt;
use arch::raw::segmentation as sgm;
use arch::raw::task::TaskStateSegment;
use kernel::mm::VirtAddr;

static mut INIT_GDT: [gdt::GdtEntry; 3] = [
    // Null
    gdt::GdtEntry::MISSING,
    // Kernel code
    gdt::GdtEntry::new(dsc::Flags::SEG_RING0_CODE, gdt::GdtFlags::LONG_MODE),
    // Kernel data
    gdt::GdtEntry::new(dsc::Flags::SEG_RING0_DATA, gdt::GdtFlags::LONG_MODE),
];

static mut INIT_GDTR : dsc::DescriptorTablePointer<gdt::GdtEntry> =  dsc::DescriptorTablePointer::<gdt::GdtEntry>::empty();

#[thread_local]
static mut GDT: [gdt::GdtEntry; 7] = [
    // Null
    gdt::GdtEntry::MISSING,
    // Kernel code
    gdt::GdtEntry::new(dsc::Flags::SEG_RING0_CODE, gdt::GdtFlags::LONG_MODE),
    // Kernel data
    gdt::GdtEntry::new(dsc::Flags::SEG_RING0_DATA, gdt::GdtFlags::LONG_MODE),
    // User code
    gdt::GdtEntry::new(dsc::Flags::SEG_RING3_CODE, gdt::GdtFlags::LONG_MODE),
    // User data
    gdt::GdtEntry::new(dsc::Flags::SEG_RING3_DATA, gdt::GdtFlags::LONG_MODE),
    // TSS
    gdt::GdtEntry::new(dsc::Flags::SEG_RING3_TASK, gdt::GdtFlags::MISSING),
    // TSS must be 16 bytes long, twice the normal size
    gdt::GdtEntry::MISSING,
];

#[thread_local]
static mut TSS: TaskStateSegment = TaskStateSegment::new();

#[thread_local]
static mut GDTR : dsc::DescriptorTablePointer<gdt::GdtEntry> =  dsc::DescriptorTablePointer::<gdt::GdtEntry>::empty();

pub const fn ring0_cs() -> sgm::SegmentSelector {
    sgm::SegmentSelector::new(1, sgm::SegmentSelector::RPL_0)
}

pub const fn ring0_ds() -> sgm::SegmentSelector {
    sgm::SegmentSelector::new(2, sgm::SegmentSelector::RPL_0)
}

pub const fn ring3_cs() -> sgm::SegmentSelector {
    sgm::SegmentSelector::new(3, sgm::SegmentSelector::RPL_3)
}

pub const fn ring3_ds() -> sgm::SegmentSelector {
    sgm::SegmentSelector::new(4, sgm::SegmentSelector::RPL_3)
}

pub fn early_init() {
    unsafe {
        INIT_GDTR.init(&INIT_GDT[..]);
        dsc::lgdt(&INIT_GDTR);

        sgm::set_cs(ring0_cs());
        sgm::load_ds(ring0_ds());
        sgm::load_es(ring0_ds());
        sgm::load_gs(ring0_ds());
        sgm::load_ss(ring0_ds());
    }
}

pub fn update_tss_rps0(new_rsp: usize) {
    unsafe {
        TSS.rsp[0] = new_rsp as u64;
    }
}

fn init_tss(stack_top: VirtAddr) {
    unsafe {
        TSS.rsp[0] = stack_top.0 as u64;

        {
            let gdt_low = &mut GDT[5];

            gdt_low.set_offset(&TSS as *const _ as u32);
            gdt_low.set_limit(::core::mem::size_of::<TaskStateSegment>() as u32);
        }

        {
            let gdt_high = &mut GDT[6];
            gdt_high.set_raw((&TSS as *const _ as u64) >> 32);
        }

        dsc::load_tr(&sgm::SegmentSelector::from_raw(5 << 3));
    }
}

//TLS available
pub fn init(stack_top: VirtAddr) {
    unsafe {
        GDTR.init(&GDT[..]);
        dsc::lgdt(&GDTR);

        sgm::set_cs(ring0_cs());
        sgm::load_ds(ring0_ds());
        sgm::load_es(ring0_ds());
        sgm::load_gs(ring0_ds());
        sgm::load_ss(ring0_ds());

        if stack_top != VirtAddr(0) {
            init_tss(stack_top);
        }
    }
}
