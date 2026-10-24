//! CPAL リアルタイム・オーディオ出力ストリーム管理

use std::sync::{Arc, Mutex};
use crate::emulator::ApuSystem;

/// デフォルト出力デバイスを用いて CPAL オーディオストリームを開始します
pub fn start_audio_stream(system: Arc<Mutex<ApuSystem>>) -> Option<cpal::Stream> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = cpal::default_host();
    let device = host.default_output_device()?;
    let config = device.default_output_config().ok()?;
    let sample_rate = config.sample_rate() as f32;
    let channels_count = config.channels() as usize;

    let stream_sys = Arc::clone(&system);
    let stream_config: cpal::StreamConfig = config.clone().into();
    let err_fn = |err| eprintln!("Audio stream error: {}", err);

    let stream = device
        .build_output_stream(
            stream_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut sys = stream_sys.lock().unwrap();
                for frame in data.chunks_mut(channels_count) {
                    let sample = sys.generate_audio_sample(sample_rate);
                    for channel in frame.iter_mut() {
                        *channel = sample;
                    }
                }
            },
            err_fn,
            None,
        )
        .ok()?;

    stream.play().ok()?;
    Some(stream)
}
