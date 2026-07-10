use std::sync::{mpsc, Arc, Mutex};
use std::collections::VecDeque;
use std::path::Path;
use std::sync::mpsc::channel;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tauri::{AppHandle, Emitter};
use crate::music_controller::decoder::{AudioInfo};

mod music_parameters;
mod decoder;
mod music_player;

pub struct MusicController {
    app_handle: AppHandle,
    device: cpal::Device,
    stream: Option<cpal::Stream>,
    pub queue: VecDeque<String>,
    pub previous_song_stack: Vec<String>,
    pub parameters: music_parameters::MusicParameters,
    pub ring_buffer: Option<Arc<Mutex<ringbuf::HeapRb<f32>>>>,
    pub queue_tx: mpsc::Sender<()>,
}

unsafe impl Send for MusicController {}
unsafe impl Sync for MusicController {}


impl MusicController {
    ///Connstructor for MusicPlayer
    pub fn new(app_handle: AppHandle) -> Result<(Self, mpsc::Receiver<()>), Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("Kein Ausgabegerät gefunden")?;

        let (tx, rx) = channel::<()>();

        let controller = MusicController {
            app_handle,
            device,
            stream: None,
            queue: VecDeque::new(),
            previous_song_stack: Vec::new(),
            parameters: music_parameters::MusicParameters::new(),
            ring_buffer: None,
            queue_tx: tx,
        };

        Ok((controller, rx))
    }

    pub fn start_song(&mut self) {
        let path_str = self.queue.get(0).unwrap().clone();
        let path = Path::new(&path_str);
        let info = self.load_song(path);
        self.play_song(&info);
    }

    fn load_song(&mut self, path: &Path) -> AudioInfo {
        let info = decoder::get_flac_info(path);
        let target_sample_rate = self.device.default_output_config().unwrap().sample_rate().0;

        let rb = Arc::new(Mutex::new(ringbuf::HeapRb::new(
            (target_sample_rate as usize) * (info.channels as usize) * 10,
        )));
        self.ring_buffer = Some(rb.clone());

        decoder::load_flac_into_buffer(path,
                                       rb.clone(),
                                       info.sample_rate,
                                       target_sample_rate,
                                       info.channels,
                                       info.bits_per_sample,
                                       self.queue_tx.clone()
        );
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
            if self.queue.is_empty() {
                self.queue.push_back(path_str.clone());
                self.start_song();
            } else {
                self.queue.push_back(path_str);
            }
        }
    }

    pub fn toggle_playback(&mut self) {
        if let Some(stream) = &self.stream {
            self.parameters.toggle_song_playback(stream);
            self.emit_playback_state();
        }
    }

    pub fn skip_forward(&mut self) {
        if let Some(stream) = &self.stream {
            self.previous_song_stack.push(self.queue.pop_front().expect("Queue should not be empty"));
            stream.pause().expect("Failed to pause stream or stream does not exist.");
            self.ring_buffer = None;
            if !self.queue.is_empty() {
                self.start_song();
            } else {
                self.parameters.is_paused = true;
                self.stream = None;
                self.emit_playback_state();
            }
        }
    }

    pub fn skip_backward(&mut self) {
        if !self.previous_song_stack.is_empty() {
            self.queue.push_front(self.previous_song_stack.pop().unwrap());
            if let Some(stream) = &self.stream {
                stream.pause().expect("Failed to pause stream or stream does not exist.");
            }
            self.ring_buffer = None;
            self.start_song();
        }
    }

    pub fn emit_playback_state(&self) {
        if let Err(err) = self.app_handle.emit("playback-state", self.parameters.is_paused) {
            eprintln!("Failed to emit playback state: {}", err);
        }
    }
    
    pub fn set_volume(&mut self, new_volume: f32) {
        self.parameters.set_volume(new_volume);
    }

    pub fn create_queue_thread(shared: Arc<Mutex<Option<MusicController>>>,
                               rx: mpsc::Receiver<()>,) {
        std::thread::spawn(move || {
            while rx.recv().is_ok() {
                let mut guard = shared.lock().unwrap();
                if let Some(player) = guard.as_mut() {
                    player.previous_song_stack.push(player.queue.pop_front().expect("Queue should not be empty"));
                    if !player.queue.is_empty() {
                        player.start_song();
                    }
                }
            }
        });
    }

}