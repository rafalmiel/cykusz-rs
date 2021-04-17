const BUFFER_SIZE: usize = 256;

pub(crate) struct InputBuffer {
    data: [u8; BUFFER_SIZE],
    e: u32,
    w: u32,
    r: u32,
}

impl InputBuffer {
    pub(crate) const fn new() -> InputBuffer {
        InputBuffer {
            data: [0u8; BUFFER_SIZE],
            e: 0,
            w: 0,
            r: 0,
        }
    }

    pub(crate) fn put_char(&mut self, data: u8) {
        if (self.e + 1) % BUFFER_SIZE as u32 != self.r {
            self.data[self.e as usize] = data;
            self.e = (self.e + 1) % BUFFER_SIZE as u32;
        }
    }

    pub(crate) fn remove_all_edit(&mut self) -> usize {
        let edit_size = if self.e < self.w {
            BUFFER_SIZE as u32 - (self.w - self.e)
        } else {
            self.e - self.w
        };

        self.e = self.w;

        edit_size as usize
    }

    pub(crate) fn remove_last_n(&mut self, n: usize) -> usize {
        let mut remaining = n;

        while self.e != self.w && remaining > 0 {
            self.e = if self.e == 0 {
                BUFFER_SIZE as u32 - 1
            } else {
                self.e - 1
            };

            remaining -= 1;
        }

        n - remaining
    }

    pub(crate) fn read(&mut self, buf: *mut u8, n: usize) -> usize {
        let mut remaining = n;
        let mut store = buf;

        while self.r != self.w && remaining > 0 {
            unsafe {
                *store = self.data[self.r as usize];

                store = store.offset(1);
            }

            remaining -= 1;
            self.r = (self.r + 1) % BUFFER_SIZE as u32;
        }

        n - remaining
    }

    pub(crate) fn commit_write(&mut self) {
        self.w = self.e;
    }

    pub(crate) fn has_data(&self) -> bool {
        self.r != self.w
    }
}
