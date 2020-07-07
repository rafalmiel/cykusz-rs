pub struct File {
    fd: usize,
}

impl File {
    pub fn new(path: &str, flags: syscall_defs::OpenFlags) -> File {
        File {
            fd: syscall::open(path, flags).expect("Failed to open file"),
        }
    }

    pub fn read(&self, buf: &mut [u8]) -> usize {
        syscall::read(self.fd, buf).expect("Failed to read a file")
    }

    pub fn write(&self, buf: &[u8]) -> usize {
        syscall::write(self.fd, buf).expect("Failed to write to file")
    }
}

impl Drop for File {
    fn drop(&mut self) {
        syscall::close(self.fd).expect("Failed to close a file");
    }
}
