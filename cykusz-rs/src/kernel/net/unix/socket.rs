use crate::kernel::fs::inode::INode;
use crate::kernel::fs::path::Path;
use crate::kernel::fs::poll::PollTable;
use crate::kernel::fs::vfs::FsError;
use crate::kernel::fs::{lookup_by_path, LookupMode};
use crate::kernel::net::socket::SocketService;
use crate::kernel::sync::{LockApi, Mutex};
use crate::kernel::utils::buffer::BufferQueue;
use crate::kernel::utils::node_map::NodeMapItem;
use crate::kernel::utils::wait_queue::{WaitQueue, WaitQueueFlags};
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};
use syscall_defs::net::{MsgFlags, MsgHdr, SockAddrPtr, SockAddrUn, SockTypeFlags};
use syscall_defs::poll::PollEventFlags;
use syscall_defs::stat::{Mode, Stat};
use syscall_defs::{OpenFlags, SyscallError, SyscallResult};

struct ConnectionQueue {
    queue: Vec<Arc<Socket>>,
    addr: SockAddrUn,
}

impl ConnectionQueue {
    fn new(addr: SockAddrUn) -> ConnectionQueue {
        ConnectionQueue {
            queue: Vec::new(),
            addr,
        }
    }

    fn addr(&self) -> &SockAddrUn {
        &self.addr
    }
}

enum SocketState {
    Disconnected,
    Connected(Arc<Socket>),
    Listening(ConnectionQueue),
    Bound(SockAddrUn),
}

pub struct Socket {
    self_ref: Weak<Socket>,
    key: Option<(usize, usize)>,
    data: Mutex<SocketState>,
    wq: WaitQueue,
    buffer: BufferQueue,

    readers: AtomicUsize,
    writers: AtomicUsize,
}

impl Socket {
    pub fn new_unbound(key: Option<(usize, usize)>) -> Arc<Socket> {
        Arc::new_cyclic(|me| Socket {
            self_ref: me.clone(),
            key,
            data: Mutex::new(SocketState::Disconnected),
            wq: WaitQueue::new(),
            buffer: BufferQueue::new_empty(false, false),

            readers: AtomicUsize::new(0),
            writers: AtomicUsize::new(0),
        })
    }

    pub fn new_connected(_flags: SockTypeFlags) -> (Arc<Socket>, Arc<Socket>) {
        let s1 = Socket::new_unbound(None);
        let s2 = Socket::new_unbound(None);

        s1.buffer.init_size(4096 * 4);
        s2.buffer.init_size(4096 * 4);

        *s1.data.lock() = SocketState::Connected(s2.clone());
        *s2.data.lock() = SocketState::Connected(s1.clone());

        s1.inc_readers();
        s1.inc_writers();
        s2.inc_readers();
        s2.inc_writers();

        (s1, s2)
    }

    fn target(&self) -> Arc<Socket> {
        if let SocketState::Connected(s) = &*self.data.lock() {
            s.clone()
        } else {
            panic!("socket not connected")
        }
    }

    fn self_ref(&self) -> Arc<Socket> {
        self.self_ref.upgrade().unwrap()
    }

    fn is_connected(&self) -> bool {
        matches!(*self.data.lock(), SocketState::Connected(_))
    }

    fn is_listening(&self) -> bool {
        matches!(*self.data.lock(), SocketState::Listening(_))
    }

    fn is_disconnected(&self) -> bool {
        matches!(*self.data.lock(), SocketState::Disconnected)
    }

    fn is_bound(&self) -> bool {
        matches!(*self.data.lock(), SocketState::Bound(_))
    }

    fn inc_readers(&self) -> usize {
        let readers = self.readers.fetch_add(1, Ordering::SeqCst) + 1;

        logln!("inc readers to {}", readers);

        if readers == 1 {
            self.buffer.set_has_readers(true);
        }

        readers
    }

    fn inc_writers(&self) -> usize {
        let writers = self.writers.fetch_add(1, Ordering::SeqCst) + 1;

        logln!("inc writers to {}", writers);

        if writers == 1 {
            self.buffer.set_has_writers(true);
        }

        writers
    }

    fn dec_readers(&self) -> usize {
        assert_ne!(self.readers.load(Ordering::Relaxed), 0);
        let readers = self.readers.fetch_sub(1, Ordering::SeqCst) - 1;

        logln!("dec readers to {}", readers);

        if readers == 0 {
            self.buffer.set_has_readers(false);
        }

        readers
    }

