use core::mem::size_of;
use core::ptr::Unique;

use crate::arch::gdt;
use crate::arch::mm::virt::p4_table_addr;
use crate::arch::raw::segmentation::SegmentSelector;
use crate::arch::x86_64::mm::phys::{allocate_order, deallocate_order};
use crate::arch::x86_64::mm::virt::p4_table;
use crate::arch::x86_64::mm::virt::table::P4Table;
use crate::arch::x86_64::mm::PAGE_SIZE;
use crate::arch::x86_64::raw::mm::MappedAddr;
use crate::kernel::mm::VirtAddr;
use crate::kernel::mm::{Frame, PhysAddr};

const USER_STACK_SIZE: usize = 0x4000;
const KERN_STACK_SIZE: usize = 4096 * 4;
const KERN_STACK_ORDER: usize = 2;

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct Context {
    /// Page Table pointer
    pub cr3: usize,
    /// RFLAGS register
    pub rflags: usize,
    /// RBX register
    pub rbp: usize,
    /// R12 register
    pub r12: usize,
    /// R13 register
    pub r13: usize,
    /// R14 register
    pub r14: usize,
    /// R15 register
    pub r15: usize,
    /// Base pointer
    pub rbx: usize,
    /// Instruction pointer
    pub rip: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct Task {
    pub ctx: Unique<Context>,
    //top of the stack, used to deallocate
    pub cr3: usize,
    pub stack_top: usize,
    pub stack_size: usize,
    pub user_stack: Option<usize>,
}

impl Context {
    #[allow(unused)]
    const fn empty() -> Context {
        Context {
            cr3: 0,
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbx: 0,
            rbp: 0,
            rflags: 0,
            rip: 0,
        }
    }
}

fn task_finished() {
    crate::kernel::sched::task_finished();
}

#[no_mangle]
pub extern "C" fn fork_get_pid() -> isize {
    crate::kernel::sched::current_id() as isize
}

fn prepare_p4<'a>() -> &'a mut P4Table {
    use crate::arch::mm::virt::current_p4_table;
    use crate::kernel::mm::allocate;

    let current_p4 = current_p4_table();
    let frame = allocate().expect("Out of mem!");
    let new_p4 = P4Table::new_mut(&frame);

    new_p4.clear();

    // Map kernel code to the user space, don't care about intel exploits for now
    for i in 256..512 {
        new_p4.set_entry(i, current_p4.entry_at(i));
    }

    new_p4
}

fn map_user(new_p4: &mut P4Table, exe: &[u8]) -> (PhysAddr, VirtAddr, VirtAddr) {
    use crate::drivers::elf::types::ProgramType;
    use crate::drivers::elf::ElfHeader;
    use crate::kernel::mm::allocate;
    use crate::kernel::mm::virt;
    use core::cmp::min;

    let elf_module = VirtAddr(exe.as_ptr() as usize);

    let hdr = unsafe { ElfHeader::load(exe) };

    for p in hdr.programs() {
        if p.p_type == ProgramType::Load {
            let flags = virt::PageFlags::USER | virt::PageFlags::from(p.p_flags);

            let virt_begin = VirtAddr(p.p_vaddr as usize).align_down(PAGE_SIZE);
            let virt_end = VirtAddr(p.p_vaddr as usize + p.p_memsz as usize);

            let mut page_offset = p.p_vaddr as usize - virt_begin.0;
            let mut code_addr = elf_module + p.p_offset as usize;

            //This should be done in a page fault in future
            for VirtAddr(virt_addr) in (virt_begin..virt_end).step_by(PAGE_SIZE) {
                let code_page = allocate().expect("Out of mem!");

                let to_copy = PAGE_SIZE - page_offset;

                unsafe {
                    code_addr.copy_to(
                        code_page.address_mapped().0 + page_offset,
                        min(to_copy, virt_end.0 - virt_addr),
                    );
                }

                code_addr += to_copy;
                page_offset = 0;

                // Map user program to the location indicated by Elf Program Section
                new_p4.map_to_flags(VirtAddr(virt_addr as usize), code_page.address(), flags);
            }
        }
    }

    // Map stack
    for a in
        (VirtAddr(0x8000_0000_0000 - USER_STACK_SIZE)..VirtAddr(0x800000000000)).step_by(PAGE_SIZE)
    {
        new_p4.map_flags(
            a,
            virt::PageFlags::USER | virt::PageFlags::WRITABLE | virt::PageFlags::NO_EXECUTE,
        );
    }

    return (
        new_p4.phys_addr(),             // page table root address
        VirtAddr(hdr.e_entry as usize), // entry point to the program
        VirtAddr(0x800000000000),       // stack pointer (4KB for now)
    );
}

fn allocate_page_table(exe: &[u8]) -> (PhysAddr, VirtAddr, VirtAddr) {
    let new_p4 = prepare_p4();

    map_user(new_p4, exe)
}

#[repr(C, packed)]
struct IretqFrame {
    pub rdi: usize,
    pub ip: usize,
    pub cs: usize,
    pub rlfags: usize,
    pub sp: usize,
    pub ss: usize,
    pub task_finished_fun: usize,
}

#[repr(C, packed)]
pub struct SysretqFrame {
    pub rflags: usize,
    pub rip: usize,
    pub stack: usize,
}

impl Default for Task {
    fn default() -> Task {
        Task::empty()
    }
}

impl Task {
    pub const fn empty() -> Task {
        Task {
            ctx: Unique::dangling(),
            cr3: 0,
            stack_top: 0,
            stack_size: 0,
            user_stack: None,
        }
    }

    pub fn is_user(&self) -> bool {
        self.user_stack.is_some()
    }

    pub fn assure_empty(&self) {
        if self.stack_top != 0 {
            panic!("[ ERROR ] ArchTask corrupted on init");
        }
        if self.stack_size != 0 {
            panic!("[ ERROR ] ArchTask corrupted on init");
        }
        if self.ctx.as_ptr() != Unique::dangling().as_ptr() {
            panic!("[ ERROR ] ArchTask corrupted on init");
        }
    }

    unsafe fn prepare_iretq_ctx(
        fun: usize,
        cs: SegmentSelector,
        ds: SegmentSelector,
        int_enabled: bool,
        sp: *mut u8,
        cr3: PhysAddr,
        param: usize,
    ) -> Unique<Context> {
        let frame: &mut IretqFrame =
            &mut *(sp.offset(-(::core::mem::size_of::<IretqFrame>() as isize)) as *mut IretqFrame);

        frame.task_finished_fun = task_finished as usize;
        frame.ss = ds.bits() as usize;
        frame.sp = sp.offset(-8) as usize;
        frame.rlfags = if int_enabled { 0x200 } else { 0x0 };
        frame.cs = cs.bits() as usize;
        frame.ip = fun as usize;
        frame.rdi = param;

        let mut ctx = Unique::new_unchecked(sp.offset(
            -(::core::mem::size_of::<Context>() as isize
                + ::core::mem::size_of::<IretqFrame>() as isize),
        ) as *mut Context);

        ctx.as_ptr().write(Context::empty());
        ctx.as_mut().rip = isr_return as usize;
        ctx.as_mut().cr3 = cr3.0;

        ctx
    }

    unsafe fn prepare_sysretq_ctx(
        fun: usize,
        int_enabled: bool,
        user_stack: Option<usize>,
        sp: *mut u8,
        cr3: PhysAddr,
    ) -> Unique<Context> {
        let frame: &mut SysretqFrame = &mut *(sp
            .offset(-(::core::mem::size_of::<SysretqFrame>() as isize))
            as *mut SysretqFrame);

        frame.stack = user_stack.unwrap() as usize;
        frame.rflags = if int_enabled { 0x200 } else { 0x0 };
        frame.rip = fun as usize;

        let mut ctx = Unique::new_unchecked(sp.offset(
            -(::core::mem::size_of::<Context>() as isize
                + ::core::mem::size_of::<SysretqFrame>() as isize),
        ) as *mut Context);

        ctx.as_ptr().write(Context::empty());
        ctx.as_mut().rip = asm_sysretq_userinit as usize;
        ctx.as_mut().cr3 = cr3.0;

        ctx
    }

    unsafe fn fork_ctx(&self, sp: *mut u8, cr3: usize) -> Unique<Context> {
        let parent_sys_frame =
            VirtAddr(self.stack_top + self.stack_size - size_of::<SysretqFrame>())
                .read_ref::<SysretqFrame>();

        let frame: &mut SysretqFrame =
            &mut *(sp.offset(-(size_of::<SysretqFrame>() as isize)) as *mut SysretqFrame);

        frame.stack = parent_sys_frame.stack;
        frame.rflags = parent_sys_frame.rflags;
        frame.rip = parent_sys_frame.rip;

        let mut ctx = Unique::new_unchecked(
            sp.offset(-(size_of::<Context>() as isize + size_of::<SysretqFrame>() as isize))
                as *mut Context,
        );

        ctx.as_ptr().write(Context::empty());
        ctx.as_mut().rip = asm_sysretq_forkinit as usize;
        ctx.as_mut().cr3 = cr3;

        ctx
    }

    fn new_sp(
        fun: usize,
        cs: SegmentSelector,
        ds: SegmentSelector,
        int_enabled: bool,
        stack: usize,
        stack_size: usize,
        cr3: PhysAddr,
        user_stack: Option<usize>,
        param: usize,
    ) -> Task {
        unsafe {
            let sp = (stack as *mut u8).offset(stack_size as isize);

            let ctx = if user_stack.is_none() {
                Task::prepare_iretq_ctx(fun, cs, ds, int_enabled, sp, cr3, param)
            } else {
                // Userspace transition is done using sysretq call
                Task::prepare_sysretq_ctx(fun, int_enabled, user_stack, sp, cr3)
            };

            Task {
                ctx,
                cr3: cr3.0,
                stack_top: sp as usize - stack_size,
                stack_size,
                user_stack,
            }
        }
    }

    fn new(
        fun: usize,
        cs: SegmentSelector,
        ds: SegmentSelector,
        int_enabled: bool,
        cr3: PhysAddr,
        user_stack: Option<usize>,
        param: usize,
    ) -> Task {
        let sp = allocate_order(KERN_STACK_ORDER).unwrap().address_mapped().0 as *mut u8;

        Task::new_sp(
            fun,
            cs,
            ds,
            int_enabled,
            sp as usize,
            KERN_STACK_SIZE,
            cr3,
            user_stack,
            param,
        )
    }

    pub fn new_kern(fun: fn()) -> Task {
        Task::new(
            fun as usize,
            gdt::ring0_cs(),
            gdt::ring0_ds(),
            true,
            p4_table_addr(),
            None,
            0,
        )
    }

    pub fn new_param_kern(fun: usize, val: usize) -> Task {
        Task::new(
            fun,
            gdt::ring0_cs(),
            gdt::ring0_ds(),
            true,
            p4_table_addr(),
            None,
            val,
        )
    }

    pub fn new_sched(fun: fn()) -> Task {
        Task::new(
            fun as usize,
            gdt::ring0_cs(),
            gdt::ring0_ds(),
            false,
            p4_table_addr(),
            None,
            0,
        )
    }

    pub fn new_user(exe: &[u8]) -> Task {
        let (new_p4, entry, stack) = allocate_page_table(exe);

        let f = unsafe { ::core::mem::transmute::<usize, fn()>(entry.0) };

        let t = Task::new(
            f as usize,
            gdt::ring3_cs(),
            gdt::ring3_ds(),
            true,
            new_p4,
            Some(stack.0),
            0,
        );

        t
    }

    pub fn fork(&self) -> Task {
        let orig_p4 = P4Table::new_at_phys(PhysAddr(self.cr3));

        let new_p4 = orig_p4.duplicate();

        let sp_top = allocate_order(KERN_STACK_ORDER).unwrap().address_mapped().0 as *mut u8;

        let sp = unsafe { sp_top.offset(KERN_STACK_SIZE as isize) };

        let new_ctx = unsafe { self.fork_ctx(sp, new_p4.phys_addr().0) };

        Task {
            ctx: new_ctx,
            cr3: new_p4.phys_addr().0,
            stack_top: sp_top as usize,
            stack_size: KERN_STACK_SIZE,
            user_stack: self.user_stack,
        }
    }

    pub fn deallocate(&mut self) {
        let cr3 = unsafe { self.ctx.as_ref().cr3 };

        if self.is_user() {
            let p4 = p4_table(PhysAddr(cr3));
            p4.deallocate_user();
        }

        self.ctx = Unique::dangling();
        deallocate_order(
            &Frame::new(MappedAddr(self.stack_top).to_phys()),
            KERN_STACK_ORDER,
        );
        self.stack_top = 0;
    }
}

extern "C" {
    pub fn switch_to(old_ctx: &mut Unique<Context>, new_ctx: &Context);
    pub fn activate_to(new_ctx: &Context);
    fn isr_return();
    fn asm_sysretq_userinit();
    fn asm_sysretq_forkinit();
}

pub fn activate_task(to: &Task) {
    unsafe {
        if to.is_user() {
            crate::arch::gdt::update_tss_rps0(to.stack_top + to.stack_size);
        }
        activate_to(to.ctx.as_ref());
    }
}

pub fn switch(from: &mut Task, to: &Task) {
    unsafe {
        if to.is_user() {
            crate::arch::gdt::update_tss_rps0(to.stack_top + to.stack_size);
        }
        switch_to(&mut from.ctx, to.ctx.as_ref());
    }
}
