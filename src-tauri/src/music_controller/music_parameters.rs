use std::sync::{Arc, Mutex};
use cpal::Stream;
use cpal::traits::StreamTrait;

pub struct MusicParameters {
    pub volume: Arc<Mutex<f32>>,
    pub is_paused: bool,
    pub time_step: f32,
}

impl MusicParameters {
    pub fn new() -> MusicParameters {
        MusicParameters {
            volume: Arc::new(Mutex::new(0.25)),
            is_paused: false,
            time_step: 0.0,
        }
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