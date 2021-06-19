use alloc::string::String;
use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::vec::Vec;

use intrusive_collections::UnsafeRef;

use syscall_defs::{
    ConnectionFlags, FcntlCmd, FileType, MMapFlags, MMapProt, OpenFD, SyscallResult,
};
use syscall_defs::{OpenFlags, SyscallError};

use crate::kernel::fs::dirent::DirEntry;
use crate::kernel::fs::path::Path;
use crate::kernel::fs::{lookup_by_path, lookup_by_real_path, LookupMode};
use crate::kernel::mm::VirtAddr;
use crate::kernel::net::ip::Ip4;
use crate::kernel::sched::{current_task, current_task_ref};
use crate::kernel::utils::wait_queue::WaitQueue;
use crate::kernel::signal::SignalEntry;
use syscall_defs::signal::SigAction;

//TODO: Check if the pointer from user is actually valid
fn make_buf_mut(b: u64, len: u64) -> &'static mut [u8] {
    unsafe { core::slice::from_raw_parts_mut(b as *mut u8, len as usize) }
}

//TODO: Check if the pointer from user is actually valid
fn make_buf(b: u64, len: u64) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(b as *const u8, len as usize) }
}

fn make_str<'a>(b: u64, len: u64) -> &'a str {
    unsafe { core::str::from_utf8_unchecked(make_buf(b, len)) }
}

pub fn sys_open(at: u64, path: u64, len: u64, mode: u64) -> SyscallResult {
    use core::convert::TryFrom;

    let mut flags =
        syscall_defs::OpenFlags::from_bits(mode as usize).ok_or(SyscallError::EINVAL)?;

    if !flags.intersects(OpenFlags::RDONLY | OpenFlags::RDWR | OpenFlags::WRONLY) {
        flags.insert(OpenFlags::RDONLY);
    }

    let at = OpenFD::try_from(at)?;

    if let OpenFD::Fd(_) = at {
        //logln!("open: at fd currently not supported");
        return Err(SyscallError::EBADFD);
    }

    if let Ok(path) = core::str::from_utf8(make_buf(path, len)) {
        let inode = crate::kernel::fs::lookup_by_path(Path::new(path), flags.into())?;

        let task = current_task_ref();

        if flags.contains(OpenFlags::DIRECTORY) && inode.inode().ftype()? != FileType::Dir {
            return Err(SyscallError::ENOTDIR);
        }

        if flags.contains(OpenFlags::TRUNC) {
            if let Err(e) = inode.inode().truncate(0) {
                println!("Truncate failed: {:?}", e);
            }
        }

        let res = Ok(task.open_file(inode, flags)?);

        logln!("sys_open: {} flags: {:?} = {}", path, flags, res.unwrap());

        //logln!("opened fd: {}", res.unwrap());

        res
    } else {
        Err(SyscallError::EINVAL)
    }
}

pub fn sys_close(fd: u64) -> SyscallResult {
    let task = current_task_ref();

    logln!("sys_close: {} task: {}", fd, task.tid());

    return if task.close_file(fd as usize) {
        Ok(0)
    } else {
        Err(SyscallError::EBADFD)
    };
}

pub fn sys_write(fd: u64, buf: u64, len: u64) -> SyscallResult {
    let fd = fd as usize;

    let task = current_task_ref();
    return if let Some(f) = task.get_handle(fd) {
        if f.flags.intersects(OpenFlags::WRONLY | OpenFlags::RDWR) {
            Ok(f.write(make_buf(buf, len))?)
        } else {
            Err(SyscallError::EACCES)
        }
    } else {
        Err(SyscallError::EBADFD)
    };
}

pub fn sys_read(fd: u64, buf: u64, len: u64) -> SyscallResult {
    let fd = fd as usize;

    let task = current_task_ref();

    logln!("sys_read fd: {} len: {} task: {}", fd, len, task.tid());

    return if let Some(f) = task.get_handle(fd) {
        if f.flags.intersects(OpenFlags::RDONLY | OpenFlags::RDWR) {
            Ok(f.read(make_buf_mut(buf, len))?)
        } else {
            logln!("eaccess");
            Err(SyscallError::EACCES)
        }
    } else {
        logln!("ebadfd");
        Err(SyscallError::EBADFD)
    };
}

