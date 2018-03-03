use arch::raw::idt;
use arch::raw::descriptor as dsc;
use arch::gdt;

static mut IDT_ENTRIES : [idt::IdtEntry; 256] = [idt::IdtEntry::MISSING; 256];
static mut IDTR : dsc::DescriptorTablePointer<idt::IdtEntry> =
    dsc::DescriptorTablePointer::<idt::IdtEntry>::empty();

macro_rules! int {
    ( $x:expr) => {
        {
            asm!("int $0" :: "N"($x));
        }
    };
}

pub fn init() {
    unsafe {
        IDTR.init(&IDT_ENTRIES[..]);
        dsc::lidt(&IDTR);

        IDT_ENTRIES[80].set_handler(int80_handler, gdt::kernel_code_segment(), dsc::Flags::SYS_RING0_INTERRUPT_GATE);
        asm!("xchg %bx, %bx");
        int!(80);

    }

    println!("IDT Initialised");
}

extern "x86-interrupt" fn int80_handler(frame: &mut idt::ExceptionStackFrame) {
    println!("INT 80!!!");
    println!("{:?}", frame);
}
