use std::time::Duration;

use bevy::asset::Asset;
use bevy::reflect::TypePath;
use bevy::audio::Decodable;
use rodio::source::TakeDuration;
use rodio::Source;

/// Procedural chiptune background music.
/// An arpeggio of square-wave notes that loops forever.
#[derive(Asset, Debug, Clone, TypePath)]
pub struct ChiptuneLoop;

/// A single square-wave note.
#[derive(Clone, Copy, Debug)]
struct Note {
    freq: f32,
    duration_secs: f32,
}

const CHIPTUNE_NOTES: &[Note] = &[
    Note { freq: 130.81, duration_secs: 0.30 }, // C3
    Note { freq: 164.81, duration_secs: 0.30 }, // E3
    Note { freq: 196.00, duration_secs: 0.30 }, // G3
    Note { freq: 246.94, duration_secs: 0.30 }, // B3
];

const SAMPLE_RATE: u32 = 44_100;

/// Custom source that cycles through the chiptune arpeggio.
pub struct ChiptuneSource {
    sample_idx: u64,
    note_idx: usize,
    sample_rate: u32,
}

impl ChiptuneSource {
    fn new() -> Self {
        Self {
            sample_idx: 0,
            note_idx: 0,
            sample_rate: SAMPLE_RATE,
        }
    }

    fn current_note(&self) -> Note {
        CHIPTUNE_NOTES[self.note_idx % CHIPTUNE_NOTES.len()]
    }

    fn samples_per_note(&self, note: Note) -> u64 {
        (note.duration_secs * self.sample_rate as f32) as u64
    }
}

impl Iterator for ChiptuneSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let note = self.current_note();
        let spn = self.samples_per_note(note);

        // Advance note if we've played enough samples
        if self.sample_idx >= spn {
            self.sample_idx = 0;
            self.note_idx += 1;
        }

        let active_note = self.current_note();
        let period = self.sample_rate as f32 / active_note.freq;
        let phase = (self.sample_idx as f32 % period) / period;

        // Square wave: positive for first half of period, negative for second half
        let sample = if phase < 0.5 { 0.15 } else { -0.15 };

        self.sample_idx += 1;
        Some(sample)
    }
}

impl Source for ChiptuneSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Decodable for ChiptuneLoop {
    type DecoderItem = f32;
    type Decoder = ChiptuneSource;

    fn decoder(&self) -> Self::Decoder {
        ChiptuneSource::new()
    }
}

/// Procedural white noise source.
pub struct WhiteNoise {
    sample_rate: u32,
    i: u64,
}

impl WhiteNoise {
    fn new(sample_rate: u32) -> Self {
        Self { sample_rate, i: 0 }
    }
}

impl Iterator for WhiteNoise {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        self.i += 1;
        // Simple hash-based pseudo-random in [-1.0, 1.0]
        let hash = self.i.wrapping_mul(0x45d9f3b).wrapping_add(0x119de1f3);
        let val = (hash % 1000) as f32 / 1000.0;
        Some(val * 2.0 - 1.0)
    }
}

impl Source for WhiteNoise {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

/// Procedural dig sound effect.
/// A short burst of white noise.
#[derive(Asset, Debug, Clone, TypePath)]
pub struct DigNoise;

impl Decodable for DigNoise {
    type DecoderItem = f32;
    type Decoder = TakeDuration<WhiteNoise>;

    fn decoder(&self) -> Self::Decoder {
        WhiteNoise::new(SAMPLE_RATE).take_duration(Duration::from_millis(80))
    }
}