pub fn sys_pread(fd: u64, buf: u64, len: u64, offset: u64) -> SyscallResult {
    let fd = fd as usize;

    let task = current_task_ref();

    return if let Some(f) = task.get_handle(fd) {
        if f.flags.intersects(OpenFlags::RDONLY | OpenFlags::RDWR) {
            Ok(f.read_at(make_buf_mut(buf, len), offset as usize)?)
        } else {
            Err(SyscallError::EACCES)
        }
    } else {
        Err(SyscallError::EBADFD)
    };
}

pub fn sys_pwrite(fd: u64, buf: u64, len: u64, offset: u64) -> SyscallResult {
    let fd = fd as usize;

    let task = current_task_ref();
    return if let Some(f) = task.get_handle(fd) {
        if f.flags.intersects(OpenFlags::WRONLY | OpenFlags::RDWR) {
            Ok(f.write_at(make_buf(buf, len), offset as usize)?)
        } else {
            Err(SyscallError::EACCES)
        }
    } else {
        Err(SyscallError::EBADFD)
    };
}

pub fn sys_seek(fd: u64, off: u64, whence: u64) -> SyscallResult {
    let fd = fd as usize;
    let off = off as isize;

    let task = current_task_ref();
    return if let Some(f) = task.get_handle(fd) {
        Ok(f.seek(off, syscall_defs::SeekWhence::from(whence))?)
    } else {
        Err(SyscallError::EBADFD)
    };
}

pub fn sys_access(at: u64, path: u64, path_len: u64, _mode: u64, _flags: u64) -> SyscallResult {
    use core::convert::TryFrom;

    let at = OpenFD::try_from(at)?;

    if let OpenFD::Fd(_) = at {
        logln_disabled!("open: at fd currently not supported");
        return Err(SyscallError::EBADFD);
    }

    let path = Path::new(make_str(path, path_len));

    lookup_by_path(path, LookupMode::None)?;

    Ok(0)
}

pub fn sys_fcntl(fd: u64, cmd: u64) -> SyscallResult {
    let cmd = FcntlCmd::from(cmd);

    match cmd {
        FcntlCmd::GetFL => {
            let task = current_task_ref();

            if let Some(handle) = task.get_handle(fd as usize) {
                Ok(handle.flags().bits())
            } else {
                Err(SyscallError::EBADFD)
            }
        }
        FcntlCmd::Inval => Err(SyscallError::EINVAL),
    }
}

pub fn sys_mmap(addr: u64, len: u64, prot: u64, flags: u64, fd: u64, offset: u64) -> SyscallResult {
    let task = current_task_ref();

    //logln!("mmap: {:#x}", addr);

    let addr = if addr != 0 {
        Some(VirtAddr(addr as usize))
    } else {
        None
    };
    let len = len as usize;
    let prot = if let Some(prot) = MMapProt::from_bits(prot as usize) {
        prot
    } else {
        return Err(SyscallError::EINVAL);
    };
    let flags = if let Some(flags) = MMapFlags::from_bits(flags as usize) {
        flags
    } else {
        return Err(SyscallError::EINVAL);
    };
    let file = if !flags.contains(MMapFlags::MAP_ANONYOMUS) {
        if let Some(file) = task.get_handle(fd as usize) {
            Some(file.inode.clone())
        } else {
            return Err(SyscallError::EINVAL);
        }
    } else {
        None
    };
    let offset = offset as usize;

    if let Some(res) = task.vm().mmap_vm(addr, len, prot, flags, file, offset) {
        Ok(res.0)
    } else {
        Err(SyscallError::EFAULT)
    }
}

pub fn sys_munmap(addr: u64, len: u64) -> SyscallResult {
    let addr = VirtAddr(addr as usize);

    let task = current_task_ref();

    if task.vm().munmap_vm(addr, len as usize) {
        Ok(0)
    } else {
        Err(SyscallError::EFAULT)
    }
}

pub fn sys_maps() -> SyscallResult {
    logln!(
        "free mem before fork: {}, used: {} heap: {}",
        crate::kernel::mm::free_mem(),
        crate::kernel::mm::used_mem(),
        crate::kernel::mm::heap::heap_mem(),
    );

    current_task_ref().vm().log_vm();

    //crate::kernel::fs::dirent::cache().print_stats();
    //crate::kernel::fs::icache::cache().print_stats();
    //crate::kernel::fs::pcache::cache().print_stats();

    Ok(0)
}

