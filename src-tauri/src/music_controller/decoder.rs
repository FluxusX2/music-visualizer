use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::yield_now;
use claxon::FlacReader;
use ringbuf::{traits::*, HeapRb};
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};

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
pub fn load_flac_into_buffer(path: &Path,
                             rb: Arc<Mutex<HeapRb<f32>>>,
                             source_sr: u32,
                             target_sr: u32,
                             channels: u32, 
                             bits_per_sample: u32) {
    let t_path: PathBuf = path.to_path_buf();

    std::thread::spawn(move || {
        let mut reader = FlacReader::open(t_path).unwrap();

        let need_resampling = source_sr != target_sr;
        let chunk_size = 1024;
        let mut input_buffers = vec![Vec::<f32>::with_capacity(chunk_size); channels as usize];
        let mut channel_idx = 0;
        let mut resampler = create_resampler(target_sr, source_sr, channels);

        for sample in reader.samples() {
            let sample = sample.expect("Failed to read sample");
            let normalized = (sample as f32) / ((1 << (bits_per_sample - 1)) as f32);

            if need_resampling {
                resample_and_push_to_buffer(&mut input_buffers,
                                            &mut channel_idx,
                                            normalized,
                                            channels,
                                            &mut resampler,
                                            chunk_size,
                                            rb.clone());
            } else {
                push_to_rb(normalized, rb.clone());
            }
        }
        if need_resampling && !input_buffers[0].is_empty() {
            let valid_frames = input_buffers[0].len();

            // Pad the rest of the chunk with silence to satisfy rubato
            for buf in input_buffers.iter_mut() {
                buf.resize(chunk_size, 0.0);
            }

            let output = resampler.as_mut().unwrap().process(&input_buffers, None).unwrap();

            // Calculate roughly how many output frames correspond to the valid input frames
            let ratio = target_sr as f64 / source_sr as f64;
            let valid_out_frames = (valid_frames as f64 * ratio).ceil() as usize;
            let final_frames = valid_out_frames.min(output[0].len());

            for frame_idx in 0..final_frames {
                for ch_idx in 0..(channels as usize) {
                    push_to_rb(output[ch_idx][frame_idx], rb.clone());
                }
            }
        }
    });
}

fn push_to_rb(sample: f32, rb: Arc<Mutex<HeapRb<f32>>>) {
    loop {
        let mut buffer = rb.lock().unwrap();

        if !buffer.is_full() {
            buffer.try_push(sample).expect("Failed to push sample");
            break;
        }
        drop(buffer);
        yield_now();
    }
}

fn create_resampler(target_sr: u32, source_sr: u32, channels: u32) -> Option<SincFixedIn<f32>> {
    let chunk_size = 1024;

    // 1. Configure the resampler if needed
    let mut resampler = {
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };
        Some(SincFixedIn::<f32>::new(
            target_sr as f64 / source_sr as f64,
            2.0, // max expected resample ratio
            params,
            chunk_size,
            channels as usize,
        ).unwrap())
    };
    resampler
}

fn resample_and_push_to_buffer(input_buffers: &mut Vec<Vec<f32>>,
                               channel_idx: &mut usize,
                               normalized: f32,
                               channels: u32,
                               resampler: &mut Option<SincFixedIn<f32>>,
                               chunk_size: usize,
                               rb: Arc<Mutex<HeapRb<f32>>>) {

    input_buffers[*channel_idx].push(normalized);
    *channel_idx = (*channel_idx + 1) % (channels as usize);

    if *channel_idx == 0 && input_buffers[0].len() == chunk_size {
        let resampler_mut = resampler.as_mut().unwrap();
        let output = resampler_mut.process(&input_buffers, None).unwrap();

        let out_frames = output[0].len();
        for frame_idx in 0..out_frames {
            for ch_idx in 0..(channels as usize) {
                push_to_rb(output[ch_idx][frame_idx], rb.clone());
            }
        }

        for buf in input_buffers.iter_mut() {
            buf.clear();
        }
    }
}