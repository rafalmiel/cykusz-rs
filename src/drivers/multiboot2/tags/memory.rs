use drivers::multiboot2::tags::Tag;

#[repr(C)]
pub struct Memory {
    pub tag:            Tag,
    pub entry_size:     u32,
    pub entry_ver:      u32,
    pub first_entry:    MemoryEntry
}

#[repr(C)]
pub struct MemoryEntry {
    pub base_addr:      u64,
    pub length:         u64,
    pub typ:            u32,
    pub reserved:       u32
}

pub struct MemoryIter {
    current:    *const MemoryEntry,
    last:       *const MemoryEntry,
    entry_size: u32
}

impl Memory {
    pub fn entries(&self) -> MemoryIter {
        MemoryIter {
            current: (&self.first_entry) as *const _,
            last: ((self as *const _) as u64 + self.tag.size as u64 - self.entry_size as u64)
                as *const _,
            entry_size: self.entry_size
        }
    }
}

impl Iterator for MemoryIter {
    type Item = &'static MemoryEntry;

    fn next(&mut self) -> Option<&'static MemoryEntry> {
        if self.current > self.last {
            None
        } else {
            let entry = unsafe {
                &*self.current
            };

            self.current =
                (self.current as u64 + self.entry_size as u64)
                    as *const MemoryEntry;

            if entry.typ == 1 {
                Some(entry)
            } else {
                self.next()
            }
        }
    }
}
