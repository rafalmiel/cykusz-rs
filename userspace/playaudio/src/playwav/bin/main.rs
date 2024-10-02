#![feature(seek_stream_len)]
#![feature(raw_ref_op)]

use std::io::{Seek, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::process::ExitCode;
use std::ptr::slice_from_raw_parts;
use syscall_defs::{MMapFlags, MMapProt};

fn send_to_daemon(song: &str) -> Result<(), ExitCode> {
    let mut wav = std::fs::File::open(song).map_err(|_e| ExitCode::from(1))?;
    let wav_size = wav.stream_len().map_err(|_e| ExitCode::from(2))? as usize;

    let wav_map = syscall_user::mmap(
        None,
        wav_size,
        MMapProt::PROT_READ,
        MMapFlags::MAP_PRIVATE,
        Some(wav.as_raw_fd() as usize),
        0,
    )
    .map_err(|_e| ExitCode::from(1))?;

    let buf = unsafe { &*slice_from_raw_parts(wav_map as *const u8, wav_size) };

    let mut socket = UnixStream::connect("/sound-daemon.pid").map_err(|_e| ExitCode::from(3))?;

    //println!("writing {} bytes", buf.len());
    let mut written = 0;
    loop {
        let n = socket
            .write(&buf[written..])
            .map_err(|_e| ExitCode::from(4))?;
        written += n;
        if written == buf.len() {
            break;
        }
    }
    println!("write finished");

    Ok(())
}

fn main() -> Result<(), ExitCode> {
    let mut args = std::env::args();

    if args.len() < 2 {
        println!("Usage: playwav <wav file path>");
        return Err(ExitCode::from(1));
    }
    args.next();

    let file = args.next().unwrap();
    Ok(send_to_daemon(file.as_str())?)
}
