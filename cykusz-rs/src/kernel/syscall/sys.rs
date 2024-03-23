use alloc::string::String;
use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::vec::Vec;

use syscall_defs::net::{MsgFlags, MsgHdr, SockAddr, SockDomain, SockOption, SockTypeFlags};
use syscall_defs::poll::{FdSet, PollEventFlags};
use syscall_defs::signal::SigAction;
use syscall_defs::stat::Mode;
use syscall_defs::time::Timespec;
use syscall_defs::{
    AtFlags, FDFlags, FcntlCmd, FileType, MMapFlags, MMapProt, OpenFD, SyscallResult,
};
use syscall_defs::{OpenFlags, SyscallError};

use crate::kernel::fs::dirent::{DirEntry, DirEntryItem};
use crate::kernel::fs::path::Path;
use crate::kernel::fs::poll::PollTable;
use crate::kernel::fs::{lookup_by_path, lookup_by_path_at, lookup_by_real_path, LookupMode};
use crate::kernel::mm::VirtAddr;
use crate::kernel::net::ip::Ip4;
use crate::kernel::net::socket::SocketService;
use crate::kernel::sched::{current_task, current_task_ref, SleepFlags};
use crate::kernel::signal::SignalEntry;
use crate::kernel::utils::types::Prefault;

//TODO: Check if the pointer from user is actually valid
fn make_buf_mut(b: u64, len: u64) -> &'static mut [u8] {
    let buf = unsafe { core::slice::from_raw_parts_mut(b as *mut u8, len as usize) };

    // Prefault userspace buffers, otherwise page fault might happen while we are holding a lock
    buf.prefault();

    buf
}

//TODO: Check if the pointer from user is actually valid
fn make_buf(b: u64, len: u64) -> &'static [u8] {
    let buf = unsafe { core::slice::from_raw_parts(b as *const u8, len as usize) };

    // Prefault userspace buffers, otherwise page fault might happen while we are holding a lock
    buf.prefault();

    buf
}

fn make_str<'a>(b: u64, len: u64) -> &'a str {
    unsafe { core::str::from_utf8_unchecked(make_buf(b, len)) }
}

fn make_path<'a>(path: u64, path_len: u64) -> Option<Path<'a>> {
    if path != 0 && path_len != 0 {
        Some(Path::new(make_str(path, path_len)))
    } else {
        None
    }
}

fn get_dir_entry(
    fd: OpenFD,
    path: Option<Path>,
    lookup_mode: LookupMode,
    get_symlink_entry: bool,
) -> Result<DirEntryItem, SyscallError> {
    logln4!(
        "get dir entry: {:?} {:?} {:?} get_symlink_entry: {}",
        fd,
        path,
        lookup_mode,
        get_symlink_entry
    );

    let task = current_task_ref();

    let file_dir = match fd {
        OpenFD::Fd(fd) => task
            .get_handle(fd)
            .ok_or(SyscallError::EBADFD)?
            .inode
            .clone(),
        OpenFD::Cwd => task.get_dent().ok_or(SyscallError::EBADFD)?.clone(),
        OpenFD::None => {
            return Err(SyscallError::EINVAL);
        }
    };

    if let Some(path) = path {
        Ok(lookup_by_path_at(
            file_dir,
            &path,
            lookup_mode,
            get_symlink_entry,
        )?)
    } else {
        Ok(file_dir.clone())
    }
}

pub fn sys_open(at: u64, path: u64, len: u64, mode: u64) -> SyscallResult {
    logln5!("sys_open {} {} {:x}", at, make_str(path, len), mode);
    let mut flags = OpenFlags::from_bits(mode as usize).ok_or(SyscallError::EINVAL)?;
    if !flags.intersects(OpenFlags::RDONLY | OpenFlags::RDWR | OpenFlags::WRONLY) {
        flags.insert(OpenFlags::RDONLY);
    }

    let at = OpenFD::try_from(at)?;

    let inode = get_dir_entry(
        at,
        make_path(path, len),
        if flags.contains(OpenFlags::CREAT) {
            LookupMode::Create
        } else {
            LookupMode::None
        },
        false,
    )?;

    let task = current_task_ref();

    if flags.contains(OpenFlags::DIRECTORY) && inode.inode().ftype()? != FileType::Dir {
        return Err(SyscallError::ENOTDIR);
    }

    if flags.contains(OpenFlags::TRUNC) {
        if let Err(e) = inode.inode().truncate(0) {
            println!("Truncate failed: {:?}", e);
        }
    }

    let res = Ok(task.open_file(inode.clone(), flags)?);

    logln5!(
        "sys_open: {} flags: {:?} = {}",
        inode.full_path(),
        flags,
        res.unwrap()
    );
    task.filetable().debug();

    res
}