pub fn sys_chdir(path: u64, len: u64) -> SyscallResult {
    if let Ok(path) = core::str::from_utf8(make_buf(path, len)) {
        if let Ok(dentry) = lookup_by_path(Path::new(path), LookupMode::None) {
            let dir = dentry.read().inode.clone();

            if dir.ftype()? == FileType::Dir {
                let task = current_task_ref();
                task.set_cwd(dentry);
                return Ok(0);
            } else {
                return Err(SyscallError::ENOTDIR);
            }
        }
    }

    Err(SyscallError::EINVAL)
}

pub fn sys_getcwd(buf: u64, len: u64) -> SyscallResult {
    let buf = make_buf_mut(buf, len);

    if let Some(pwd) = current_task_ref().get_pwd() {
        if pwd.len() > len as usize {
            Err(SyscallError::EIO)
        } else {
            buf[..pwd.len()].copy_from_slice(pwd.as_bytes());
            Ok(pwd.len())
        }
    } else {
        Err(SyscallError::EINVAL)
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
                Err(SyscallError::EEXIST)
            }
        } else {
            Err(SyscallError::ENOTDIR)
        }
    } else {
        Err(SyscallError::EINVAL)
    }
}

pub fn sys_getdents(fd: u64, buf: u64, len: u64) -> SyscallResult {
    let fd = fd as usize;

    let task = current_task_ref();
    return if let Some(f) = task.get_handle(fd) {
        Ok(f.get_dents(make_buf_mut(buf, len))?)
    } else {
        Err(SyscallError::EBADFD)
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
        Err(SyscallError::ENOTDIR)
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
        return Err(SyscallError::ENOTDIR);
    }
}

