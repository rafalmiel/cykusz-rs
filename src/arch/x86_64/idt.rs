use arch::raw::idt;
use arch::raw::descriptor as dsc;

static mut IDT_ENTRIES : [idt::IdtEntry; 256] = [idt::IdtEntry::MISSING; 256];
static mut IDTR : dsc::DescriptorTablePointer<idt::IdtEntry> = dsc::DescriptorTablePointer::<idt::IdtEntry>::empty();

pub fn init() {
    unsafe {
        IDTR.init(&IDT_ENTRIES[..]);
        dsc::lidt(&IDTR);
    }

    println!("IDT Initialised");
}
