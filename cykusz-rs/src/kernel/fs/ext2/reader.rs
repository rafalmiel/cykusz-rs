use crate::kernel::fs::ext2::disk::inode::INode;
use crate::kernel::fs::ext2::Ext2Filesystem;
use crate::kernel::utils::slice::ToBytesMut;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

pub struct INodeReader<'a> {
    inode: &'a INode,
    fs: Weak<Ext2Filesystem>,
    offset: usize,
}

impl<'a> INodeReader<'a> {
    pub fn new(inode: &'a INode, fs: Weak<Ext2Filesystem>, offset: usize) -> INodeReader<'a> {
        INodeReader::<'a> {
            inode,
            fs: fs.clone(),
            offset,
        }
    }

    fn fs(&self) -> Arc<Ext2Filesystem> {
        self.fs.upgrade().unwrap()
    }

    fn get_block(&self, mut block_num: usize) -> Option<usize> {
        if block_num < 12 {
            return Some(self.inode.direct_ptrs()[block_num] as usize);
        }

        let block_size = self.fs().superblock().block_size();

        block_num -= 12;

        let entries_per_block = block_size / core::mem::size_of::<u32>();

        let mut buf = Vec::<u32>::new();
        buf.resize(entries_per_block, 0);

        if block_num < entries_per_block {
            let ptr = self.inode.s_indir_ptr();

            if ptr == 0 {
                return None;
            }

            self.fs()
                .read_block(ptr as usize, buf.as_mut_slice().to_bytes_mut());

            return Some(buf[block_num] as usize);
        }

        let entries_per_dblock = entries_per_block * entries_per_block;

        if block_num < entries_per_dblock {
            let mut ptr = self.inode.d_indir_ptr();

            if ptr == 0 {
                return None;
            }

            self.fs()
                .read_block(ptr as usize, buf.as_mut_slice().to_bytes_mut());

            let off1 = block_num / entries_per_block;
            let off2 = block_num % entries_per_block;

            ptr = buf[off1];

            if ptr == 0 {
                return None;
            }

            self.fs()
                .read_block(ptr as usize, buf.as_mut_slice().to_bytes_mut());

            ptr = buf[off2];

            if ptr == 0 {
                return None;
            }

            return Some(ptr as usize);
        }

        let entried_per_tblock = entries_per_dblock * entries_per_block;

        if block_num < entried_per_tblock {
            let mut ptr = self.inode.t_indir_ptr();

            if ptr == 0 {
                return None;
            }

            self.fs()
                .read_block(ptr as usize, buf.as_mut_slice().to_bytes_mut());

            let off1 = block_num / entries_per_dblock;

            block_num = block_num - off1 * entries_per_block;

            let off2 = block_num / entries_per_block;
            let off3 = block_num % entries_per_block;

            self.fs()
                .read_block(ptr as usize, buf.as_mut_slice().to_bytes_mut());

            ptr = buf[off1];

            if ptr == 0 {
                return None;
            }

            self.fs()
                .read_block(ptr as usize, buf.as_mut_slice().to_bytes_mut());

            ptr = buf[off2];

            if ptr == 0 {
                return None;
            }

            self.fs()
                .read_block(ptr as usize, buf.as_mut_slice().to_bytes_mut());

            ptr = buf[off3];

            if ptr == 0 {
                return None;
            }

            return Some(ptr as usize);
        } else {
            None
        }
    }

    pub fn read(&mut self, dest: &mut [u8]) -> usize {
        use core::cmp::min;

        let file_size = self.inode.size_lower() as usize;

        if self.offset >= file_size {
            return 0;
        }

        let fs = self.fs();
        let block_size = fs.superblock().block_size();

        let buffer_size = min(file_size - self.offset, dest.len());
        let mut rem = buffer_size;

        let mut dest_off = 0;
        while rem > 0 && self.offset < file_size {
            let block_num = self.offset / block_size;
            let block_offset = self.offset % block_size;

            if let Some(block) = self.get_block(block_num) {
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
