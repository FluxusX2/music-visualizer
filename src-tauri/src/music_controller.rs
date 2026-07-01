use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use std::path::Path;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::traits::Consumer;
use crate::music_controller::decoder::AudioInfo;

mod music_parameters;
mod decoder;
mod music_player;

pub struct MusicController {
    host: cpal::Host,
    device: cpal::Device,
    stream: Option<cpal::Stream>,
    pub queue: VecDeque<String>,
    pub parameters: music_parameters::MusicParameters,
    pub ring_buffer: Option<Arc<Mutex<ringbuf::HeapRb<f32>>>>,
}

// cpal::Device und cpal::Stream auf Windows (WASAPI) implementieren Send nicht,
// aber MusicPlayer lebt in einem Mutex<Option<MusicPlayer>> in AppState und
// wird nur sequentiell über Tauri-Commands zugegriffen.
unsafe impl Send for MusicController {}
unsafe impl Sync for MusicController {}


impl MusicController {
    ///Connstructor for MusicPlayer
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("Kein Ausgabegerät gefunden")?;

        Ok(MusicController {
            host,
            device,
            stream: None,
            queue: VecDeque::new(),
            parameters: music_parameters::MusicParameters::new(),
            ring_buffer: None, // 10 Sekunden Puffer
        })
    }

    pub fn start_song(&mut self) {
        let path_str = self.queue.pop_front().unwrap();
        let path = Path::new(&path_str);
        let info = self.load_song(path);
        self.play_song(&info);

    }

    fn load_song(&mut self, path: &Path) -> AudioInfo {
        let info = decoder::get_flac_info(path);
        let rb = Arc::new(Mutex::new(ringbuf::HeapRb::new(
            (info.sample_rate as usize) * (info.channels as usize) * 10,
        )));
        self.ring_buffer = Some(rb.clone());
        decoder::load_flac_into_buffer(path, rb);
        info
    }

    fn play_song(&mut self, info: &AudioInfo) {
        music_player::play_song(self, info);
    }

    pub fn add_to_queue(&mut self, path_str: String) {
        if !path_str.ends_with(".flac") {
            return;
        }
        if Path::new(&path_str).is_file() {
            self.queue.push_back(path_str);
        }
    }

}