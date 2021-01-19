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

        let mut inode = self.inode.d_inode_writer();
        let id = inode.id();

        let block_size = self.fs().superblock().block_size();

        let next_block_num = (inode.size_lower() as usize).ceil_div(block_size);

        if let Some(new_block) = fs.alloc_block(id) {
            let new_blocks = self.set_block(next_block_num, new_block.block(), id, &mut inode)?;

            inode.inc_size_lower(block_size as u32);
            inode.inc_sector_count(new_blocks as u32 * sb.sectors_per_block() as u32);

            drop(inode);

            Some(new_block)
        } else {
            None
        }
    }

    fn set_block(
        &self,
        mut block_num: usize,
        ptr: usize,
        inode_id: usize,
        inode: &mut INode,
    ) -> Option<usize> {
        let mut new_blocks = 0;
        if block_num < 12 {
            assert_eq!(inode.direct_ptrs()[block_num], 0);
            inode.direct_ptrs_mut()[block_num] = ptr as u32;
            new_blocks += 1;
            return Some(new_blocks);
        }

        let block_size = self.fs().superblock().block_size();

        block_num -= 12;

        let entries_per_block = block_size / core::mem::size_of::<u32>();

        let mut revert_ptrs = Vec::<usize>::with_capacity(4);

        let mut buf = self.fs().make_slice_buf::<u32>();

        let mut set_ptrs = |mut ptr: usize,
                            offsets: &[usize],
                            val: usize,
                            reverts: &mut Vec<usize>|
         -> Option<usize> {
            let mut new_blocks = 0;
            if ptr == 0 {
                return None;
            }

            let result: Option<usize> = try {
                for (i, &offset) in offsets.iter().enumerate() {
                    self.fs().read_block(ptr, buf.slice_mut().to_bytes_mut())?;

                    if i == offsets.len() - 1 {
                        assert_eq!(buf.slice()[offset], 0);
                        buf.slice_mut()[offset] = val as u32;

                        new_blocks += 1;

                        self.fs()
                            .write_block(ptr, buf.slice().to_bytes())
                            .expect("Write block failed");
                    } else {
                        if buf.slice()[offset] as usize == 0 {
                            let p = self.fs().group_descs().alloc_block_ptr(inode_id)?;

                            reverts.push(p);

                            new_blocks += 1;
                            buf.slice_mut()[offset] = p as u32;

                            self.fs()
                                .write_block(ptr, buf.slice().to_bytes())
                                .expect("Write block failed");
                        }

                        ptr = buf.slice()[offset] as usize;
                    }
                }

                new_blocks
            };

            if result.is_none() {
                let fs = self.fs();
                let bg = fs.group_descs();

                for &ptr in reverts.iter() {
                    bg.free_block_ptr(ptr);
                }
            }

            result
        };

        if block_num < entries_per_block {
            if inode.s_indir_ptr() == 0 {
                let id = self.fs().group_descs().alloc_block_ptr(inode_id)?;

                revert_ptrs.push(id);

                new_blocks += 1;
                inode.set_s_indir_ptr(id as u32);
            }

            new_blocks += set_ptrs(
                inode.s_indir_ptr() as usize,
                &[block_num],
                ptr,
                &mut revert_ptrs,
            )?;
            return Some(new_blocks);
        }

        let entries_per_dblock = entries_per_block * entries_per_block;

        if block_num < entries_per_dblock {
            let off1 = block_num / entries_per_block;
            let off2 = block_num % entries_per_block;

            if inode.d_indir_ptr() == 0 {
                let id = self.fs().group_descs().alloc_block_ptr(inode_id)?;

                revert_ptrs.push(id);

                new_blocks += 1;
                inode.set_d_indir_ptr(id as u32);
            }

            new_blocks += set_ptrs(
                inode.d_indir_ptr() as usize,
                &[off1, off2],
                ptr,
                &mut revert_ptrs,
            )?;
            return Some(new_blocks);
        }

        let entried_per_tblock = entries_per_dblock * entries_per_block;

        if block_num < entried_per_tblock {
            let off1 = block_num / entries_per_dblock;

            block_num = block_num - off1 * entries_per_dblock;

            let off2 = block_num / entries_per_block;
            let off3 = block_num % entries_per_block;

            if inode.t_indir_ptr() == 0 {
                let id = self.fs().group_descs().alloc_block_ptr(inode_id)?;

                revert_ptrs.push(id);

                new_blocks += 1;
                inode.set_t_indir_ptr(id as u32);
            }

            new_blocks += set_ptrs(
                inode.t_indir_ptr() as usize,
                &[off1, off2, off3],
                ptr,
                &mut revert_ptrs,
            )?;
        }

        return Some(new_blocks);
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
