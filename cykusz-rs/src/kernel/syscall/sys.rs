use alloc::string::String;
use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::vec::Vec;

use intrusive_collections::UnsafeRef;

use syscall_defs::{ConnectionFlags, FcntlCmd, FileType, SyscallResult};
use syscall_defs::{OpenFlags, SyscallError};

use crate::kernel::fs::dirent::DirEntry;
use crate::kernel::fs::icache::INodeItemStruct;
use crate::kernel::fs::path::Path;
use crate::kernel::fs::{lookup_by_path, lookup_by_real_path, LookupMode};
use crate::kernel::mm::virt::PageFlags;
use crate::kernel::mm::VirtAddr;
use crate::kernel::mm::PAGE_SIZE;
use crate::kernel::net::ip::Ip4;
use crate::kernel::sched::current_task;
use crate::kernel::task::filetable::FileHandle;
use crate::kernel::utils::types::Align;
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
        let inode = crate::kernel::fs::lookup_by_path(Path::new(path), flags.into())?;

        let task = current_task();

        if flags.contains(OpenFlags::DIRECTORY) && inode.inode().ftype()? != FileType::Dir {
            return Err(SyscallError::NotDir);
        }

        if flags.contains(OpenFlags::CREAT) {
            if let Err(e) = inode.inode().truncate() {
                println!("Truncate failed: {:?}", e);
            }
        }

        if let Some(fd) = task.open_file(inode, flags) {
            return Ok(fd);
        } else {
            Err(SyscallError::NoDev)
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

pub fn sys_fcntl(fd: u64, cmd: u64) -> SyscallResult {
    let cmd = FcntlCmd::from(cmd);

    match cmd {
        FcntlCmd::GetFL => {
            let task = current_task();

            if let Some(handle) = task.get_handle(fd as usize) {
                Ok(handle.flags().bits())
            } else {
                Err(SyscallError::BadFD)
            }
        }
        FcntlCmd::Inval => Err(SyscallError::Inval),
    }
}

pub fn sys_mmap(addr: u64, size: u64) -> SyscallResult {
    let size = size.align_up(PAGE_SIZE as u64);
    let addr = addr.align(PAGE_SIZE as u64);

    for a in (addr..addr + size).step_by(PAGE_SIZE) {
        crate::kernel::mm::map_flags(VirtAddr(a as usize), PageFlags::USER | PageFlags::WRITABLE);
    }

    Ok(0)
}

pub fn sys_chdir(path: u64, len: u64) -> SyscallResult {
    if let Ok(path) = core::str::from_utf8(make_buf(path, len)) {
        if let Ok(dentry) = lookup_by_path(Path::new(path), LookupMode::None) {
            let dir = dentry.read().inode.clone();

            if dir.ftype()? == FileType::Dir {
                let task = current_task();
                task.set_cwd(dentry);
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

    if let Some(pwd) = current_task().get_pwd() {
        if pwd.len() > len as usize {
            Err(SyscallError::IO)
        } else {
            buf[..pwd.len()].copy_from_slice(pwd.as_bytes());
            Ok(pwd.len())
        }
    } else {
        Err(SyscallError::Inval)
    }
}

pub fn sys_mkdir(path: u64, len: u64) -> SyscallResult {
    if let Ok(path) = core::str::from_utf8(make_buf(path, len)) {
        let path = Path::new(path);

        let (inode, name) = {
            let (dir, target) = path.containing_dir();

            (lookup_by_path(dir, LookupMode::None)?.inode(), target)
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

pub fn sys_symlink(
    target: u64,
    target_len: u64,
    linkpath: u64,
    linkpath_len: u64,
) -> SyscallResult {
    let target = make_str(target, target_len);
    let path = make_str(linkpath, linkpath_len);

    let path = Path::new(path);

    let (inode, name) = {
        let (dir, target) = path.containing_dir();

        (lookup_by_path(dir, LookupMode::None)?.inode(), target)
    };

    if inode.ftype()? == FileType::Dir {
        inode.symlink(name.str(), target)?;

        Ok(0)
    } else {
        Err(SyscallError::NotDir)
    }
}

pub fn sys_rmdir(path: u64, path_len: u64) -> SyscallResult {
    let path = Path::new(make_str(path, path_len));

    let (_, name) = path.containing_dir();

    let dir = lookup_by_real_path(path, LookupMode::None)?;

    if dir.inode().ftype()? == FileType::Dir {
        dir.inode().rmdir(name.str())?;

        dir.drop_from_cache();

        Ok(0)
    } else {
        return Err(SyscallError::NotDir);
    }
}

pub fn sys_unlink(path: u64, path_len: u64) -> SyscallResult {
    let path = Path::new(make_str(path, path_len));

    let (_, name) = path.containing_dir();

    let file = lookup_by_real_path(path, LookupMode::None)?;

    if let Some(dir) = file.parent() {
        if dir.inode().ftype()? == FileType::Dir && file.inode().ftype()? != FileType::Dir {
            dir.inode().unlink(name.str())?;

            file.drop_from_cache();
        }

        Ok(0)
    } else {
        return Err(SyscallError::Fault);
    }
}

pub fn sys_link(target: u64, target_len: u64, linkpath: u64, linkpath_len: u64) -> SyscallResult {
    let target = make_str(target, target_len);
    let path = make_str(linkpath, linkpath_len);

    let target_entry = lookup_by_real_path(Path::new(target), LookupMode::None)?;

    let path = Path::new(path);

    let (inode, name) = {
        let (dir, name) = path.containing_dir();

        (lookup_by_path(dir, LookupMode::None)?.inode(), name)
    };

    if Weak::as_ptr(&inode.fs()) != Weak::as_ptr(&target_entry.inode().fs()) {
        return Err(SyscallError::Inval);
    }

    if inode.ftype()? == FileType::Dir {
        inode.link(name.str(), target_entry.inode())?;
    } else {
        return Err(SyscallError::NotDir);
    }

    Ok(0)
}

pub fn sys_rename(oldpath: u64, oldpath_len: u64, newpath: u64, newpath_len: u64) -> SyscallResult {
    let old_path = Path::new(make_str(oldpath, oldpath_len));
    let new_path = Path::new(make_str(newpath, newpath_len));

    let old = lookup_by_real_path(old_path, LookupMode::None)?;

    let (new, name) = {
        let (dir, name) = new_path.containing_dir();

        (lookup_by_real_path(dir, LookupMode::None)?, name)
    };

    if new.inode().fs().as_ptr() != old.inode().fs().as_ptr() {
        return Err(SyscallError::Access);
    }

    if new.inode().ftype()? != FileType::Dir {
        return Err(SyscallError::NotDir);
    }

    if old.inode().ftype()? == FileType::Dir {
        // Check whether we are not moving directory to itself
        let mut c = Some(new.clone());

        while let Some(i) = c.clone() {
            if Arc::downgrade(&i).as_ptr() == Arc::downgrade(&old).as_ptr() {
                return Err(SyscallError::Inval);
            }

            c = i.parent();
        }
    }

    new.inode().rename(old.clone(), name.str())?;

    let cache = crate::kernel::fs::dirent::cache();

    cache.rehash(&old, |e| {
        e.update_parent(Some(new));
        e.update_name(String::from(name.str()));
    });

    Ok(0)
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

        use crate::kernel::fs::icache;

        let cache = icache::cache();

        let item = cache.make_item_no_cache(INodeItemStruct::from(socket));

        if let Some(fd) = task.open_file(DirEntry::inode_wrap(item), OpenFlags::RDWR) {
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

        use crate::kernel::fs::icache;

        let cache = icache::cache();

        let item = cache.make_item_no_cache(INodeItemStruct::from(socket));

        if let Some(fd) = task.open_file(DirEntry::inode_wrap(item), OpenFlags::RDWR) {
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
                if let Ok(f) =
                    handle
                        .inode
                        .inode()
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
        let dev = lookup_by_path(Path::new(dev_path), LookupMode::None)?.inode();
        let dest = lookup_by_path(Path::new(dest_path), LookupMode::None)?;

        if let Some(dev) = crate::kernel::block::get_blkdev_by_id(dev.device()?.id()) {
            if let Some(fs) = crate::kernel::fs::ext2::Ext2Filesystem::new(dev) {
                if let Ok(_) = crate::kernel::fs::mount::mount(dest, fs) {
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

    if let Err(e) = crate::kernel::fs::mount::umount(node) {
        Err(e)?
    } else {
        Ok(0)
    }
}

pub fn sys_time() -> SyscallResult {
    Ok(crate::kernel::time::unix_timestamp() as usize)
}

pub fn sys_exit() -> ! {
    crate::kernel::sched::task_finished()
}

pub fn sys_sleep(time_ns: u64) -> SyscallResult {
    let t = current_task();
    t.sleep(time_ns as usize);
    Ok(0)
}

pub fn sys_fork() -> SyscallResult {
    println!(
        "free mem before fork: {}, used: {} heap: {}",
        crate::kernel::mm::free_mem(),
        crate::kernel::mm::used_mem(),
        crate::kernel::mm::heap::heap_mem(),
    );
    //println!("icache stats");
    //crate::kernel::fs::icache::cache().print_stats();
    //println!("dir entry stats");
    //crate::kernel::fs::dirent::cache().print_stats();
    println!("page cache stats");
    crate::kernel::fs::pcache::cache().print_stats();
    crate::kernel::sched::fork();

    Ok(0)
}

pub fn sys_exec(path: u64, path_len: u64) -> SyscallResult {
    let path = Path::new(make_str(path, path_len));

    let prog = lookup_by_path(path, LookupMode::None)?;

    if let Some(fh) = FileHandle::new(0, prog, OpenFlags::RDONLY) {
        if let Ok(exe) = fh.read_all() {
            drop(fh);

            crate::kernel::sched::exec(exe);
        } else {
            Err(SyscallError::Fault)
        }
    } else {
        Err(SyscallError::Inval)
    }
}

pub fn sys_poweroff() -> ! {
    crate::kernel::sched::close_all_tasks();
    crate::kernel::fs::dirent::cache().clear();
    crate::kernel::fs::icache::cache().clear();
    crate::kernel::fs::pcache::cache().clear();
    crate::kernel::fs::mount::umount_all();

    crate::arch::acpi::power_off()
}

pub fn sys_reboot() -> SyscallResult {
    if !crate::arch::acpi::reboot() {
        Err(SyscallError::NoEnt)
    } else {
        Ok(0)
    }
}
