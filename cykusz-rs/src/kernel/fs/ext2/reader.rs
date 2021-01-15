use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::kernel::fs::ext2::buf_block::BufBlock;
use crate::kernel::fs::ext2::disk::inode::{FileType, INode};
use crate::kernel::fs::ext2::inode::LockedExt2INode;
use crate::kernel::fs::ext2::Ext2Filesystem;
use crate::kernel::utils::slice::{ToBytes, ToBytesMut};
use crate::kernel::utils::types::{Align, CeilDiv};

pub struct INodeReader {
    inode: Arc<LockedExt2INode>,
    offset: usize,
}

impl INodeReader {
    pub fn new(inode: Arc<LockedExt2INode>, offset: usize) -> INodeReader {
        INodeReader { inode, offset }
    }

    fn fs(&self) -> Arc<Ext2Filesystem> {
        self.inode.fs()
    }

    pub fn append_block(&mut self) -> Option<BufBlock> {
        let fs = self.fs();
        let sb = fs.superblock();

        let mut inode = self.inode.write();
        let id = inode.id();

        let block_size = self.fs().superblock().block_size();

        let next_block_num = (inode.d_inode().size_lower() as usize).ceil_div(block_size);

        if let Some(new_block) = fs.alloc_block(id) {
            let d_inode = inode.d_inode_mut();

            self.set_block(next_block_num, new_block.block(), d_inode);

            d_inode.set_size_lower(d_inode.size_lower() + block_size as u32);
            d_inode.set_sector_count(d_inode.sector_count() as u32 + sb.sectors_per_block() as u32);

            fs.group_descs().write_d_inode(id, &d_inode);

            Some(new_block)
        } else {
            None
        }
    }

    fn set_block(&self, mut block_num: usize, ptr: usize, inode: &mut INode) {
        if block_num < 12 {
            inode.direct_ptrs_mut()[block_num] = ptr as u32;
            return;
        }

        let block_size = self.fs().superblock().block_size();

        block_num -= 12;

        let entries_per_block = block_size / core::mem::size_of::<u32>();

        let mut buf = Vec::<u32>::new();
        buf.resize(entries_per_block, 0);

        let mut set_ptrs = |mut ptr: usize, offsets: &[usize], val: usize| -> bool {
            if ptr == 0 {
                return false;
            }

            for (i, &offset) in offsets.iter().enumerate() {
                if self
                    .fs()
                    .read_block(ptr, buf.as_mut_slice().to_bytes_mut())
                    .is_none()
                {
                    return false;
                }

                if i == offsets.len() - 1 {
                    buf[offset] = val as u32;
                } else {
                    ptr = buf[offset] as usize;

                    if ptr == 0 {
                        return false;
                    }
                }
            }

            true
        };

        if block_num < entries_per_block {
            set_ptrs(inode.s_indir_ptr() as usize, &[block_num], ptr);
            return;
        }

        let entries_per_dblock = entries_per_block * entries_per_block;

        if block_num < entries_per_dblock {
            let off1 = block_num / entries_per_block;
            let off2 = block_num % entries_per_block;

            set_ptrs(inode.d_indir_ptr() as usize, &[off1, off2], ptr);
            return;
        }

        let entried_per_tblock = entries_per_dblock * entries_per_block;

        if block_num < entried_per_tblock {
            let off1 = block_num / entries_per_dblock;

            block_num = block_num - off1 * entries_per_dblock;

            let off2 = block_num / entries_per_block;
            let off3 = block_num % entries_per_block;

            set_ptrs(inode.t_indir_ptr() as usize, &[off1, off2, off3], ptr);
            return;
        }
    }

    fn get_block(&self, mut block_num: usize, inode: &INode) -> Option<usize> {
        if block_num < 12 {
            return Some(inode.direct_ptrs()[block_num] as usize);
        }

        let block_size = self.fs().superblock().block_size();

        block_num -= 12;

        let entries_per_block = block_size / core::mem::size_of::<u32>();

        let mut buf = Vec::<u32>::new();
        buf.resize(entries_per_block, 0);

        let mut read_ptrs = |mut ptr: usize, offsets: &[usize]| -> Option<usize> {
            if ptr == 0 {
                return None;
            }

            for &offset in offsets {
                self.fs()
                    .read_block(ptr, buf.as_mut_slice().to_bytes_mut())?;

                ptr = buf[offset] as usize;

                if ptr == 0 {
                    return None;
                }
            }

            Some(ptr)
        };

        if block_num < entries_per_block {
            return read_ptrs(inode.s_indir_ptr() as usize, &[block_num]);
        }

        let entries_per_dblock = entries_per_block * entries_per_block;

        if block_num < entries_per_dblock {
            let off1 = block_num / entries_per_block;
            let off2 = block_num % entries_per_block;

            return read_ptrs(inode.d_indir_ptr() as usize, &[off1, off2]);
        }

        let entried_per_tblock = entries_per_dblock * entries_per_block;

        if block_num < entried_per_tblock {
            let off1 = block_num / entries_per_dblock;

            block_num = block_num - off1 * entries_per_dblock;

            let off2 = block_num / entries_per_block;
            let off3 = block_num % entries_per_block;

            return read_ptrs(inode.t_indir_ptr() as usize, &[off1, off2, off3]);
        }

        None
    }

    pub fn read_block(&mut self) -> Option<BufBlock> {
        let linode = self.inode.read();
        let inode = linode.d_inode();

        let file_size = inode.size_lower() as usize;

        let fs = self.fs();

        if self.offset >= file_size {
            return None;
        }

        if inode.ftype() == FileType::Symlink && file_size <= 60 {
            return None;
        }

        let block_size = fs.superblock.block_size();

        self.offset = self.offset.align_up(block_size);

        let block_num = self.offset / block_size;

        let rem = core::cmp::min(block_size, file_size - self.offset);

        if let Some(ptr) = self.get_block(block_num, inode) {
            let mut buf = fs.make_buf_size(rem);
            buf.set_block(ptr);

            fs.read_block(ptr, buf.bytes_mut())?;

            self.offset += rem;

            Some(buf)
        } else {
            None
        }
    }

    pub fn read(&mut self, dest: &mut [u8]) -> usize {
        use core::cmp::min;

        let linode = self.inode.read();
        let inode = linode.d_inode();

        let file_size = inode.size_lower() as usize;

        let buffer_size = min(file_size - self.offset, dest.len());

        if inode.ftype() == FileType::Symlink && file_size <= 60 {
            dest[..buffer_size].copy_from_slice(
                &inode.block_ptrs().to_bytes()[self.offset..self.offset + buffer_size],
            );

            return buffer_size;
        }

        let fs = self.fs();
        let block_size = fs.superblock().block_size();

        let mut rem = buffer_size;

        let mut dest_off = 0;
        while rem > 0 && self.offset < file_size {
            let block_num = self.offset / block_size;
            let block_offset = self.offset % block_size;

            if let Some(block) = self.get_block(block_num, inode) {
                let mut buf = Vec::<u8>::new();
                buf.resize(block_size, 0);

                if let Some(read) = fs.read_block(block, buf.as_mut_slice()) {
                    let to_copy = min(rem, read - block_offset);

                    dest[dest_off..dest_off + to_copy]
                        .copy_from_slice(&buf.as_slice()[block_offset..block_offset + to_copy]);

                    self.offset += to_copy;
                    rem -= to_copy;
                    dest_off += to_copy;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        buffer_size - rem
    }
}
