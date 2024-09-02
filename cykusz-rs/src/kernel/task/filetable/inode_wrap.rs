use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

use syscall_defs::poll::PollEventFlags;
use syscall_defs::stat::Stat;
use syscall_defs::{FileType, OpenFlags};

use crate::kernel::device::dev_t::DevId;
use crate::kernel::fs::dirent::{DirEntry, DirEntryItem};
use crate::kernel::fs::filesystem::Filesystem;
use crate::kernel::fs::icache::INodeItem;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::pcache::{CachedAccess, MappedAccess};
use crate::kernel::fs::poll::PollTable;
use crate::kernel::fs::vfs;
use crate::kernel::fs::vfs::{DirEntIter, Metadata};
use crate::kernel::net::socket::SocketService;

pub struct INodeOpsWrap {
    dev_inode: Option<Arc<dyn INode>>,
    fs_inode: DirEntryItem,
    self_ref: Weak<INodeOpsWrap>,
}

fn get_dev_inode(inode: &DirEntryItem) -> Option<Arc<dyn INode>> {
    let fs_inode = inode.inode();

    return match fs_inode.ftype() {
        Ok(FileType::Fifo) => {
            if let Some(ino) = crate::kernel::fs::pipe::pipes().get_or_insert_default(&fs_inode) {
                Some(ino)
            } else {
                None
            }
        }
        Ok(FileType::Socket) => Some(crate::kernel::net::unix::sockets().get(&fs_inode)?.clone()),
        Ok(FileType::Char) | Ok(FileType::Block) => {
            crate::kernel::device::find_device(fs_inode.device_id()?).and_then(|d| Some(d.inode()))
        }
        _ => None,
    };
}

impl INodeOpsWrap {
    pub fn new(inode: DirEntryItem) -> Arc<INodeOpsWrap> {
        Arc::new_cyclic(|me| INodeOpsWrap {
            dev_inode: get_dev_inode(&inode),
            fs_inode: inode,
            self_ref: me.clone(),
        })
    }

    fn get_inode(&self) -> Arc<dyn INode> {
        if let Some(i) = &self.dev_inode {
            i.clone()
        } else {
            self.fs_inode.inode().inode_arc()
        }
    }

    pub fn get_fs_dir_item(&self) -> DirEntryItem {
        return self.fs_inode.clone();
    }

    pub fn get_dir_item(&self) -> DirEntryItem {
        let e = DirEntry::inode_wrap(self.self_ref.upgrade().unwrap().clone());
        e.update_name(self.fs_inode.name());
        e.update_parent(self.fs_inode.parent());
        e
    }
}

macro_rules! impl_delegate {
    ($name:tt, $res: ty) => {
        fn $name(&self) -> $res {
            self.get_inode().$name()
        }
    };
    ($name:tt, $res: ty, $($v:tt: $t:ty),*) => {
        fn $name(&self, $($v: $t),+) -> $res {
            self.get_inode().$name($($v,)*)
        }
    }
}

macro_rules! impl_delegate_fs {
    ($name:tt, $res: ty) => {
        fn $name(&self) -> $res {
            self.fs_inode.inode().$name()
        }
    };
    ($name:tt, $res: ty, $($v:tt: $t:ty),*) => {
        fn $name(&self, $($v: $t),+) -> $res {
            self.fs_inode.inode().$name($($v,)*)
        }
    }
}

impl INode for INodeOpsWrap {
    impl_delegate!(id, vfs::Result<usize>);
    impl_delegate!(ftype, vfs::Result<FileType>);
    impl_delegate!(metadata, vfs::Result<Metadata>);
    impl_delegate_fs!(stat, vfs::Result<Stat>);
    impl_delegate!(lookup, vfs::Result<DirEntryItem>, parent: DirEntryItem, name: &str);
    impl_delegate!(mkdir, vfs::Result<INodeItem>, name: &str);
    impl_delegate!(rmdir, vfs::Result<()>, name: &str);
    impl_delegate!(unlink, vfs::Result<()>, name: &str);
    impl_delegate!(read_at, vfs::Result<usize>, offset: usize, buf: &mut [u8], flags: OpenFlags);
    impl_delegate!(read_all, vfs::Result<Vec<u8>>);
    impl_delegate!(write_at, vfs::Result<usize>, offset: usize, buf: &[u8], flags: OpenFlags);
    impl_delegate!(poll, vfs::Result<PollEventFlags>, poll_table: Option<&mut PollTable>, flags: PollEventFlags);
    impl_delegate!(fs, Option<Weak<dyn Filesystem>>);
    impl_delegate!(create, vfs::Result<DirEntryItem>, parent: DirEntryItem,name: &str, ftype: FileType);
    impl_delegate!(open, vfs::Result<()>, flags: OpenFlags);
    impl_delegate!(close, (), flags: OpenFlags);
    impl_delegate!(mknode, vfs::Result<INodeItem>, parent: DirEntryItem, name: &str, mode: syscall_defs::stat::Mode, devid: DevId);
    impl_delegate!(symlink, vfs::Result<()>, name: &str, target: &str);
    impl_delegate!(link, vfs::Result<()>, name: &str, target: INodeItem);
    impl_delegate!(rename, vfs::Result<()>, old: DirEntryItem, new_name: &str);
    impl_delegate!(chmod, vfs::Result<()>, mode: syscall_defs::stat::Mode);
    impl_delegate!(utime, vfs::Result<()>, times: &[syscall_defs::time::Timespec; 2]);
    impl_delegate!(truncate, vfs::Result<()>, size: usize);
    impl_delegate!(dir_ent, vfs::Result<Option<DirEntryItem>>, parent: DirEntryItem, idx: usize);
    impl_delegate!(dir_iter, Option<Arc<dyn DirEntIter>>, parent: DirEntryItem);
    impl_delegate!(device_id, Option<DevId>);
    impl_delegate!(ioctl, vfs::Result<usize>, cmd: usize, arg: usize);
    impl_delegate!(sync, vfs::Result<()>);
    impl_delegate!(as_cacheable, Option<Arc<dyn CachedAccess>>);
    impl_delegate!(as_mappable, Option<Arc<dyn MappedAccess>>);
    impl_delegate!(as_socket, Option<Arc<dyn SocketService>>);
    impl_delegate!(debug, ());
}
