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

#[derive(serde::Serialize, Clone)]
pub struct PlaybackProgress {
    pub position: f64,
    pub duration: f64,
}

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
        let path_str = self.queue.front().unwrap().clone();
        let path = Path::new(&path_str);
        let info = self.load_song(path, 0.0);
        self.play_song(&info);
        self.emit_current_song(&path_str);
    }

    /// Seeks the current song to `position_secs` by restarting decoding from that position.
    pub fn seek(&mut self, position_secs: f64) {
        if self.queue.is_empty() {
            return;
        }
        if let Some(stream) = &self.stream {
            stream.pause().expect("Failed to pause stream or stream does not exist.");
        }
        self.ring_buffer = None;

        let path_str = self.queue.front().unwrap().clone();
        let path = Path::new(&path_str);
        let clamped = position_secs.max(0.0);
        let info = self.load_song(path, clamped);
        self.play_song(&info);
    }

    fn load_song(&mut self, path: &Path, start_position_secs: f64) -> AudioInfo {
        let info = decoder::get_audio_info(path);
        let target_sample_rate = self.device.default_output_config().unwrap().sample_rate().0;

        let rb = Arc::new(Mutex::new(ringbuf::HeapRb::new(
            (target_sample_rate as usize) * (info.channels as usize) * 10,
        )));
        self.ring_buffer = Some(rb.clone());

        let duration_secs = info.total_frames
            .map(|frames| frames as f64 / info.sample_rate as f64)
            .unwrap_or(0.0);

        let skip_frames = (start_position_secs * info.sample_rate as f64).round() as u64;
        let start_frame_target = (start_position_secs * target_sample_rate as f64).round() as u64;
        self.parameters.reset_position(target_sample_rate, duration_secs, start_frame_target);

        decoder::load_audio_into_buffer(path,
                                       rb,
                                       info.sample_rate,
                                       target_sample_rate,
                                       info.channels,
                                       skip_frames,
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

    /// Emits the path of the song that is now the one actually loaded/playing,
    /// so the frontend can update things like the currently displayed cover art.
    pub fn emit_current_song(&self, path_str: &str) {
        if let Err(err) = self.app_handle.emit("song-changed", path_str) {
            eprintln!("Failed to emit current song: {}", err);
        }
    }

    pub fn emit_progress(&self) {
        let progress = PlaybackProgress {
            position: self.parameters.position_secs(),
            duration: self.parameters.duration_secs,
        };
        if let Err(err) = self.app_handle.emit("playback-progress", progress) {
            eprintln!("Failed to emit playback progress: {}", err);
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
                    } else {
                        player.parameters.is_paused = true;
                        player.stream = None;
                        player.emit_playback_state();
                        player.emit_current_song("");
                    }
                }
            }
        });
    }

    /// Periodically emits the current playback position/duration so the frontend progress bar
    /// can stay in sync without polling.
    pub fn create_progress_thread(shared: Arc<Mutex<Option<MusicController>>>) {
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(200));
                let guard = shared.lock().unwrap();
                if let Some(player) = guard.as_ref() {
                    if !player.parameters.is_paused {
                        player.emit_progress();
                    }
                }
            }
        });
    }

}