use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};
use std::fs::File;
use std::process::ExitCode;
use std::sync::Arc;
use wavers::Samples;

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

    let mut sf2 = File::open(soundfont_path).unwrap();
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
