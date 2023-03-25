use core::ptr::*;

use ::acpica::*;
pub use pci_map::get_irq_mapping;

use crate::kernel::sync::IrqGuard;

pub mod acpica;
pub mod pci_map;

pub fn init() {
    let _guard = IrqGuard::new();
    unsafe {
        assert_eq!(AcpiInitializeSubsystem(), AE_OK);
        assert_eq!(
            AcpiInitializeTables(core::ptr::null_mut(), 16, false),
            AE_OK
        );
        assert_eq!(AcpiLoadTables(), AE_OK);
        assert_eq!(
            AcpiEnableSubsystem(ACPI_FULL_INITIALIZATION as UINT32),
            AE_OK
        );
        assert_eq!(
            AcpiInstallAddressSpaceHandler(
                ACPI_ROOT_OBJECT,
                3,
                Some(self::acpica::ec::embedded_ctl),
                Some(self::acpica::ec::embedded_ctl_setup),
                null_mut(),
            ),
            AE_OK
        );

        assert_eq!(
            AcpiInitializeObjects(ACPI_FULL_INITIALIZATION as UINT32),
            AE_OK
        );

        assert_eq!(AcpiEnable(), AE_OK);
    }

    pci_map::init();

    println!("[ OK ] ACPI Initialized");
}

platform_2_init!(init);