pub fn sys_close(fd: u64) -> SyscallResult {
    let task = current_task_ref();

    logln5!("sys_close: {} task: {}", fd, task.tid());

    return if task.close_file(fd as usize) {
        task.filetable().debug();
        Ok(0)
    } else {
        Err(SyscallError::EBADFD)
    };
}

pub fn sys_write(fd: u64, buf: u64, len: u64) -> SyscallResult {
    let fd = fd as usize;

    let task = current_task_ref();

    return if let Some(f) = task.get_handle(fd) {
        if f.flags().intersects(OpenFlags::WRONLY | OpenFlags::RDWR) {
            Ok(f.write(make_buf(buf, len))?)
        } else {
            logln4!("write fd {} = EACCESS", fd);
            Err(SyscallError::EACCES)
        }
    } else {
        logln4!("write fd {} = EBADFD", fd);
        Err(SyscallError::EBADFD)
    };
}

pub fn sys_read(fd: u64, buf: u64, len: u64) -> SyscallResult {
    let fd = fd as usize;

    let task = current_task_ref();

    logln4!("sys_read fd: {} len: {} task: {}", fd, len, task.tid());

    return if let Some(f) = task.get_handle(fd) {
        if f.flags().intersects(OpenFlags::RDONLY | OpenFlags::RDWR) {
            Ok(f.read(make_buf_mut(buf, len))?)
        } else {
            logln2!("eaccess");
            Err(SyscallError::EACCES)
        }
    } else {
        logln2!("ebadfd");
        Err(SyscallError::EBADFD)
    };
}

pub fn sys_readlink(
    at: u64,
    path: u64,
    path_len: u64,
    buf: u64,
    max_size: u64,
    len: u64,
) -> SyscallResult {
    let inode = get_dir_entry(
        OpenFD::try_from(at)?,
        make_path(path, path_len),
        LookupMode::None,
        true,
    )?;

    if inode.inode().ftype()? != FileType::Symlink {
        return Err(SyscallError::EINVAL);
    }

    let link = crate::kernel::fs::read_link(&inode.inode())?;

    if link.len() > max_size as usize {
        return Err(SyscallError::EFAULT);
    }

    make_buf_mut(buf, link.len() as u64).copy_from_slice(link.as_bytes());

    logln4!("read link read {}, len: {}", link, link.len());

    unsafe {
        *VirtAddr(len as usize).read_mut() = link.len();
    }

    Ok(0)
}

