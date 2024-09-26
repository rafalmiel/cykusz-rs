use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};

use syscall_user::util::read_all_to_string;

fn read_and_answer(mut s: UnixStream) -> std::io::Result<()> {
    loop {
        println!("server awaiting msg from client");

        let str = read_all_to_string::<1, _>(&mut s)?;

        println!("server recv: {}", str.trim());

        s.write_all(str.as_bytes())?;
    }
}

fn main() -> std::io::Result<()> {
    let listener = UnixListener::bind("/unix-socket")?;

    loop {
        let mut pollfd = [syscall_defs::poll::PollFd::new(
            listener.as_raw_fd() as i32,
            syscall_defs::poll::PollEventFlags::READ,
        )];

        if let Ok(res) = syscall_user::poll(&mut pollfd, -1) {
            if res == 0
                || !pollfd[0]
                    .revents
                    .contains(syscall_defs::poll::PollEventFlags::READ)
            {
                continue;
            }
            println!("server: poll result: {}", res);
            match listener.accept() {
                Ok((s, _addr)) => {
                    println!("server: got client");

                    if let Err(_e) = read_and_answer(s) {
                        println!("server: client disconnected, awaiting next...");
                    }
                }
                Err(e) => {
                    println!("server: accept err: {:?}", e);
                }
            }
        }
    }
}
