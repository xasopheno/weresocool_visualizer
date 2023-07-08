mod graph;
use crossbeam_channel as channel;
use error_iter::ErrorIter as _;
use graph::*;
use log::error;
use pixels::{Error, Pixels, SurfaceTexture};
use portaudio as pa;
use std::convert::TryInto;
use std::sync::mpsc::{Receiver, Sender};
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

const WIDTH: u32 = 2048;
const HEIGHT: u32 = 1024;
const COUNT: u32 = 2048;

const LOGICAL_WIDTH: u32 = 1024;
const LOGICAL_HEIGHT: u32 = 512;
const BUFFER_SIZE: usize = 1024 * 4;
const FFT_DIV: usize = 8;

pub fn get_output_settings(pa: &pa::PortAudio) -> Result<pa::stream::OutputSettings<f32>, Error> {
    let def_output = pa.default_output_device().unwrap();
    let output_info = pa.device_info(def_output).unwrap();
    // println!("Default output device info: {:#?}", &output_info);
    let latency = output_info.default_low_output_latency;
    let output_params = pa::StreamParameters::new(def_output, 2, true, latency);

    let output_settings = pa::OutputStreamSettings::new(output_params, 48000.0, BUFFER_SIZE as u32);

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

    let (s_fft_r, r_fft_r) = channel::unbounded();
    let (s_fft_l, r_fft_l) = channel::unbounded();
    let (s_audio, r_audio) = channel::unbounded();
    let r_audio = Arc::new(Mutex::new(r_audio));
    let r_audio_clone = Arc::clone(&r_audio);
    let (read_fn_l, fft_handle_l) = WscFFT::spawn(BUFFER_SIZE as usize, r_fft_l);
    let (read_fn_r, fft_handle_r) = WscFFT::spawn(BUFFER_SIZE as usize, r_fft_r);

    // Open the audio file with hound
    let mut reader = hound::WavReader::open("./src/mmoodd.wav").unwrap();
    let spec = reader.spec();
    println!("{:?}", spec);

    // Initialize the PortAudio device
    let pa = pa::PortAudio::new().unwrap();
    let output_stream_settings = get_output_settings(&pa)?;
    dbg!(output_stream_settings);

    let mut stream = pa
        .open_non_blocking_stream(
            output_stream_settings,
            move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
                let r_audio_lock = r_audio_clone.lock().unwrap();
                let sender_l = s_fft_l.clone();
                let sender_r = s_fft_r.clone();
                let mut audio_data = (*r_audio_lock)
                    .recv()
                    .unwrap_or_else(|_| vec![0.0; frames * 2]); // *2 for stereo

                // If your FFT function needs mono data, you might want to average the stereo samples
                let fft_data_l: Vec<f32> = audio_data
                    .chunks(2)
                    .map(|stereo_sample| stereo_sample[0])
                    .collect();

                let fft_data_r: Vec<f32> = audio_data
                    .chunks(2)
                    .map(|stereo_sample| stereo_sample[0])
                    .collect();

                sender_l.send(fft_data_l).unwrap();
                sender_r.send(fft_data_r).unwrap();

                for frame in 0..frames {
                    let index = frame * 2; // *2 for stereo
                    buffer[index] = audio_data.remove(0); // Left channel
                    buffer[index + 1] = audio_data.remove(0); // Right channel
                }

                pa::Continue
            },
        )
        .unwrap();
    let mut interleaved_buffer = vec![0.0; BUFFER_SIZE * 2];

    // Create a new thread to handle audio processing
    thread::spawn(move || {
        let mut counter = 0;

        for sample in reader.samples::<f32>() {
            let sample = sample.unwrap();
            let index = counter % (BUFFER_SIZE * 2); // *2 for stereo

            interleaved_buffer[index] = sample;

            counter += 1;

            if counter % (BUFFER_SIZE * 2) == 0 {
                // *2 for stereo
                s_audio.send(interleaved_buffer.clone()).unwrap();
            }
        }
    });
    // Start the PortAudio stream
    stream.start().unwrap();

    event_loop.run(move |event, _, control_flow| {
        // The one and only event that winit_input_helper doesn't have for us...
        if let Event::RedrawRequested(_) = event {
            let fft_results_l = read_fn_l(); // Read the FFT results here
            let fft_results_r = read_fn_r(); // Read the FFT results here
            let l = fft_results_l[2..&fft_results_l.len() / FFT_DIV].to_vec();
            let r = fft_results_r[2..&fft_results_r.len() / FFT_DIV].to_vec();
            graph.update_bargraph(&[&l[..], &r[..]].concat()); // Assuming `graph` has an update method to handle new FFT results
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
