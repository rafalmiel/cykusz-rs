mod music;
mod sounds;

use crate::cykusz::audio::music::Music;
use fon::samp::{Samp16, Samp32};
use kittyaudio::Frame;
use sounds::Sounds;
use std::os::unix::net::UnixStream;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

pub struct Audio {
    sounds: Sounds,
    music: Music,

    audio_thread_data: Arc<AudioThreadData>,
    #[allow(unused)]
    handle: Option<std::thread::JoinHandle<()>>,
}

struct AudioThreadData {
    mixer: Arc<kittyaudio::RecordMixer>,
    audio_on: AtomicBool,
}

impl AudioThreadData {
    fn new() -> Arc<AudioThreadData> {
        Arc::new(AudioThreadData {
            mixer: Arc::new(kittyaudio::RecordMixer::new()),
            audio_on: AtomicBool::new(true),
        })
    }

    fn keep_running(&self) -> bool {
        self.audio_on.load(Ordering::Relaxed)
    }

    fn stop(&self) {
        self.audio_on.store(false, Ordering::Relaxed)
    }

    fn mixer(&self) -> &kittyaudio::RecordMixer {
        &self.mixer
    }
}

impl Audio {
    pub fn new() -> Result<Audio, ExitCode> {
        let thread_data = AudioThreadData::new();
        let mut stream = playaudio::open();
        let audio = Audio {
            sounds: Sounds::new(thread_data.mixer.clone())?,
            music: Music::new(thread_data.mixer.clone())?,
            audio_thread_data: thread_data.clone(),
            handle: Some(std::thread::spawn(move || {
                Audio::audio_thread(thread_data, &mut stream)
            })),
        };

        Ok(audio)
    }

    fn audio_thread(audio: Arc<AudioThreadData>, stream: &mut UnixStream) {
        let mut frames = [Frame::default(); 256];

        while audio.keep_running() {
            audio.mixer().fill_buffer(44100, &mut frames);

            let buf = unsafe {
                std::slice::from_raw_parts(frames.as_ptr() as *const f32, frames.len() * 2)
            };

            // Resample to our format
            let audio = fon::Audio::<Samp32, 2>::with_f32_buffer(44100, buf);
            let mut audio = fon::Audio::<Samp16, 2>::with_audio(44100, &audio);

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

    fn thread_handle(&self) -> &JoinHandle<()> {
        self.handle.as_ref().unwrap()
    }

    pub fn poll(&mut self) {
        if self.thread_handle().is_finished() {
            panic!("Audio thread terminated unexpectedly... Exiting");
        }
        self.music().poll()
    }
}

impl Drop for Audio {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            if !handle.is_finished() {
                self.audio_thread_data.stop();
                let _res = handle.join();
            }
        }
    }
}