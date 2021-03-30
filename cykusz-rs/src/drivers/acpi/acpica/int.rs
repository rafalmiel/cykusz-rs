#![allow(non_snake_case)]
#![allow(unused_variables)]

use acpica::*;

use crate::arch::int::{mask_int, set_irq_dest};
use crate::kernel::sync::Spin;

fn acpi_irq() -> bool {
    let c = CTX.lock_irq();
    let ctx = c.as_ref().unwrap();

    unsafe {
        ctx.handler.unwrap()(ctx.ctx);
    }

    core::mem::drop(c);

    return true;
}

struct Ctx {
    handler: ACPI_OSD_HANDLER,
    ctx: *mut core::ffi::c_void,
}

unsafe impl Send for Ctx {}

static CTX: Spin<Option<Ctx>> = Spin::new(None);

#[no_mangle]
#[linkage = "external"]
extern "C" fn AcpiOsInstallInterruptHandler(
    InterruptNumber: UINT32,
    ServiceRoutine: ACPI_OSD_HANDLER,
    Context: *mut ::core::ffi::c_void,
) -> ACPI_STATUS {
    use crate::arch::idt::*;

    if ServiceRoutine.is_none() {
        return AE_BAD_PARAMETER;
    }

    if !has_handler(InterruptNumber as usize + 32) {
        let mut ctx = CTX.lock_irq();

        if ctx.is_some() {
            return AE_ALREADY_EXISTS;
        }

        *ctx = Some(Ctx {
            handler: ServiceRoutine,
            ctx: Context,
        });

        set_irq_dest(InterruptNumber as u8, InterruptNumber as u8 + 32);
        add_shared_irq_handler(InterruptNumber as usize + 32, acpi_irq);

        AE_OK
    } else {
        AE_ALREADY_EXISTS
    }
}

#[no_mangle]
#[linkage = "external"]
extern "C" fn AcpiOsRemoveInterruptHandler(
    InterruptNumber: UINT32,
    ServiceRoutine: ACPI_OSD_HANDLER,
) -> ACPI_STATUS {
    mask_int(InterruptNumber as u8, true);
    crate::arch::idt::remove_handler(InterruptNumber as usize + 32);

    AE_OK
}
