use arch::raw::msr;
use kernel::mm::VirtAddr;

#[repr(C)]
struct ThreadPtr {
    self_ptr: VirtAddr,
}

impl ThreadPtr {
    pub unsafe fn new_at(addr: VirtAddr) -> &'static mut ThreadPtr {
        addr.read_mut::<ThreadPtr>()
    }

    pub fn setup(&mut self) {
        self.self_ptr = VirtAddr(self as *mut _ as usize);

        unsafe {
            msr::wrmsr(msr::IA32_FS_BASE, self.self_ptr.0 as u64);
        }
    }
}

pub fn init(stack_top: VirtAddr) {
    extern {
        static __tdata_start: u8;
        static __tdata_end: u8;
    }

    unsafe {
        let size = &__tdata_end as *const u8 as usize - &__tdata_start as *const u8 as usize;
        let mapped = VirtAddr(&__tdata_start as *const _ as usize);

        let tls = ::kernel::mm::heap::allocate_align(
            size + 8, 8
        ).expect("Out of memory!");

        let ptr = mapped.0 as *const u8;

        ptr.copy_to(tls, size);

        let thread = ThreadPtr::new_at(VirtAddr(tls.offset(size as isize) as usize));

        thread.setup();
    }

    ::arch::gdt::init(stack_top);
}
