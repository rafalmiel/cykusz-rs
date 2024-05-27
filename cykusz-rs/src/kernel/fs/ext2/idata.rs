use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ops::Index;

use crate::kernel::fs::ext2::buf_block::{BufBlock, SliceBlock};
use crate::kernel::fs::ext2::disk::inode::{FileType, INode};
use crate::kernel::fs::ext2::inode::LockedExt2INode;
use crate::kernel::fs::ext2::Ext2Filesystem;
use crate::kernel::fs::vfs::{FsError, Result};
use crate::kernel::sched::current_task_ref;
use crate::kernel::utils::slice::{ToBytes, ToBytesMut};
use crate::kernel::utils::types::{Align, CeilDiv};

struct Offsets {
    vec: Vec<usize>,
}

impl Offsets {
    fn new() -> Offsets {
        Offsets {
            vec: Vec::with_capacity(4),
        }
    }

    fn slice(&self) -> &[usize] {
        self.vec.as_slice()
    }

    fn len(&self) -> usize {
        self.vec.len()
    }

    fn add(&mut self, offset: usize) {
        self.vec.push(offset);
    }
}

impl Index<usize> for Offsets {
    type Output = usize;

    fn index(&self, index: usize) -> &Self::Output {
        &self.vec[index]
    }
}

pub struct INodeData {
    inode: Arc<LockedExt2INode>,
    offset: usize,
    synced: bool,
}

impl INodeData {
    pub fn new(inode: Arc<LockedExt2INode>, offset: usize) -> INodeData {
        INodeData {
            inode,
            offset,
            synced: false,
        }
    }

    pub fn new_synced(inode: Arc<LockedExt2INode>, offset: usize, synced: bool) -> INodeData {
        INodeData {
            inode,
            offset,
            synced,
        }
    }

    fn fs(&self) -> Arc<Ext2Filesystem> {
        self.inode.ext2_fs()
    }

    fn get_offsets(mut block_num: usize, block_size: usize) -> Offsets {
        let mut offsets = Offsets::new();

        let entries_per_block = block_size / core::mem::size_of::<u32>();

        if block_num < 12 {
            offsets.add(block_num);

            return offsets;
        }

        block_num -= 12;

        if block_num < entries_per_block {
            offsets.add(12);
            offsets.add(block_num);

            return offsets;
        }

        block_num -= entries_per_block;

        let entries_per_d_block = entries_per_block * entries_per_block;

        if block_num < entries_per_d_block {
            offsets.add(13);
            offsets.add(block_num / entries_per_block);
            offsets.add(block_num % entries_per_block);

            return offsets;
        }

        block_num -= entries_per_d_block;

        let entries_per_t_block = entries_per_d_block * entries_per_block;

        if block_num < entries_per_t_block {
            offsets.add(14);
            let off1 = block_num / entries_per_d_block;

            offsets.add(off1);

            block_num = block_num - off1 * entries_per_d_block;

            offsets.add(block_num / entries_per_block);
            offsets.add(block_num % entries_per_block);

            return offsets;
        } else {
            panic!("Block num too big?");
        }
    }

    #[allow(unused)]
    fn next_block_num(&self) -> usize {
        let fs = self.fs();
        let sb = fs.superblock();

        let inode = self.inode.read_debug(21);
        let inode = inode.d_inode();

        let block_size = sb.block_size();

        (inode.size_lower() as usize).ceil_div(block_size)
    }

    pub fn alloc_block_at(&mut self, block_num: usize) -> Option<BufBlock> {
        let fs = self.fs();
        let sb = fs.superblock();

        let mut inode = self.inode.d_inode_writer();
        let id = inode.id();

        if let Some(new_block) = fs.alloc_block(id) {
            logln_disabled!("allocated block {}", new_block.block());
            if let Some(new_blocks) = self.set_block(block_num, new_block.block(), id, &mut inode) {
                inode.inc_sector_count(new_blocks as u32 * sb.sectors_per_block() as u32);

                drop(inode);

                Some(new_block)
            } else {
                drop(inode);

                fs.group_descs().free_block_ptr(new_block.block() as usize);

                None
            }
        } else {
            None
        }
    }

