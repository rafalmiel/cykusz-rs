use std::os::unix::net::UnixStream;
use std::io::prelude::*;

use syscall_user::util::read_all_to_string;

fn loop_and_read(mut s: UnixStream) -> std::io::Result<()> {
    loop {
        let mut buffer = String::new();

        std::io::stdin().read_line(&mut buffer)?;

        if buffer.len() == 0 {
            return Ok(());
        }

        s.write_all(buffer.as_bytes())?;

        println!("client: recv {}", read_all_to_string::<1, _>(&mut s)?.trim());
    }
}

fn main() -> std::io::Result<()> {
    let stream = UnixStream::connect("/unix-socket")?;
    loop_and_read(stream)?;
    Ok(())
}