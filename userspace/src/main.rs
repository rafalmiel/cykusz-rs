#![no_std]
#![no_main]
#![feature(asm)]
#![feature(lang_items)]

extern crate rlibc;
extern crate syscall_defs;
#[macro_use]
extern crate syscall_user as syscall;

use syscall_defs::{OpenFlags, SysDirEntry};

pub mod file;
pub mod lang;

fn make_str(buf: &[u8]) -> &str {
    core::str::from_utf8(&buf)
        .expect("Invalid UTF-8 string")
        .trim_end_matches("\n")
}

fn ls(path: &str) {
    if let Ok(fd) = syscall::open(path, OpenFlags::DIRECTORY) {
        let mut buf = [0u8; 1024];

        loop {
            if let Ok(datalen) = syscall::getdents(fd, buf.as_mut_ptr(), buf.len()) {
                if datalen == 0 {
                    break;
                }

                let mut offset = 0;
                let struct_len = core::mem::size_of::<SysDirEntry>();

                loop {
                    let dentry = unsafe { &*(buf.as_ptr().offset(offset) as *const SysDirEntry) };
                    let namebytes = unsafe {
                        core::slice::from_raw_parts(
                            dentry.name.as_ptr(),
                            dentry.reclen - struct_len,
                        )
                    };
                    if let Ok(name) = core::str::from_utf8(namebytes) {
                        println!("{:<12} {:?}", name, dentry.typ);
                    } else {
                        break;
                    }

                    offset += dentry.reclen as isize;
                    if offset as usize >= datalen {
                        break;
                    }
                }
            }
        }

        if let Err(_) = syscall::close(fd) {
            println!("Failed to close file {}", fd);
        }
    } else {
        println!("Failed to open a directory")
    }
}

fn exec(cmd: &str) {
    if cmd.starts_with("cd ") {
        let path = &cmd[3..];

        if let Err(e) = syscall::chdir(path.trim()) {
            println!("Failed to change dir: {:?}", e);
        }
    } else if cmd.starts_with("mkdir ") {
        let path = &cmd[6..];

        for p in path.split_whitespace() {
            if let Err(e) = syscall::mkdir(p.trim()) {
                println!("Failed to mkdir: {:?}", e);
            }
        }
    } else if cmd.starts_with("ls ") {
        let path = &cmd[3..];
        let p = path.split_whitespace();
        let print_hdr = path.contains(" ");
        for (idx, name) in p.enumerate() {
            let name = name.trim();
            if print_hdr {
                if idx > 0 {
                    println!("");
                }
                println!("{}:", name);
            }
            ls(name);
        }
    } else if cmd == "ls" {
        ls(".")
    } else if cmd == "cd" {
        // do nothing for now, TODO: move to home?
    } else if cmd == "exit" {
        syscall::exit();
    } else {
        println!(
            "shell: {}: command not found",
            cmd.split(" ").nth(0).unwrap()
        );
    }
}

fn main_cd() -> ! {
    println!("Shell starting...");
    loop {
        let mut buf = [0u8; 256];
        let mut pwd = [0u8; 1024];
        let pwd_r = syscall::getcwd(pwd.as_mut_ptr(), pwd.len()).expect("Failed to get cwd");
        let pwd_str = make_str(&pwd[0..pwd_r]);

        let mut to_p = pwd_str.split("/").last().unwrap();
        to_p = if to_p == "" { "/" } else { to_p };

        print!("[root {}]# ", to_p);

        let r = syscall::read(1, buf.as_mut_ptr(), buf.len()).unwrap();

        let cmd = make_str(&buf[..r]).trim();

        if cmd == "pwd" {
            println!("{}", pwd_str);
        } else {
            exec(cmd);
        }
    }
}

#[allow(dead_code)]
fn main() -> ! {
    use file::*;

    loop {
        syscall::chdir("dev").expect("Failed to change dir");

        // We are not allowed to exit yet, need to implement exit system call
        let mut buf = [0u8; 256];

        let mut pwd = [0u8; 16];
        let pwd_r = syscall::getcwd(pwd.as_mut_ptr(), pwd.len()).expect("Failed to get cwd");

        print!("[root {}]# ", make_str(&pwd[0..pwd_r]));

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
