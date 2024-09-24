#![feature(seek_stream_len)]

use std::io::Seek;
use std::os::fd::AsRawFd;
use std::process::ExitCode;
use std::ptr::{slice_from_raw_parts, slice_from_raw_parts_mut};
use syscall_defs::{MMapFlags, MMapProt};

fn play(song: &str) -> Result<(), ExitCode> {
    let mut wav = std::fs::File::open(song).map_err(|_e| ExitCode::from(1))?;

    let wav_size = wav.stream_len().map_err(|_e| ExitCode::from(2))? as usize;

    let hda_dev = std::fs::File::open("/dev/hda").map_err(|_e| ExitCode::from(3))?;

    let hda_map = syscall_user::mmap(
        None,
        2048 * 32 + 4096,
        MMapProt::PROT_READ | MMapProt::PROT_WRITE,
        MMapFlags::MAP_SHARED,
        Some(hda_dev.as_raw_fd() as usize),
        0,
    )
    .map_err(|_e| ExitCode::from(4))?;

    let wav_map = syscall_user::mmap(
        None,
        wav_size,
        MMapProt::PROT_READ,
        MMapFlags::MAP_PRIVATE,
        Some(wav.as_raw_fd() as usize),
        0,
    )
    .map_err(|_e| ExitCode::from(4))?;

    for addr in (wav_map..wav_map + wav_size).step_by(4096) {
        let _ = unsafe {
            // prefault to read the file into memory
            (addr as *const u64).read_volatile()
        };
    }

    //println!("file size: {}", wav_size);

    let get_current_hda_block =
        || unsafe { (hda_map as *const u64).offset(4).read_volatile() as usize / 2048 };

    let mut file_block = 0;
    let buf_block_count = 32;

    let wav_data = unsafe { &*slice_from_raw_parts((wav_map + 44) as *const u8, wav_size - 44) };
    let hda_data =
        unsafe { &mut *slice_from_raw_parts_mut((hda_map + 4096) as *mut u8, 2048 * 32) };

    let mut wp_block = (get_current_hda_block() + 6) % buf_block_count;

    while file_block * 2048 < wav_data.len() {
        let rem = wav_data.len() - (file_block * 2048);

        let chunk = std::cmp::min(rem, 2048);

        while ((get_current_hda_block() + 3) % buf_block_count) != wp_block {
            // yield cpu for other tasks
            let _ = syscall_user::yield_execution();
        }

        hda_data[wp_block * 2048..wp_block * 2048 + chunk]
            .copy_from_slice(&wav_data[file_block * 2048..file_block * 2048 + chunk]);

        wp_block = (wp_block + 1) % buf_block_count;
        file_block += 1;
    }

    while get_current_hda_block() != wp_block {
        // yield cpu for other tasks
        let _ = syscall_user::yield_execution();
    }

    hda_data.fill(0); //silence

    Ok(())
}

fn main() -> Result<(), ExitCode> {
    let mut args = std::env::args();

    if args.len() < 2 {
        println!("Usage: playwav [-d] <dest dir path>");
        return Err(ExitCode::from(1));
    }

    args.next();
    let mut daemonize = false;

    let mut file = args.next().unwrap();
    if let Some(arg2) = args.next() {
        if file == "-d" {
            daemonize = true;
            file = arg2;
        }
    }

    if !daemonize {
        return Ok(play(file.as_str())?);
    }

    let daemon = syscall_user::fork().expect("fork failed");

    if daemon > 0 {
        return Ok(())
    }

    let _sid = syscall_user::setsid().expect("setsid failed");

    syscall_user::chdir("/").expect("chdir failed");

    syscall_user::close(0).expect("close 0 faield");
    syscall_user::close(1).expect("close 1 faield");
    syscall_user::close(2).expect("close 2 faield");

    Ok(play(file.as_str())?)
}
