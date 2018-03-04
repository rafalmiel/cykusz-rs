use arch::raw::descriptor as dsc;
use arch::raw::gdt;
use arch::raw::segmentation as sgm;

static INIT_GDT: [gdt::GdtEntry; 3] = [
    // Null
    gdt::GdtEntry::MISSING,
    // Kernel code
    gdt::GdtEntry::new(dsc::Flags::SEG_RING0_CODE, gdt::GdtFlags::LONG_MODE),
    // Kernel data
    gdt::GdtEntry::new(dsc::Flags::SEG_RING0_DATA, gdt::GdtFlags::MISSING),
];

pub const fn ring0_cs() -> sgm::SegmentSelector {
    sgm::SegmentSelector::new(1, sgm::SegmentSelector::RPL_0)
}

pub const fn ring0_ds() -> sgm::SegmentSelector {
    sgm::SegmentSelector::new(2, sgm::SegmentSelector::RPL_0)
}

pub fn init() {
    let mut gdtr= dsc::DescriptorTablePointer::<gdt::GdtEntry>::empty();
    gdtr.init(&INIT_GDT[..]);
    unsafe {
        dsc::lgdt(&gdtr);

        sgm::set_cs(ring0_cs());
        sgm::load_ds(ring0_ds());
        sgm::load_es(ring0_ds());
        sgm::load_fs(ring0_ds());
        sgm::load_gs(ring0_ds());
        sgm::load_ss(ring0_ds());
    }

    println!("[ OK ] GDT Initialised");
}
