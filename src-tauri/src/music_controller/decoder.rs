use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use std::thread::yield_now;

use ringbuf::{traits::*, HeapRb, SharedRb};
use ringbuf::storage::Heap;
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct AudioInfo {
    pub sample_rate: u32,
    pub channels: u32,
    pub bits_per_sample: u32,
    pub total_frames: Option<u64>,
}

pub fn get_audio_info(path: &Path) -> AudioInfo {
    let file = Box::new(File::open(path).expect("Failed to open file"));
    let mss = MediaSourceStream::new(file, Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .expect("Failed to probe audio format");

    let track = probed.format.default_track().expect("No default audio track found");
    let params = &track.codec_params;

    AudioInfo {
        sample_rate: params.sample_rate.unwrap_or(44100),
        channels: params.channels.map(|c| c.count() as u32).unwrap_or(2),
        bits_per_sample: params.bits_per_sample.unwrap_or(16),
        total_frames: params.n_frames,
    }
}

pub fn load_audio_into_buffer(
    path: &Path,
    rb: Arc<Mutex<SharedRb<Heap<f32>>>>,
    source_sr: u32,
    target_sr: u32,
    channels: u32,
    skip_frames: u64,
    tx: Sender<()>
) {
    let t_path: PathBuf = path.to_path_buf();

    std::thread::spawn(move || {
        let file = Box::new(File::open(t_path).expect("Failed to open file"));
        let mss = MediaSourceStream::new(file, Default::default());

        let hint = Hint::new();
        let mut format = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .expect("Failed to probe audio format")
            .format;

        let track = format.default_track().expect("No default audio track found");
        let track_id = track.id;

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .expect("Failed to build decoder");

        let mut samples_to_skip_manually = 0;
        if skip_frames > 0 {
            if let Ok(seeked) = format.seek(
                SeekMode::Accurate,
                SeekTo::TimeStamp { ts: skip_frames, track_id },
            ) {
                if skip_frames > seeked.actual_ts {
                    samples_to_skip_manually = ((skip_frames - seeked.actual_ts) * channels as u64) as usize;
                }
            }
        }

        let need_resampling = source_sr != target_sr;
        let chunk_size = 1024;
        let mut input_buffers = vec![Vec::<f32>::with_capacity(chunk_size); channels as usize];
        let mut channel_idx = 0;
        let mut resampler = create_resampler(target_sr, source_sr, channels);

        let mut sample_buf = None;

        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(Error::DecodeError(_)) => continue, // Skips "broken" frame
                Err(_) => break, // End of file / Error
            };

            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(decoded) => {
                    if sample_buf.is_none() {
                        let spec = *decoded.spec();
                        let duration = decoded.capacity() as u64;
                        sample_buf = Some(SampleBuffer::<f32>::new(duration, spec));
                    }

                    let buf = sample_buf.as_mut().unwrap();
                    buf.copy_interleaved_ref(decoded);
                    let samples = buf.samples();

                    for &normalized in samples {
                        if samples_to_skip_manually > 0 {
                            samples_to_skip_manually -= 1;
                            continue;
                        }

                        if need_resampling {
                            resample_and_push_to_buffer(&mut input_buffers,
                                                        &mut channel_idx,
                                                        normalized,
                                                        channels,
                                                        &mut resampler,
                                                        chunk_size,
                                                        &rb);
                        } else {
                            push_to_rb(normalized, &rb);
                        }
                    }
                }
                Err(Error::DecodeError(_)) => continue,
                Err(_) => break,
            }
        }

        if need_resampling && !input_buffers[0].is_empty() {
            let valid_frames = input_buffers[0].len();

            for buf in input_buffers.iter_mut() {
                buf.resize(chunk_size, 0.0);
            }

            let output = resampler.as_mut().unwrap().process(&input_buffers, None).unwrap();

            let ratio = target_sr as f64 / source_sr as f64;
            let valid_out_frames = (valid_frames as f64 * ratio).ceil() as usize;
            let final_frames = valid_out_frames.min(output[0].len());

            for frame_idx in 0..final_frames {
                for ch_idx in 0..(channels as usize) {
                    push_to_rb(output[ch_idx][frame_idx], &rb);
                }
            }
        }

        while !rb.lock().unwrap().is_empty() {
            yield_now();
        }

        tx.send(()).unwrap();
    });
}

fn push_to_rb(sample: f32, rb: &Arc<Mutex<HeapRb<f32>>>) {
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

    let resampler = {
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };
        Some(SincFixedIn::<f32>::new(
            target_sr as f64 / source_sr as f64,
            2.0,
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
                               rb: &Arc<Mutex<HeapRb<f32>>>) {

    input_buffers[*channel_idx].push(normalized);
    *channel_idx = (*channel_idx + 1) % (channels as usize);

    if *channel_idx == 0 && input_buffers[0].len() == chunk_size {
        let resampler_mut = resampler.as_mut().unwrap();
        let output = resampler_mut.process(&input_buffers, None).unwrap();

        let out_frames = output[0].len();
        for frame_idx in 0..out_frames {
            for ch_idx in 0..(channels as usize) {
                push_to_rb(output[ch_idx][frame_idx], &rb);
            }
        }

        for buf in input_buffers.iter_mut() {
            buf.clear();
        }
    }
}