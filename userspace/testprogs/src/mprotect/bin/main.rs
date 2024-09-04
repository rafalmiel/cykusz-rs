use syscall_defs::{MMapFlags, MMapProt};
use syscall_user::{fork, mmap, mprotect, munmap, sleep};

fn main() {
    let addr =
        mmap(Some(0x400000 - 0x1000), 0x6000,
             MMapProt::PROT_READ | MMapProt::PROT_WRITE | MMapProt::PROT_EXEC,
             MMapFlags::MAP_PRIVATE| MMapFlags::MAP_ANONYOMUS,
             None, 0).expect("mmap failed");

    mprotect(addr + 0x1000, 0x4000, MMapProt::PROT_READ).expect("mprotect failed");
    munmap(addr + 0x2000, 0x1000).expect("munmap failed");

    let addr2 =
        mmap(None, 0x2000,
             MMapProt::PROT_READ | MMapProt::PROT_WRITE,
             MMapFlags::MAP_SHARED | MMapFlags::MAP_ANONYOMUS, None, 0)
            .expect("MMap shared anon failed");

    let pid = fork().expect("Fork failed");
    let mut last_val = u64::MAX;
    let mut val = u64::MAX;

    println!("fork: {}", pid);

    if pid == 0 {
        while val != 10 {
            val = unsafe {
                (addr2 as *const u64).read_volatile()
            };
            println!("pid 0 read val: {val} {last_val}");

            if val != last_val {
                println!("val: {val}");
                last_val = val;
            }
            sleep(500).expect("sleep failed");
        }
    } else {
        println!("child pid {}", pid);
        while val != 10 {
            val += 1;
            println!("pid {pid} write {val}");
            unsafe {
                (addr2 as *mut u64).write_volatile(val);
            }
            sleep(1000).expect("sleep failed");
        }
    }

    println!("finished {pid}");

}