use std::io::Write;
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