    fn dec_writers(&self) -> usize {
        assert_ne!(self.writers.load(Ordering::Relaxed), 0);
        let writers = self.writers.fetch_sub(1, Ordering::SeqCst) - 1;

        logln!("dec writers to {}", writers);

        if writers == 0 {
            self.buffer.set_has_writers(false);
        }

        writers
    }

    fn has_writers(&self) -> bool {
        self.writers.load(Ordering::Relaxed) > 0
    }

    fn has_readers(&self) -> bool {
        self.readers.load(Ordering::Relaxed) > 0
    }
}

impl NodeMapItem for Socket {
    fn new(key: Option<(usize, usize)>) -> Arc<Self> {
        Self::new_unbound(key)
    }

    fn key(&self) -> Option<(usize, usize)> {
        self.key
    }
}

impl SocketService for Socket {
    fn listen(&self, _backlog: i32) -> SyscallResult {
        let mut lock = self.data.lock();
        if let SocketState::Bound(addr) = &*lock {
            *lock = SocketState::Listening(ConnectionQueue::new(*addr));

            Ok(0)
        } else {
            Err(SyscallError::EINVAL)
        }
    }

    fn accept(
        &self,
        mut sock_addr: SockAddrPtr,
        addrlen: Option<&mut u32>,
    ) -> Result<Arc<dyn SocketService>, SyscallError> {
        if !self.is_listening() {
            return Err(SyscallError::EADDRINUSE);
        }

        let mut lock = self
            .wq
            .wait_lock_for(WaitQueueFlags::empty(), &self.data, |mx| {
                if let SocketState::Listening(q) = &**mx {
                    !q.queue.is_empty()
                } else {
                    false
                }
            })?
            .unwrap();

        if let SocketState::Listening(q) = &mut *lock {
            let client = q.queue.pop().ok_or(SyscallError::ECONNREFUSED)?;
            let new = Socket::new_unbound(None);

            new.buffer.init_size(4096 * 4);
            client.buffer.init_size(4096 * 4);

            *new.data.lock() = SocketState::Connected(client.clone());
            *client.data.lock() = SocketState::Connected(new.clone());

            new.inc_readers();
            new.inc_writers();
            client.inc_readers();
            client.inc_writers();

            client.wq.notify_all();

            if !sock_addr.is_null() {
                if let Some(len) = addrlen {
                    if *len as usize >= core::mem::size_of::<SockAddrUn>() {
                        *len = core::mem::size_of_val(q.addr()) as u32;
                        *sock_addr.as_sock_addr_un_mut() = *q.addr();
                    }
                }
            }

            Ok(new)
        } else {
            Err(SyscallError::EINVAL)
        }
    }

    fn bind(&self, sock_addr: SockAddrPtr, _addrlen: u32) -> SyscallResult {
        if !self.is_disconnected() {
            return Err(SyscallError::ENOTSUP);
        }

        let sock_addr = sock_addr.as_sock_addr_un();

        let path = Path::new(sock_addr.path());
        let (dir, name) = path.containing_dir();

        let dir = lookup_by_path(&dir, LookupMode::None)?;

        let mut sockets = crate::kernel::net::unix::sockets();

        let inode = dir
            .inode()
            .mknode(dir.clone(), name.str(), Mode::IFSOCK, 0)?;

        if let Err(e) = sockets.insert(&inode.inode_arc(), &self.self_ref()) {
            dir.inode().unlink(name.str())?;
            return Err(e);
        }

        *self.data.lock() = SocketState::Bound(*sock_addr);

        Ok(0)
    }

    fn connect(&self, sock_addr: SockAddrPtr, _addrlen: u32) -> SyscallResult {
        let sock_addr = sock_addr.as_sock_addr_un();

        let path = Path::new(sock_addr.path());

        let dir = lookup_by_path(&path, LookupMode::None)?;

        let mut sockets = crate::kernel::net::unix::sockets();

        let target = sockets
            .get(&dir.inode().inode_arc())
            .ok_or(SyscallError::EADDRNOTAVAIL)?;

        if let SocketState::Listening(queue) = &mut *target.data.lock() {
            queue.queue.push(self.self_ref().clone())
        } else {
            return Err(SyscallError::ECONNREFUSED);
        }

        target.wq.notify_all();

        self.wq
            .wait_for(WaitQueueFlags::empty(), || self.is_connected())?;

        Ok(0)
    }

    fn msg_send(&self, hdr: &MsgHdr, flags: MsgFlags) -> SyscallResult {
        let iovecs = hdr.iovecs();

        let mut total = 0;

        for iovec in iovecs {
            total += self.write_at(0, iovec.get_bytes(), OpenFlags::from(flags))?;
        }

        Ok(total)
    }

