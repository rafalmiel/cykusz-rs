use syscall_defs::{FileType, SyscallResult};
use syscall_defs::{OpenFlags, SyscallError};

use crate::kernel::fs::path::Path;
use crate::kernel::fs::{lookup_by_path, LookupMode};
use crate::kernel::sched::current_task;

fn make_buf_mut(b: u64, len: u64) -> &'static mut [u8] {
    unsafe { core::slice::from_raw_parts_mut(b as *mut u8, len as usize) }
}

fn make_buf(b: u64, len: u64) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(b as *const u8, len as usize) }
}

pub fn sys_open(path: u64, len: u64, mode: u64) -> SyscallResult {
    let flags = syscall_defs::OpenFlags::from_bits(mode as usize).ok_or(SyscallError::Inval)?;

    if let Ok(path) = core::str::from_utf8(make_buf(path, len)) {
        if let Ok(inode) = crate::kernel::fs::lookup_by_path(Path::new(path), flags.into()) {
            let task = current_task();

            if flags.contains(OpenFlags::DIRECTORY) && inode.ftype()? != FileType::Dir {
                return Err(SyscallError::NotDir);
            }

            if flags.contains(OpenFlags::CREAT) {
                if let Err(e) = inode.truncate() {
                    println!("Truncate failed: {:?}", e);
                }
            }

            if let Some(fd) = task.open_file(inode, flags) {
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
        if f.flags.contains(OpenFlags::WRONLY) || f.flags.contains(OpenFlags::RDWR) {
            Ok(f.write(make_buf(buf, len))?)
        } else {
            Err(SyscallError::Access)
        }
    } else {
        Err(SyscallError::BadFD)
    };
}

pub fn sys_read(fd: u64, buf: u64, len: u64) -> SyscallResult {
    let fd = fd as usize;

    let task = current_task();
    return if let Some(f) = task.get_handle(fd) {
        if f.flags.contains(OpenFlags::RDONLY) || f.flags.contains(OpenFlags::RDWR) {
            Ok(f.read(make_buf_mut(buf, len))?)
        } else {
            Err(SyscallError::Access)
        }
    } else {
        Err(SyscallError::BadFD)
    };
}

pub fn sys_chdir(path: u64, len: u64) -> SyscallResult {
    if let Ok(path) = core::str::from_utf8(make_buf(path, len)) {
        if let Ok(dir) = lookup_by_path(Path::new(path), LookupMode::None) {
            if dir.ftype()? == FileType::Dir {
                let task = current_task();
                task.set_cwd(dir, path);
                return Ok(0);
            } else {
                return Err(SyscallError::NotDir);
            }
        }
    }

    Err(SyscallError::Inval)
}

pub fn sys_getcwd(buf: u64, len: u64) -> SyscallResult {
    let buf = make_buf_mut(buf, len);

    let pwd = current_task().get_pwd();

    if pwd.len() > len as usize {
        Err(SyscallError::IO)
    } else {
        buf[..pwd.len()].copy_from_slice(pwd.as_bytes());
        Ok(pwd.len())
    }
}

pub fn sys_mkdir(path: u64, len: u64) -> SyscallResult {
    if let Ok(path) = core::str::from_utf8(make_buf(path, len)) {
        let path = Path::new(path);

        let (inode, name) = {
            let (dir, target) = path.containing_dir();

            (lookup_by_path(dir, LookupMode::None)?, target)
        };

        if inode.ftype()? == FileType::Dir {
            if !["", ".", ".."].contains(&name.str()) {
                inode.mkdir(name.str())?;
                Ok(0)
            } else {
                Err(SyscallError::Exists)
            }
        } else {
            Err(SyscallError::NotDir)
        }
    } else {
        Err(SyscallError::Inval)
    }
}

pub fn sys_getdents(fd: u64, buf: u64, len: u64) -> SyscallResult {
    let fd = fd as usize;

    let task = current_task();
    return if let Some(f) = task.get_handle(fd) {
        Ok(f.getdents(make_buf_mut(buf, len))?)
    } else {
        Err(SyscallError::BadFD)
    };
}

pub fn sys_exit() -> ! {
    crate::task_test::start();
    crate::kernel::sched::task_finished()
}

pub fn sys_sleep(time_ns: u64) -> SyscallResult {
    let t = current_task();
    t.sleep(time_ns as usize);
    Ok(0)
}

pub fn sys_poweroff() -> ! {
    crate::arch::acpi::power_off()
}

pub fn sys_reboot() -> SyscallResult {
    if !crate::arch::acpi::reboot() {
        Err(SyscallError::NoEnt)
    } else {
        Ok(0)
    }
}
