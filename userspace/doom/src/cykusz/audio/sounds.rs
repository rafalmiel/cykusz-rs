use crate::doomgeneric::{
    sfxinfo_t, W_CacheLumpNum, W_CheckNumForName, W_GetNumForName, W_LumpLength, W_ReleaseLumpNum,
};
use fon::samp::{Samp8, Samp32};
use kittyaudio::{Frame, SoundHandle};
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::process::ExitCode;
use std::sync::Arc;

pub struct Sounds {
    use_prefix: bool,
    mixer: Arc<kittyaudio::RecordMixer>,
    sounds: [Option<SoundHandle>; 16],
}

impl sfxinfo_t {
    fn lump_name(&self, use_prefix: bool) -> String {
        let link = if self.link != std::ptr::null_mut() {
            unsafe { self.link.as_ref_unchecked() }
        } else {
            self
        };

        let name_bytes: &[u8] = unsafe { std::mem::transmute(&link.name[..]) };

        if use_prefix {
            format!("{}{}", "ds", std::str::from_utf8(name_bytes).unwrap())
        } else {
            std::str::from_utf8(name_bytes).unwrap().to_string()
        }
    }

    fn get_sound(&self) -> Option<Box<kittyaudio::Sound>> {
        let lumpnum = self.lumpnum;
        let data: *mut u8 =
            unsafe { W_CacheLumpNum(lumpnum, crate::doomgeneric::PU_STATIC as c_int) as *mut u8 };
        let lumplen = unsafe { W_LumpLength(lumpnum as c_uint) };

        if lumplen <= 8 {
            return None;
        }

        let data = unsafe { std::slice::from_raw_parts_mut(data, lumplen as usize) };

        if data[0] != 0x03 || data[1] != 0x00 {
            return None;
        }

        let sample_rate = ((data[3] as u32) << 8) | (data[2] as u32);
        let length: usize = ((data[7] as usize) << 24)
            | ((data[6] as usize) << 16)
            | ((data[5] as usize) << 8)
            | (data[4] as usize);

        if (length > lumplen as usize - 8) || (length <= 48) {
            return None;
        }

        let data = &data[16..length - 16];

        // do conversion
        let audio = fon::Audio::<Samp8, 1>::with_u8_buffer(sample_rate, data);
        let audio = fon::Audio::<Samp32, 2>::with_audio(44100, &audio);

        let frames: &[Frame] = unsafe { std::mem::transmute(audio.as_slice()) };

        let snd = kittyaudio::Sound::from_frames(44100, frames);

        unsafe {
            W_ReleaseLumpNum(lumpnum);
        }

        Some(Box::new(snd))
    }

    fn cache(&mut self) {
        if let Some(snd) = self.get_sound() {
            let r = Box::into_raw(snd);

            self.driver_data = r as *mut c_void;
        }
    }
}

impl Sounds {
    pub fn new(mixer: Arc<kittyaudio::RecordMixer>) -> Result<Sounds, ExitCode> {
        Ok(Sounds {
            use_prefix: false,
            mixer,
            sounds: [
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None,
            ],
        })
    }

    pub fn init(&mut self, use_prefix: bool) -> bool {
        self.use_prefix = use_prefix;

        true
    }

    pub fn shutdown(&mut self) {}

    pub fn sfx_lumpnum(&self, sfx: &mut sfxinfo_t) -> c_int {
        let mut name = sfx.lump_name(self.use_prefix);

        unsafe { W_GetNumForName(name.as_mut_ptr() as *mut c_char) }
    }

    pub fn update(&mut self) {
        for s in &mut self.sounds {
            if let Some(snd) = s {
                if snd.finished() {
                    *s = None;
                }
            }
        }
    }

    pub fn update_params(&mut self, channel: c_int, vol: c_int, sep: c_int) {
        if let Some(s) = &mut self.sounds[channel as usize] {
            let volume = (vol as f32) / 128f32;
            let sepr = (sep as f32) / 256f32;
            s.set_volume(volume);
            s.set_panning(sepr);
        }
    }

    pub fn start(&mut self, sfx: &mut sfxinfo_t, channel: c_int, vol: c_int, sep: c_int) -> c_int {
        let sound =
            unsafe { (sfx.driver_data as *mut kittyaudio::Sound).as_ref_unchecked() }.clone();

        self.sounds[channel as usize] = Some(self.mixer.play(sound));

        self.update_params(channel, vol, sep);

        channel
    }

    pub fn stop(&mut self, channel: c_int) {
        if let Some(s) = &mut self.sounds[channel as usize] {
            s.seek_to_end();
        }
    }

    pub fn is_playing(&self, channel: c_int) -> bool {
        if channel < 0 || channel >= 16 {
            return false;
        }

        if let Some(s) = &self.sounds[channel as usize] {
            !s.finished()
        } else {
            false
        }
    }

    pub fn cache(&mut self, sounds: &mut [sfxinfo_t]) {
        for sound in sounds {
            let lump_name = sound.lump_name(self.use_prefix);

            sound.lumpnum = unsafe { W_CheckNumForName(lump_name.as_ptr() as *mut c_char) };

            if sound.lumpnum == -1 {
                continue;
            }

            sound.cache();
        }
    }
}
