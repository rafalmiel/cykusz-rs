use crate::arch::raw::descriptor as dsc;
use crate::arch::raw::gdt;
use crate::arch::raw::segmentation as sgm;
use crate::arch::raw::task::TaskStateSegment;
use crate::kernel::mm::VirtAddr;

static mut INIT_GDT: [gdt::GdtEntry; 3] = [
    // Null
    gdt::GdtEntry::MISSING,
    // Kernel code
    gdt::GdtEntry::new(dsc::Flags::SEG_RING0_CODE, gdt::GdtFlags::LONG_MODE),
    // Kernel data
    gdt::GdtEntry::new(dsc::Flags::SEG_RING0_DATA, gdt::GdtFlags::LONG_MODE),
];

static mut INIT_GDTR: dsc::DescriptorTablePointer<gdt::GdtEntry> =
    dsc::DescriptorTablePointer::<gdt::GdtEntry>::empty();

#[thread_local]
static mut GDT: [gdt::GdtEntry; 7] = [
    // Null
    gdt::GdtEntry::MISSING,
    // Kernel code
    gdt::GdtEntry::new(dsc::Flags::SEG_RING0_CODE, gdt::GdtFlags::LONG_MODE),
    // Kernel data
    gdt::GdtEntry::new(dsc::Flags::SEG_RING0_DATA, gdt::GdtFlags::LONG_MODE),
    // User data
    // Order of user segments (DS, CS) is required by syscall specific handling
    gdt::GdtEntry::new(dsc::Flags::SEG_RING3_DATA, gdt::GdtFlags::LONG_MODE),
    // User code
    gdt::GdtEntry::new(dsc::Flags::SEG_RING3_CODE, gdt::GdtFlags::LONG_MODE),
    // TSS
    gdt::GdtEntry::new(dsc::Flags::SEG_RING3_TASK, gdt::GdtFlags::MISSING),
    // TSS must be 16 bytes long, twice the normal size
    gdt::GdtEntry::MISSING,
];

#[thread_local]
pub static mut TSS: TaskStateSegment = TaskStateSegment::new();

#[thread_local]
static mut GDTR: dsc::DescriptorTablePointer<gdt::GdtEntry> =
    dsc::DescriptorTablePointer::<gdt::GdtEntry>::empty();

pub const fn ring0_cs() -> sgm::SegmentSelector {
    sgm::SegmentSelector::new(1, sgm::SegmentSelector::RPL_0)
}

pub const fn ring0_ds() -> sgm::SegmentSelector {
    sgm::SegmentSelector::new(2, sgm::SegmentSelector::RPL_0)
}

pub const fn ring3_cs() -> sgm::SegmentSelector {
    sgm::SegmentSelector::new(4, sgm::SegmentSelector::RPL_3)
}

pub const fn ring3_ds() -> sgm::SegmentSelector {
    sgm::SegmentSelector::new(3, sgm::SegmentSelector::RPL_3)
}

pub fn early_init() {
    unsafe {
        let init_gdtr = (&raw mut INIT_GDTR).as_mut_unchecked();
        init_gdtr.init(&INIT_GDT[..]);
        dsc::lgdt(init_gdtr);

        sgm::set_cs(ring0_cs());
        sgm::load_ds(ring0_ds());
        sgm::load_es(ring0_ds());
        sgm::load_gs(ring0_ds());
        sgm::load_ss(ring0_ds());
    }
}

pub fn update_tss_rps0(new_rsp: usize) {
    unsafe {
        let tss = (&raw mut TSS).as_mut_unchecked();
        tss.rsp[0] = new_rsp as u64;
    }
}

#[inline(never)]
fn init_tss(stack_top: VirtAddr, fs_base: u64) {
    unsafe {
        let tss = (&raw mut TSS).as_mut_unchecked();
        tss.rsp[0] = stack_top.0 as u64;
        tss.kern_fs_base = fs_base;

        let gdt = (&raw mut GDT).as_mut_unchecked();

        {
            let gdt_low = &mut gdt[5];
            //logln!("here {:p}", gdt_low as *mut _);

            gdt_low.set_offset((&raw const TSS) as u32);
            gdt_low.set_limit(::core::mem::size_of::<TaskStateSegment>() as u32);
        }

        {
            let gdt_high = &mut gdt[6];
            gdt_high.set_raw(((&raw const TSS) as u64) >> 32);
        }

        dsc::load_tr(&sgm::SegmentSelector::from_raw(5 << 3));

        use crate::arch::raw::msr;
        msr::wrmsr(msr::IA32_KERNEL_GS_BASE, (&raw const TSS) as u64);
    }
}

//TLS available
pub fn init(stack_top: VirtAddr, fs_base: u64) {
    unsafe {
        let gdtr = (&raw mut GDTR).as_mut_unchecked();
        gdtr.init(&GDT[..]);
        dsc::lgdt(gdtr);

        sgm::set_cs(ring0_cs());
        sgm::load_ds(ring0_ds());
        sgm::load_es(ring0_ds());
        sgm::load_gs(ring0_ds());
        sgm::load_ss(ring0_ds());

        if stack_top != VirtAddr(0) {
            init_tss(stack_top, fs_base);
        }
    }
}
