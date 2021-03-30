pub struct File {
    fd: usize,
}

impl File {
    pub fn new(path: &str, flags: syscall_defs::OpenFlags) -> Option<File> {
        match syscall::open(path, flags) {
            Ok(fd) => Some(File { fd }),
            Err(e) => {
                println!("Failed to open file: {:?}", e);
                None
            }
        }
    }

    pub fn read(&self, buf: &mut [u8]) -> usize {
        match syscall::read(self.fd, buf) {
            Ok(s) => s,
            Err(e) => {
                println!("File read failed: {:?}", e);
                0
            }
        }
    }

    pub fn write(&self, buf: &[u8]) -> usize {
        match syscall::write(self.fd, buf) {
            Ok(s) => s,
            Err(e) => {
                println!("File write failed: {:?}", e);
                0
            }
        }
    }
}

impl Drop for File {
    fn drop(&mut self) {
        syscall::close(self.fd).expect("Failed to close a file");
    }
}
