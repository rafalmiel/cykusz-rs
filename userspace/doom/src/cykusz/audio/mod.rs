mod music;
mod sounds;

use crate::cykusz::audio::music::Music;
use crate::doomgeneric::sfxinfo_t;
use fon::chan::{Ch16, Ch32};
use kittyaudio::Frame;
use sounds::Sounds;
use std::os::raw::c_int;
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

    pub fn sound_init(&mut self, use_prefix: bool) {
        self.sounds.init(use_prefix);
    }

    pub fn sound_shutdown(&mut self) {
        self.sounds.shutdown();
    }

    pub fn sound_sfx_lumpnum(&self, sfx: &mut sfxinfo_t) -> c_int {
        self.sounds.sfx_lumpnum(sfx)
    }

    pub fn sound_update(&mut self) {
        self.sounds.update()
    }

    pub fn sound_update_params(&mut self, channel: c_int, vol: c_int, sep: c_int) {
        self.sounds.update_params(channel, vol, sep)
    }

    pub fn sound_start(
        &mut self,
        sfx: &mut sfxinfo_t,
        channel: c_int,
        vol: c_int,
        sep: c_int,
    ) -> c_int {
        self.sounds.start(sfx, channel, vol, sep)
    }

    pub fn sound_stop(&mut self, channel: c_int) {
        self.sounds.stop(channel)
    }

    pub fn sound_is_playing(&self, channel: c_int) -> bool {
        self.sounds.is_playing(channel)
    }

    pub fn sound_cache(&mut self, sounds: &mut [sfxinfo_t]) {
        self.sounds.cache_sounds(sounds)
    }

    pub fn music_shutdown(&mut self) {
        self.music.shutdown()
    }

    pub fn music_set_volume(&mut self, volume: c_int) {
        self.music.set_volume(volume)
    }

    pub fn music_pause(&mut self) {
        self.music.pause()
    }

    pub fn music_resume(&mut self) {
        self.music.resume()
    }

    pub fn music_register_song(&mut self, data: &[u8]) -> *mut () {
        self.music.register_song(data)
    }

    pub fn music_unregister_song(&mut self, handle: *mut ()) {
        self.music.unregister_song(handle)
    }

    pub fn music_play_song(&mut self, handle: *mut (), looping: bool) {
        self.music.play_song(handle, looping)
    }

    pub fn music_stop_song(&mut self) {
        self.music.stop_song()
    }

    pub fn music_is_playing(&self) -> bool {
        self.music.is_playing()
    }

    pub fn music_poll(&mut self) {
        self.music.poll()
    }
}
