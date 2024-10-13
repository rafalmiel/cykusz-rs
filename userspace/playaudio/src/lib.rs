use std::io::Write;
use std::os::unix::net::UnixStream;
use std::process::ExitCode;

pub fn play(buf: &[u8]) -> Result<(), ExitCode> {
    let mut socket = UnixStream::connect("/sound-daemon.pid").map_err(|_e| ExitCode::from(3))?;

    play_into(&mut socket, buf)?;

    Ok(())
}

pub fn open() -> UnixStream {
    UnixStream::connect("/sound-daemon.pid").unwrap()
}

pub fn play_into(s: &mut UnixStream, buf: &[u8]) -> Result<(), ExitCode> {
    let mut written = 0;
    loop {
        let n = s
            .write(&buf[written..])
            .map_err(|_e| ExitCode::from(4))?;
        written += n;
        if written == buf.len() {
            break;
        }
    }

    Ok(())
}
