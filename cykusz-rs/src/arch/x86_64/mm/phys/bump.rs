use crate::kernel::mm::VirtAddr;
use crate::kernel::sync::Spin;

const CAPACITY: usize = 4096 * 128;

#[repr(align(4096))]
struct Mem {
    mem: [u8; CAPACITY],
    cur: usize,
}

static MEM: Spin<Mem> = Spin::new(Mem {
    mem: [0u8; CAPACITY],
    cur: 0,
});

pub fn alloc(size: usize) -> VirtAddr {
    let mut mem = MEM.lock();

    if mem.cur + size > CAPACITY {
        panic!("Bump alloc out of mem!");
    }

    let ret = VirtAddr(unsafe { mem.mem.as_ptr().offset(mem.cur as isize) as usize });

    mem.cur += size;

    ret
}
