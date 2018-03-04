#[macro_use]
pub mod newtype;
#[macro_use]
pub mod cpuio;
pub mod idt;
pub mod gdt;
pub mod io;
pub mod segmentation;
pub mod descriptor;
pub mod mm;
pub mod ctrlregs;
pub mod msr;