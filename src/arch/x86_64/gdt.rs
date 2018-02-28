use arch::baremtl::gdt;

static INIT_GDT: [gdt::GdtEntry; 3] = [
    // Null
    gdt::GdtEntry::null(),
    // Kernel code
    gdt::GdtEntry::new(gdt::GdtAccessFlags::RING0_CODE, gdt::GdtFlags::LONG_MODE),
    // Kernel data
    gdt::GdtEntry::new(gdt::GdtAccessFlags::RING0_DATA, gdt::GdtFlags::BLANK),
];

static mut INIT_GDTR : gdt::DescriptorTablePointer = gdt::DescriptorTablePointer::empty();

pub fn init() {
    unsafe {
        asm!("xchg %bx, %bx");
        INIT_GDTR.init(&INIT_GDT[..]);
        gdt::lgdt(&INIT_GDTR);

        gdt::set_cs(gdt::SegmentSelector::new(1 as u16, gdt::PrivilegeLevel::Ring0));
        gdt::load_ds(gdt::SegmentSelector::new(2 as u16, gdt::PrivilegeLevel::Ring0));
        gdt::load_es(gdt::SegmentSelector::new(2 as u16, gdt::PrivilegeLevel::Ring0));
        gdt::load_fs(gdt::SegmentSelector::new(2 as u16, gdt::PrivilegeLevel::Ring0));
        gdt::load_gs(gdt::SegmentSelector::new(2 as u16, gdt::PrivilegeLevel::Ring0));
        gdt::load_ss(gdt::SegmentSelector::new(2 as u16, gdt::PrivilegeLevel::Ring0));
    }

    println!("GDT initialised");
}
