use core::ptr::*;

use ::acpica::*;
pub use pci_map::get_irq_mapping;

pub mod acpica;
pub mod pci_map;

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
                Some(self::acpica::ec::embedded_ctl),
                Some(self::acpica::ec::embedded_ctl_setup),
                null_mut(),
            ),
            AE_OK
        );

        assert_eq!(AcpiInitializeObjects(0), AE_OK);

        assert_eq!(AcpiEnable(), AE_OK);
    }

    pci_map::init();

    println!("[ OK ] ACPI Initialized");
}

platform_2_init!(init);
