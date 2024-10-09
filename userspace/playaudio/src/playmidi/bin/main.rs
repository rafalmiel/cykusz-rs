use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};
use std::fs::File;
use std::io::Read;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;
use syscall_defs::{MMapFlags, MMapProt};
use wavers::Samples;

#[allow(dead_code)]
struct MMapFileReader<'a> {
    buffer: &'a [u8],
    pos: usize,
}

#[allow(dead_code)]
impl MMapFileReader<'_> {
    fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file = File::open(path)?;

        let size = file.metadata()?.len() as usize;

        println!("size: {size}");

        let mmap = syscall_user::mmap(
            None,
            size,
            MMapProt::PROT_READ,
            MMapFlags::MAP_PRIVATE,
            Some(file.as_raw_fd() as usize),
            0,
        )
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::Other))?;

        let buffer = unsafe { std::slice::from_raw_parts(mmap as *const u8, size) };

        Ok(MMapFileReader::<'_> { buffer, pos: 0 })
    }
}

impl<'a> Read for MMapFileReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let to_read = std::cmp::min(buf.len(), self.buffer.len() - self.pos);
        buf[..to_read].copy_from_slice(&self.buffer[self.pos..self.pos + to_read]);
        self.pos += to_read;
        Ok(to_read)
    }
}

fn main() -> Result<(), ExitCode> {
    let mut args = std::env::args();

    if args.len() < 3 {
        println!("Usage: playmidi <soundfont> <midi>");
        return Err(ExitCode::FAILURE);
    }
    args.next();

    let soundfont_path = args.next().unwrap();
    let midi_path = args.next().unwrap();

    println!("soundfont: {soundfont_path}, midi: {midi_path}");

    let mut sf2 = MMapFileReader::open(soundfont_path).map_err(|_| ExitCode::FAILURE)?;
    let sound_font = Arc::new(SoundFont::new(&mut sf2).unwrap());

    println!("Creating sequencer...");

    // Create the MIDI file sequencer.
    let settings = SynthesizerSettings::new(44100);
    let synthesizer = Synthesizer::new(&sound_font, &settings).unwrap();
    let mut sequencer = MidiFileSequencer::new(synthesizer);

    println!("Loading midi...");

    // Load the MIDI file.
    let mut mid = File::open(midi_path).unwrap();
    let midi_file = Arc::new(MidiFile::new(&mut mid).unwrap());

    println!("Sequencer play...");

    sequencer.play(&midi_file, false);

    // The output buffer.
    let sample_count = (settings.sample_rate as f64 * midi_file.get_length()) as usize;
    let mut left: Vec<f32> = vec![0_f32; sample_count];
    let mut right: Vec<f32> = vec![0_f32; sample_count];

    println!("Render waveform {} {}", left.len(), right.len());

    // Render the waveform.
    sequencer.render(&mut left, &mut right);

    println!("Convert...");

    let left: Samples<i16> = wavers::Samples::from(left).convert();
    let right: Samples<i16> = wavers::Samples::from(right).convert();

    println!("Collect...");

    let both = left
        .iter()
        .zip(right.iter())
        .map(|(a, b)| (*a, *b))
        .collect::<Vec<(i16, i16)>>();

    println!("Send...");

    let buf = unsafe { std::slice::from_raw_parts(both.as_ptr() as *const u8, both.len() * 4) };

    playaudio::play(buf)?;

    Ok(())
}
