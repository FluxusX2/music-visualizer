use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleRate;
use hound::{WavReader, SampleFormat};
use claxon::FlacReader;

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

/// Gemeinsame Audiodaten-Struktur für WAV und FLAC.
struct AudioData {
    samples: Vec<f32>,
    channels: u16,
    sample_rate: u32,
}

/// Reads the content of a .wav file and returns it in f32 format with the spec.
fn read_wav(path: &str) -> Result<AudioData, Box<dyn std::error::Error>> {
    let mut reader = WavReader::open(path)
        .map_err(|e| format!("WAV-Datei nicht gefunden: {}", e))?;
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

    Ok(AudioData {
        samples,
        channels: spec.channels,
        sample_rate: spec.sample_rate,
    })
}

/// Reads the content of a .flac file and returns it in f32 format.
fn read_flac(path: &str) -> Result<AudioData, Box<dyn std::error::Error>> {
    let mut reader = FlacReader::open(path)
        .map_err(|e| format!("FLAC-Datei nicht gefunden: {}", e))?;
    let info = reader.streaminfo();
    let channels = info.channels as u16;
    let sample_rate = info.sample_rate;
    let bits = info.bits_per_sample;
    let max_val = (1i64 << (bits - 1)) as f32;

    let mut samples: Vec<f32> = Vec::new();
    let mut blocks = reader.blocks();
    let mut buf: Vec<i32> = Vec::new();
    loop {
        match blocks.read_next_or_eof(buf) {
            Ok(Some(block)) => {
                let num_channels = block.channels() as usize;
                let num_samples = block.duration() as usize;
                // Kanäle interleaven
                for i in 0..num_samples {
                    for ch in 0..num_channels {
                        let s = block.sample(ch as u32, i as u32);
                        samples.push(s as f32 / max_val);
                    }
                }
                buf = block.into_buffer();
            }
            Ok(None) => break,
            Err(e) => return Err(format!("FLAC-Lesefehler: {}", e).into()),
        }
    }

    Ok(AudioData {
        samples,
        channels,
        sample_rate,
    })
}

/// Liest eine Audio-Datei (WAV oder FLAC) anhand der Dateiendung.
fn read_samples(path: &str) -> Result<AudioData, Box<dyn std::error::Error>> {
    let lower = path.to_lowercase();
    if lower.ends_with(".flac") {
        read_flac(path)
    } else if lower.ends_with(".wav") {
        read_wav(path)
    } else {
        Err(format!("Nicht unterstütztes Dateiformat: {}", path).into())
    }
}

/// Creates a cpal::stream from a .wav or .flac file.
fn load_stream(
    device: &cpal::Device,
    path: &str,
    song_finished: Arc<Mutex<bool>>,
    volume: Arc<Mutex<f32>>,
) -> Result<cpal::Stream, Box<dyn std::error::Error>> {

    let audio = read_samples(path)?;

    let config = cpal::StreamConfig {
        channels: audio.channels,
        sample_rate: SampleRate(audio.sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    let samples = Arc::new(audio.samples);
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
