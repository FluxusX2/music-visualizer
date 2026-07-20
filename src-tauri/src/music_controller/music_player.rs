use std::thread::yield_now;
use std::sync::atomic::Ordering;
use cpal::traits::{DeviceTrait, StreamTrait};
use ringbuf::traits::*;
use crate::music_controller::decoder::AudioInfo;
use crate::music_controller::MusicController;

pub fn play_song(music_controller: &mut MusicController, info: &AudioInfo) {

    let target_sr = music_controller.device.default_output_config()
        .expect("No default config")
        .sample_rate()
        .0;

    let config = cpal::StreamConfig {
        channels: info.channels as cpal::ChannelCount,
        sample_rate: cpal::SampleRate(target_sr),
        buffer_size: cpal::BufferSize::Default,
    };

    let err_fn = |err| eprintln!("An error occurred on the output audio stream: {}", err);
    let rb = std::sync::Arc::clone(music_controller.ring_buffer.as_ref().unwrap());
    let vol = std::sync::Arc::clone(&music_controller.parameters.volume);
    let played_frames = std::sync::Arc::clone(&music_controller.parameters.played_frames);
    let channels = info.channels as u64;

    let stream = music_controller.device.build_output_stream(
        &config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let vol = *vol.lock().unwrap();
            let mut buffer = rb.lock().unwrap();
            for sample in data.iter_mut() {
                *sample = buffer.try_pop().unwrap_or(0.0) * vol;
            }
            drop(buffer);
            played_frames.fetch_add(data.len() as u64 / channels, Ordering::Relaxed);
        },
        err_fn,
        None,
    ).expect("Device does not support this channel configuration");

    let target = (info.sample_rate as usize) * (info.channels as usize) * 2;

    //Assert that buffer is full enough (at least 2s of audio).
    loop {
        let filled = {
            let rb = music_controller.ring_buffer.as_ref().unwrap().lock().unwrap();
            let occupied = rb.occupied_len();
            drop(rb);
            occupied
        };

        if filled >= target {
            break;
        }
        yield_now();
    }
    
    stream.play().expect("Failed to play the stream");
    
    music_controller.stream = Some(stream);
    music_controller.parameters.is_paused = false;
    music_controller.emit_playback_state();

}