    fn msg_recv(&self, hdr: &mut MsgHdr, flags: MsgFlags) -> SyscallResult {
        let iovecs = hdr.iovecs_mut();

        let mut total = 0;

        for iovec in iovecs {
            if total > 0 && !self.buffer.has_data() {
                break;
            }

            let offset = if flags.contains(MsgFlags::MSG_PEEK) {
                return Err(SyscallError::ENOTSUP);
            } else {
                0
            };

            let read = self.read_at(offset, iovec.get_bytes_mut(), OpenFlags::from(flags))?;

            if read == 0 {
                return Ok(total);
            }

            total += read
        }

        return Ok(total);
    }

    fn as_inode(&self) -> Option<Arc<dyn INode>> {
        Some(self.self_ref.upgrade()?.clone())
    }
}

impl INode for Socket {
    fn stat(&self) -> crate::kernel::fs::vfs::Result<Stat> {
        let mut stat = Stat::default();

        stat.st_mode.insert(Mode::IFSOCK);
        stat.st_mode.insert(Mode::IRWXU);
        stat.st_mode.insert(Mode::IRWXG);
        stat.st_mode.insert(Mode::IRWXO);

        Ok(stat)
    }

    fn read_at(
        &self,
        _offset: usize,
        buf: &mut [u8],
        flags: OpenFlags,
    ) -> crate::kernel::fs::vfs::Result<usize> {
        Ok(self
            .buffer
            .read_data_flags(buf, WaitQueueFlags::from(flags))?)
    }

    fn write_at(
        &self,
        _offset: usize,
        buf: &[u8],
        flags: OpenFlags,
    ) -> crate::kernel::fs::vfs::Result<usize> {
        let target = if let SocketState::Connected(target) = &*self.data.lock() {
            target.clone()
        } else {
            return Err(FsError::NotSupported);
        };

        dbgln!(unix, "Writing {} data", buf.len());

        Ok(target
            .buffer
            .append_data_flags(buf, WaitQueueFlags::from(flags))?)
    }

    fn poll(
        &self,
        poll_table: Option<&mut PollTable>,
        flags: PollEventFlags,
    ) -> crate::kernel::fs::vfs::Result<PollEventFlags> {
        let mut res_flags = PollEventFlags::empty();
        let target = if flags.contains(PollEventFlags::WRITE) {
            if let SocketState::Connected(target) = &*self.data.lock() {
                Some(target.clone())
            } else {
                return Err(FsError::NotSupported);
            }
        } else {
            None
        };
        let is_listening = if flags.contains(PollEventFlags::READ) {
            if self.is_listening() {
                let data = self.data.lock();

                if let SocketState::Listening(q) = &*data {
                    if !q.queue.is_empty() {
                        res_flags.insert(PollEventFlags::READ);
                    }
                } else {
                    return Err(FsError::NotSupported);
                }

                true
            } else {
                if self.buffer.has_data() {
                    res_flags.insert(PollEventFlags::READ);
                }

                if !self.has_writers() {
                    res_flags.insert(PollEventFlags::HUP);
                }

                false
            }
        } else {
            false
        };
        if flags.contains(PollEventFlags::WRITE) {
            if target.clone().unwrap().buffer.available_size() > 0 {
                res_flags.insert(PollEventFlags::WRITE);
            }

            if !target.clone().unwrap().buffer.has_readers() {
                res_flags.insert(PollEventFlags::ERR);
            }
        }

        if let Some(p) = poll_table {
            if flags.contains(PollEventFlags::READ) {
                if !is_listening {
                    p.listen(&self.buffer.readers_queue());
                } else {
                    p.listen(&self.wq);
                }
            }
            if flags.contains(PollEventFlags::WRITE) {
                p.listen(&target.unwrap().buffer.writers_queue());
            }
        }

        Ok(res_flags)
    }

    fn open(&self, _flags: OpenFlags) -> crate::kernel::fs::vfs::Result<()> {
        Ok(())
    }

    fn close(&self, flags: OpenFlags) {
        if !self.is_connected() {
            return;
        }

        if flags.is_readable() {
            self.dec_readers();
        }

        if flags.is_writable() && self.is_connected() {
            self.target().dec_writers();
        }
    }

    fn ioctl(&self, cmd: usize, arg: usize) -> crate::kernel::fs::vfs::Result<usize> {
        logln!("got unix socket ioctl cmd: {:#x} arg: {}", cmd, arg);
        Ok(0)
    }

    fn as_socket(&self) -> Option<Arc<dyn SocketService>> {
        Some(self.self_ref().clone())
    }
}
