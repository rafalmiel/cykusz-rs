use std::io::Read;
use std::os::fd::AsRawFd;
use syscall_defs::poll::{PollEventFlags, PollFd};

pub fn read_all<const BYTES: usize, T: Read + AsRawFd>(s: &mut T) -> std::io::Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut buf = [0u8; BYTES];

    let mut pollfds = [PollFd::new(s.as_raw_fd() as i32, PollEventFlags::READ)];

    let mut first = true;

    while let Ok(found) = crate::poll(&mut pollfds, if first { -1 } else { 0 }) {
        if found == 0 || !pollfds[0].revents.contains(PollEventFlags::READ) {
            break;
        }

        let read = s.read(&mut buf)?;
        out.extend_from_slice(&buf[..read]);

        if read < std::mem::size_of_val(&buf) {
            break;
        }

        first = false;
    }

    if !out.is_empty() {
        Ok(out)
    } else {
        Err(std::io::Error::other("writer disconnected"))
    }
}

pub fn read_all_to_string<const BYTES: usize, T: Read + AsRawFd>(
    s: &mut T,
) -> std::io::Result<String> {
    let mut str = String::new();
    let bytes = read_all::<BYTES, T>(s)?;
    str.push_str(unsafe { std::str::from_utf8_unchecked(bytes.as_slice()) });

    Ok(str)
}