pub fn sys_pread(fd: u64, buf: u64, len: u64, offset: u64) -> SyscallResult {
    let fd = fd as usize;

    let task = current_task_ref();

    return if let Some(f) = task.get_handle(fd) {
        if f.flags().intersects(OpenFlags::RDONLY | OpenFlags::RDWR) {
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
        if f.flags().intersects(OpenFlags::WRONLY | OpenFlags::RDWR) {
            logln4!("pwrite fd {}", fd);
            Ok(f.write_at(make_buf(buf, len), offset as usize)?)
        } else {
            logln4!("pwrite fd {} = EACCESS", fd);
            Err(SyscallError::EACCES)
        }
    } else {
        logln4!("pwrite fd {} = EBADFD", fd);
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
    let at = OpenFD::try_from(at)?;

    get_dir_entry(at, make_path(path, path_len), LookupMode::None, false)?;

    Ok(0)
}

pub fn sys_fcntl(fd: u64, cmd: u64, flags: u64) -> SyscallResult {
    let cmd = FcntlCmd::from(cmd);

    logln5!("SYS_FCNTL {} {:?} {}", fd as isize, cmd, flags);

    match cmd {
        FcntlCmd::GetFD => {
            let task = current_task_ref();

            let handle = &task
                .filetable()
                .get_fd(fd as usize)
                .ok_or(SyscallError::EBADFD)?;
            Ok(handle.fd_flags().bits() as usize)
        }
        FcntlCmd::SetFD => {
            let task = current_task_ref();

            let handle = &task
                .filetable()
                .get_fd(fd as usize)
                .ok_or(SyscallError::EBADFD)?;
            handle.set_fd_flags(FDFlags::from_bits_truncate(flags));

            Ok(0)
        }
        FcntlCmd::GetFL => {
            let task = current_task_ref();

            let handle = task.get_handle(fd as usize).ok_or(SyscallError::EBADFD)?;
            Ok(handle.flags().bits())
        }
        FcntlCmd::SetFL => {
            let task = current_task_ref();

            let handle = task.get_handle(fd as usize).ok_or(SyscallError::EBADFD)?;
            handle.add_flags(OpenFlags::from_bits_truncate(flags as usize));

            Ok(0)
        }
        FcntlCmd::DupFD => {
            let task = current_task_ref();

            let res = task
                .filetable()
                .duplicate(fd as usize, FDFlags::empty(), flags as usize);
            task.filetable().debug();
            res
        }
        FcntlCmd::DupFDCloexec => {
            let task = current_task_ref();

            let res = task
                .filetable()
                .duplicate(fd as usize, FDFlags::FD_CLOEXEC, flags as usize);
            task.filetable().debug();
            res
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
    let prot = MMapProt::from_bits(prot as usize).ok_or(SyscallError::EINVAL)?;
    let flags = MMapFlags::from_bits(flags as usize).ok_or(SyscallError::EINVAL)?;
    let file = if !flags.contains(MMapFlags::MAP_ANONYOMUS) {
        Some(
            task.get_handle(fd as usize)
                .ok_or(SyscallError::EBADFD)?
                .inode
                .clone(),
        )
    } else {
        None
    };
    let offset = offset as usize;

    if let Some(res) = task.vm().mmap_vm(addr, len, prot, flags, file, offset) {
        //logln!("mmap at {} len: 0x{:X} | {:?}", res, len, flags);
        task.vm().log_vm();
        Ok(res.0)
    } else {
        Err(SyscallError::EFAULT)
    }
}

pub fn sys_munmap(addr: u64, len: u64) -> SyscallResult {
    let addr = VirtAddr(addr as usize);

    let task = current_task_ref();

    if task.vm().munmap_vm(addr, len as usize) {
        logln!("munmap at {} len: 0x{:X}", addr, len);
        //task.vm().log_vm();
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

pub fn sys_chdir(at: u64, path: u64, len: u64) -> SyscallResult {
    let dir = get_dir_entry(
        OpenFD::try_from(at)?,
        make_path(path, len),
        LookupMode::None,
        false,
    )?;

    let inode = dir.inode();

    return if inode.ftype()? == FileType::Dir {
        let task = current_task_ref();
        task.set_cwd(dir);
        Ok(0)
    } else {
        Err(SyscallError::ENOTDIR)
    };
}

pub fn sys_getcwd(buf: u64, len: u64) -> SyscallResult {
    logln!("getcwd len: {}", len);
    let buf = make_buf_mut(buf, if len > 0 { len } else { 255 });

    if let Some(pwd) = current_task_ref().get_pwd() {
        if pwd.len() > len as usize {
            Err(SyscallError::EIO)
        } else {
            logln!("getcwd {}", pwd);
            buf[..pwd.len()].copy_from_slice(pwd.as_bytes());
            Ok(pwd.len())
        }
    } else {
        Err(SyscallError::EINVAL)
    }
}

pub fn sys_mkdir(at: u64, path: u64, path_len: u64) -> SyscallResult {
    let at = OpenFD::try_from(at)?;

    let path = make_path(path, path_len).ok_or(SyscallError::EINVAL)?;

    let (inode, name) = {
        let (dir, target) = path.containing_dir();

        (
            get_dir_entry(at, Some(dir), LookupMode::None, false)?.inode(),
            target,
        )
    };

    if inode.ftype()? != FileType::Dir {
        return Err(SyscallError::ENOTDIR);
    }

    if ["", ".", ".."].contains(&name.str()) {
        return Err(SyscallError::EEXIST);
    }

    inode.mkdir(name.str())?;

    Ok(0)
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
    at: u64,
    linkpath: u64,
    linkpath_len: u64,
) -> SyscallResult {
    let target = make_str(target, target_len);
    let path = make_str(linkpath, linkpath_len);

    let path = Path::new(path);

    let (inode, name) = {
        let (dir, target) = path.containing_dir();

        (
            get_dir_entry(OpenFD::try_from(at)?, Some(dir), LookupMode::None, false)?.inode(),
            target,
        )
    };

    if inode.ftype()? == FileType::Dir {
        inode.symlink(name.str(), target)?;

        Ok(0)
    } else {
        Err(SyscallError::ENOTDIR)
    }
}

fn remove_dir(file: &DirEntryItem, path: &Path) -> SyscallResult {
    if file.inode().ftype()? != FileType::Dir {
        return Err(SyscallError::ENOTDIR);
    }

    let (_, name) = path.containing_dir();
    file.inode().rmdir(name.str())?;

    file.drop_from_cache();

    return Ok(0);
}

pub fn sys_rmdir(path: u64, path_len: u64) -> SyscallResult {
    let path = Path::new(make_str(path, path_len));
    let dir = lookup_by_real_path(&path, LookupMode::None)?;

    remove_dir(&dir, &path)
}

pub fn sys_unlink(at: u64, path: u64, path_len: u64, flags: u64) -> SyscallResult {
    let at = OpenFD::try_from(at)?;

    let file = get_dir_entry(at, make_path(path, path_len), LookupMode::None, true)?;

    log!("unlink inode: ");
    file.inode().debug();

    let path = Path::new(make_str(path, path_len));
    logln4!("sys_unlink: {}, flags: {}", path.str(), flags);

    let flags = AtFlags::from_bits(flags).ok_or(SyscallError::EINVAL)?;

    if flags.contains(AtFlags::REMOVEDIR) {
        return remove_dir(&file, &path);
    }

    let (_, name) = path.containing_dir();

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

pub fn sys_link(
    target_at: u64,
    target: u64,
    target_len: u64,
    link_at: u64,
    linkpath: u64,
    linkpath_len: u64,
) -> SyscallResult {
    let target_entry = get_dir_entry(
        target_at.try_into()?,
        make_path(target, target_len),
        LookupMode::None,
        true,
    )?;

    let (inode, name) = {
        let path = Path::new(make_str(linkpath, linkpath_len));

        let (dir, name) = path.containing_dir();

        (
            get_dir_entry(link_at.try_into()?, Some(dir), LookupMode::None, false)?.inode(),
            name,
        )
    };

    if !Weak::ptr_eq(&inode.fs().unwrap(), &target_entry.inode().fs().unwrap()) {
        return Err(SyscallError::EINVAL);
    }

    if inode.ftype()? == FileType::Dir {
        inode.link(name.str(), target_entry.inode())?;
    } else {
        return Err(SyscallError::ENOTDIR);
    }

    Ok(0)
}

pub fn sys_chmod(at: u64, path: u64, path_len: u64, mode: u64, flags: u64) -> SyscallResult {
    let flags = AtFlags::from_bits(flags).ok_or(SyscallError::EINVAL)?;
    logln5!(
        "sys_chmod: {:?} {:?} {:#o}",
        OpenFD::try_from(at)?,
        make_path(path, path_len),
        mode
    );
    let inode = get_dir_entry(
        at.try_into()?,
        make_path(path, path_len),
        LookupMode::None,
        flags.contains(AtFlags::SYMLINK_NOFOLLOW),
    )?;

    inode.inode().chmod(Mode::mode_bits_truncate(mode as u32))?;

    Ok(0)
}

pub fn sys_utime(at: u64, path: u64, path_len: u64, times: u64, flags: u64) -> SyscallResult {
    let flags = AtFlags::from_bits(flags).ok_or(SyscallError::EINVAL)?;
    logln5!(
        "sys_utime: {:?} {:?} {:#x}",
        OpenFD::try_from(at)?,
        make_path(path, path_len),
        times
    );
    let inode = get_dir_entry(
        at.try_into()?,
        make_path(path, path_len),
        LookupMode::None,
        flags.contains(AtFlags::SYMLINK_NOFOLLOW),
    )?;

    let times = &if times != 0 {
        unsafe { VirtAddr(times as usize).read::<[Timespec; 2]>() }
    } else {
        let now = crate::kernel::time::unix_timestamp() as u32;
        [Timespec::from_secs(now as usize); 2]
    };

    inode.inode().utime(times)?;

    Ok(0)
}

pub fn sys_rename(
    old_at: u64,
    oldpath: u64,
    oldpath_len: u64,
    new_at: u64,
    newpath: u64,
    newpath_len: u64,
) -> SyscallResult {
    let old = get_dir_entry(
        old_at.try_into()?,
        make_path(oldpath, oldpath_len),
        LookupMode::None,
        true,
    )?;

    let (new, name) = {
        let new_path = Path::new(make_str(newpath, newpath_len));

        let (dir, name) = new_path.containing_dir();

        (
            get_dir_entry(new_at.try_into()?, Some(dir), LookupMode::None, true)?,
            name,
        )
    };

    if !Weak::ptr_eq(&new.inode().fs().unwrap(), &old.inode().fs().unwrap()) {
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

pub fn sys_socket(domain: u64, typ: u64, _protocol: u64) -> SyscallResult {
    logln4!("sys socket AAAAAAAAA {} {}", domain, typ);
    let sock_domain = SockDomain::try_from(domain)?;
    let typ = SockTypeFlags::new(typ);

    logln4!("type flags: {:?}", typ);

    let task = current_task_ref();

    let res = Ok(task.open_file(
        DirEntry::inode_wrap(crate::kernel::net::socket::new(sock_domain, typ)?),
        OpenFlags::RDWR,
    )?);

    logln5!("sys_socket res = {:?}", res);
    task.filetable().debug();

    res
}

fn get_socket(fd: usize) -> Result<Arc<dyn SocketService>, SyscallError> {
    let task = current_task_ref();

    let sock = task.get_handle(fd).ok_or(SyscallError::EBADFD)?;

    Ok(sock
        .inode
        .inode()
        .as_socket()
        .ok_or(SyscallError::ENOTSOCK)?)
}

pub fn sys_bind(sockfd: u64, addr_ptr: u64, addrlen: u64) -> SyscallResult {
    let sock = get_socket(sockfd as usize)?;

    let addr = unsafe { VirtAddr(addr_ptr as usize).read_ref::<SockAddr>() };

    sock.bind(addr, addrlen as u32)
}

pub fn sys_connect(sockfd: u64, addr_ptr: u64, addrlen: u64) -> SyscallResult {
    let sock = get_socket(sockfd as usize)?;

    let addr = unsafe { VirtAddr(addr_ptr as usize).read_ref::<SockAddr>() };

    sock.connect(addr, addrlen as u32)
}

pub fn sys_accept(fd: u64, addr_ptr: u64, addr_len: u64) -> SyscallResult {
    let sock = get_socket(fd as usize)?;

    let (ptr, len) = if addr_ptr != 0 && addr_len != 0 {
        unsafe {
            (
                Some(VirtAddr(addr_ptr as usize).read_mut::<SockAddr>()),
                Some(VirtAddr(addr_len as usize).read_mut::<u32>()),
            )
        }
    } else {
        (None, None)
    };

    let sock = sock.accept(ptr, len)?;

    let task = current_task_ref();

    Ok(task.open_file(
        DirEntry::inode_wrap(sock.as_inode().ok_or(SyscallError::EFAULT)?),
        OpenFlags::RDWR,
    )?)
}

pub fn sys_listen(fd: u64, backlog: u64) -> SyscallResult {
    let sock = get_socket(fd as usize)?;

    sock.listen(backlog as i32)
}

pub fn sys_msg_recv(sockfd: u64, hdr: u64, flags: u64) -> SyscallResult {
    logln5!(
        "sys_msg_recv fd: {} hdr: {:#x}, flags: {:#x}",
        sockfd,
        hdr,
        flags
    );
    if hdr == 0 {
        return Err(SyscallError::EINVAL);
    }

    let hdr = unsafe { VirtAddr(hdr as usize).read_mut::<MsgHdr>() };

    let sock = get_socket(sockfd as usize)?;

    logln5!("msg_recv got socket!");

    sock.msg_recv(hdr, MsgFlags::from_bits(flags).ok_or(SyscallError::EINVAL)?)
}

pub fn sys_msg_send(sockfd: u64, hdr: u64, flags: u64) -> SyscallResult {
    logln5!("sys_msg_send {} {:#x} {:#x}", sockfd, hdr, flags);
    if hdr == 0 {
        return Err(SyscallError::EINVAL);
    }

    let hdr = unsafe { VirtAddr(hdr as usize).read_ref::<MsgHdr>() };

    let sock = get_socket(sockfd as usize)?;

    logln5!("sys_msg_send got socket!");

    sock.msg_send(hdr, MsgFlags::from_bits(flags).ok_or(SyscallError::EINVAL)?)
}

pub fn sys_setsockopt(fd: u64, layer: u64, number: u64, buffer: u64, size: u64) -> SyscallResult {
    logln5!("setsockopt: {} {} {}", fd, layer, number);
    let sock = get_socket(fd as usize)?;

    sock.set_socket_option(
        layer as i32,
        SockOption::try_from(number)?,
        buffer as *const (),
        size as u32,
    )
}

pub fn sys_getsockopt(fd: u64, layer: u64, number: u64, buffer: u64, size: u64) -> SyscallResult {
    logln5!("setsockopt: {} {} {}", fd, layer, number);
    let sock = get_socket(fd as usize)?;

    sock.get_socket_option(
        layer as i32,
        SockOption::try_from(number)?,
        buffer as *mut (),
        if size == 0 {
            None
        } else {
            unsafe { Some(VirtAddr(size as usize).read_mut::<u32>()) }
        },
    )
}

pub fn sys_select(
    nfds: u64,
    readfds: u64,
    writefds: u64,
    exceptfds: u64,
    timeout: u64,
    _sigmask: u64,
) -> SyscallResult {
    let timeout = if timeout == 0 {
        None
    } else {
        unsafe { Some(VirtAddr(timeout as usize).read_ref::<Timespec>()) }
    };

    logln5!(
        "sys_select nfds: {} {:x} {:x} {:x} {:?}",
        nfds,
        readfds,
        writefds,
        exceptfds,
        timeout
    );

    current_task_ref().filetable().debug();

    let mut input: [Option<(&mut FdSet, PollEventFlags)>; 3] = [None, None, None];

    let mut initfds = |idx: usize, addr: usize, flags: PollEventFlags| {
        if addr == 0 {
            return;
        }

        let fdset = unsafe { VirtAddr(addr).read_mut::<FdSet>() };

        logln5!("select fdset {} {:?}", idx, fdset.fds);

        input[idx] = Some((fdset, flags));
    };

    initfds(0, readfds as usize, PollEventFlags::READ);
    initfds(1, writefds as usize, PollEventFlags::WRITE);
    initfds(2, exceptfds as usize, PollEventFlags::empty());

    let mut to_poll = Vec::<(usize, PollEventFlags)>::new();

    for fd in 0..256 {
        let mut tp: Option<(usize, PollEventFlags)> = None;
        for inp in &mut input {
            if let Some(i) = inp {
                if i.0.is_set(fd) {
                    if let Some(t) = &mut tp {
                        t.1.insert(i.1);
                    } else {
                        tp = Some((fd, i.1));
                    }
                }
            }
        }
        if let Some(t) = tp {
            to_poll.push(t);
        }
    }

    logln5!("to poll: {:?}", to_poll);

    if let Some(fd) = &mut input[0] {
        fd.0.zero()
    }
    if let Some(fd) = &mut input[1] {
        fd.0.zero()
    }
    if let Some(fd) = &mut input[2] {
        fd.0.zero()
    }

    let task = current_task_ref();
    let mut first = true;
    let mut found = 0;
    let mut poll_table = PollTable::new(nfds as usize);

    let mut timed_out = false;

    'search: loop {
        for (fd, flags) in &to_poll {
            logln2!("select: checking fd {}", fd);
            if let Some(handle) = task.get_handle(*fd) {
                if let Ok(f) = handle
                    .inode
                    .inode()
                    .poll(if first { Some(&mut poll_table) } else { None }, *flags)
                {
                    logln5!("select found flags {:?}", f);
                    if !f.is_empty() {
                        let mut did_found = false;

                        if f.contains(PollEventFlags::READ) && flags.contains(PollEventFlags::READ)
                        {
                            if let Some(fd2) = &mut input[0] {
                                fd2.0.set(*fd);
                                did_found = true;
                            }
                        }
                        if f.contains(PollEventFlags::WRITE)
                            && flags.contains(PollEventFlags::WRITE)
                        {
                            if let Some(fd2) = &mut input[1] {
                                fd2.0.set(*fd);
                                did_found = true;
                            }
                        }

                        if did_found {
                            found += 1;
                        }
                    }
                } else {
                    break 'search;
                }
            }
        }

        logln2!(
            "select await io timeout: {:?}",
            timeout.and_then(|t| Some(t.to_nanoseconds()))
        );
        logln5!("select found {}, timed_out: {}", found, timed_out);
        if found == 0 && !timed_out {
            task.await_io_timeout(
                timeout.and_then(|t| Some(t.to_nanoseconds())),
                SleepFlags::empty(),
            )?;

            timed_out = task.sleep_until() == 0 && timeout.is_some();
            logln2!("timedout: {}", timed_out);
        } else {
            break 'search;
        }

        first = false;
    }

    logln5!("select found {}", found);

    return Ok(found);
}

pub fn sys_poll(fds: u64, nfds: u64, timeout: u64) -> SyscallResult {
    if nfds == 0 {
        return Ok(0);
    }

    let timeout_ms = timeout as i32;

    let fds = if fds != 0 {
        unsafe { VirtAddr(fds as usize).as_slice_mut::<syscall_defs::poll::PollFd>(nfds as usize) }
    } else {
        return Err(SyscallError::EINVAL);
    };
    logln4!("POLL {:?}", fds);

    let task = current_task_ref();

    let mut poll_table = PollTable::new(nfds as usize);
    let mut first = true;
    let mut found = 0;
    let mut timed_out = false;

    'search: loop {
        for fd in &mut *fds {
            if fd.fd < 0 {
                fd.revents = PollEventFlags::empty();
                continue;
            }
            if let Some(handle) = task.get_handle(fd.fd as usize) {
                let f = handle
                    .inode
                    .inode()
                    .poll(if first { Some(&mut poll_table) } else { None }, fd.events)?;
                if !f.is_empty() {
                    found += 1;

                    fd.revents = f;

                    logln4!("found {}: {:?}", fd.fd, fd.revents);
                }
            } else {
                fd.revents = PollEventFlags::NVAL;
            }
        }

        if found == 0 && !timed_out {
            task.await_io_timeout(
                if timeout_ms >= 0 {
                    Some(timeout_ms as usize * 1000)
                } else {
                    None
                },
                SleepFlags::empty(),
            )?;

            timed_out = task.sleep_until() == 0 && timeout_ms >= 0;
            logln2!("timedout: {}", timed_out);
        } else {
            break 'search;
        }

        first = false;
    }

    Ok(found)
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

    if fs != "ext2" {
        return Err(SyscallError::EINVAL);
    }

    let dev = lookup_by_path(&Path::new(dev_path), LookupMode::None)?.inode();
    let dest = lookup_by_path(&Path::new(dest_path), LookupMode::None)?;

    let dev =
        crate::kernel::block::get_blkdev_by_id(dev.device()?.id()).ok_or(SyscallError::ENODEV)?;

    let fs = crate::kernel::fs::ext2::Ext2Filesystem::new(dev).ok_or(SyscallError::EINVAL)?;

    crate::kernel::fs::mount::mount(dest, fs)
        .and(Ok(0))
        .or(Err(SyscallError::EFAULT))
}

pub fn sys_umount(path: u64, path_len: u64) -> SyscallResult {
    let path = make_str(path, path_len);

    let node = lookup_by_path(&Path::new(path), LookupMode::None)?;

    if let Err(e) = crate::kernel::fs::mount::umount(node) {
        Err(e)?
    } else {
        Ok(0)
    }
}

pub fn sys_time() -> SyscallResult {
    Ok(crate::kernel::time::unix_timestamp() as usize)
}

pub fn sys_ticksns() -> SyscallResult {
    Ok(crate::kernel::timer::current_ns() as usize)
}

pub fn sys_exit(status: u64) -> ! {
    crate::kernel::sched::exit(syscall_defs::waitpid::Status::Exited(status));
}

pub fn sys_sleep(time_ns: u64) -> SyscallResult {
    logln2!("sys_sleep {}", time_ns);
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

    let prog = lookup_by_path(&Path::new(path), LookupMode::None)?;

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

    crate::kernel::sched::exec(prog, args, envs)?
}

pub fn sys_spawn_thread(entry: u64, stack: u64) -> SyscallResult {
    let thread =
        crate::kernel::sched::spawn_thread(VirtAddr(entry as usize), VirtAddr(stack as usize));

    Ok(thread.tid())
}

pub fn sys_waitpid(pid: u64, status: u64, flags: u64) -> SyscallResult {
    use syscall_defs::waitpid::*;

    let current = current_task_ref();

    let status = unsafe { VirtAddr(status as usize).read_mut::<u32>() };

    let mut st = Status::Invalid(0);

    let res = current.wait_pid(
        pid as isize,
        &mut st,
        WaitPidFlags::from_bits_truncate(flags as usize) | WaitPidFlags::EXITED,
    )?;

    *status = st.into();

    res
}

pub fn sys_getpid() -> SyscallResult {
    Ok(current_task_ref().pid())
}

pub fn sys_getppid() -> SyscallResult {
    if let Some(p) = &current_task_ref().get_parent() {
        Ok(p.pid())
    } else {
        Ok(0)
    }
}

pub fn sys_getpgid(_pid: u64) -> SyscallResult {
    Ok(current_task_ref().gid())
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
    logln5!("ioctl {} 0x{:x}", fd, cmd);
    let current = current_task_ref();

    if let Some(handle) = current.get_handle(fd as usize) {
        Ok(handle.inode.inode().ioctl(cmd as usize, arg as usize)?)
    } else {
        Err(SyscallError::EBADFD)
    }
}

pub fn sys_sigaction(sig: u64, sigact: u64, old: u64) -> SyscallResult {
    if sig == 32 {
        //temporary hack to make mlibc happy
        return Err(SyscallError::ENOSYS);
    }

    let new = if sigact == 0 {
        None
    } else {
        unsafe { Some(VirtAddr(sigact as usize).read_ref::<SigAction>()) }
    };

    logln!(
        "sigaction: {:#x} {:?} size: {}",
        sigact,
        new,
        core::mem::size_of::<SigAction>()
    );

    let entry = if let Some(new) = new {
        Some(SignalEntry::from_sigaction(*new)?)
    } else {
        None
    };

    let old = if old == 0 {
        None
    } else {
        unsafe { Some(VirtAddr(old as usize).read_mut::<SigAction>()) }
    };

    logln5!("sigaction: {} {:?}, old: {:?}", sig, entry, old);

    current_task_ref()
        .signals()
        .set_signal(sig as usize, entry, old)
}

pub fn sys_sigprocmask(how: u64, set: u64, old_set: u64) -> SyscallResult {
    logln2!("sigprocmask: {} {} {}", how, set, old_set);
    let how = syscall_defs::signal::SigProcMask::from(how);
    let set = if set > 0 {
        Some(unsafe { VirtAddr(set as usize).read::<u64>() })
    } else {
        None
    };
    let old_set = if old_set > 0 {
        Some(unsafe { VirtAddr(old_set as usize).read_mut::<u64>() })
    } else {
        None
    };
    current_task_ref().signals().set_mask(how, set, old_set);
    Ok(0)
}

pub fn sys_kill(pid: u64, sig: u64) -> SyscallResult {
    logln4!(
        "kill: {} -> {} {}",
        current_task_ref().tid(),
        pid as i64,
        sig
    );
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
        a if a < -1 => Ok(crate::kernel::session::get_group((-a) as usize)
            .and_then(|g| {
                g.signal(sig as usize);

                Some(0)
            })
            .ok_or(SyscallError::ESRCH)?),
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
    let fds = unsafe { core::slice::from_raw_parts_mut(fds as *mut u32, 2) };

    let pipe = crate::kernel::fs::pipe::Pipe::new();

    let entry = DirEntry::inode_wrap(pipe);

    let task = current_task_ref();

    let flags = OpenFlags::from_bits(flags as usize).ok_or(SyscallError::EINVAL)?;

    let f1 = OpenFlags::RDONLY | (flags & OpenFlags::CLOEXEC);
    let f2 = OpenFlags::WRONLY | (flags & OpenFlags::CLOEXEC);

    if let Ok(fd1) = task.open_file(entry.clone(), f1) {
        if let Ok(fd2) = task.open_file(entry, f2) {
            fds[0] = fd1 as u32;
            fds[1] = fd2 as u32;

            return Ok(0);
        } else {
            task.close_file(fd1);
        }
    }

    Err(SyscallError::EINVAL)
}

pub fn sys_dup(fd: u64, flags: u64) -> SyscallResult {
    let task = current_task_ref();

    let flags = OpenFlags::from_bits(flags as usize).ok_or(SyscallError::EINVAL)?;

    let res = task
        .filetable()
        .duplicate(fd as usize, FDFlags::from(flags), 0);

    logln5!("dup {} {:?} = {:?}", fd, flags, res);
    task.filetable().debug();

    res
}

pub fn sys_dup2(fd: u64, new_fd: u64, flags: u64) -> SyscallResult {
    let task = current_task_ref();

    if fd == new_fd {
        return Ok(fd as usize);
    }

    let flags = OpenFlags::from_bits(flags as usize).ok_or(SyscallError::EINVAL)?;

    let res = task
        .filetable()
        .duplicate_at(fd as usize, new_fd as usize, FDFlags::from(flags));
    logln5!("dup2 {} {} {:?} = {:?}", fd, new_fd, flags, res);
    task.filetable().debug();

    return res;
}

pub fn sys_truncate(fd: u64, size: u64) -> SyscallResult {
    logln4!("truncate {} to size {}", fd, size);
    let task = current_task_ref();

    task.filetable()
        .get_handle(fd as usize)
        .ok_or(SyscallError::EBADFD)?
        .inode
        .inode()
        .truncate(size as usize)
        .map(|_r| Ok(0))?
}

pub fn sys_futex_wake(uaddr: u64) -> SyscallResult {
    let uaddr = VirtAddr(uaddr as usize);

    //println!("[ FUTEX ] wake {} {}", uaddr, unsafe {
    //    uaddr.read_volatile::<u32>()
    //});

    crate::kernel::futex::futex().wake(uaddr)
}

pub fn sys_stat(fd: u64, path: u64, path_len: u64, stat: u64, flags: u64) -> SyscallResult {
    let fd = OpenFD::try_from(fd)?;
    let flags = AtFlags::from_bits(flags).ok_or(SyscallError::EINVAL)?;

    let file = get_dir_entry(
        fd,
        make_path(path, path_len),
        LookupMode::None,
        flags.contains(AtFlags::SYMLINK_NOFOLLOW),
    )?;

    let stat = unsafe { VirtAddr(stat as usize).read_mut::<syscall_defs::stat::Stat>() };

    *stat = file.inode().stat()?;

    logln!("fstatat {:?}, {:?}", fd, stat);

    Ok(0)
}

pub fn sys_getrlimit(resource: u64, rlimit: u64) -> SyscallResult {
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
    logln5!("{}", make_str(str, str_len));

    Ok(0)
}

pub fn sys_sync() -> SyscallResult {
    crate::kernel::fs::mount::sync_all();
    crate::kernel::block::sync_all();

    Ok(0)
}

pub fn sys_fsync(fd: u64) -> SyscallResult {
    let file = current_task_ref()
        .filetable()
        .get_handle(fd as usize)
        .ok_or(SyscallError::EBADFD)?;

    file.inode.inode().sync()?;

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

pub fn sys_yield() -> SyscallResult {
    crate::kernel::sched::reschedule();

    Ok(0)
}
