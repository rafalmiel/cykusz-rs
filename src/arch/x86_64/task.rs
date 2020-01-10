use core::ptr::Unique;

use crate::arch::gdt;
use crate::arch::mm::virt::p4_table_addr;
use crate::arch::raw::segmentation::SegmentSelector;
use crate::arch::x86_64::mm::PAGE_SIZE;
use crate::arch::x86_64::mm::virt::table::P4Table;
use crate::kernel::mm::heap::allocate_align as heap_allocate_align;
use crate::kernel::mm::heap::deallocate_align as heap_deallocate_align;
use crate::kernel::mm::MappedAddr;
use crate::kernel::mm::PhysAddr;
use crate::kernel::mm::VirtAddr;

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
    pub stack_top: usize,
    pub stack_size: usize,
    pub is_user: bool,
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

fn prepare_p4<'a>() -> &'a mut P4Table {
    use crate::arch::mm::phys::allocate;
    use crate::arch::mm::virt::current_p4_table;

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

fn map_user<'a>(new_p4: &'a mut P4Table, elf_module: MappedAddr) -> (PhysAddr, VirtAddr, VirtAddr) {
    use crate::arch::mm::phys::allocate;
    use crate::kernel::mm::virt;
    use crate::drivers::elf::types::ProgramType;
    use crate::drivers::elf::ElfHeader;
    use core::cmp::min;

    let hdr = unsafe { ElfHeader::load(elf_module) };

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
                    code_addr.copy_to(code_page.address_mapped().0 + page_offset,
                                      min(to_copy,virt_end.0 - virt_addr)
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
    new_p4.map_flags(
        VirtAddr(0x7ffffffff000),
        virt::PageFlags::USER | virt::PageFlags::WRITABLE | virt::PageFlags::NO_EXECUTE,
    );

    return (
        new_p4.phys_addr(),             // page table root address
        VirtAddr(hdr.e_entry as usize), // entry point to the program
        VirtAddr(0x7fffffffffff),       // stack pointer (4KB for now)
    );
}

fn allocate_page_table(elf_module: MappedAddr, _code_size: usize) -> (PhysAddr, VirtAddr, VirtAddr) {

    let new_p4 = prepare_p4();

    map_user(new_p4, elf_module)
}

#[repr(C, packed)]
struct IretqFrame {
    pub ip: usize,
    pub cs: usize,
    pub rlfags: usize,
    pub sp: usize,
    pub ss: usize,
    pub task_finished_fun: usize,
}

impl Task {
    pub const fn empty() -> Task {
        Task {
            ctx: Unique::empty(),
            stack_top: 0,
            stack_size: 0,
            is_user: false,
        }
    }

    pub fn assure_empty(&self) {
        if self.stack_top != 0 {
            panic!("[ ERROR ] ArchTask corrupted on init");
        }
        if self.stack_size != 0 {
            panic!("[ ERROR ] ArchTask corrupted on init");
        }
        if self.ctx.as_ptr() != Unique::empty().as_ptr() {
            panic!("[ ERROR ] ArchTask corrupted on init");
        }
    }

    fn new_sp(
        fun: fn(),
        cs: SegmentSelector,
        ds: SegmentSelector,
        int_enabled: bool,
        stack: usize,
        stack_size: usize,
        cr3: PhysAddr,
        user_stack: Option<usize>,
    ) -> Task {
        unsafe {
            let sp = (stack as *mut u8).offset(stack_size as isize);
            let frame: &mut IretqFrame = &mut *(sp
                .offset(-(::core::mem::size_of::<IretqFrame>() as isize))
                as *mut IretqFrame);

            frame.task_finished_fun = task_finished as usize;
            frame.ss = ds.bits() as usize;
            frame.sp = user_stack.unwrap_or(sp.offset(-8) as usize);
            frame.rlfags = if int_enabled { 0x200 } else { 0x0 };
            frame.cs = cs.bits() as usize;
            frame.ip = fun as usize;

            let mut ctx = Unique::new_unchecked(sp.offset(
                -(::core::mem::size_of::<Context>() as isize
                    + ::core::mem::size_of::<IretqFrame>() as isize),
            ) as *mut Context);
            ctx.as_ptr().write(Context::empty());
            ctx.as_mut().rip = isr_return as usize;
            ctx.as_mut().cr3 = cr3.0;

            Task {
                ctx,
                stack_top: sp as usize - stack_size,
                stack_size,
                is_user: user_stack.is_some(),
            }
        }
    }

    fn new(
        fun: fn(),
        cs: SegmentSelector,
        ds: SegmentSelector,
        int_enabled: bool,
        cr3: PhysAddr,
        user_stack: Option<usize>,
    ) -> Task {
        let sp = heap_allocate_align(4096 * 16, 4096).unwrap();

        Task::new_sp(
            fun,
            cs,
            ds,
            int_enabled,
            sp as usize,
            4096 * 16,
            cr3,
            user_stack,
        )
    }

    pub fn new_kern(fun: fn()) -> Task {
        Task::new(
            fun,
            gdt::ring0_cs(),
            gdt::ring0_ds(),
            true,
            p4_table_addr(),
            None,
        )
    }

    pub fn new_sched(fun: fn()) -> Task {
        Task::new(
            fun,
            gdt::ring0_cs(),
            gdt::ring0_ds(),
            false,
            p4_table_addr(),
            None,
        )
    }

    pub fn new_user(elf: MappedAddr, code_size: usize) -> Task {
        let (new_p4, entry, stack) = allocate_page_table(elf, code_size);

        let f = unsafe { ::core::mem::transmute::<usize, fn()>(entry.0) };

        Task::new(
            f,
            gdt::ring3_cs(),
            gdt::ring3_ds(),
            true,
            new_p4,
            Some(stack.0),
        )
    }

    pub fn deallocate(&mut self) {
        self.ctx = Unique::empty();
        heap_deallocate_align(self.stack_top as *mut u8, self.stack_size, 4096);
        self.stack_top = 0;
    }
}

extern "C" {
    pub fn switch_to(old_ctx: &mut Unique<Context>, new_ctx: &Context);
    pub fn activate_to(new_ctx: &Context);
    fn isr_return();
}

pub fn switch(from: &mut Task, to: &Task) {
    unsafe {
        if to.is_user {
            crate::arch::gdt::update_tss_rps0(to.stack_top + to.stack_size);
        }
        switch_to(&mut from.ctx, to.ctx.as_ref());
    }
}

pub fn activate_task(to: &Task) {
    unsafe {
        if to.is_user {
            crate::arch::gdt::update_tss_rps0(to.stack_top + to.stack_size);
        }
        activate_to(to.ctx.as_ref());
    }
}
