#![no_std]
#![no_main]
#![feature(thread_local)]

extern crate alloc;
extern crate program;
extern crate syscall_defs;
#[macro_use]
extern crate syscall_user as syscall;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use chrono::{Datelike, Timelike};
use syscall::bochs;

use syscall_defs::{MMapFlags, MMapProt, OpenFlags, SysDirEntry, SyscallError};

use crate::file::File;
use syscall_defs::signal::{SigAction, SignalFlags, SignalHandler};

pub mod file;
pub mod nc;

#[thread_local]
pub static mut TEST: usize = 33;

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

#[allow(dead_code)]
fn base_10_bytes(mut n: u64, buf: &mut [u8]) -> &[u8] {
    if n == 0 {
        return b"0";
    }
    let mut i = 0;
    while n > 0 {
        buf[i] = (n % 10) as u8 + b'0';
        n /= 10;
        i += 1;
    }
    let slice = &mut buf[..i];
    slice.reverse();
    &*slice
}

fn get_tty_fd() -> usize {
    syscall::open("/dev/tty", OpenFlags::RDWR).expect("Failed to get tty fd")
}

struct Tty {
    fd: usize,
}

impl Tty {
    fn new() -> Tty {
        Tty { fd: get_tty_fd() }
    }

    fn close(&self) {
        syscall::close(self.fd).expect("Failed to close tty");
    }

    fn detach(&self) {
        syscall::ioctl(self.fd, syscall_defs::ioctl::tty::TIOCNOTTY, 0)
            .expect("Failed to detach tty");
    }

    fn attach(&self) {
        syscall::ioctl(self.fd, syscall_defs::ioctl::tty::TIOCSCTTY, 0)
            .expect("Failed to attach tty");
    }

    fn set_fg(&self, gid: usize) {
        syscall::ioctl(self.fd, syscall_defs::ioctl::tty::TIOCSPGRP, gid)
            .expect("Failed to set fg terminal group");
    }
}

impl Drop for Tty {
    fn drop(&mut self) {
        self.close();
    }
}

fn start_process(path: &str, args: Option<&[&str]>, env: Option<&[&str]>) {
    let tty = Tty::new();

    if let Ok(id) = syscall::fork() {
        if id == 0 {
            // Make the process a group leader
            if let Err(e) = syscall::setpgid(0, 0) {
                println!("setpgid failed {:?}", e);

                syscall::exit(0);
            }

            tty.set_fg(syscall::getpid().expect("Failed to get pid"));

            if let Err(e) = syscall::exec(path, args, env) {
                println!("shell: {:?}", e);
            } else {
                unreachable!();
            }

            syscall::exit(0);
        } else {
            if let Err(e) = syscall::setpgid(id, id) {
                println!("parent setpgid failed {:?}", e);
            }

            let mut status = 0u32;

            while let Err(SyscallError::EINTR) = syscall::waitpid(id, &mut status) {}

            if status != 0x200 {
                println!("shell: process exit with status: {:#x}", status);
            }

            tty.set_fg(syscall::getpid().expect("Failed to get pid"));
        }
    } else {
        println!("fork failed");
    }
}

