use std::os::unix::net::UnixStream;
use std::io::prelude::*;

fn main() -> std::io::Result<()> {
    let mut stream = UnixStream::connect("/unix-socket")?;
    println!("client connected");
    stream.write_all(b"hello world")?;
    println!("client sent");
    Ok(())
}