use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::yield_now;
use claxon::FlacReader;
use ringbuf::{traits::*, HeapRb};

pub struct AudioInfo {
    pub sample_rate: u32,
    pub channels: u32,
    pub bits_per_sample: u32,
}

pub fn get_flac_info(path: &Path) -> AudioInfo {
    let reader = FlacReader::open(&*path).unwrap();
    let stream_info = reader.streaminfo();
    AudioInfo {
        sample_rate: stream_info.sample_rate,
        channels: stream_info.channels as u32,
        bits_per_sample: stream_info.bits_per_sample as u32,
    }
}

///Loads the FLAC file and decodes it into a ring buffer for playback.
pub fn load_flac_into_buffer(path: &Path, rb: Arc<Mutex<HeapRb<f32>>>) {
    let t_path: PathBuf = path.to_path_buf();

    std::thread::spawn(move || {
        let mut reader = FlacReader::open(t_path).unwrap();

        for sample in reader.samples() {
            let sample = sample.expect("Failed to read sample");
            let normalized = (sample as f32) / (i16::MAX as f32);

            loop {
                let mut buffer = rb.lock().unwrap();

                if !buffer.is_full() {
                    buffer.try_push(normalized).expect("Failed to push sample");
                    break;
                }
                drop(buffer);
                yield_now();
            }
        }
    });
}