fn exec(cmd: &str) {
    if cmd.starts_with("cd ") {
        let mut iter = cmd.split_whitespace();
        iter.next();
        if let Some(path) = iter.next() {
            if let Err(e) = syscall::chdir(path.trim()) {
                println!("Failed to change dir: {:?}", e);
            }
        } else {
            println!("Param error");
        }
    } else if cmd.starts_with("mkdir ") {
        let mut iter = cmd.split_whitespace();
        iter.next();

        while let Some(p) = iter.next() {
            if let Err(e) = syscall::mkdir(p.trim()) {
                println!("Failed to mkdir: {:?}", e);
            }

            if let Some(f) = File::new(p.trim(), OpenFlags::DIRECTORY) {
                f.sync();
            }
            let mut s = String::new();
            s += p;
            s += "/..";
            if let Some(f) = File::new(s.as_str(), OpenFlags::DIRECTORY) {
                f.sync();
            }

        }
    } else if cmd.starts_with("host ") {
        let mut iter = cmd.split_whitespace();
        iter.next();

        if let Some(name) = iter.next() {
            let mut res = [0u8; 4];

            if let Ok(_) = syscall::getaddrinfo(name, &mut res) {
                println!("{:?}", res);
            } else {
                println!("getaddrinfo failed");
            }
        } else {
            println!("Param error");
        }
    } else if cmd.starts_with("ls ") {
        let mut iter = cmd.split_whitespace();
        iter.next();
        let p = iter;
        let print_hdr = cmd.split_whitespace().count() > 2;
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
        syscall::exit(0);
    } else if cmd.starts_with("sleep") {
        let mut args = cmd.split_whitespace();
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
        let mut args = cmd.split_whitespace();
        args.next();

        if let Some(port) = args.next() {
            if let Ok(port) = port.parse::<u32>() {
                nc::bind(port);
            } else {
                println!("Invalid port format");
            }
        } else {
            println!("Format: bind <port>");
        }
    } else if cmd.starts_with("connect") {
        let mut args = cmd.split_whitespace();

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
            if let Err(e) = syscall::mount(dev, dest, "ext2") {
                println!("Mount failed: {:?}", e);
            } /* else {
                  syscall::chdir("/home");
                  for i in 0..2500u64 {
                      let mut buf = [0u8; 9];
                      let res = base_10_bytes(i, &mut buf);
                      syscall::mkdir(unsafe { core::str::from_utf8_unchecked(res) });
                  }
              }*/
        } else {
            println!("Param err");
        }
    } else if cmd.starts_with("umount ") {
        let mut split = cmd.split_whitespace();
        split.next();

        if let Some(path) = split.next() {
            if let Err(e) = syscall::umount(path) {
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
                let mut buf = [0u8; 256];
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
    } else if cmd.starts_with("date") {
        if let Ok(t) = syscall::time() {
            let time = chrono::NaiveDateTime::from_timestamp_opt(t as i64, 0).unwrap();
            println!(
                "{}-{}-{} {}:{}:{}",
                time.year(),
                time.month(),
                time.day(),
                time.hour(),
                time.minute(),
                time.second()
            );
        } else {
            println!("Time syscall failed");
        }
    } else if cmd.starts_with("ln ") {
        let mut split = cmd.split_whitespace();
        split.next();

        if let (Some(target), Some(path)) = (split.next(), split.next()) {
            if target == "-s" {
                let target = path;
                if let Some(path) = split.next() {
                    if let Err(e) = syscall::symlink(target, path) {
                        println!("ln -s failed: {:?}", e);
                    }
                }
            } else {
                if let Err(e) = syscall::link(target, path) {
                    println!("ln failed: {:?}", e);
                }
            }
        } else {
            println!("param error");
        }
    } else if cmd.starts_with("write ") {
        let mut split = cmd.split_whitespace();
        split.next();

        if let Some(path) = split.next() {
            if let Some(file) = File::new(path, OpenFlags::RDWR) {
                let mut buf = [0u8; 64];
                loop {
                    if let Ok(r) = syscall::read(0, &mut buf) {
                        if r > 0 {
                            let w = file.write(&buf[..r]);

                            if w < r {
                                println!("Disk full?");
                                break;
                            }
                        //loop {
                        //    let bigbuf = ['L' as u8; 786];
                        //    let written = file.write(&bigbuf);

                        //    if written < 768 {
                        //        break;
                        //    }

                        //    //println!("written: {}", written);
                        //}
                        //println!("no space");
                        } else {
                            break;
                        }
                    } else {
                        println!("stdin read failed");
                        break;
                    }
                }

                file.sync();
            }
        }
    } else if cmd.starts_with("create ") {
        let mut split = cmd.split_whitespace();
        split.next();

        if let Some(path) = split.next() {
            if let Some(_) = File::new(path, OpenFlags::RDWR | OpenFlags::CREAT) {
                println!("Created file {}", path);
            } else {
                println!("Failed to create file {}", path);
            }
        }
    } else if cmd.starts_with("rmdir ") {
        let mut split = cmd.split_whitespace();
        split.next();

        while let Some(path) = split.next() {
            if let Err(e) = syscall::rmdir(path) {
                println!("rmdir failed: {:?}", e);
            }
        }
    } else if cmd.starts_with("rm ") {
        let mut split = cmd.split_whitespace();
        split.next();

        while let Some(path) = split.next() {
            if let Err(e) = syscall::unlink(path) {
                println!("unlink failed: {:?}", e);
            }
        }
    } else if cmd.starts_with("mv ") {
        let mut split = cmd.split_whitespace();
        split.next();

        if let (Some(old), Some(new)) = (split.next(), split.next()) {
            if let Err(e) = syscall::rename(old, new) {
                println!("mv failed: {:?}", e);
            }
        }
    } else if cmd == "fork" {
        let tty = Tty::new();
        tty.detach();
        if let Ok(id) = syscall::fork() {
            if id > 0 {
                syscall::exit(0);
            } else {
                if let Err(e) = syscall::setsid() {
                    println!("setsid failed {:?}", e);
                }
                tty.attach();
            }
        } else {
            println!("fork failed");
        }
    } else if cmd == "exec" {
        let tty = Tty::new();
        tty.detach();
        if let Ok(id) = syscall::fork() {
            if id > 0 {
                println!("Exec new shell with id: {}", id);
            }
            if id == 0 {
                if let Err(e) = syscall::setsid() {
                    println!("setsid failed {:?}", e);
                }
                tty.attach();
                syscall::exec("/bin/shell", None, None).expect("Failed to exec shell");
            } else {
                syscall::exit(0);
            }
        } else {
            println!("fork failed");
        }
    } else if cmd == "mmap" {
        if let Ok(file) = syscall::open("/home/mmap.bin", OpenFlags::RDWR) {
            if let Ok(addr) = syscall::mmap(
                None,
                4096,
                MMapProt::PROT_READ | MMapProt::PROT_WRITE,
                MMapFlags::MAP_SHARED,
                Some(file),
                0,
            ) {
                let ptr = addr as *mut u8;

                for i in (0..4096).step_by(4) {
                    unsafe {
                        ptr.offset(i).write('W' as u8);
                    }
                }
            } else {
                println!("mmap faileld");
            }
            syscall::close(file).expect("Failed to close file");
        } else {
            println!("file open failed");
        }
    } else if cmd == "mmap_read" {
        if let Ok(file) = syscall::open("/home/mmap.bin", OpenFlags::RDWR) {
            if let Ok(addr) = syscall::mmap(
                Some(0x5000_0000),
                4096,
                MMapProt::PROT_READ | MMapProt::PROT_WRITE,
                MMapFlags::MAP_FIXED | MMapFlags::MAP_SHARED,
                Some(file),
                0,
            ) {
                let ptr = addr as *mut u8;

                for i in (0..4096).step_by(4) {
                    println!("{}", unsafe { ptr.offset(i).read() });
                }
            } else {
                println!("mmap faileld");
            }
            syscall::close(file).expect("Failed to close file");
        } else {
            println!("file open failed");
        }
    } else if cmd == "mmap_cont" {
        let ptr = 0x5000_0000 as *mut u8;
        for i in (0..4096).step_by(6) {
            unsafe {
                ptr.offset(i).write('A' as u8);
            }
        }
    } else if cmd == "munmap" {
        let addr = 0x5000_0000;

        if let Err(e) = syscall::munmap(addr, 0x1000) {
            println!("munmap failed: {:?}", e);
        }
    } else if cmd == "maps" {
        if let Err(e) = syscall::maps() {
            println!("maps failed {:?}", e);
        }
    } else if cmd == "sync" {
        if let Err(e) = syscall::sync() {
            println!("sync failed {:?}", e);
        }
    } else if cmd == "signal_test" {
        signal_test();
    } else if cmd.is_empty() {
        return;
    } else if cmd == "hello_test" {
        for _ in 0..100 {
            start_process("/bin/hello", Some(&["hello"]), None);
        }
    } else if cmd == "ansi_test" {
        println!(
            "\x1b[0;0H\x1b[0;107;91m===========================\x1b[0;6H\x1b[1K\x1b[0m\x1b[0J"
        );
        //println!("\x1b[22;3HHi")
    } else if cmd == "dup_test" {
        if let Ok(new_fd) = syscall::dup(1, syscall_defs::OpenFlags::empty()) {
            println!("duplicate fd: {}", new_fd);

            syscall::close(new_fd).expect("Failed to close dup fd");
        }
    } else if cmd.starts_with("gcc_test ") {
        let mut split = cmd.split_whitespace();

        split.next();

        let reps = if let Some(n) = split.next() {
            if let Ok(rep) = n.parse::<usize>() {
                rep
            } else {
                50
            }
        } else {
            50
        };

        for _ in 0..reps {
            start_process(
                "/usr/bin/gcc",
                Some(&["/usr/bin/gcc", "/test.c", "-o", "/test"]),
                Some(&["PATH=/usr/bin", "TERM=cykusz"]),
            );
        }
    } else if cmd == "pipe_test" {
        let mut fds = [0u32; 2];

        if let Ok(_) = syscall::pipe(&mut fds, syscall_defs::OpenFlags::empty()) {
            if let Ok(id) = syscall::fork() {
                if id > 0 {
                    syscall::write(fds[1] as usize, b"Hello from pipe\n").expect("write failed");

                    syscall::close(fds[0] as usize).expect("close failed");
                    syscall::close(fds[1] as usize).expect("close failed");

                    let mut status = 0u32;
                    syscall::waitpid(id, &mut status).expect("waitpid failed");
                } else {
                    syscall::close(fds[1] as usize).expect("close failed");

                    let mut buf = [0u8; 128];

                    while let Ok(r) = syscall::read(fds[0] as usize, &mut buf) {
                        if r == 0 {
                            break;
                        }
                        syscall::write(1, &mut buf[..r]).expect("write failed");
                    }

                    syscall::exit(0);
                }
            } else {
                println!("fork failed");
            }
        }
    } else {
        let mut split = cmd.split_whitespace();

        let mut args = Vec::<&str>::new();

        while let Some(a) = split.next() {
            args.push(a);
        }

        if !cmd.is_empty() {
            start_process(
                args[0],
                Some(args.as_slice()),
                Some(&["PATH=/usr/bin", "TERM=cykusz"]),
            );
        }
    }
}

pub fn sigint_handler(sig: usize) {
    println!("signal received! {}", sig);
    bochs();

    set_ready(true);
}

static mut READY: bool = false;

fn set_ready(r: bool) {
    unsafe {
        (&mut READY as *mut bool).write_volatile(r);
    }
}

fn ready() -> bool {
    unsafe { (&READY as *const bool).read_volatile() }
}

fn signal_test() {
    while !ready() {}

    set_ready(false);
}

#[allow(dead_code)]
fn sigchld_handler(_sig: usize) {
    let mut status = 0u32;
    let pid = syscall::waitpid(0, &mut status);

    if let Ok(pid) = pid {
        println!("child died: {}, status: {:#x}", pid, status);
    }
}

#[no_mangle]
pub fn main() {
    {
        let tty = Tty::new();
        tty.attach();
    }

    if let Err(e) = syscall::sigaction(
        syscall_defs::signal::SIGINT,
        Some(&SigAction::new(
            SignalHandler::Handle(sigint_handler),
            0,
            SignalFlags::RESTART,
        )),
        None,
    ) {
        println!("Failed to install signal handler: {:?}", e);
    }
    if let Err(e) = syscall::sigaction(
        syscall_defs::signal::SIGQUIT,
        Some(&SigAction::new(
            SignalHandler::Ignore,
            0,
            SignalFlags::empty(),
        )),
        None,
    ) {
        println!("Failed to install signal handler: {:?}", e);
    }
    if let Err(e) = syscall::sigaction(
        syscall_defs::signal::SIGCHLD,
        Some(&SigAction::new(
            SignalHandler::Ignore,
            0,
            SignalFlags::RESTART,
        )),
        None,
    ) {
        println!("Failed to install signal handler: {:?}", e);
    }
    if let Err(e) = syscall::sigaction(
        syscall_defs::signal::SIGHUP,
        Some(&SigAction::new(
            SignalHandler::Ignore,
            0,
            SignalFlags::RESTART,
        )),
        None,
    ) {
        println!("Failed to install signal handler: {:?}", e);
    }

    loop {
        let mut buf = [0u8; 256];
        let mut pwd = [0u8; 1024];
        let pwd_r = syscall::getcwd(&mut pwd).expect("Failed to get cwd");
        let pwd_str = make_str(&pwd[0..pwd_r]);

        let mut to_p = pwd_str.split("/").last().unwrap();
        to_p = if to_p == "" { "/" } else { to_p };

        print!("[root {}]# ", to_p);

        if let Ok(r) = syscall::read(0, &mut buf) {
            let cmd = make_str(&buf[..r]).trim();

            if cmd == "pwd" {
                println!("{}", pwd_str);
            } else {
                exec(cmd);
            }
        }
    }
}

#[allow(dead_code)]
fn main_old() -> ! {
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
