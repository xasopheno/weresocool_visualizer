mod fft_handler;
mod graph_handler;
mod grid;
mod window_handler;
use crate::graph_handler::GraphHandler;
use crossbeam_channel as channel;
use fft_handler::FFTHandler;
use pa::{NonBlocking, Output, Stream};
use pixels::Error;
use portaudio as pa;
use std::sync::{Arc, Mutex};
use std::thread;
use window_handler::WindowHandler;
use winit::{
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
};
use winit_input_helper::WinitInputHelper;

const WIDTH: u32 = 2048;
const HEIGHT: u32 = 1024 / 7;
const LOGICAL_WIDTH: u32 = 1024;
const LOGICAL_HEIGHT: u32 = 512 / 9;
const BUFFER_SIZE: usize = 1024 * 2;
const FFT_DIV: usize = 24;

fn main() -> Result<(), Error> {
    let (audio_channel_sender, audio_receiever) = channel::unbounded();
    let audio_receiver = Arc::new(Mutex::new(audio_receiever));

    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();

    let window_handler = WindowHandler::new(LOGICAL_WIDTH, LOGICAL_HEIGHT, &event_loop);

    let mut graph_handler = GraphHandler::new(
        WIDTH as usize,
        HEIGHT as usize,
        BUFFER_SIZE as usize,
        BUFFER_SIZE / FFT_DIV,
        &window_handler.window,
    )
    .unwrap();

    let (fft_sender_l, fft_sender_r) = graph_handler.get_fft_senders();

    let _stream_handle = spawn_audio_player(audio_receiver.clone(), fft_sender_l, fft_sender_r);
    _ = spawn_audio_reader("./src/for_sam.wav".into(), audio_channel_sender);

    event_loop.run(move |event, _, control_flow| {
        if let Event::RedrawRequested(_) = event {
            window_handler.window.set_visible(true);
            if let Err(_) = graph_handler.update_and_draw() {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        if input.update(&event) {
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }
            if let Some(size) = input.window_resized() {
                // _ = pixels.resize_buffer(size.width, size.height);
                _ = graph_handler.resize_surface(size.width, size.height);
            }
        }

        if let Event::MainEventsCleared = event {
            window_handler.window.request_redraw();
        }
    });
}

fn spawn_audio_reader(
    filename: String,
    audio_channel_sender: channel::Sender<Vec<f32>>,
) -> Result<(), ()> {
    let mut reader = hound::WavReader::open(filename).unwrap();
    let spec = reader.spec();
    println!("{:?}", spec);

    thread::spawn(move || {
        let mut counter = 0;
        let mut interleaved_buffer = vec![0.0; BUFFER_SIZE * 2];

        for sample in reader.samples::<f32>() {
            let sample = sample.unwrap();
            let index = counter % (BUFFER_SIZE * 2);

            interleaved_buffer[index] = sample;

            counter += 1;

            if counter % (BUFFER_SIZE * 2) == 0 {
                audio_channel_sender
                    .send(interleaved_buffer.clone())
                    .unwrap();
            }
        }
    });

    Ok(())
}

fn spawn_audio_player(
    audio_receiver: Arc<Mutex<channel::Receiver<Vec<f32>>>>,
    sender_l: Arc<channel::Sender<Vec<f32>>>,
    sender_r: Arc<channel::Sender<Vec<f32>>>,
) -> Result<Stream<NonBlocking, Output<f32>>, ()> {
    let pa = pa::PortAudio::new().unwrap();
    let output_stream_settings = get_output_settings(&pa).unwrap();

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

fn get_output_settings(pa: &pa::PortAudio) -> Result<pa::stream::OutputSettings<f32>, Error> {
    let def_output = pa.default_output_device().unwrap();
    let output_info = pa.device_info(def_output).unwrap();
    let latency = output_info.default_low_output_latency;
    let output_params = pa::StreamParameters::new(def_output, 2, true, latency);

    let output_settings = pa::OutputStreamSettings::new(output_params, 48000.0, BUFFER_SIZE as u32);

    Ok(output_settings)
}