pub fn sys_unlink(at: u64, path: u64, path_len: u64, flags: u64) -> SyscallResult {
    use core::convert::TryFrom;

    if flags != 0 {
        return Err(SyscallError::EINVAL);
    }

    let at = OpenFD::try_from(at)?;

    if let OpenFD::Fd(_) = at {
        logln_disabled!("open: at fd currently not supported");
        return Err(SyscallError::EBADFD);
    }

    let path = Path::new(make_str(path, path_len));

    logln!("sys_unlink: {}", path.str());

    let (_, name) = path.containing_dir();

    let file = lookup_by_real_path(path, LookupMode::None)?;

    log!("unlink inode: ");
    file.inode().debug();

    if let Some(dir) = file.parent() {
        if dir.inode().ftype()? == FileType::Dir && file.inode().ftype()? != FileType::Dir {
            dir.inode().unlink(name.str())?;

            file.drop_from_cache();
        }

        Ok(0)
    } else {
        return Err(SyscallError::EFAULT);
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

    if Weak::as_ptr(&inode.fs().unwrap()) != Weak::as_ptr(&target_entry.inode().fs().unwrap()) {
        return Err(SyscallError::EINVAL);
    }

    if inode.ftype()? == FileType::Dir {
        inode.link(name.str(), target_entry.inode())?;
    } else {
        return Err(SyscallError::ENOTDIR);
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

    if new.inode().fs().unwrap().as_ptr() != old.inode().fs().unwrap().as_ptr() {
        return Err(SyscallError::EACCES);
    }

    if new.inode().ftype()? != FileType::Dir {
        return Err(SyscallError::ENOTDIR);
    }

    if old.inode().ftype()? == FileType::Dir {
        // Check whether we are not moving directory to itself
        let mut c = Some(new.clone());

        while let Some(i) = c.clone() {
            if Arc::downgrade(&i).as_ptr() == Arc::downgrade(&old).as_ptr() {
                return Err(SyscallError::EINVAL);
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
        let ip = crate::kernel::net::dns::get_ip_by_host(name.as_bytes())?;

        return if blen as usize >= core::mem::size_of::<Ip4>() {
            let buf = make_buf_mut(buf, blen);

            buf.copy_from_slice(&ip.v);

            Ok(core::mem::size_of::<Ip4>())
        } else {
            Err(SyscallError::EFAULT)
        };
    }

    Err(SyscallError::EINVAL)
}

pub fn sys_bind(port: u64, flags: u64) -> SyscallResult {
    let flags =
        syscall_defs::ConnectionFlags::from_bits(flags as usize).ok_or(SyscallError::EINVAL)?;

    let socket = if flags.contains(ConnectionFlags::UDP) {
        crate::kernel::net::socket::udp_bind(port as u32)
    } else {
        crate::kernel::net::socket::tcp_bind(port as u32)
    };

    if let Some(socket) = socket {
        let task = current_task_ref();

        Ok(task.open_file(DirEntry::inode_wrap(socket), OpenFlags::RDWR)?)
    } else {
        Err(SyscallError::EBUSY)
    }
}

pub fn sys_connect(host: u64, host_len: u64, port: u64, flags: u64) -> SyscallResult {
    let flags =
        syscall_defs::ConnectionFlags::from_bits(flags as usize).ok_or(SyscallError::EINVAL)?;

    let host = Ip4::new(make_buf(host, host_len));

    let socket = if flags.contains(ConnectionFlags::UDP) {
        crate::kernel::net::socket::udp_connect(host, port as u32)
    } else {
        crate::kernel::net::socket::tcp_connect(host, port as u32)
    };

    if let Some(socket) = socket {
        let task = current_task_ref();

        Ok(task.open_file(DirEntry::inode_wrap(socket), OpenFlags::RDWR)?)
    } else {
        Err(SyscallError::EBUSY)
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

    let task = current_task_ref();

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
            task.await_io()?;
        }

        first = false;
    }

    if let Some(fd) = fd_found {
        Ok(fd)
    } else {
        Err(SyscallError::EFAULT)
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
                    Err(SyscallError::EFAULT)
                }
            } else {
                Err(SyscallError::EINVAL)
            }
        } else {
            Err(SyscallError::ENODEV)
        }
    } else {
        Err(SyscallError::EINVAL)
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

pub fn sys_exit(status: u64) -> ! {
    crate::kernel::sched::exit(status as isize)
}

pub fn sys_sleep(time_ns: u64) -> SyscallResult {
    let t = current_task_ref();
    t.sleep(time_ns as usize)?;
    Ok(0)
}

pub fn sys_fork() -> SyscallResult {
    //println!("icache stats");
    //crate::kernel::fs::icache::cache().print_stats();
    //println!("dir entry stats");
    //crate::kernel::fs::dirent::cache().print_stats();
    //println!("page cache stats");
    //crate::kernel::fs::pcache::cache().print_stats();
    let child = crate::kernel::sched::fork();

    Ok(child.tid())
}

pub fn sys_exec(
    path: u64,
    path_len: u64,
    args: u64,
    args_len: u64,
    envs: u64,
    envs_len: u64,
) -> SyscallResult {
    let path = make_str(path, path_len);

    logln!("sys_exec: {}", path);

    let prog = lookup_by_path(Path::new(path), LookupMode::None)?;

    prog.inode().debug();

    let args = if args_len > 0 {
        Some(syscall_defs::exec::from_syscall_slice(
            args as usize,
            args_len as usize,
        ))
    } else {
        None
    };

    let envs = if envs_len > 0 {
        Some(syscall_defs::exec::from_syscall_slice(
            envs as usize,
            envs_len as usize,
        ))
    } else {
        None
    };

    crate::kernel::sched::exec(prog, args, envs);
}

pub fn sys_spawn_thread(entry: u64, stack: u64) -> SyscallResult {
    let thread =
        crate::kernel::sched::spawn_thread(VirtAddr(entry as usize), VirtAddr(stack as usize));

    Ok(thread.tid())
}

pub fn sys_waitpid(pid: u64, status: u64, _flags: u64) -> SyscallResult {
    let current = current_task_ref();

    //logln!("sys_waitpid: {} status addr: {:#x}", pid, status);

    let status = unsafe { VirtAddr(status as usize).read_mut::<u32>() };

    //logln!("sys_waitpid");

    Ok(current.wait_pid(pid as usize, status)?)
}

pub fn sys_getpid() -> SyscallResult {
    Ok(current_task_ref().pid())
}

pub fn sys_gettid() -> SyscallResult {
    Ok(current_task_ref().tid())
}

pub fn sys_setsid() -> SyscallResult {
    let task = current_task();

    crate::kernel::session::sessions().set_sid(task.clone())?;

    task.terminal().disconnect(None);

    Ok(0)
}

pub fn sys_setpgid(pid: u64, pgid: u64) -> SyscallResult {
    crate::kernel::session::sessions().set_pgid(pid as usize, pgid as usize)
}

pub fn sys_exit_thread() -> ! {
    crate::kernel::sched::exit_thread();
}

pub fn sys_ioctl(fd: u64, cmd: u64, arg: u64) -> SyscallResult {
    let current = current_task_ref();

    if let Some(handle) = current.get_handle(fd as usize) {
        Ok(handle.inode.inode().ioctl(cmd as usize, arg as usize)?)
    } else {
        Err(SyscallError::EBADFD)
    }
}

pub fn sys_sigaction(
    sig: u64,
    sigact: u64,
    sigreturn: u64,
    old: u64,
) -> SyscallResult {
    if sig == 34 {
        //temporary hack to make mlibc happy
        return Err(SyscallError::ENOSYS);
    }

    let new = if sigact == 0 {
        None
    } else {
        unsafe {
            Some(VirtAddr(sigact as usize).read_ref::<SigAction>())
        }
    };

    logln!("sigaction: {:#x} {:?} size: {}", sigact, new, core::mem::size_of::<SigAction>());

    let entry = if let Some(new) = new {
        Some(SignalEntry::from_sigaction(*new, sigreturn as usize)?)
    } else {
        None
    };

    let old = if old == 0 {
        None
    } else {
        unsafe {
            Some(VirtAddr(old as usize).read_mut::<SigAction>())
        }
    };

    logln!("sigaction: {} {:?}, old: {:?}", sig, entry, old);

    current_task_ref().signals().set_signal(sig as usize, entry, old);

    Ok(0)
}

pub fn sys_sigprocmask(how: u64, set: u64, old_set: u64) -> SyscallResult {
    let how = syscall_defs::signal::SigProcMask::from(how);
    let set = unsafe { VirtAddr(set as usize).read::<u64>() };
    let old_set = if old_set > 0 {
        Some(unsafe { VirtAddr(old_set as usize).read_mut::<u64>() })
    } else {
        None
    };
    current_task_ref().signals().set_mask(how, set, old_set);
    Ok(0)
}

pub fn sys_kill(pid: u64, sig: u64) -> SyscallResult {
    logln!("sys_kill: pid: {}, sig: {}", pid as isize, sig);
    match pid as isize {
        a if a > 0 => {
            let task = crate::kernel::sched::get_task(a as usize).ok_or(SyscallError::ESRCH)?;

            task.signal(sig as usize);

            Ok(0)
        }
        0 => {
            let task = current_task_ref();

            Ok(crate::kernel::session::sessions()
                .get_group(task.sid(), task.gid())
                .and_then(|g| {
                    g.signal(sig as usize);

                    Some(0)
                })
                .ok_or(SyscallError::ESRCH)?)
        }
        -1 => {
            panic!("kill -1 not supported")
        }
        a if a < -1 => {
            let task = crate::kernel::sched::get_task(-a as usize).ok_or(SyscallError::ESRCH)?;

            Ok(crate::kernel::session::sessions()
                .get_group(task.sid(), task.gid())
                .and_then(|g| {
                    g.signal(sig as usize);

                    Some(0)
                })
                .ok_or(SyscallError::ESRCH)?)
        }
        _ => {
            unreachable!()
        }
    }
}

pub fn sys_futex_wait(uaddr: u64, expected: u64) -> SyscallResult {
    let uaddr = VirtAddr(uaddr as usize);
    let expected = expected as u32;

    //println!("[ FUTEX ] wait {} {} {}", uaddr, expected, crate::kernel::int::is_enabled());

    crate::kernel::futex::futex().wait(uaddr, expected)
}

pub fn sys_pipe(fds: u64, flags: u64) -> SyscallResult {
    let fds = unsafe { core::slice::from_raw_parts_mut(fds as *mut u64, 2) };

    let pipe = crate::kernel::fs::pipe::Pipe::new();

    let entry = DirEntry::inode_wrap(pipe);

    let task = current_task_ref();

    let flags = OpenFlags::from_bits(flags as usize).ok_or(SyscallError::EINVAL)?;

    let f1 = OpenFlags::RDONLY | (flags & OpenFlags::CLOEXEC);
    let f2 = OpenFlags::WRONLY | (flags & OpenFlags::CLOEXEC);

    if let Ok(fd1) = task.open_file(entry.clone(), f1) {
        if let Ok(fd2) = task.open_file(entry, f2) {
            fds[0] = fd1 as u64;
            fds[1] = fd2 as u64;

            return Ok(0);
        } else {
            task.close_file(fd1);
        }
    }

    Err(SyscallError::EINVAL)
}

pub fn sys_dup(fd: u64, flags: u64) -> SyscallResult {
    let task = current_task_ref();

    let flags =
        OpenFlags::from_bits(flags as usize).ok_or(SyscallError::EINVAL)? & OpenFlags::CLOEXEC;

    task.filetable().duplicate(fd as usize, flags)
}

pub fn sys_dup2(fd: u64, new_fd: u64, flags: u64) -> SyscallResult {
    let task = current_task_ref();

    let flags =
        OpenFlags::from_bits(flags as usize).ok_or(SyscallError::EINVAL)? & OpenFlags::CLOEXEC;

    task.filetable()
        .duplicate_at(fd as usize, new_fd as usize, flags)
}

pub fn sys_futex_wake(uaddr: u64) -> SyscallResult {
    let uaddr = VirtAddr(uaddr as usize);

    //println!("[ FUTEX ] wake {} {}", uaddr, unsafe {
    //    uaddr.read_volatile::<u32>()
    //});

    crate::kernel::futex::futex().wake(uaddr)
}

pub fn sys_stat(path: u64, path_len: u64, stat: u64) -> SyscallResult {
    let _str = make_str(path, path_len);
    let path = Path::new(make_str(path, path_len));

    let stat = unsafe { VirtAddr(stat as usize).read_mut::<syscall_defs::stat::Stat>() };

    let inode = lookup_by_path(path, LookupMode::None)?;

    *stat = inode.inode().stat()?;

    //logln!("stat {}, {:?}", str, stat);

    Ok(0)
}

pub fn sys_fstat(fd: u64, stat: u64) -> SyscallResult {
    let file = current_task_ref()
        .filetable()
        .get_handle(fd as usize)
        .ok_or(SyscallError::EBADFD)?;

    let stat = unsafe { VirtAddr(stat as usize).read_mut::<syscall_defs::stat::Stat>() };

    *stat = file.inode.inode().stat()?;

    //logln!("fstat {}, {:?}", fd, stat);

    Ok(0)
}

pub fn sys_getrlimit(resource: u64, rlimit: u64) -> SyscallResult {
    use core::convert::TryFrom;

    let resource = syscall_defs::resource::RLimitKind::try_from(resource)?;

    let out = unsafe { VirtAddr(rlimit as usize).read_mut::<syscall_defs::resource::RLimit>() };

    match resource {
        syscall_defs::resource::RLimitKind::NOFile => {
            out.cur = 256;
            out.max = 256;
        }
        _ => {
            out.cur = u64::MAX;
            out.cur = u64::MAX;
        }
    }

    Ok(0)
}

pub fn sys_debug(str: u64, str_len: u64) -> SyscallResult {
    logln!("{}", make_str(str, str_len));

    Ok(0)
}

pub fn sys_sync() -> SyscallResult {
    crate::kernel::fs::mount::sync_all();
    crate::kernel::block::sync_all();

    Ok(0)
}

pub fn sys_poweroff() -> ! {
    crate::kernel::sched::close_all_tasks();
    println!("[ SHUTDOWN ] Closed all tasks");
    crate::kernel::fs::dirent::cache().clear();
    println!("[ SHUTDOWN ] Cleared dir cache");
    crate::kernel::fs::icache::cache().clear();
    println!("[ SHUTDOWN ] Cleared inode cache");
    crate::kernel::fs::mount::umount_all();
    println!("[ SHUTDOWN ] Unmounted fs");

    crate::arch::acpi::power_off()
}

pub fn sys_reboot() -> SyscallResult {
    if !crate::arch::acpi::reboot() {
        Err(SyscallError::ENOENT)
    } else {
        Ok(0)
    }
}
