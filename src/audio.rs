use crossbeam_channel as channel;
use pa::{NonBlocking, Output, Stream};
use pixels::Error;
use portaudio as pa;
use std::sync::{Arc, Mutex};
use std::thread;

pub fn spawn_audio(
    config: &crate::core::WereSoCoolSpectrumConfig,
    filename: String,
    fft_sender_l: Arc<channel::Sender<Vec<f32>>>,
    fft_sender_r: Arc<channel::Sender<Vec<f32>>>,
) -> Stream<NonBlocking, Output<f32>> {
    let (audio_channel_sender, audio_receiever) = channel::unbounded();
    let audio_receiver = Arc::new(Mutex::new(audio_receiever));

    let stream_handle =
        spawn_audio_player(config, audio_receiver.clone(), fft_sender_l, fft_sender_r).unwrap();

    _ = spawn_audio_reader(config, filename, audio_channel_sender);

    stream_handle
}

fn spawn_audio_reader(
    config: &crate::core::WereSoCoolSpectrumConfig,
    filename: String,
    audio_channel_sender: channel::Sender<Vec<f32>>,
) -> Result<(), ()> {
    let mut reader = hound::WavReader::open(filename).unwrap();
    let spec = reader.spec();
    println!("{:?}", spec);
    let buffer_size = config.audio_buffer_size;

    thread::spawn(move || {
        let mut counter = 0;
        let mut interleaved_buffer = vec![0.0; buffer_size * 2];

        for sample in reader.samples::<f32>() {
            let sample = sample.unwrap();
            let index = counter % (buffer_size * 2);

            interleaved_buffer[index] = sample;

            counter += 1;

            if counter % (buffer_size * 2) == 0 {
                audio_channel_sender
                    .send(interleaved_buffer.clone())
                    .unwrap();
            }
        }
    });

    Ok(())
}

fn spawn_audio_player(
    config: &crate::core::WereSoCoolSpectrumConfig,
    audio_receiver: Arc<Mutex<channel::Receiver<Vec<f32>>>>,
    sender_l: Arc<channel::Sender<Vec<f32>>>,
    sender_r: Arc<channel::Sender<Vec<f32>>>,
) -> Result<Stream<NonBlocking, Output<f32>>, ()> {
    let pa = pa::PortAudio::new().unwrap();
    let output_stream_settings = get_output_settings(config, &pa).unwrap();

    let mut stream = pa
        .open_non_blocking_stream(
            output_stream_settings,
            move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
                let r_audio_lock = audio_receiver.lock().unwrap();
                let audio_data = (*r_audio_lock)
                    .recv()
                    .unwrap_or_else(|_| vec![0.0; frames * 2]); // *2 for stereo

                sender_l
                    .send(audio_data.iter().step_by(2).cloned().collect())
                    .unwrap();
                sender_r
                    .send(audio_data.iter().skip(1).step_by(2).cloned().collect())
                    .unwrap();

                for (frame, chunk) in audio_data.chunks_exact(2).enumerate() {
                    let index = frame * 2;
                    buffer[index] = chunk[0];
                    buffer[index + 1] = chunk[1];
                }

                pa::Continue
            },
        )
        .unwrap();

    stream.start().unwrap();

    Ok(stream)
}

fn get_output_settings(
    config: &crate::core::WereSoCoolSpectrumConfig,
    pa: &pa::PortAudio,
) -> Result<pa::stream::OutputSettings<f32>, Error> {
    let def_output = pa.default_output_device().unwrap();
    let output_info = pa.device_info(def_output).unwrap();
    let latency = output_info.default_low_output_latency;
    let output_params = pa::StreamParameters::new(def_output, 2, true, latency);

    let output_settings = pa::OutputStreamSettings::new(
        output_params,
        config.sample_rate as f64,
        config.audio_buffer_size as u32,
    );

    Ok(output_settings)
}
