use syscall_defs::{MMapFlags, MMapProt};
use syscall_user::{mmap, mprotect, munmap};

fn main() {
    let addr =
        mmap(Some(0x1000), 0x6000,
             MMapProt::PROT_READ | MMapProt::PROT_WRITE | MMapProt::PROT_EXEC,
             MMapFlags::MAP_PRIVATE| MMapFlags::MAP_ANONYOMUS,
             None, 0).expect("mmap failed");

    mprotect(addr + 0x1000, 0x4000, MMapProt::PROT_READ).expect("mprotect failed");
    munmap(addr + 0x2000, 0x1000).expect("munmap failed");
}