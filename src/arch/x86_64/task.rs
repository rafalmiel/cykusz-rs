use arch::raw::segmentation::SegmentSelector;
use arch::gdt;
use kernel::mm::heap::allocate as heap_allocate;

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct Context {
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
pub struct ContextMutPtr(pub *mut Context);

impl ContextMutPtr {
    pub const fn null() -> ContextMutPtr {
       ContextMutPtr(::core::ptr::null_mut())
    }
}

//unsafe impl Send for ContextMutPtr {}

#[derive(Copy, Clone, Debug)]
pub struct Task {
    pub ctx: ContextMutPtr,
    //top of the stack, used to deallocate
    pub stack_top: usize,
    pub stack_size: usize
}

impl Context {

    #[allow(unused)]
    const fn empty() -> Context {
        Context {
            rflags: 0,
            rbp: 0,
            rbx: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rip: 0
        }
    }
}

fn task_finished()
{
    ::kernel::sched::task_finished();
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
            ctx: ContextMutPtr(::core::ptr::null_mut()),
            stack_top: 0,
            stack_size: 0
        }
    }

    fn new_sp(fun: fn (), cs: SegmentSelector, ds: SegmentSelector, int_enabled: bool, stack: usize, stack_size: usize) -> Task {
        unsafe {
            let sp = (stack as *mut u8).offset(stack_size as isize);
            let frame: &mut IretqFrame =
                &mut *(sp.offset(
                    -(::core::mem::size_of::<IretqFrame>() as isize)
                ) as *mut IretqFrame);

            frame.task_finished_fun = task_finished as usize;
            frame.ds                = ds.bits() as usize;
            frame.sp                = sp.offset(-8) as usize;
            frame.rlfags            = if int_enabled { 0x200 } else { 0x0 };
            frame.cs                = cs.bits() as usize;
            frame.ip                = fun as usize;

            let ctx = sp.offset(
                -(::core::mem::size_of::<Context>() as isize + ::core::mem::size_of::<IretqFrame>() as isize)) as *mut Context;
            (*ctx).rip = isr_return as usize;

            Task {
                ctx: ContextMutPtr(ctx),
                stack_top: sp as usize - stack_size,
                stack_size
            }
        }

    }

    fn new(fun: fn (), cs: SegmentSelector, ds: SegmentSelector, int_enabled: bool) -> Task {
        let sp = unsafe {
            heap_allocate(::core::alloc::Layout::from_size_align_unchecked(4096*8, 4096)).unwrap()
        };

        Task::new_sp(fun, cs, ds, int_enabled, sp as usize, 4096*4)
    }

    pub fn new_kern(fun: fn ()) -> Task {
        Task::new(fun, gdt::ring0_cs(), gdt::ring0_ds(), true)
    }

    pub fn new_sched(fun: fn ()) -> Task {
        Task::new(fun, gdt::ring0_cs(), gdt::ring0_ds(), false)
    }

    pub fn deallocate(&mut self) {
        self.ctx = ContextMutPtr::null();
        unsafe {
            ::kernel::mm::heap::deallocate(
                self.stack_top as *mut u8,
                ::core::alloc::Layout::from_size_align_unchecked(self.stack_size, 4096)
            );
        }
        self.stack_top = 0;
    }
}


extern "C" {
    pub fn switch_to(old_ctx: *mut *mut Context, new_ctx: *const Context);
    fn isr_return();
}

pub fn switch(from: &mut Task, to: &Task) {
    unsafe {
        switch_to((&mut from.ctx.0) as *mut *mut Context, to.ctx.0);
    }
}