    pub fn append_block(&mut self, inc_size: usize) -> Option<BufBlock> {
        let fs = self.fs();
        let sb = fs.superblock();

        let size = self.inode.read_debug(22).d_inode().size_lower();

        let block_size = sb.block_size();

        let next_block_num = (size as usize).ceil_div(block_size);

        if let Some(new_block) = self.alloc_block_at(next_block_num) {
            let mut inode = self.inode.d_inode_writer();
            inode.inc_size_lower(inc_size as u32);

            drop(inode);

            Some(new_block)
        } else {
            None
        }
    }

    fn revert(&self, inode: &mut INode, offsets: &[usize], ptr_vec: &[usize]) {
        let fs = self.fs();

        let mut cur = offsets.len() - 1;

        let mut delete = offsets[cur] == 0;

        while delete && cur > 0 {
            cur -= 1;

            fs.group_descs().free_block_ptr(ptr_vec[cur]);

            delete = offsets[cur] == 0;
        }

        if cur == 0 {
            inode.block_ptrs_mut()[offsets[cur]] = 0;
        } else {
            let mut b = fs.make_slice_buf_from::<u32>(ptr_vec[cur - 1]);

            b.slice_mut()[offsets[cur - 1]] = 0;

            fs.write_block(ptr_vec[cur - 1], b.slice().to_bytes());
        }
    }

    fn set_block(
        &self,
        block_num: usize,
        val: usize,
        inode_id: usize,
        inode: &mut INode,
    ) -> Option<usize> {
        let fs = self.fs();

        let block_size = fs.superblock().block_size();

        let offsets = Self::get_offsets(block_num, block_size);

        let last = offsets.len() - 1;

        let mut ptr = 0;
        let mut ptr_vec = Vec::<usize>::new();

        let mut buf = SliceBlock::<u32>::empty();

        let mut new_blocks = 0;

        let mut was_alloc = false;

        for (i, &o) in offsets.slice().iter().enumerate() {
            let ptrs = if i == 0 {
                inode.block_ptrs_mut()
            } else {
                if buf.is_empty() {
                    buf.alloc(block_size / core::mem::size_of::<u32>());
                }

                if !was_alloc {
                    fs.read_block(ptr, buf.slice_mut().to_bytes_mut())
                        .expect("Failed to read block");
                } else {
                    buf.slice_mut().iter_mut().for_each(|el| {
                        *el = 0u32;
                    })
                }
                buf.slice_mut()
            };

            let sync = if last == i {
                if ptrs[o] != 0 {
                    logln!("!!!INode corrupted, ptrs: {:?}", ptrs);
                    logln!("INode {} corrupted: {:?}", inode_id, inode);
                    logln!(
                        "block_num: {}, offsets: {:?}, i: {}, o: {}",
                        block_num,
                        offsets.slice(),
                        i,
                        o
                    );
                    panic!("INode corrupted");
                }
                ptrs[o] = val as u32;

                was_alloc = false;

                new_blocks += 1;

                i > 0
            } else {
                if ptrs[o] == 0 {
                    if let Some(p) = { fs.group_descs().alloc_block_ptr(inode_id) } {
                        ptrs[o] = p as u32;

                        new_blocks += 1;

                        was_alloc = true;

                        i > 0
                    } else {
                        // Allocation failed, Revert...
                        self.revert(inode, &offsets.vec[..=i], &ptr_vec);

                        return None;
                    }
                } else {
                    was_alloc = false;
                    false
                }
            };

            let prev_ptr = ptr;

            ptr = ptrs[o] as usize;

            ptr_vec.push(ptr);

            if sync {
                fs.write_block(prev_ptr, buf.slice().to_bytes());
            }
        }

        Some(new_blocks)
    }

