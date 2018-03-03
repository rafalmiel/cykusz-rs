use arch::raw::gdt;
use arch::raw::segmentation as sgm;
use arch::raw::descriptor as dsc;

static INIT_GDT: [gdt::GdtEntry; 3] = [
    // Null
    gdt::GdtEntry::MISSING,
    // Kernel code
    gdt::GdtEntry::new(dsc::Flags::SEG_RING0_CODE, gdt::GdtFlags::LONG_MODE),
    // Kernel data
    gdt::GdtEntry::new(dsc::Flags::SEG_RING0_DATA, gdt::GdtFlags::MISSING),
];

static mut INIT_GDTR : dsc::DescriptorTablePointer<gdt::GdtEntry> =
    dsc::DescriptorTablePointer::<gdt::GdtEntry>::empty();

pub const fn kernel_code_segment() -> sgm::SegmentSelector {
    sgm::SegmentSelector::new(1, sgm::SegmentSelector::RPL_0)
}

pub const fn kernel_data_segment() -> sgm::SegmentSelector {
    sgm::SegmentSelector::new(2, sgm::SegmentSelector::RPL_0)
}

pub fn init() {
    unsafe {
        INIT_GDTR.init(&INIT_GDT[..]);
        dsc::lgdt(&INIT_GDTR);

        sgm::set_cs(  kernel_code_segment());
        sgm::load_ds(kernel_data_segment());
        sgm::load_es(kernel_data_segment());
        sgm::load_fs(kernel_data_segment());
        sgm::load_gs(kernel_data_segment());
        sgm::load_ss(kernel_data_segment());
    }

    println!("GDT initialised");
}
