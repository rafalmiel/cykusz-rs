#![feature(seek_stream_len)]
#![feature(raw_ref_op)]

use fon::chan::{Samp16, Samp32};
use kittyaudio::Frame;
use std::process::ExitCode;

fn main() -> Result<(), ExitCode> {
    let mut args = std::env::args();

    if args.len() < 2 {
        println!("Usage: play <wav file path>");
        return Err(ExitCode::from(1));
    }
    args.next();

    let file = args.next().unwrap();
    println!("Opening {file}");
    let song = kittyaudio::Sound::from_path(file.as_str()).unwrap();

    println!("Opened {file}");

    let mixer = kittyaudio::RecordMixer::new();
    mixer.play(song);

    println!("Sent to mixer");

    let mut frames = [Frame::default(); 4096];

    let mut stream = playaudio::open();

    while !mixer.is_finished() {
        mixer.fill_buffer(44100, &mut frames);

        let buf =
            unsafe { std::slice::from_raw_parts(frames.as_ptr() as *const f32, frames.len() * 2) };

        // Resample to our format
        let audio = fon::Audio::<Samp32, 2>::with_f32_buffer(44100, buf);
        let mut audio = fon::Audio::<Samp16, 2>::with_audio(44100, &audio);

        let buf = unsafe {
            std::slice::from_raw_parts(audio.as_i16_slice().as_ptr() as *const u8, audio.len() * 4)
        };

        playaudio::play_into(&mut stream, buf)?
    }

    Ok(())
}
