use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleRate;
use hound::WavReader;

pub struct MusicPlayer {
    stream: cpal::Stream,
}

// cpal::Stream auf Windows (WASAPI) implementiert Send/Sync nicht,
// obwohl es sicher ist, das Objekt zwischen Threads zu bewegen,
// solange der Zugriff durch einen Mutex serialisiert wird.
unsafe impl Send for MusicPlayer {}
unsafe impl Sync for MusicPlayer {}

impl MusicPlayer {
    pub fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or("Kein Ausgabegerät gefunden")?;

        let mut reader = WavReader::open(path)
            .map_err(|e| format!("Datei nicht gefunden: {}", e))?;

        let spec = reader.spec();
        let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(
            reader.samples::<i16>()
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(|s| s as f32 / i16::MAX as f32)
                .collect()
        ));
        let pos = Arc::new(Mutex::new(0usize));

        let config = cpal::StreamConfig {
            channels: spec.channels,
            sample_rate: SampleRate(spec.sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let samples_clone = Arc::clone(&samples);
        let pos_clone = Arc::clone(&pos);

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _| {
                let samples = samples_clone.lock().unwrap();
                let mut p = pos_clone.lock().unwrap();

                for sample in data.iter_mut() {
                    *sample = if *p < samples.len() {
                        let v = samples[*p];
                        *p += 1;
                        v
                    } else {
                        0.0
                    };
                }
            },
            |err| eprintln!("Stream-Fehler: {}", err),
            None,
        )?;

        stream.play()?;

        Ok(MusicPlayer { stream })
    }

    pub fn play(&self) -> Result<(), cpal::PlayStreamError> {
        self.stream.play()
    }

    pub fn pause(&self) -> Result<(), cpal::PauseStreamError> {
        self.stream.pause()
    }
}