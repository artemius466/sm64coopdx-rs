//! Audio engine — stub.
//!
//! Ported from `src/audio/` and `src/pc/audio/`. The upstream audio engine
//! mixes sequence + SFX + env audio into interleaved 16-bit samples and feeds
//! them to the platform's audio backend (miniaudio on PC). Full port is a later
//! pass; here we model the sample mixer contract.

/// A single stereo sample frame (interleaved L/R, 16-bit).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct AudioSample {
    pub left: i16,
    pub right: i16,
}

impl AudioSample {
    pub fn new(left: i16, right: i16) -> Self {
        AudioSample { left, right }
    }
}

/// The audio driver contract. Mirrors `struct AudioAPI` in `src/audio/external.h`.
pub trait AudioDriver {
    /// Number of sample frames currently buffered.
    fn buffered(&self) -> usize;
    /// Number of sample frames the backend wants buffered.
    fn desired_buffered(&self) -> usize;
    /// Feed a chunk of interleaved samples to the backend.
    fn play(&mut self, samples: &[i16]);
    /// Set the master volume (0..=127).
    fn set_volume(&mut self, volume: u8);
}

/// A no-op in-memory driver for headless runs.
pub struct NullAudioDriver {
    volume: u8,
    total_played: usize,
}

impl NullAudioDriver {
    pub fn new() -> Self {
        NullAudioDriver { volume: 127, total_played: 0 }
    }
}

impl Default for NullAudioDriver {
    fn default() -> Self {
        NullAudioDriver::new()
    }
}

impl AudioDriver for NullAudioDriver {
    fn buffered(&self) -> usize {
        0
    }
    fn desired_buffered(&self) -> usize {
        0
    }
    fn play(&mut self, samples: &[i16]) {
        self.total_played += samples.len();
    }
    fn set_volume(&mut self, volume: u8) {
        self.volume = volume;
    }
}
