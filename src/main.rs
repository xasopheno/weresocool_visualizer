mod audio;
mod core;
mod fft_handler;
mod graph_handler;
mod grid;
mod window_handler;
use crate::audio::spawn_audio;
use pixels::Error;
use weresocool_visualizer::{WereSoCoolSpectrum, WereSoCoolSpectrumConfig};

fn main() -> Result<(), Error> {
    let config = WereSoCoolSpectrumConfig::new();
    let (mut spectrum, (fft_sender_l, fft_sender_r)) = WereSoCoolSpectrum::new(&config)?;

    let _audio_stream_handle = spawn_audio(
        &config,
        "./src/for_sam.wav".into(),
        fft_sender_l,
        fft_sender_r,
    );

    spectrum.run()?;

    Ok(())
}
