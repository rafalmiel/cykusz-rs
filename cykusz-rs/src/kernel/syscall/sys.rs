use crate::kernel::sched::current_task;

fn make_buf_mut(b: u64, len: u64) -> &'static mut [u8] {
    unsafe { core::slice::from_raw_parts_mut(b as *mut u8, len as usize) }
}

fn make_buf(b: u64, len: u64) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(b as *const u8, len as usize) }
}

pub fn sys_open(path: u64, len: u64, mode: u64) -> u64 {
    if let Ok(path) = core::str::from_utf8(make_buf(path, len)) {
        if let Ok(inode) = crate::kernel::fs::lookup_by_path(path) {
            let task = current_task();

            if mode == 1 {
                if let Err(e) = inode.truncate() {
                    println!("Truncate failed: {:?}", e);
                }
            }

            if let Some(fd) = task.open_file(inode) {
                return fd as u64;
            }
        } else {
            println!("Failed lookup_by_path");
        }
    }

    return 0;
}

pub fn sys_close(fd: u64) -> u64 {
    let task = current_task();

    if task.close_file(fd as usize) {
        return 0;
    } else {
        return 666;
    }
}

pub fn sys_write(fd: u64, buf: u64, len: u64) -> u64 {
    let fd = fd as usize;

    let task = current_task();
    let n = if let Some(f) = task.get_handle(fd) {
        f.write(make_buf(buf, len)).unwrap()
    } else {
        0
    };

    return n as u64;
}

pub fn sys_read(fd: u64, buf: u64, len: u64) -> u64 {
    let fd = fd as usize;

    let task = current_task();
    if let Some(f) = task.get_handle(fd) {
        return f.read(make_buf_mut(buf, len)).unwrap() as u64;
    }

    return 0;
}
