#![allow(non_snake_case)]
#![allow(unused_variables)]
/* automatically generated by rust-bindgen */


use acpica::*;
use crate::kernel::mm::*;
use crate::kernel::sync::{Spin, Semaphore};
use alloc::boxed::Box;
use crate::arch::x86_64::raw::cpuio::Port;
use crate::kernel::timer::busy_sleep;
use core::ffi::VaList;
use crate::arch::x86_64::raw::idt::ExceptionStackFrame;
use crate::arch::x86_64::int::{set_irq_dest, mask_int};

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsInitialize() -> ACPI_STATUS {
    AE_OK
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsTerminate() -> ACPI_STATUS {
    AE_OK
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsGetRootPointer() -> ACPI_PHYSICAL_ADDRESS {

    let mut val = 0;
    // SAFE: Called from within ACPI init context
    match unsafe { AcpiFindRootPointer(&mut val) }
    {
        AE_OK => {
            println!("Found root pointer: 0x{:x}", val);
        },
        e @ _ => {
            println!("Failed to find ACPI root pointer : {}", e);
            return 0;
        },
    }

    val as ACPI_PHYSICAL_ADDRESS
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsPredefinedOverride(
        InitVal: *const ACPI_PREDEFINED_NAMES,
        NewVal: *mut ACPI_STRING,
    ) -> ACPI_STATUS {
    unsafe {
        *NewVal = 0 as *mut _;
    }
    AE_OK
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsTableOverride(
        ExistingTable: *mut ACPI_TABLE_HEADER,
        NewTable: *mut *mut ACPI_TABLE_HEADER,
    ) -> ACPI_STATUS {
    unsafe {
        *NewTable = 0 as *mut _;
    }
    AE_OK
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsPhysicalTableOverride(
        ExistingTable: *mut ACPI_TABLE_HEADER,
        NewAddress: *mut ACPI_PHYSICAL_ADDRESS,
        NewTableLength: *mut UINT32,
    ) -> ACPI_STATUS {
    unsafe {
        *NewAddress = 0;
    }

    AE_OK
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsCreateLock(OutHandle: *mut *mut ::core::ffi::c_void) -> ACPI_STATUS {
    unsafe {
        let spin = Spin::<()>::new( () );
        *OutHandle = Box::into_raw(Box::new(spin)) as *mut core::ffi::c_void;
    }
    AE_OK
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsDeleteLock(Handle: *mut ::core::ffi::c_void) {
    unsafe {
        let b = Box::from_raw(Handle as *mut Spin::<()>);
        core::mem::drop(b)
    }
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsAcquireLock(Handle: *mut ::core::ffi::c_void) -> ACPI_SIZE {
    unsafe {
        let b = &*(Handle as *mut Spin::<()>);
        b.unguarded_obtain();
        0
    }
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsReleaseLock(Handle: *mut ::core::ffi::c_void, Flags: ACPI_SIZE) {
    unsafe {
        let b = &*(Handle as *mut Spin::<()>);
        b.unguarded_release();
    }
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsCreateSemaphore(
        MaxUnits: UINT32,
        InitialUnits: UINT32,
        OutHandle: *mut *mut ::core::ffi::c_void,
    ) -> ACPI_STATUS {
    let sem = Semaphore::new(InitialUnits as isize, MaxUnits as isize);
    unsafe {
        *OutHandle = Box::into_raw(Box::new(sem)) as *mut core::ffi::c_void;
    }

    AE_OK
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsDeleteSemaphore(Handle: *mut ::core::ffi::c_void) -> ACPI_STATUS {
    unsafe {
        let b = Box::from_raw(Handle as *mut Semaphore);
        core::mem::drop(b)
    }

    AE_OK
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsWaitSemaphore(
        Handle: *mut ::core::ffi::c_void,
        Units: UINT32,
        Timeout: UINT16,
    ) -> ACPI_STATUS {
    unsafe {
        let s = &*(Handle as *mut Semaphore);
        s.acquire();
    }

    AE_OK
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsSignalSemaphore(Handle: *mut ::core::ffi::c_void, Units: UINT32) -> ACPI_STATUS {
    unsafe {
        let s = &*(Handle as *mut Semaphore);
        s.release();
    }

    AE_OK

}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsAllocate(Size: ACPI_SIZE) -> *mut ::core::ffi::c_void {
    let a = crate::kernel::mm::heap::allocate(Size as usize + core::mem::size_of::<usize>()).unwrap() as *mut usize;
    unsafe {
        *a = Size as usize;
    }

    return unsafe { a.offset(1) } as *mut ::core::ffi::c_void;
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsFree(Memory: *mut ::core::ffi::c_void) {
    let a = Memory as *mut usize;
    let (ptr, size) = unsafe {
        let s = a.offset(-1).read();
        (a.offset(-1), s + core::mem::size_of::<usize>())
    };

    crate::kernel::mm::heap::deallocate(ptr as *mut u8, size);
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsMapMemory(
        Where: ACPI_PHYSICAL_ADDRESS,
        Length: ACPI_SIZE,
    ) -> *mut ::core::ffi::c_void {

    PhysAddr(Where as usize).to_mapped().0 as *mut core::ffi::c_void
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsUnmapMemory(LogicalAddress: *mut ::core::ffi::c_void, Size: ACPI_SIZE) {
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsGetPhysicalAddress(
        LogicalAddress: *mut ::core::ffi::c_void,
        PhysicalAddress: *mut ACPI_PHYSICAL_ADDRESS,
    ) -> ACPI_STATUS {

    unsafe {
        (PhysicalAddress as *mut isize).write(MappedAddr(LogicalAddress as usize).to_phys().0 as isize)
    }

    AE_OK
}


extern "x86-interrupt" fn acpi_irq(_frame: &mut ExceptionStackFrame) {
    println!("ACPI INT");
    let c = CTX.lock();
    let ctx = c.as_ref().unwrap();

    unsafe {
        ctx.handler.unwrap()(ctx.ctx);
    }
}

struct Ctx {
    handler: ACPI_OSD_HANDLER,
    ctx: *mut core::ffi::c_void,
}

unsafe impl Send for Ctx {}

static CTX: Spin<Option<Ctx>> = Spin::new(None);

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsInstallInterruptHandler(
        InterruptNumber: UINT32,
        ServiceRoutine: ACPI_OSD_HANDLER,
        Context: *mut ::core::ffi::c_void,
    ) -> ACPI_STATUS {

    use crate::arch::idt::*;

    if ServiceRoutine.is_none() || Context.is_null() {
        return AE_BAD_PARAMETER;
    }

    if !has_handler(InterruptNumber as usize + 32) {

        let mut ctx = CTX.lock();

        if ctx.is_some() {
            return AE_ALREADY_EXISTS;
        }

        *ctx = Some(Ctx {
            handler: ServiceRoutine,
            ctx: Context,
        });

        set_irq_dest(InterruptNumber as u8, InterruptNumber as u8 + 32);
        set_handler(InterruptNumber as usize + 32, acpi_irq);

        AE_OK
    } else {
        AE_ALREADY_EXISTS
    }
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsRemoveInterruptHandler(
        InterruptNumber: UINT32,
        ServiceRoutine: ACPI_OSD_HANDLER,
    ) -> ACPI_STATUS {
    mask_int(InterruptNumber as u8, true);
    crate::arch::idt::remove_handler(InterruptNumber as usize + 32);

    AE_OK
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsGetThreadId() -> UINT64 {
    1
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsExecute(
        Type: ACPI_EXECUTE_TYPE,
        Function: ACPI_OSD_EXEC_CALLBACK,
        Context: *mut ::core::ffi::c_void,
    ) -> ACPI_STATUS {
    unimplemented!()
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsWaitEventsComplete() {
    unimplemented!()
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsSleep(Milliseconds: UINT64) {
    use crate::kernel::sched::current_task;

    current_task().sleep(Milliseconds as usize * 1_000_000);
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsStall(Microseconds: UINT32) {
    busy_sleep(Microseconds as u64 * 1000)
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsReadPort(
        Address: ACPI_IO_ADDRESS,
        Value: *mut UINT32,
        Width: UINT32,
    ) -> ACPI_STATUS {
    unsafe {
        *Value = match Width {
            8 => Port::<u8>::new(Address as u16).read() as i32,
            16 => Port::<u16>::new(Address as u16).read() as i32,
            32 => Port::<u32>::new(Address as u16).read() as i32,
            _ => panic!("Unsupported port")
        };

        AE_OK
    }
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsWritePort(Address: ACPI_IO_ADDRESS, Value: UINT32, Width: UINT32) -> ACPI_STATUS {
    unsafe {
        match Width {
            8 => Port::<u8>::new(Address as u16).write(Value as u8),
            16 => Port::<u16>::new(Address as u16).write(Value as u16),
            32 => Port::<u32>::new(Address as u16).write(Value as u32),
            _ => panic!("Unsupported port")
        }

        AE_OK
    }
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsReadMemory(
        Address: ACPI_PHYSICAL_ADDRESS,
        Value: *mut UINT64,
        Width: UINT32,
    ) -> ACPI_STATUS {
    unsafe {
        *Value = match Width {
            8 => PhysAddr(Address as usize).to_mapped().read::<u8>() as i64,
            16 => PhysAddr(Address as usize).to_mapped().read::<u16>() as i64,
            32 => PhysAddr(Address as usize).to_mapped().read::<u32>() as i64,
            64 => PhysAddr(Address as usize).to_mapped().read::<u64>() as i64,
            _ => panic!("Invalid Width"),
        };

        AE_OK
    }
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsWriteMemory(
        Address: ACPI_PHYSICAL_ADDRESS,
        Value: UINT64,
        Width: UINT32,
    ) -> ACPI_STATUS {
    unsafe {
        match Width {
            8 => PhysAddr(Address as usize).to_mapped().store::<u8>(Value as u8),
            16 => PhysAddr(Address as usize).to_mapped().store::<u16>(Value as u16),
            32 => PhysAddr(Address as usize).to_mapped().store::<u32>(Value as u32),
            64 => PhysAddr(Address as usize).to_mapped().store::<u64>(Value as u64),
            _ => panic!("Invalid Width"),
        };

        AE_OK
    }
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsReadPciConfiguration(
        PciId: *mut ACPI_PCI_ID,
        Reg: UINT32,
        Value: *mut UINT64,
        Width: UINT32,
    ) -> ACPI_STATUS {
    unimplemented!()
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsWritePciConfiguration(
        PciId: *mut ACPI_PCI_ID,
        Reg: UINT32,
        Value: UINT64,
        Width: UINT32,
    ) -> ACPI_STATUS {
    unimplemented!()
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsReadable(Pointer: *mut ::core::ffi::c_void, Length: ACPI_SIZE) -> BOOLEAN {
    true
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsWritable(Pointer: *mut ::core::ffi::c_void, Length: ACPI_SIZE) -> BOOLEAN {
    true
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsGetTimer() -> UINT64 {
    //100s ns
    crate::arch::dev::hpet::current_ns() as i64 / 100
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsSignal(Function: UINT32, Info: *mut ::core::ffi::c_void) -> ACPI_STATUS {
    if Function == ACPI_SIGNAL_FATAL as i32 {
        panic!("ACPI_SIGNAL_FATAL");
    }

    AE_OK
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsEnterSleep(SleepState: UINT8, RegaValue: UINT32, RegbValue: UINT32)
        -> ACPI_STATUS {
    AE_OK
}

//unsafe fn c_string_to_str<'a>(c_str: *const i8) -> &'a str {
//    ::core::str::from_utf8( ::memory::c_string_as_byte_slice(c_str).unwrap_or(b"INVALID") ).unwrap_or("UTF-8")
//}
//fn get_uint(Args: &mut VaList, size: usize) -> u64 {
//    // (uncheckable) SAFE: Could over-read from stack, returning junk
//    unsafe {
//        match size
//        {
//            0 => Args.arg::<u32>() as u64,
//            1 => Args.arg::<u32>() as u64,
//            2 => Args.arg::<u64>(),
//            _ => unreachable!(),
//        }
//    }
//}
//fn get_int(Args: &mut VaList, size: usize) -> i64 {
//    // (uncheckable) SAFE: Could over-read from stack, returning junk
//    unsafe {
//        match size
//        {
//            0 => Args.arg::<i32>() as i64,
//            1 => Args.arg::<i32>() as i64,
//            2 => Args.arg::<i64>(),
//            _ => unreachable!(),
//        }
//    }
//}

#[no_mangle]
#[linkage="external"]
#[allow(dead_code)]
extern "C" fn AcpiOsVprintf(Format: *const i8, mut Args: core::ffi::VaList)
{
    /*
    use sync::mutex::LazyMutex;
    struct Buf([u8; 256]);
    impl Buf {
        fn new() -> Self {
            // SAFE: POD
            unsafe { ::core::mem::zeroed() }
        }
    }
    impl AsMut<[u8]> for Buf { fn as_mut(&mut self) -> &mut [u8] { &mut self.0 } }
    impl AsRef<[u8]> for Buf { fn as_ref(&self) -> &[u8] { &self.0 } }
    static TEMP_BUFFER: LazyMutex<::lib::string::FixedString<Buf>> = LazyMutex::new();

    // Acquire input and lock
    // SAFE: Format string is valid for function
    let fmt = unsafe { c_string_to_str(Format) };
    let mut lh = TEMP_BUFFER.lock_init(|| ::lib::string::FixedString::new(Buf::new()));

    // Expand format string
    let mut it = fmt.chars();
    while let Some(mut c) = it.next()
    {
        if c == '\n' {
            // Flush
            println!("{}", *lh);
            lh.clear();
        }
        else if c != '%' {
            lh.push_char(c);
        }
        else {
            use core::fmt::Write;

            c = match it.next() { Some(v)=>v,_=>return };

            let mut align_left = false;
            if c == '-' {
                align_left = true;
                c = match it.next() { Some(v)=>v,_=>return };
            }

            let mut width = 0;
            while let Some(d) = c.to_digit(10) {
                width = width * 10 + d;
                c = match it.next() { Some(v)=>v,_=>return };
            }

            let mut precision = !0;
            if c == '.' {
                precision = 0;
                c = match it.next() { Some(v)=>v,_=>return };
                while let Some(d) = c.to_digit(10) {
                    precision = precision * 10 + d;
                    c = match it.next() { Some(v)=>v,_=>return };
                }
            }

            let size = if c == 'l' {
                c = match it.next() { Some(v)=>v,_=>return };
                if c == 'l' {
                    c = match it.next() { Some(v)=>v,_=>return };
                    2
                }
                else {
                    1
                }
            }
            else {
                0
            };

            // TODO: Use undocumented (but public) APIs in ::core::fmt
            // to create an Arguments structure from this information
            //let spec = ::core::fmt::rt::v1::FormatSpec {
            //	fill: ' ',
            //	align: ::core::fmt::rt::v1::Alignment::Unknown,
            //	flags: 0,
            //	precision: ::core::fmt::rt::v1::Count::Is(precision),
            //	width: ::core::fmt::rt::v1::Count::Is(width),
            //	};
            let _ = align_left;

            match c
            {
                'x' => {
                    let _ = write!(&mut *lh, "{:x}", get_uint(&mut Args, size));
                },
                'X' => {
                    let val = get_uint(&mut Args, size);
                    let _ = write!(&mut *lh, "{:X}", val);
                },
                'd' => {
                    let val = get_int(&mut Args, size);
                    let _ = write!(&mut *lh, "{}", val);
                },
                'u' => {
                    let val = get_uint(&mut Args, size);
                    let _ = write!(&mut *lh, "{}", val);
                },
                'p' => {
                    // (uncheckable) SAFE: Could over-read from stack, returning junk
                    let _ = write!(&mut *lh, "{:p}", unsafe { Args.get::<*const u8>() });
                },
                'c' => {
                    // (uncheckable) SAFE: Could over-read from stack, returning junk
                    let _ = write!(&mut *lh, "{}", unsafe { Args.get::<u32>() as u8 as char });
                },
                's' => {
                    // SAFE: Does as much validation as possible, if ACPICA misbehaves... well, we're in trouble
                    let slice = unsafe {
                        let ptr = Args.get::<*const u8>();
                        if precision < !0 {
                            ::core::str::from_utf8(::core::slice::from_raw_parts(ptr, precision as usize)).unwrap_or("")
                        }
                        else {
                            c_string_to_str(ptr as *const i8)
                        }
                    };
                    let _ = write!(&mut *lh, "{}", slice);
                },
                _ => {
                    panic!("AcpiOsVprintf - Unknown format code {}", c);
                },
            }
        }
    }
    */
}
