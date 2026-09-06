use crate::arch::mm::phys::{MemZone, PhysPage, allocate_zone};
use crate::kernel::mm::virt::PageFlags;
use crate::kernel::mm::{Frame, PAGE_SIZE, VirtAddr, map_to_flags, unmap};
use alloc::vec::Vec;

const SLAB_COUNT: usize = 10; // 8 16 32 64 128 256 512 1024 2048 4096
const SLAB_SHIFT: usize = 3; // trailing_zeroes of 8 = 3
const MIN_ALLOC: usize = 8;
const MAX_ALLOC: usize = 4096;

/// Finds slab idx for this chunk_size, we assume chunk_size is already properly aligned
fn find_slab_idx(chunk_size: usize) -> Option<usize> {
    let idx = chunk_size.trailing_zeros() as usize - SLAB_SHIFT;

    Some(idx)
}

/// Calculate chunk size given the size, rounded up to the next power of two
fn chunk_size(mut size: usize) -> Option<usize> {
    size = core::cmp::max(size, MIN_ALLOC).next_power_of_two();

    (size <= MAX_ALLOC).then_some(size)
}

#[derive(Copy, Clone)]
struct Slab(&'static PhysPage);

impl From<&'static PhysPage> for Slab {
    fn from(value: &'static PhysPage) -> Self {
        Self(value)
    }
}

impl PartialEq for Slab {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.0, other.0)
    }
}

impl Slab {
    fn new(chunk_size: usize, zone: MemZone) -> Option<Self> {
        let frame = allocate_zone(zone)?;
        let phys = frame.address();

        let phys_page = frame.address().to_phys_page()?;

        let (head_addr, free_count) = {
            let mut page = phys_page.lock_pt();
            let slab_info = page.as_slab_meta();
            slab_info.free_head = phys.to_virt();
            slab_info.free_count = (PAGE_SIZE / chunk_size) as u16;
            slab_info.chunk_size = chunk_size as u16;
            slab_info.mem_zone = zone;

            (slab_info.free_head, slab_info.free_count)
        };
        match zone {
            MemZone::ZoneDma | MemZone::ZoneDma32 => {
                // For dma access we map page as NO_CACHE
                map_to_flags(
                    phys.to_virt(),
                    phys,
                    PageFlags::WRITABLE | PageFlags::NO_CACHE,
                );
            }
            MemZone::ZoneNormal => {
                map_to_flags(phys.to_virt(), phys, PageFlags::WRITABLE);
            }
        }

        // Form a linked list of free chunks
        for (idx, addr) in (head_addr..(head_addr + PAGE_SIZE))
            .step_by(chunk_size)
            .enumerate()
        {
            if idx < free_count as usize - 1 {
                unsafe {
                    addr.store::<usize>(addr.0 + chunk_size);
                }
            } else {
                // this is the last element, set it point to null
                unsafe {
                    addr.store::<usize>(0usize);
                }
            }
        }

        Some(Self(phys_page))
    }

    fn phys_page(&self) -> &'static PhysPage {
        self.0
    }
}

pub struct SlabAlloc {
    // Slabs that are full for each SLAB_COUNT (free_count == 0)
    full_slabs: [[Vec<Slab>; 3]; SLAB_COUNT],

    // Free slabs for each MemZone (3) and slab count (SLAB_COUNT)
    free_slabs: [[Vec<Slab>; 3]; SLAB_COUNT],
}

impl SlabAlloc {
    pub fn new() -> Self {
        let full_slabs = core::array::from_fn(|_| core::array::from_fn(|_| Vec::new()));
        let free_slabs = core::array::from_fn(|_| core::array::from_fn(|_| Vec::new()));

        Self {
            full_slabs,
            free_slabs,
        }
    }

    pub fn allocate_zone(&mut self, size: usize, zone: MemZone) -> Option<VirtAddr> {
        let chunk_size = chunk_size(size)?;
        let idx = find_slab_idx(chunk_size)?;

        if self.free_slabs[idx][zone as usize].is_empty() {
            // There are no free slabs available, so allocate a new one for this chunk_size and zone
            let new_slab = Slab::new(chunk_size, zone)?;

            self.free_slabs[idx][zone as usize].push(new_slab);
        }

        let free_slabs = &mut self.free_slabs[idx][zone as usize];

        let slab = free_slabs.last()?;

        let res = {
            let mut page = slab.phys_page().lock_pt();
            let slab_meta = page.as_slab_meta();
            assert!(slab_meta.free_count > 0);

            // free_head is our result
            let res = slab_meta.free_head;
            slab_meta.free_count -= 1;
            unsafe {
                // Update head with the next pointer
                slab_meta.free_head = res.read::<VirtAddr>();
            }

            if slab_meta.free_count == 0 {
                // If it was a last chunk, add that slab to the full slabs list
                self.full_slabs[idx][zone as usize].push(*slab);

                // And remove from free slabs list
                free_slabs.pop();
            }

            res
        };

        Some(res)
    }

    pub fn free(&mut self, addr: VirtAddr) -> Option<()> {
        let phys_page = addr.to_phys().to_phys_page()?;
        let slab: Slab = phys_page.into();

        let (should_dealloc, idx, mem_zone) = {
            let mut phys = phys_page.lock_pt();
            let slab_meta = phys.as_slab_meta();

            let idx = find_slab_idx(slab_meta.chunk_size as usize)?;

            let total_count = PAGE_SIZE / slab_meta.chunk_size as usize;

            // Was the slab full before deallocation?
            let was_full = slab_meta.free_count == 0;

            unsafe {
                // Update next pointer to the head of the slab
                addr.store::<VirtAddr>(slab_meta.free_head);
            }
            // This chunk is now head
            slab_meta.free_head = addr;
            slab_meta.free_count += 1;

            if was_full {
                // If the slab was full, now it isn't! Remove it from full_slabs list
                // and add it to the free_slabs one
                let full_slabs = &mut self.full_slabs[idx][slab_meta.mem_zone as usize];
                let pos = full_slabs.iter().position(|&e| e == slab)?;

                full_slabs.swap_remove(pos);

                self.free_slabs[idx][slab_meta.mem_zone as usize].push(phys_page.into());
            }

            (
                slab_meta.free_count == total_count as u16,
                idx,
                slab_meta.mem_zone,
            )
        };

        if should_dealloc {
            // If the slab is empty we return it to the buddy allocator
            let free_slabs = &mut self.free_slabs[idx][mem_zone as usize];

            let pos = free_slabs
                .iter()
                .position(|&e| e == slab)
                .expect("Could not find a PhysPage pos in free_slabs");

            // Remove it from the free_slabs list
            free_slabs.swap_remove(pos);

            let phys_addr = phys_page.to_phys_addr();
            // Unmap the page
            unmap(phys_addr.to_virt());
            // Finally - deallocate the frame
            crate::kernel::mm::deallocate(&Frame::new(phys_addr));
        }

        Some(())
    }
}
