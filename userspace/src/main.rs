#![no_std]
#![no_main]
#![feature(llvm_asm)]
#![feature(lang_items)]

extern crate rlibc;
extern crate syscall_defs;
#[macro_use]
extern crate syscall_user as syscall;

use syscall_defs::{OpenFlags, SysDirEntry};

pub mod file;
pub mod lang;
pub mod nc;

fn make_str(buf: &[u8]) -> &str {
    core::str::from_utf8(&buf)
        .expect("Invalid UTF-8 string")
        .trim_end_matches("\n")
}

fn ls(path: &str) {
    if let Ok(fd) = syscall::open(path, OpenFlags::DIRECTORY) {
        let mut buf = [0u8; 1024];

        loop {
            if let Ok(datalen) = syscall::getdents(fd, &mut buf) {
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
            } else {
                println!("Failed to get dents");
                break;
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
    } else if cmd.starts_with("host ") {
        let name = &cmd[5..];

        let mut res = [0u8; 4];

        if let Ok(_) = syscall::getaddrinfo(name, &mut res) {
            println!("{:?}", res);
        } else {
            println!("getaddrinfo failed");
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
    } else if cmd.starts_with("sleep") {
        let mut args = cmd.split_ascii_whitespace();
        let ms = if let Some(arg) = args.nth(1) {
            if let Ok(ms) = arg.parse::<usize>() {
                ms
            } else {
                3000
            }
        } else {
            3000
        };

        if let Err(e) = syscall::sleep(ms) {
            println!("Sleep failed.. {:?}", e);
        }
    } else if cmd == "poweroff" {
        syscall::poweroff();
    } else if cmd == "reboot" {
        if let Err(e) = syscall::reboot() {
            println!("Reboot failed.. {:?}", e);
        }
    } else if cmd.starts_with("bind") {
        if cmd.len() < 6 {
            println!("Format: bind <port>");
            return;
        }

        if let Ok(port) = cmd[5..].parse::<u32>() {
            nc::bind(port);
        }
    } else if cmd.starts_with("connect") {
        let mut args = cmd.split_ascii_whitespace();

        if args.clone().count() >= 3 {
            let addr = args.nth(1).unwrap();
            let port = args.next().unwrap();

            let mut ip = [0u8; 4];
            let mut cnt = 0;
            for (i, s) in addr.split(".").enumerate() {
                if let Ok(v) = s.parse::<u8>() {
                    ip[i] = v;
                } else {
                    break;
                }
                cnt += 1;
            }

            if cnt < 4 {
                if let Err(e) = syscall::getaddrinfo(addr, &mut ip) {
                    println!("Failed to get host address {:?}", e);
                    return;
                }
            }

            if let Ok(port) = port.parse::<u32>() {
                nc::connect(port, &ip);
            } else {
                println!("Failed to parse port");
            }
        }
    } else if cmd.starts_with("mount ") {
        let mut split = cmd.split_whitespace();
        split.next();

        if let (Some(dev), Some(dest)) = { (split.next(), split.next()) } {
            if let Err(e) = syscall_user::mount(dev, dest, "ext2") {
                println!("Mount failed: {:?}", e);
            }
        } else {
            println!("Param err");
        }
    } else if cmd.starts_with("umount ") {
        let mut split = cmd.split_whitespace();
        split.next();

        if let Some(path) = split.next() {
            if let Err(e) = syscall_user::umount(path) {
                println!("Umount failed: {:?}", e);
            }
        } else {
            println!("Param error");
        }
    } else if cmd.starts_with("cat ") {
        let mut split = cmd.split_whitespace();
        split.next();

        if let Some(path) = split.next() {
            if let Some(file) = file::File::new(path, syscall_defs::OpenFlags::RDONLY) {
                let mut buf = [0u8; 16];
                let mut read = file.read(&mut buf);

                while read > 0 {
                    print!("{}", unsafe {
                        core::str::from_utf8_unchecked(&buf[..read])
                    });

                    read = file.read(&mut buf);
                }
            }
        } else {
            println!("Param error");
        }
    } else if cmd.is_empty() {
        return;
    } else {
        println!(
            "shell: {}: command not found",
            cmd.split(" ").nth(0).unwrap()
        );
    }
}

fn main_cd() -> ! {
    loop {
        let mut buf = [0u8; 256];
        let mut pwd = [0u8; 1024];
        let pwd_r = syscall::getcwd(&mut pwd).expect("Failed to get cwd");
        let pwd_str = make_str(&pwd[0..pwd_r]);

        let mut to_p = pwd_str.split("/").last().unwrap();
        to_p = if to_p == "" { "/" } else { to_p };

        print!("[root {}]# ", to_p);

        let r = syscall::read(1, &mut buf).unwrap();

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
        let pwd_r = syscall::getcwd(&mut pwd).expect("Failed to get cwd");

        print!("[root {}]# ", make_str(&pwd[0..pwd_r]));

        // Read some data from stdin
        let r = syscall::read(1, &mut buf).unwrap();

        {
            // Write data from stdin into the file
            File::new("test_file", OpenFlags::WRONLY | OpenFlags::CREAT)
                .unwrap()
                .write(&buf[..r]);
        }

        unsafe {
            // Zero out buffer
            buf.as_mut_ptr().write_bytes(0, buf.len());
        }

        {
            // Read data from the file and print the result
            let read = File::new("test_file", OpenFlags::RDONLY)
                .unwrap()
                .read(&mut buf);

            let s = make_str(&buf[..read]);

            println!("> read {} bytes: {}]", read, s);
        }

        syscall::chdir("../").expect("Failed to change dir");
    }
}
