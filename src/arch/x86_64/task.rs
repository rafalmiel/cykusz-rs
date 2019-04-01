use core::ptr::Unique;

use crate::arch::gdt;
use crate::arch::mm::virt::p4_table_addr;
use crate::arch::raw::segmentation::SegmentSelector;
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
    pub rip: usize
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
            rip: 0
        }
    }
}

fn task_finished()
{
    crate::kernel::sched::task_finished();
}

fn allocate_page_table(fun: MappedAddr, code_size: usize) -> PhysAddr {
    use crate::arch::mm::virt::{current_p4_table, table::P4Table};
    use crate::arch::mm::phys::allocate;
    use crate::kernel::mm::virt;

    let current_p4 = current_p4_table();
    let frame = allocate().expect("Out of mem!");
    let new_p4 = P4Table::new_mut(&frame);

    new_p4.clear();

    for i in 256..512 {
        new_p4.set_entry(i, current_p4.entry_at(i));
    }

    let code = allocate().expect("Out of mem!");
    for i in 0..code_size {
        unsafe {
            (code.address_mapped() + i).store(
                (fun + i).read::<u8>()
            );
        }
    }

    new_p4.map_to_flags(VirtAddr(0x40000), code.address(), virt::PageFlags::USER);
    new_p4.map_flags(VirtAddr(0x60000),
                     virt::PageFlags::USER | virt::PageFlags::WRITABLE | virt::PageFlags::NO_EXECUTE);
    new_p4.phys_addr()
}


#[repr(C, packed)]
struct IretqFrame {
    pub ip: usize,
    pub cs: usize,
    pub rlfags: usize,
    pub sp: usize,
    pub ds: usize,
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

    fn new_sp(fun: fn (), cs: SegmentSelector, ds: SegmentSelector, int_enabled: bool, stack: usize, stack_size: usize, cr3: PhysAddr, user_stack: Option<usize>) -> Task {
        unsafe {
            let sp = (stack as *mut u8).offset(stack_size as isize);
            let frame: &mut IretqFrame =
                &mut *(sp.offset(
                    -(::core::mem::size_of::<IretqFrame>() as isize)
                ) as *mut IretqFrame);

            frame.task_finished_fun = task_finished as usize;
            frame.ds                = ds.bits() as usize;
            frame.sp                = user_stack.unwrap_or(sp.offset(-8) as usize);
            frame.rlfags            = if int_enabled { 0x200 } else { 0x0 };
            frame.cs                = cs.bits() as usize;
            frame.ip                = fun as usize;

            let mut ctx = Unique::new_unchecked(sp.offset(
                -(::core::mem::size_of::<Context>() as isize + ::core::mem::size_of::<IretqFrame>() as isize)) as *mut Context);
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

    fn new(fun: fn (), cs: SegmentSelector, ds: SegmentSelector, int_enabled: bool, cr3: PhysAddr, user_stack: Option<usize>) -> Task {
        let sp = heap_allocate_align(4096*16, 4096).unwrap();

        Task::new_sp(fun, cs, ds, int_enabled, sp as usize, 4096*16, cr3, user_stack)
    }

    pub fn new_kern(fun: fn ()) -> Task {
        Task::new(fun, gdt::ring0_cs(), gdt::ring0_ds(), true,  p4_table_addr(), None)
    }

    pub fn new_sched(fun: fn ()) -> Task {
        Task::new(fun, gdt::ring0_cs(), gdt::ring0_ds(), false, p4_table_addr(), None)
    }

    pub fn new_user(fun: MappedAddr, code_size: usize, stack: usize) -> Task {

        let new_p4 = allocate_page_table(fun, code_size);

        let f = unsafe { ::core::mem::transmute::<usize, fn()>(0x40000) };

        Task::new(f, gdt::ring3_cs(), gdt::ring3_ds(), true, new_p4, Some(stack))
    }

    pub fn deallocate(&mut self) {
        self.ctx = Unique::empty();
        heap_deallocate_align(
            self.stack_top as *mut u8,
            self.stack_size, 4096
        );
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
        switch_to(
            &mut from.ctx,
            to.ctx.as_ref()
        );
    }
}

pub fn activate_task(to: &Task) {
    unsafe {
        if to.is_user {
            crate::arch::gdt::update_tss_rps0(to.stack_top + to.stack_size);
        }
        activate_to(
            to.ctx.as_ref()
        );
    }
}
