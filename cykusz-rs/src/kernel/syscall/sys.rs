use crate::kernel::sched::current_task;
use syscall_defs::SyscallError;
use syscall_defs::SyscallResult;

fn make_buf_mut(b: u64, len: u64) -> &'static mut [u8] {
    unsafe { core::slice::from_raw_parts_mut(b as *mut u8, len as usize) }
}

fn make_buf(b: u64, len: u64) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(b as *const u8, len as usize) }
}

pub fn sys_open(path: u64, len: u64, mode: u64) -> SyscallResult {
    if let Ok(path) = core::str::from_utf8(make_buf(path, len)) {
        if let Ok(inode) = crate::kernel::fs::lookup_by_path(path) {
            let task = current_task();

            if mode == 1 {
                if let Err(e) = inode.truncate() {
                    println!("Truncate failed: {:?}", e);
                }
            }

            if let Some(fd) = task.open_file(inode) {
                return Ok(fd);
            } else {
                Err(SyscallError::NoDev)
            }
        } else {
            println!("Failed lookup_by_path");
            Err(SyscallError::NoEnt)
        }
    } else {
        Err(SyscallError::Inval)
    }
}

pub fn sys_close(fd: u64) -> SyscallResult {
    let task = current_task();

    if task.close_file(fd as usize) {
        return Ok(0);
    } else {
        return Err(SyscallError::BadFD);
    }
}

pub fn sys_write(fd: u64, buf: u64, len: u64) -> SyscallResult {
    let fd = fd as usize;

    let task = current_task();
    return if let Some(f) = task.get_handle(fd) {
        Ok(f.write(make_buf(buf, len))?)
    } else {
        Err(SyscallError::BadFD)
    };
}

pub fn sys_read(fd: u64, buf: u64, len: u64) -> SyscallResult {
    let fd = fd as usize;

    let task = current_task();
    if let Some(f) = task.get_handle(fd) {
        return Ok(f.read(make_buf_mut(buf, len))?);
    }

    return Err(SyscallError::BadFD);
}
