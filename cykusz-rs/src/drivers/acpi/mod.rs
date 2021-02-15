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
        println!("Acpi Subsystem Initialized");
        assert_eq!(
            AcpiInitializeTables(core::ptr::null_mut(), 16, false),
            AE_OK
        );
        println!("acpi Tables initialized");
        assert_eq!(AcpiLoadTables(), AE_OK);
        println!("Acpi tables loaded");
        assert_eq!(AcpiEnableSubsystem(0), AE_OK);
        println!("Acpi subsystem enabled");

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
        println!("Acpi Installed Address Space Handlers");

        assert_eq!(AcpiInitializeObjects(0), AE_OK);

        println!("Acpi Initialized Objects");

        assert_eq!(AcpiEnable(), AE_OK);

        println!("Acpi Enabled");
    }

    pci_map::init();

    println!("[ OK ] ACPI Initialized");
}

platform_2_init!(init);
