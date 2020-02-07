pub struct File {
    fd: usize,
}

impl File {
    pub fn new_readonly(path: &str) -> File {
        File {
            fd: syscall::open(path, true).expect("Failed to open file"),
        }
    }

    pub fn new_writeonly(path: &str) -> File {
        File {
            fd: syscall::open(path, false).expect("Failed to open file"),
        }
    }

    pub fn read(&self, buf: &mut [u8]) -> usize {
        syscall::read(self.fd, buf.as_mut_ptr(), buf.len()).expect("Failed to read a file")
    }

    pub fn write(&self, buf: &[u8]) -> usize {
        syscall::write(self.fd, buf.as_ptr(), buf.len()).expect("Failed to write to file")
    }
}

impl Drop for File {
    fn drop(&mut self) {
        syscall::close(self.fd).expect("Failed to close a file");
    }
}
