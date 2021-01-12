use alloc::vec::Vec;

use intrusive_collections::UnsafeRef;

use syscall_defs::{ConnectionFlags, FileType, SyscallResult};
use syscall_defs::{OpenFlags, SyscallError};

use crate::kernel::fs::path::Path;
use crate::kernel::fs::{lookup_by_path, LookupMode};
use crate::kernel::net::ip::Ip4;
use crate::kernel::sched::current_task;
use crate::kernel::utils::wait_queue::WaitQueue;

//TODO: Check if the pointer from user is actually valid
fn make_buf_mut(b: u64, len: u64) -> &'static mut [u8] {
    unsafe { core::slice::from_raw_parts_mut(b as *mut u8, len as usize) }
}

//TODO: Check if the pointer from user is actually valid
fn make_buf(b: u64, len: u64) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(b as *const u8, len as usize) }
}

fn make_str<'a>(b: u64, len: u64) -> &'a str {
    core::str::from_utf8(make_buf(b, len)).expect("Invalid str")
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
        if f.flags.intersects(OpenFlags::WRONLY | OpenFlags::RDWR) {
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
        if f.flags.intersects(OpenFlags::RDONLY | OpenFlags::RDWR) {
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

    if pwd.0.len() > len as usize {
        Err(SyscallError::IO)
    } else {
        buf[..pwd.0.len()].copy_from_slice(pwd.0.as_bytes());
        Ok(pwd.0.len())
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
        Ok(f.get_dents(make_buf_mut(buf, len))?)
    } else {
        Err(SyscallError::BadFD)
    };
}

pub fn sys_getaddrinfo(name: u64, nlen: u64, buf: u64, blen: u64) -> SyscallResult {
    if let Ok(name) = core::str::from_utf8(make_buf(name, nlen)) {
        let res = crate::kernel::net::dns::get_ip_by_host(name.as_bytes());

        if let Some(ip) = res {
            return if blen as usize >= core::mem::size_of::<Ip4>() {
                let buf = make_buf_mut(buf, blen);

                buf.copy_from_slice(&ip.v);

                Ok(core::mem::size_of::<Ip4>())
            } else {
                Err(SyscallError::Fault)
            };
        }
    }

    Err(SyscallError::Inval)
}

pub fn sys_bind(port: u64, flags: u64) -> SyscallResult {
    let flags =
        syscall_defs::ConnectionFlags::from_bits(flags as usize).ok_or(SyscallError::Inval)?;

    let socket = if flags.contains(ConnectionFlags::UDP) {
        crate::kernel::net::socket::udp_bind(port as u32)
    } else {
        crate::kernel::net::socket::tcp_bind(port as u32)
    };

    if let Some(socket) = socket {
        let task = current_task();

        if let Some(fd) = task.open_file(socket, OpenFlags::RDWR) {
            Ok(fd)
        } else {
            Err(SyscallError::Fault)
        }
    } else {
        Err(SyscallError::Busy)
    }
}

pub fn sys_connect(host: u64, host_len: u64, port: u64, flags: u64) -> SyscallResult {
    let flags =
        syscall_defs::ConnectionFlags::from_bits(flags as usize).ok_or(SyscallError::Inval)?;

    let host = Ip4::new(make_buf(host, host_len));

    let socket = if flags.contains(ConnectionFlags::UDP) {
        crate::kernel::net::socket::udp_connect(host, port as u32)
    } else {
        crate::kernel::net::socket::tcp_connect(host, port as u32)
    };

    if let Some(socket) = socket {
        let task = current_task();

        if let Some(fd) = task.open_file(socket, OpenFlags::RDWR) {
            Ok(fd)
        } else {
            Err(SyscallError::Fault)
        }
    } else {
        Err(SyscallError::Busy)
    }
}

pub struct PollTable {
    queues: Vec<UnsafeRef<WaitQueue>>,
}

impl PollTable {
    pub fn listen(&mut self, queue: &WaitQueue) {
        queue.add_task(current_task());
        self.queues
            .push(unsafe { UnsafeRef::from_raw(queue as *const _) });
    }
}

impl Drop for PollTable {
    fn drop(&mut self) {
        let task = current_task();
        for q in &self.queues {
            q.remove_task(task.clone());
        }
    }
}

pub fn sys_select(fds: u64, fds_len: u64) -> SyscallResult {
    let buf = make_buf(fds, fds_len);

    let task = current_task();

    let mut fd_found: Option<usize> = None;
    let mut first = true;
    let mut poll_table = PollTable {
        queues: Vec::with_capacity(fds_len as usize),
    };

    'search: loop {
        for fd in buf {
            if let Some(handle) = task.get_handle(*fd as usize) {
                if let Ok(f) = handle
                    .inode
                    .poll(if first { Some(&mut poll_table) } else { None })
                {
                    if f {
                        fd_found = Some(*fd as usize);
                        break 'search;
                    }
                } else {
                    break 'search;
                }
            } else {
                println!("fd {} not found", fd);
            }
        }

        if fd_found.is_none() {
            task.await_io();
        }

        first = false;
    }

    if let Some(fd) = fd_found {
        Ok(fd)
    } else {
        Err(SyscallError::Fault)
    }
}

pub fn sys_mount(
    src: u64,
    src_len: u64,
    dest: u64,
    dest_len: u64,
    fs: u64,
    fs_len: u64,
) -> SyscallResult {
    let dev_path = make_str(src, src_len);
    let dest_path = make_str(dest, dest_len);
    let fs = make_str(fs, fs_len);

    if fs == "ext2" {
        let dev = lookup_by_path(Path::new(dev_path), LookupMode::None)?;
        let dest = lookup_by_path(Path::new(dest_path), LookupMode::None)?;

        if let Some(dev) = crate::kernel::block::get_blkdev_by_id(dev.device()?.id()) {
            if let Some(fs) = crate::kernel::fs::ext2::Ext2Filesystem::new(dev) {
                if let Ok(_) = dest.mount(fs) {
                    Ok(0)
                } else {
                    Err(SyscallError::Fault)
                }
            } else {
                Err(SyscallError::Inval)
            }
        } else {
            Err(SyscallError::NoDev)
        }
    } else {
        Err(SyscallError::Inval)
    }
}

pub fn sys_umount(path: u64, path_len: u64) -> SyscallResult {
    let path = make_str(path, path_len);

    let node = lookup_by_path(Path::new(path), LookupMode::None)?;

    if let Err(e) = node.umount() {
        Err(e)?
    } else {
        Ok(0)
    }
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
