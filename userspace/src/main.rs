#![no_std]
#![no_main]
#![feature(asm)]
#![feature(lang_items)]

extern crate rlibc;
extern crate syscall_defs;
#[macro_use]
extern crate syscall_user as syscall;

use syscall_defs::OpenFlags;

pub mod file;
pub mod lang;

fn make_str(buf: &[u8]) -> &str {
    core::str::from_utf8(&buf)
        .expect("Invalid UTF-8 string")
        .trim_end_matches("\n")
}

fn main() -> ! {
    use file::*;

    loop {
        syscall::chdir("dev").expect("Failed to change dir");

        // We are not allowed to exit yet, need to implement exit system call
        let mut buf = [0u8; 256];

        print!("[root /]# ");

        // Read some data from stdin
        let r = syscall::read(1, buf.as_mut_ptr(), buf.len()).unwrap();

        {
            // Write data from stdin into the file
            File::new("test_file", OpenFlags::WRONLY | OpenFlags::CREAT).write(&buf[..r]);
        }

        unsafe {
            // Zero out buffer
            buf.as_mut_ptr().write_bytes(0, buf.len());
        }

        {
            // Read data from the file and print the result
            let read = File::new("test_file", OpenFlags::RDONLY).read(&mut buf);

            let s = make_str(&buf[..read]);

            println!("> read {} bytes: {}]", read, s);
        }

        syscall::chdir("../").expect("Failed to change dir");
    }
}
