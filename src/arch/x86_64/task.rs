use arch::raw::segmentation::SegmentSelector;

#[derive(Clone, Debug)]
#[repr(C)]
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

unsafe impl Send for ContextMutPtr {}

#[derive(Copy, Clone, Debug)]
pub struct Task {
    pub ctx: ContextMutPtr,
    //top of the stack, used to deallocate
    pub stack_top: usize,
}

impl Context {

    #[allow(unused)]
    const fn empty() -> Context {
        Context {
            rflags: 0,
            rbp: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rbx: 0,
            rip: 0
        }
    }
}

fn task_finished()
{

}

impl Task {
    fn new_sp(fun: fn (), cs: SegmentSelector, ds: SegmentSelector, int_enabled: bool, stack: usize, stack_size: usize) -> Task {
        unsafe {
            let sp = (stack as *mut u8).offset(stack_size as isize);
            *(sp.offset(-8) as *mut usize) = task_finished as usize;//task finished function
            *(sp.offset(-16) as *mut usize) = ds.bits() as usize;//task finished function
            *(sp.offset(-24) as *mut usize) = sp.offset(-8) as usize;                           //sp
            *(sp.offset(-32) as *mut usize) = if int_enabled { 0x200 } else { 0x0 };                                            //rflags enable interrupts
            *(sp.offset(-40) as *mut usize) = cs.bits() as usize;//cs
            *(sp.offset(-48) as *mut usize) = fun as usize;                                     //rip
            let ctx = sp.offset(-(::core::mem::size_of::<Context>() as isize + 48)) as *mut Context;
            (*ctx).rip = isr_return as usize;
            Task {
                ctx: ContextMutPtr(ctx),
                stack_top: sp as usize - stack_size,
            }
        }

    }

    pub fn new(fun: fn (), cs: SegmentSelector, ds: SegmentSelector, int_enabled: bool) -> Task {
        let sp = unsafe {
            ::kernel::mm::heap::allocate(::core::alloc::Layout::from_size_align_unchecked(4096*4, 4096)).unwrap()
        };

        Task::new_sp(fun, cs, ds, int_enabled, sp as usize, 4096*4)
    }

    pub fn new_sched(fun: fn ()) -> Task {
        Task::new(fun, SegmentSelector::new(1, SegmentSelector::RPL_0), SegmentSelector::new(0, SegmentSelector::RPL_0), false)
    }

    pub fn new_kern(fun: fn ()) -> Task {
        Task::new(fun, SegmentSelector::new(1, SegmentSelector::RPL_0), SegmentSelector::new(0, SegmentSelector::RPL_0), true)
    }

    pub fn new_user(fun: fn(), stack: usize, stack_size: usize) -> Task {
        Task::new_sp(
            fun,
            SegmentSelector::new(3, SegmentSelector::RPL_3),
            SegmentSelector::new(4, SegmentSelector::RPL_3),
            true,
            stack, stack_size)
    }

    pub const fn empty() -> Task {
        Task {
            ctx: ContextMutPtr(::core::ptr::null_mut()),
            stack_top: 0,
        }
    }

    pub fn deallocate(&mut self) {
        self.ctx = ContextMutPtr(::core::ptr::null_mut());
        //unsafe {
        //    HEAP.dealloc(self.stack_top as *mut u8, ::core::alloc::Layout::from_size_align_unchecked(4096*4, 4096));
        //}
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