use crate::kernel::mm::VirtAddr;

pub struct ModuleFun(pub *const ());

unsafe impl Sync for ModuleFun {}

#[macro_export]
macro_rules! module_init {
    ($name: ident) => {
        #[used]
        #[link_section = ".devinit.drv"]
        static __PTR_INIT: crate::kernel::module::ModuleFun =
            crate::kernel::module::ModuleFun($name as *const ());
    };
}

#[macro_export]
macro_rules! module_fini {
    ($name: ident) => {
        #[used]
        #[link_section = ".devfini.drv"]
        static __PTR_FINI: crate::kernel::module::ModuleFun =
            crate::kernel::module::ModuleFun($name as *const ());
    };
}

#[macro_export]
macro_rules! platform_init {
    ($name: ident) => {
        #[used]
        #[link_section = ".devinit.plat"]
        static __PTR_INIT: crate::kernel::module::ModuleFun =
            crate::kernel::module::ModuleFun($name as *const ());
    };
}

#[macro_export]
macro_rules! platform_fini {
    ($name: ident) => {
        #[used]
        #[link_section = ".devfini.plat"]
        static __PTR_FINI: crate::kernel::module::ModuleFun =
            crate::kernel::module::ModuleFun($name as *const ());
    };
}

#[macro_export]
macro_rules! platform_2_init {
    ($name: ident) => {
        #[used]
        #[link_section = ".devinit.plat_2"]
        static __PTR_INIT: crate::kernel::module::ModuleFun =
            crate::kernel::module::ModuleFun($name as *const ());
    };
}

#[macro_export]
macro_rules! platform_2_fini {
    ($name: ident) => {
        #[used]
        #[link_section = ".devfini.plat_2"]
        static __PTR_FINI: crate::kernel::module::ModuleFun =
            crate::kernel::module::ModuleFun($name as *const ());
    };
}

unsafe fn run_range(start: VirtAddr, end: VirtAddr) {
    (start..end).step_by(8).for_each(|ptr| {
        let f = ptr.read::<fn()>();
        f();
    });
}

pub fn init_all() {
    extern "C" {
        static __kernel_devinit_start: usize;
        static __kernel_devinit_end: usize;
    }

    // Iterate over .devinit section containing pointers to module initialisation functions
    unsafe {
        run_range(
            VirtAddr(&__kernel_devinit_start as *const usize as usize),
            VirtAddr(&__kernel_devinit_end as *const usize as usize),
        );
    }
}

pub fn fini_all() {
    extern "C" {
        static __kernel_devfini_start: usize;
        static __kernel_devfini_end: usize;
    }

    // Iterate over .devfini section containing pointers to module finalisation functions
    unsafe {
        run_range(
            VirtAddr(&__kernel_devfini_start as *const usize as usize),
            VirtAddr(&__kernel_devfini_end as *const usize as usize),
        );
    }
}
