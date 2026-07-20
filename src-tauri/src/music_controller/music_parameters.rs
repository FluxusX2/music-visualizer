use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use cpal::Stream;
use cpal::traits::StreamTrait;

pub struct MusicParameters {
    pub volume: Arc<Mutex<f32>>,
    pub is_paused: bool,
    pub time_step: f32,
    /// Number of output frames (at `sample_rate`) consumed by the audio callback for the
    /// currently loaded song. Used to derive the playback position for the progress bar.
    pub played_frames: Arc<AtomicU64>,
    /// The output (device) sample rate of the currently loaded song.
    pub sample_rate: u32,
    /// The total duration of the currently loaded song, in seconds.
    pub duration_secs: f64,
}

impl MusicParameters {
    pub fn new() -> MusicParameters {
        MusicParameters {
            volume: Arc::new(Mutex::new(0.25)),
            is_paused: true,
            time_step: 0.0,
            played_frames: Arc::new(AtomicU64::new(0)),
            sample_rate: 0,
            duration_secs: 0.0,
        }
    }

    /// Returns the current playback position in seconds.
    pub fn position_secs(&self) -> f64 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        self.played_frames.load(Ordering::Relaxed) as f64 / self.sample_rate as f64
    }

    /// Resets the position tracking for a newly loaded song, optionally starting at
    /// `start_frame` (used when seeking).
    pub fn reset_position(&mut self, sample_rate: u32, duration_secs: f64, start_frame: u64) {
        self.sample_rate = sample_rate;
        self.duration_secs = duration_secs;
        self.played_frames = Arc::new(AtomicU64::new(start_frame));
    }

    /// Sets the playback volume (0.0 = silent, 1.0 = full).
    pub fn set_volume(&self, vol: f32) {
        let clamped = vol.clamp(0.0, 1.0);
        *self.volume.lock().unwrap() = clamped;
    }

    pub fn toggle_song_playback(&mut self, stream: &Stream) {
        if self.is_paused {
            stream.play().unwrap();
            self.is_paused = false;
        } else {
            stream.pause().unwrap();
            self.is_paused = true;
        }
    }
}