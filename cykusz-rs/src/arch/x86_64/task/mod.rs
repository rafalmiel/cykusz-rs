use alloc::vec::Vec;
use core::mem::size_of;
use core::ptr::Unique;

use syscall_defs::exec::ExeArgs;
use syscall_defs::{MMapFlags, MMapProt};

use crate::arch::gdt;
use crate::arch::gdt::update_tss_rps0;
use crate::arch::idt::RegsFrame;
use crate::arch::mm::phys::{allocate_order, deallocate_order};
use crate::arch::mm::virt::p4_table;
use crate::arch::mm::virt::table::P4Table;
use crate::arch::mm::virt::{activate_table, current_p4_table, p4_table_addr};
use crate::arch::raw::idt::InterruptFrame;
use crate::arch::raw::mm::MappedAddr;
use crate::arch::raw::segmentation::SegmentSelector;
use crate::arch::syscall::SyscallFrame;
use crate::arch::utils::StackHelper;
use crate::kernel::mm::virt::PageFlags;
use crate::kernel::mm::{allocate, VirtAddr, PAGE_SIZE};
use crate::kernel::mm::{Frame, PhysAddr};
use crate::kernel::task::vm::{TlsVmInfo, VM};

mod args;

const USER_STACK_SIZE: usize = 0x64000;
const KERN_STACK_SIZE: usize = 4096 * 4;
const KERN_STACK_ORDER: usize = 2;

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct Context {
    /// Page Table pointer
    pub cr3: usize,
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
    /// RFLAGS register
    pub rflags: usize,
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
    pub user_fs_base: usize,
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
    crate::kernel::sched::exit(0);
}

fn prepare_p4<'a>() -> &'a mut P4Table {
    let current_p4 = current_p4_table();

    let new_p4 = P4Table::new();

    // Map kernel code to the user space, don't care about intel exploits for now
    for i in 256..512 {
        new_p4.set_entry(i, current_p4.entry_at(i));
    }

    new_p4
}

#[allow(unused)]
fn prepare_tls(vm: &VM, p_table: &mut P4Table, tls: &TlsVmInfo) -> VirtAddr {
    let mmap = vm
        .mmap_vm(
            Some(tls.mmap_addr_hint),
            tls.mem_size + 8,
            MMapProt::PROT_READ | MMapProt::PROT_WRITE,
            MMapFlags::MAP_ANONYOMUS | MMapFlags::MAP_PRIVATE,
            None,
            0,
        )
        .expect("Failed to mmap tls");

    for (num, m) in (mmap..(mmap + VirtAddr(tls.mem_size + 8)).align_up(PAGE_SIZE))
        .step_by(PAGE_SIZE)
        .enumerate()
    {
        let frame = allocate().expect("Failed to allocate tls frame");

        let offset = num * PAGE_SIZE;
        if offset < tls.file_size {
            let rem = tls.file_size - offset;
            let to_read = core::cmp::min(PAGE_SIZE, rem);

            if let Ok(r) = tls.file.inode().read_at(tls.file_offset + offset, unsafe {
                frame.address_mapped().as_bytes_mut(to_read)
            }) {
                if r != to_read {
                    panic!("Failed to read tls data");
                }
            } else {
                panic!("Failed to read tls data");
            }
        }

        p_table.map_to_flags(m, frame.address(), PageFlags::USER | PageFlags::WRITABLE);
    }

    let phys = p_table.to_phys(mmap + tls.mem_size).unwrap();
    unsafe {
        phys.to_mapped().store(mmap + tls.mem_size);
    }

    mmap + tls.mem_size
}

#[repr(C, packed)]
struct KTaskInitFrame {
    pub rdi: usize,
    pub int: InterruptFrame,
    pub task_finished_fun: usize,
}

impl Default for Task {
    fn default() -> Task {
        Task::empty()
    }
}

