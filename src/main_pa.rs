mod graph;
use crossbeam_channel as channel;
use crossbeam_channel::{Receiver, Sender};
use error_iter::ErrorIter as _;
use graph::*;
use log::error;
use pixels::{Error, Pixels, SurfaceTexture};
use portaudio as pa;
use std::convert::TryInto;
// use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use weresocool_fft::WscFFT;
use weresocool_fft::*;
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 1024;
const COUNT: u32 = 1024;

const LOGICAL_WIDTH: u32 = 1024;
const LOGICAL_HEIGHT: u32 = 512;

pub fn get_output_settings(pa: &pa::PortAudio) -> Result<pa::stream::OutputSettings<f32>, Error> {
    let def_output = pa.default_output_device().unwrap();
    let output_info = pa.device_info(def_output).unwrap();
    // println!("Default output device info: {:#?}", &output_info);
    let latency = output_info.default_low_output_latency;
    let output_params = pa::StreamParameters::new(def_output, 2, true, latency);

    let output_settings = pa::OutputStreamSettings::new(output_params, 48000.0, 1024);

    Ok(output_settings)
}
fn main() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();

    let window = {
        let size = LogicalSize::new(LOGICAL_WIDTH as f64, LOGICAL_HEIGHT as f64);
        WindowBuilder::new()
            .with_title("weresoFFT")
            .with_inner_size(size)
            // .with_inner_size(scaled_size)
            // .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH, HEIGHT, surface_texture)?
    };

    let mut graph = Grid::new_bargraph(WIDTH as usize, HEIGHT as usize);

    let buffer_size = 1024;

    let (s_fft, r_fft) = channel::unbounded();
    let (s_audio, r_audio): (Sender<Vec<f32>>, Receiver<Vec<f32>>) = channel::unbounded();
    let r_audio = Arc::new(Mutex::new(r_audio));
    let r_audio_clone = Arc::clone(&r_audio);
    let (read_fn, fft_handle) = WscFFT::spawn(buffer_size, r_fft);

    let wav_reader = hound::WavReader::open("src/simple.wav").unwrap();
    let spec = wav_reader.spec();
    let samples: Vec<_> = wav_reader
        .into_samples::<f32>()
        .filter_map(Result::ok)
        .collect();

    let pa = pa::PortAudio::new().unwrap();

    let settings = get_output_settings(&pa)?;

    let callback = move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
        let mut idx = 0;
        for frame in 0..frames {
            buffer[frame] = (samples[idx] as f32) / std::i16::MAX as f32;
            idx += 1;
            if idx >= samples.len() {
                return pa::Complete;
            }
        }

        pa::Continue
    };

    let mut stream = pa.open_non_blocking_stream(settings, callback).unwrap();
    stream.start().unwrap();

    // let mut stream = pa.open_non_blocking_stream(settings, callback).unwrap();
    // stream.start().unwrap();
    // let mut interleaved_buffer = vec![0.0; buffer_size * 2];

    // // Create a new thread to handle audio processing
    // thread::spawn(move || {
    // let mut counter = 0;

    // let sender = s_fft.clone();
    // for sample in reader.samples::<f32>() {
    // let sample = sample.unwrap();
    // let index = counter % buffer_size;

    // interleaved_buffer[index * 2] = sample; // Left channel
    // interleaved_buffer[index * 2 + 1] = sample; // Right channel

    // counter += 1;

    // if counter % buffer_size == 0 {
    // s_audio.send(interleaved_buffer.clone()).unwrap();
    // }

    // sender
    // .send(interleaved_buffer.clone()[0..interleaved_buffer.len() / 2].to_vec())
    // .unwrap();
    // }
    // });
    // Start the PortAudio stream
    // stream.start().unwrap();

    event_loop.run(move |event, _, control_flow| {
        // The one and only event that winit_input_helper doesn't have for us...
        if let Event::RedrawRequested(_) = event {
            let fft_results = read_fn(); // Read the FFT results here
            graph.update(&fft_results); // Assuming `graph` has an update method to handle new FFT results
            graph.draw(pixels.frame_mut());
            if let Err(err) = pixels.render() {
                log_error("pixels.render", err);
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        // For everything else, for let winit_input_helper collect events to build its state.
        // It returns `true` when it is time to update our game state and request a redraw.
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.close_requested() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                if let Err(err) = pixels.resize_surface(size.width, size.height) {
                    log_error("pixels.resize_surface", err);
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }
            window.request_redraw();
        }
    });
}

fn log_error<E: std::error::Error + 'static>(method_name: &str, err: E) {
    error!("{method_name}() failed: {err}");
    for source in err.sources().skip(1) {
        error!("  Caused by: {source}");
    }
}
