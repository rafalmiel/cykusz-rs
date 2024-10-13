use kittyaudio::SoundHandle;
use std::io::Read;
use std::os::raw::{c_int, c_void};
use std::ptr::null_mut;
use std::sync::Arc;

struct MusicData<'a>(&'a [u8]);

struct MidiData {
    data: Vec<u8>,
    pos: usize,
}

impl MidiData {
    fn new(data: Vec<u8>) -> MidiData {
        MidiData { data, pos: 0 }
    }
}

impl Read for MidiData {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let to_read = std::cmp::min(buf.len(), self.data.len() - self.pos);
        buf[..to_read].copy_from_slice(&self.data[self.pos..self.pos + to_read]);
        self.pos += to_read;
        Ok(to_read)
    }
}

impl MidiData {
    fn to_sound(mut self, sound_font: &Arc<rustysynth::SoundFont>) -> kittyaudio::Sound {
        use rustysynth::*;
        let settings = SynthesizerSettings::new(44100);
        let synthesizer = Synthesizer::new(sound_font, &settings).unwrap();
        let mut sequencer = MidiFileSequencer::new(synthesizer);

        let midi_file = Arc::new(MidiFile::new(&mut self).unwrap());

        sequencer.play(&midi_file, false);

        let sample_count = (settings.sample_rate as f64 * midi_file.get_length()) as usize;
        let mut left: Vec<f32> = vec![0_f32; sample_count];
        let mut right: Vec<f32> = vec![0_f32; sample_count];

        sequencer.render(&mut left, &mut right);

        let both = left
            .iter()
            .zip(right.iter())
            .map(|(a, b)| (*a, *b))
            .collect::<Vec<(f32, f32)>>();

        kittyaudio::Sound::from_frames(44100, unsafe { std::mem::transmute(both.as_slice()) })
    }
}

impl<'a> MusicData<'a> {
    pub fn new(data: &'a [u8]) -> MusicData<'a> {
        MusicData(data)
    }

    fn is_midi(&self) -> bool {
        self.0.len() > 4 && &self.0[..4] == b"MThd"
    }

    fn to_midi(&self) -> Option<MidiData> {
        if self.is_midi() {
            return Some(MidiData::new(self.0.to_owned()));
        }

        use crate::doomgeneric::*;
        unsafe {
            let instream = mem_fopen_read(self.0.as_ptr() as *mut c_void, self.0.len());
            let outstream = mem_fopen_write();

            let result = mus2mid(instream, outstream);

            let res = if result == 0 {
                let mut outbuf: *mut c_void = null_mut();
                let mut outbuf_len: usize = 0;

                mem_get_buf(outstream, &raw mut outbuf, &raw mut outbuf_len);

                let data = std::slice::from_raw_parts_mut(outbuf as *mut u8, outbuf_len);

                Some(MidiData::new(data.to_owned()))
            } else {
                None
            };

            mem_fclose(instream);
            mem_fclose(outstream);

            res
        }
    }
}

trait SoundHandleHelper {
    fn loop_all(&self, looping: bool);
}

impl SoundHandleHelper for SoundHandle {
    fn loop_all(&self, looping: bool) {
        if !looping {
            self.set_loop_enabled(false);
        } else {
            let index = self.index();
            self.seek_to_end();
            let last_idx = self.index();
            self.seek_to_index(index);
            self.set_loop_enabled(true);
            self.set_loop_index(0..=last_idx);
        }
    }
}

pub struct Music {
    mixer: Arc<kittyaudio::RecordMixer>,
    music: std::collections::HashMap<usize, SoundHandle>,
    current: Option<SoundHandle>,
    sound_font: Arc<rustysynth::SoundFont>,
    id: usize,
}

impl Music {
    pub fn new(mixer: Arc<kittyaudio::RecordMixer>) -> Music {
        let mut sf = std::fs::File::open("/FluidR3_GM.sf2").unwrap();
        Music {
            mixer,
            music: std::collections::HashMap::new(),
            current: None,
            sound_font: Arc::new(rustysynth::SoundFont::new(&mut sf).unwrap()),
            id: 1,
        }
    }

    pub fn shutdown(&mut self) {}

    pub fn set_volume(&mut self, volume: c_int) {
        if let Some(track) = &self.current {
            let volume = (volume as f32) / 128f32;

            track.set_volume(volume);
        }
    }

    pub fn pause(&mut self) {
        if let Some(track) = &self.current {
            track.pause();
        }
    }

    pub fn resume(&mut self) {
        if let Some(track) = &self.current {
            track.resume();
        }
    }

    pub fn register_song(&mut self, data: &[u8]) -> *mut () {
        let msc = MusicData::new(data);
        let midi = msc.to_midi();

        if let Some(midi) = midi {
            let sound = SoundHandle::new(midi.to_sound(&self.sound_font));

            self.music.insert(self.id, sound);
            self.id += 1;

            return (self.id - 1) as *mut ();
        }

        0 as *mut ()
    }

    pub fn unregister_song(&mut self, handle: *mut ()) {
        self.music.remove(&(handle as usize));
    }

    pub fn play_song(&mut self, handle: *mut (), looping: bool) {
        let handle = self.music.get(&(handle as usize));

        if let Some(handle) = handle {
            self.current = Some(handle.clone());

            handle.loop_all(looping);

            self.mixer.play(handle.clone());
        }
    }

    pub fn stop_song(&mut self) {
        if let Some(handle) = &self.current {
            handle.pause();
            handle.set_loop_enabled(false);
            handle.seek_to_end();
        }
    }

    pub fn is_playing(&self) -> bool {
        if let Some(handle) = &self.current {
            !handle.finished()
        } else {
            false
        }
    }

    pub fn poll(&mut self) {}
}
