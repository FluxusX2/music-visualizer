use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleRate;
use hound::{WavReader, SampleFormat};

pub struct MusicPlayer {
    device: cpal::Device,
    current_stream: Option<cpal::Stream>,
    pub queue: VecDeque<String>,
    pub is_paused: bool,
    song_finished: Arc<Mutex<bool>>,
    pub volume: Arc<Mutex<f32>>,
}

// cpal::Device und cpal::Stream auf Windows (WASAPI) implementieren Send nicht,
// aber MusicPlayer lebt in einem Mutex<Option<MusicPlayer>> in AppState und
// wird nur sequentiell über Tauri-Commands zugegriffen.
unsafe impl Send for MusicPlayer {}
unsafe impl Sync for MusicPlayer {}


impl MusicPlayer {

    ///Connstructor for MusicPlayer
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("Kein Ausgabegerät gefunden")?;

        Ok(MusicPlayer {
            device,
            current_stream: None,
            queue: VecDeque::new(),
            is_paused: false,
            song_finished: Arc::new(Mutex::new(false)),
            volume: Arc::new(Mutex::new(1.0)),
        })
    }

    /// Adds Song to queue.
    pub fn enqueue(&mut self, path: String) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_stream.is_none() {
            self.start_song(&path)?;
            self.is_paused = false;
        } else {
            self.queue.push_back(path);
        }
        Ok(())
    }

    ///Skips to the next Song in queue.
    /// Returns true, if a Song was started.
    pub fn advance_queue(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let finished = {
            let mut f = self.song_finished.lock().unwrap();
            if *f { *f = false; true } else { false }
        };

        if finished || self.current_stream.is_none() {
            self.current_stream = None;
            if let Some(path) = self.queue.pop_front() {
                self.start_song(&path)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Remove song at given Index.
    pub fn remove_from_queue(&mut self, index: usize) -> bool {
        if index < self.queue.len() {
            self.queue.remove(index);
            true
        } else {
            false
        }
    }

    /// Returns the queue as a String-Vector.
    pub fn get_queue(&self) -> Vec<String> {
        self.queue.iter().cloned().collect()
    }

    ///Plays current song.
    pub fn play(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.is_paused = false;
        if let Some(stream) = &self.current_stream {
            stream.play()?;
        }
        Ok(())
    }

    ///Pauses current song.
    pub fn pause(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.is_paused = true;
        if let Some(stream) = &self.current_stream {
            stream.pause()?;
        }
        Ok(())
    }

    /// Sets the playback volume (0.0 = silent, 1.0 = full).
    pub fn set_volume(&self, vol: f32) {
        let clamped = vol.clamp(0.0, 1.0);
        *self.volume.lock().unwrap() = clamped;
    }

    /// Starts a song from the given path and sets up the stream.
    fn start_song(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let new_finished = Arc::new(Mutex::new(false));
        let stream = load_stream(&self.device, path, Arc::clone(&new_finished), Arc::clone(&self.volume))?;
        self.current_stream = Some(stream);
        self.song_finished = new_finished;
        Ok(())
    }
}

/// Reads the content of a .wav file and returns it in f32 format with the spec.
fn read_samples(path: &str) -> Result<(Vec<f32>, hound::WavSpec), Box<dyn std::error::Error>> {
    let mut reader = WavReader::open(path)
        .map_err(|e| format!("Datei nicht gefunden: {}", e))?;
    let spec = reader.spec();

    let samples: Vec<f32> = match (spec.sample_format, spec.bits_per_sample) {
        (SampleFormat::Float, _) => reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()?,
        (SampleFormat::Int, 16) => reader
            .samples::<i16>()
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|s| s as f32 / i16::MAX as f32)
            .collect(),
        (SampleFormat::Int, b) if b <= 32 => reader
            .samples::<i32>()
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|s| s as f32 / i32::MAX as f32)
            .collect(),
        _ => {
            return Err(format!(
                "Unbekanntes WAV-Format: {:?} {}bit",
                spec.sample_format, spec.bits_per_sample
            )
            .into())
        }
    };

    Ok((samples, spec))
}

/// Creates a cpal::stream from .wav file.
fn load_stream(
    device: &cpal::Device,
    path: &str,
    song_finished: Arc<Mutex<bool>>,
    volume: Arc<Mutex<f32>>,
) -> Result<cpal::Stream, Box<dyn std::error::Error>> {

    let (samples, spec) = read_samples(path)?;

    let config = cpal::StreamConfig {
        channels: spec.channels,
        sample_rate: SampleRate(spec.sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    let samples = Arc::new(samples);
    let pos = Arc::new(Mutex::new(0usize));
    let samples_clone = Arc::clone(&samples);
    let pos_clone = Arc::clone(&pos);

    let stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], _| {
            let mut p = pos_clone.lock().unwrap();
            let vol = *volume.lock().unwrap();
            for sample in data.iter_mut() {
                *sample = if *p < samples_clone.len() {
                    let v = samples_clone[*p] * vol;
                    *p += 1;
                    v
                } else {
                    0.0
                };
            }
            // Song zu Ende → Signal setzen
            if *p >= samples_clone.len() {
                if let Ok(mut finished) = song_finished.try_lock() {
                    *finished = true;
                }
            }
        },
        |err| eprintln!("Stream-Fehler: {}", err),
        None,
    )?;

    stream.play()?;
    Ok(stream)
}
