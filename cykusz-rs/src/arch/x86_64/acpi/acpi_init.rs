use core::ptr::*;

use acpica::*;

pub fn init() {
    unsafe {
        assert_eq!(AcpiInitializeSubsystem(), AE_OK);
        assert_eq!(
            AcpiInitializeTables(core::ptr::null_mut(), 16, false),
            AE_OK
        );
        assert_eq!(AcpiLoadTables(), AE_OK);
        assert_eq!(AcpiEnableSubsystem(0), AE_OK);

        assert_eq!(
            AcpiInstallAddressSpaceHandler(
                ACPI_ROOT_OBJECT,
                3,
                Some(super::acpica::ec::embedded_ctl),
                Some(super::acpica::ec::embedded_ctl_setup),
                null_mut()
            ),
            AE_OK
        );

        assert_eq!(AcpiInitializeObjects(0), AE_OK);
    }

    crate::arch::acpi::pci::init();
}