impl Task {
    pub fn empty() -> Task {
        Task {
            ctx: Unique::dangling(),
            cr3: 0,
            stack_top: 0,
            stack_size: 0,
            user_stack: None,
            user_fs_base: 0,
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
        sp: usize,
        cr3: PhysAddr,
        param: usize,
    ) -> Unique<Context> {
        let mut sp_tmp = sp as u64;

        let mut helper = StackHelper::new(&mut sp_tmp);

        let frame = helper.next::<KTaskInitFrame>();

        frame.task_finished_fun = task_finished as usize;
        frame.int.ss = ds.bits() as u64;
        frame.int.sp = (sp - 8) as u64;
        frame.int.cf = if int_enabled { 0x200 } else { 0x0 };
        frame.int.cs = cs.bits() as u64;
        frame.int.ip = fun as u64;
        frame.rdi = param;

        let ctx = helper.next::<Context>();

        *ctx = Context::empty();
        ctx.rip = isr_return as usize;
        ctx.cr3 = cr3.0;

        Unique::new_unchecked(ctx as *mut Context)
    }

    unsafe fn prepare_sysretq_ctx(
        fun: usize,
        int_enabled: bool,
        user_stack: Option<usize>,
        sp: usize,
        cr3: PhysAddr,
    ) -> Unique<Context> {
        let mut sp = sp as u64;

        let mut helper = StackHelper::new(&mut sp);

        let frame = helper.next::<SyscallFrame>();

        frame.rsp = user_stack.unwrap() as u64;
        frame.rflags = if int_enabled { 0x200 } else { 0x0 };
        frame.rip = fun as u64;

        let ctx = helper.next::<Context>();

        *ctx = Context::empty();
        ctx.rip = asm_sysretq_userinit as usize;
        ctx.cr3 = cr3.0;

        Unique::new_unchecked(ctx as *mut Context)
    }

    unsafe fn syscall_frame(&self) -> &SyscallFrame {
        VirtAddr(self.stack_top + self.stack_size - size_of::<SyscallFrame>())
            .read_ref::<SyscallFrame>()
    }

    unsafe fn syscall_regs(&self) -> &RegsFrame {
        VirtAddr(
            self.stack_top + self.stack_size - size_of::<SyscallFrame>() - size_of::<RegsFrame>(),
        )
        .read_ref::<RegsFrame>()
    }

    unsafe fn fork_ctx(&self, sp: usize, cr3: usize) -> Unique<Context> {
        let parent_sys_frame = self.syscall_frame();
        let parent_regs_frame = self.syscall_regs();

        let mut sp = sp as u64;

        let mut helper = StackHelper::new(&mut sp);

        *helper.next::<SyscallFrame>() = *parent_sys_frame;

        let regs = helper.next::<RegsFrame>();
        *regs = *parent_regs_frame;
        regs.rax = 0;

        let ctx = helper.next::<Context>();

        *ctx = Context::empty();
        ctx.rip = asm_sysretq_forkinit as usize;
        ctx.cr3 = cr3;

        Unique::new_unchecked(ctx as *mut Context)
    }

    unsafe fn fork_thread_ctx(
        &self,
        entry: usize,
        user_stack: usize,
        sp: usize,
    ) -> Unique<Context> {
        let mut sp = sp as u64;

        let mut helper = StackHelper::new(&mut sp);

        let sys_frame = helper.next::<SyscallFrame>();

        sys_frame.rip = entry as u64;
        sys_frame.rflags = 0x200;
        sys_frame.rsp = user_stack as u64;

        let regs = helper.next::<RegsFrame>();
        *regs = RegsFrame::default();

        let ctx = helper.next::<Context>();

        *ctx = Context::empty();
        ctx.rip = asm_sysretq_forkinit as usize;
        ctx.cr3 = self.cr3;

        Unique::new_unchecked(ctx as *mut Context)
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
            let sp = stack + stack_size;

            let ctx = if user_stack.is_none() {
                Task::prepare_iretq_ctx(fun, cs, ds, int_enabled, sp, cr3, param)
            } else {
                // Userspace transition is done using sysretq call
                Task::prepare_sysretq_ctx(fun, int_enabled, user_stack, sp, cr3)
            };

            Task {
                ctx,
                cr3: cr3.0,
                stack_top: sp - stack_size,
                stack_size,
                user_stack,
                user_fs_base: 0,
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
        let sp = allocate_order(KERN_STACK_ORDER).unwrap().address_mapped().0;

        Task::new_sp(
            fun,
            cs,
            ds,
            int_enabled,
            sp,
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

    pub fn new_user(entry: VirtAddr, vm: &VM, tls_vm: Option<TlsVmInfo>) -> Task {
        let p_table = prepare_p4();

        vm.mmap_vm(
            Some(VirtAddr(0x8000_0000_0000 - USER_STACK_SIZE)),
            USER_STACK_SIZE,
            MMapProt::PROT_WRITE | MMapProt::PROT_READ,
            MMapFlags::MAP_FIXED | MMapFlags::MAP_PRIVATE | MMapFlags::MAP_ANONYOMUS,
            None,
            0,
        );

        let tls_ptr = if let Some(tls) = &tls_vm {
            prepare_tls(vm, p_table, tls)
        } else {
            VirtAddr(0)
        };

        let f = unsafe { ::core::mem::transmute::<usize, fn()>(entry.0) };

        let mut t = Task::new(
            f as usize,
            gdt::ring3_cs(),
            gdt::ring3_ds(),
            true,
            p_table.phys_addr(),
            Some(0x8000_0000_0000),
            0,
        );

        t.user_fs_base = tls_ptr.0;

        t
    }

    pub fn fork(&self) -> Task {
        let orig_p4 = P4Table::new_mut_at_phys(PhysAddr(self.cr3));

        let new_p4 = orig_p4.duplicate();

        // We marked writable entries in parent and child process as readonly to enable COW
        // Flush the pagetable of the current process
        crate::arch::mm::virt::flush_all();

        let sp_top = allocate_order(KERN_STACK_ORDER).unwrap().address_mapped().0;

        let sp = sp_top + KERN_STACK_SIZE;

        let new_ctx = unsafe { self.fork_ctx(sp, new_p4.phys_addr().0) };

        Task {
            ctx: new_ctx,
            cr3: new_p4.phys_addr().0,
            stack_top: sp_top,
            stack_size: KERN_STACK_SIZE,
            user_stack: self.user_stack,
            user_fs_base: self.user_fs_base,
        }
    }

    pub fn fork_thread(&self, entry: usize, user_stack: usize) -> Task {
        let sp_top = allocate_order(KERN_STACK_ORDER).unwrap().address_mapped().0;

        let p4 = p4_table(PhysAddr(self.cr3 as usize));
        p4.ref_table();

        let sp = sp_top + KERN_STACK_SIZE;

        let new_ctx = unsafe { self.fork_thread_ctx(entry, user_stack, sp) };

        Task {
            ctx: new_ctx,
            cr3: self.cr3,
            stack_top: sp_top,
            stack_size: KERN_STACK_SIZE,
            user_stack: Some(user_stack),
            user_fs_base: self.user_fs_base,
        }
    }

    pub fn exec(
        &mut self,
        entry: VirtAddr,
        vm: &VM,
        tls_vm: Option<TlsVmInfo>,
        args: Option<ExeArgs>,
        envs: Option<ExeArgs>,
    ) -> ! {
        let args = args.map(|a| args::Args::new(a));
        let envs = envs.map(|e| args::Args::new(e));

        let p_table = if self.is_user() {
            let p_table = current_p4_table();
            p_table.deallocate_user();
            p_table
        } else {
            prepare_p4()
        };

        let mut user_stack: u64 = 0x8000_0000_0000;

        vm.mmap_vm(
            Some(VirtAddr(user_stack as usize - USER_STACK_SIZE)),
            USER_STACK_SIZE,
            MMapProt::PROT_WRITE | MMapProt::PROT_READ,
            MMapFlags::MAP_FIXED | MMapFlags::MAP_PRIVATE | MMapFlags::MAP_ANONYOMUS,
            None,
            0,
        );

        let tls_ptr = if let Some(tls) = &tls_vm {
            prepare_tls(vm, p_table, tls)
        } else {
            VirtAddr(0)
        };

        self.cr3 = p_table.phys_addr().0;
        self.user_fs_base = tls_ptr.0;
        self.user_stack = Some(0x8000_0000_0000);

        self.ctx = Unique::dangling();

        update_tss_rps0(self.stack_top + self.stack_size);

        unsafe {
            activate_table(p_table);
        }

        // Prepare user stack program arguments
        let mut helper = StackHelper::new(&mut user_stack);

        let mut envp = Vec::<u64>::new();
        let mut argp = Vec::<u64>::new();

        if let Some(e) = envs {
            envp = e.write_strings(&mut helper);
        };
        if let Some(a) = args {
            argp = a.write_strings(&mut helper);
        }

        helper.align_down();

        let len = envp.len() + 1 + argp.len() + 1 + 1;

        // If we write odd number of 8bytes later, add one 0u64 to keep the 16 byte alignment
        if len % 2 == 1 {
            unsafe {
                helper.write(0u64);
            }
        }

        unsafe {
            helper.write(0u64);
            helper.write_slice(envp.as_slice()); // char *const envp[]
            helper.write(0u64);
            helper.write_slice(argp.as_slice()); // char *const argv[]
            helper.write(argp.len()); // int argc
        }

        drop(envp);
        drop(argp);
        drop(tls_vm);

        unsafe {
            asm_jmp_user(helper.current() as usize, entry.0, 0x200);
        }
    }

    fn unref_page_table(&mut self) {
        let p4 = p4_table(PhysAddr(self.cr3));

        if self.is_user() {
            p4.unref_table_with(|p| {
                logln_disabled!("dealloc user");
                p.deallocate_user();
            });
        } else {
            p4.unref_table();
        }
    }

    pub fn deallocate(&mut self) {
        self.ctx = Unique::dangling();

        self.unref_page_table();

        deallocate_order(
            &Frame::new(MappedAddr(self.stack_top).to_phys()),
            KERN_STACK_ORDER,
        );

        self.stack_top = 0;
    }

    pub fn update_user_fs(&mut self, val: VirtAddr) {
        self.user_fs_base = val.0;
    }
}

extern "C" {
    pub fn switch_to(old_ctx: &mut Unique<Context>, new_ctx: &Context);
    pub fn activate_to(new_ctx: &Context);
    fn isr_return();
    fn asm_sysretq_userinit();
    fn asm_sysretq_forkinit();
    fn asm_jmp_user(ustack: usize, entry: usize, rflags: usize) -> !;
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
