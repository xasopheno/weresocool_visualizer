mod audio;
mod core;
mod fft_handler;
mod graph_handler;
mod grid;
mod window_handler;
use crate::audio::spawn_audio;
use crate::core::{WereSoCoolSpectrumConfig, WereSoCoolSpectrumCore};
use pixels::Error;

fn main() -> Result<(), Error> {
    let config = WereSoCoolSpectrumConfig::new();
    let (mut spectrum, (fft_sender_l, fft_sender_r)) = WereSoCoolSpectrumCore::new(&config)?;

    let _audio_stream_handle = spawn_audio(
        &config,
        "./src/mmoodd.wav".into(),
        fft_sender_l,
        fft_sender_r,
    );

    spectrum.run()?;

    Ok(())
}
