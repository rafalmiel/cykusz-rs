mod music;
mod sounds;

use crate::cykusz::audio::music::Music;
use fon::chan::{Ch16, Ch32};
use kittyaudio::Frame;
use sounds::Sounds;
use std::os::unix::net::UnixStream;
use std::process::ExitCode;
use std::sync::Arc;

pub struct Audio {
    sounds: Sounds,
    music: Music,
    #[allow(unused)]
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Audio {
    pub fn new() -> Result<Audio, ExitCode> {
        let mixer = Arc::new(kittyaudio::RecordMixer::new());
        Ok(Audio {
            sounds: Sounds::new(mixer.clone()),
            music: Music::new(mixer.clone()),
            handle: Some(std::thread::spawn(move || {
                let mut stream = playaudio::open();

                Self::audio_thread(&mut stream, &mixer);
            })),
        })
    }

    fn audio_thread(stream: &mut UnixStream, mixer: &Arc<kittyaudio::RecordMixer>) {
        let mut frames = [Frame::default(); 256];

        loop {
            mixer.fill_buffer(44100, &mut frames);

            let buf = unsafe {
                std::slice::from_raw_parts(frames.as_ptr() as *const f32, frames.len() * 2)
            };

            // Resample to our format
            let audio = fon::Audio::<Ch32, 2>::with_f32_buffer(44100, buf);
            let mut audio = fon::Audio::<Ch16, 2>::with_audio(44100, &audio);

            let buf = unsafe {
                std::slice::from_raw_parts(
                    audio.as_i16_slice().as_ptr() as *const u8,
                    audio.len() * 4,
                )
            };

            playaudio::play_into(stream, buf).unwrap();
        }
    }

    pub fn sound(&mut self) -> &mut Sounds {
        &mut self.sounds
    }

    pub fn music(&mut self) -> &mut Music {
        &mut self.music
    }
}