    fn get_block(&self, block_num: usize, d_inode: &INode) -> Option<usize> {
        let fs = self.fs();

        let block_size = fs.superblock().block_size();

        let offsets = Self::get_offsets(block_num, block_size);

        let mut ptr = 0;

        let mut buf = fs.make_slice_buf::<u32>();

        for (i, &o) in offsets.slice().iter().enumerate() {
            if i == 0 {
                assert!(o < 15);

                ptr = d_inode.block_ptrs()[o];
            } else {
                fs.read_block(ptr as usize, buf.slice_mut().to_bytes_mut())
                    .expect("Read failed");

                ptr = buf.slice()[o];
            }

            if ptr == 0 {
                return None;
            }
        }

        Some(ptr as usize)
    }

    pub fn read_block_at(&mut self, block_num: usize) -> Option<BufBlock> {
        let linode = self.inode.read_debug(23);
        let inode = linode.d_inode();

        let file_size = inode.size_lower() as usize;

        let fs = self.fs();

        if inode.ftype() == FileType::Symlink && file_size <= 60 {
            return None;
        }

        if let Some(ptr) = self.get_block(block_num, inode) {
            let buf = fs.make_buf_from(ptr);

            Some(buf)
        } else {
            None
        }
    }

    pub fn read_next_block(&mut self) -> Option<BufBlock> {
        let linode = self.inode.read_debug(24);
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
            if current_task_ref().locks() > 0 {
                logln!("idata read next: locks > 0");
            }
            fs.read_block(ptr, buf.slice_mut());
            buf.set_block(ptr);

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

            let mut buf = Vec::<u8>::new();
            buf.resize(block_size, 0);

            if let Some(block) = self.get_block(block_num, inode) {
                if current_task_ref().locks() > 0 {
                    logln!("idata read: locks > 0");
                }
                if let None = fs.read_block(block, buf.as_mut_slice()) {
                    break;
                }
            }

            let to_copy = min(rem, block_size - block_offset);

            dest[dest_off..dest_off + to_copy]
                .copy_from_slice(&buf.as_slice()[block_offset..block_offset + to_copy]);

            self.offset += to_copy;
            rem -= to_copy;
            dest_off += to_copy;
        }

        buffer_size - rem
    }

    pub fn write(&mut self, mut data: &[u8], allow_grow: bool) -> Result<usize> {
        let linode = self.inode.read_debug(26);
        let inode = linode.d_inode();

        let mut file_size = inode.size_lower() as usize;
        let file_type = inode.ftype();

        drop(linode);

        if file_type == FileType::Symlink && self.offset + data.len() <= 60 {
            let mut writer = self.inode.d_inode_writer();

            writer.block_ptrs_mut().to_bytes_mut()[self.offset..self.offset + data.len()]
                .copy_from_slice(&data);
            self.offset += data.len();

            writer.set_size_lower(self.offset as u32);

            return Ok(data.len());
        }

        if self.offset > file_size {
            return Err(FsError::InvalidParam);
        }

        data = if !allow_grow && self.offset + data.len() > file_size {
            // we just update the file content so write up to the size of it
            &data[..file_size - self.offset]
        } else {
            data
        };

        let fs = self.fs();
        let block_size = fs.superblock().block_size();

        let mut rem = data.len();

        while rem > 0 {
            let block_num = self.offset / block_size;
            let block_offset = self.offset % block_size;

            if let Some(mut block) = if block_offset == 0 && file_size == self.offset {
                assert!(allow_grow);
                self.append_block(core::cmp::min(block_size, rem))
            } else {
                if let Some(block) = self.read_block_at(block_num) {
                    Some(block)
                } else {
                    self.alloc_block_at(block_num)
                }
            } {
                let to_write = core::cmp::min(block_size - block_offset, rem);
                let data_offset = data.len() - rem;

                block.slice_mut()[block_offset..block_offset + to_write]
                    .copy_from_slice(&data[data_offset..data_offset + to_write]);

                fs.write_block_sync(block.block(), block.slice(), self.synced);

                self.offset += to_write;

                if self.offset > file_size {
                    let mut writer = self.inode.d_inode_writer();

                    writer.set_size_lower(self.offset as u32);
                    file_size = self.offset;
                }

                rem -= to_write;
            } else {
                break;
            }
        }

        Ok(data.len() - rem)
    }
}
