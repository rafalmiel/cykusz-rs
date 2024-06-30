#![feature(seek_stream_len)]

use std::io::Seek;
use std::os::fd::AsRawFd;
use std::process::ExitCode;
use std::ptr::{slice_from_raw_parts, slice_from_raw_parts_mut};
use syscall_defs::{MMapFlags, MMapProt};

fn main() -> Result<(), ExitCode> {
    let mut args = std::env::args();

    if args.len() != 2 {
        println!("Usage: umount <dest dir path>");
        return Err(ExitCode::from(1));
    }

    args.next();

    let file = args.next().unwrap();

    let mut wav = std::fs::File::open(file).map_err(|_e| ExitCode::from(1))?;

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

    println!("file size: {}", wav_size);

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

        while ((get_current_hda_block() + 3) % buf_block_count) != wp_block {}

        hda_data[wp_block * 2048..wp_block * 2048 + chunk]
            .copy_from_slice(&wav_data[file_block * 2048..file_block * 2048 + chunk]);

        wp_block = (wp_block + 1) % buf_block_count;
        file_block += 1;
    }

    while get_current_hda_block() != wp_block {}

    hda_data.fill(0); //silence

    Ok(())
}