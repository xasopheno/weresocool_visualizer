mod grid;
use crossbeam_channel as channel;
use error_iter::ErrorIter as _;
use grid::*;
use log::error;
use pixels::{Error, Pixels, SurfaceTexture};
use portaudio as pa;
use std::sync::{Arc, Mutex};
use std::thread;
use weresocool_fft::WscFFT;
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

const WIDTH: u32 = 2048;
const HEIGHT: u32 = 1024;
const LOGICAL_WIDTH: u32 = 1024;
const LOGICAL_HEIGHT: u32 = 512;
const BUFFER_SIZE: usize = 1024 * 4;
const FFT_DIV: usize = 20;

struct FFTHandler {
    buffer_size: usize,
    num_results: usize,
    read_fn: Box<dyn Fn() -> Vec<f32>>,
}

impl FFTHandler {
    fn new(buffer_size: usize, num_results: usize, r_fft: channel::Receiver<Vec<f32>>) -> Self {
        let (read_fn, _) = WscFFT::spawn(buffer_size, r_fft);
        FFTHandler {
            buffer_size,
            num_results,
            read_fn: Box::new(read_fn),
        }
    }

    fn read_results(&self) -> Vec<f32> {
        let results = (self.read_fn)();
        results[2..self.num_results].to_vec()
    }
}

struct WindowHandler {
    width: u32,
    height: u32,
    window: winit::window::Window,
}

impl WindowHandler {
    fn new(width: u32, height: u32, event_loop: &EventLoop<()>) -> Self {
        let size = LogicalSize::new(width as f64, height as f64);
        let window = WindowBuilder::new()
            .with_title("weresoFFT")
            .with_inner_size(size)
            .build(&event_loop)
            .unwrap();

        WindowHandler {
            width,
            height,
            window,
        }
    }

    fn inner_size(&self) -> (u32, u32) {
        let size = self.window.inner_size();
        (size.width, size.height)
    }
}

struct GraphHandler {
    width: usize,
    height: usize,
    grid: Grid,
}

impl GraphHandler {
    fn new(width: usize, height: usize) -> Self {
        let grid = Grid::new_bargraph(width, height);
        GraphHandler {
            width,
            height,
            grid,
        }
    }

    fn update_and_draw(&mut self, pixels: &mut [u8], l: &[f32], r: &[f32]) {
        self.grid.update_bargraph(l, r);
        self.grid.draw(pixels);
    }
}

fn get_output_settings(pa: &pa::PortAudio) -> Result<pa::stream::OutputSettings<f32>, Error> {
    let def_output = pa.default_output_device().unwrap();
    let output_info = pa.device_info(def_output).unwrap();
    let latency = output_info.default_low_output_latency;
    let output_params = pa::StreamParameters::new(def_output, 2, true, latency);

    let output_settings = pa::OutputStreamSettings::new(output_params, 48000.0, BUFFER_SIZE as u32);

    Ok(output_settings)
}

fn main() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();

    let window_handler = WindowHandler::new(LOGICAL_WIDTH, LOGICAL_HEIGHT, &event_loop);

    let mut pixels = {
        let (window_width, window_height) = window_handler.inner_size();
        let surface_texture =
            SurfaceTexture::new(window_width, window_height, &window_handler.window);
        Pixels::new(WIDTH, HEIGHT, surface_texture)?
    };

    let mut graph_handler = GraphHandler::new(WIDTH as usize, HEIGHT as usize);

    let (s_fft_r, r_fft_r) = channel::unbounded();
    let (s_fft_l, r_fft_l) = channel::unbounded();
    let (s_audio, r_audio) = channel::unbounded();
    let r_audio = Arc::new(Mutex::new(r_audio));
    let r_audio_clone = Arc::clone(&r_audio);

    let fft_handler_l = FFTHandler::new(BUFFER_SIZE as usize, BUFFER_SIZE / FFT_DIV, r_fft_l);
    let fft_handler_r = FFTHandler::new(BUFFER_SIZE as usize, BUFFER_SIZE / FFT_DIV, r_fft_r);

    let mut reader = hound::WavReader::open("./src/for_sam.wav").unwrap();
    let spec = reader.spec();
    println!("{:?}", spec);

    let pa = pa::PortAudio::new().unwrap();
    let output_stream_settings = get_output_settings(&pa)?;

    let mut stream = pa
        .open_non_blocking_stream(
            output_stream_settings,
            move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
                let r_audio_lock = r_audio_clone.lock().unwrap();
                let sender_l = s_fft_l.clone();
                let sender_r = s_fft_r.clone();
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

    let mut interleaved_buffer = vec![0.0; BUFFER_SIZE * 2];

    thread::spawn(move || {
        let mut counter = 0;

        for sample in reader.samples::<f32>() {
            let sample = sample.unwrap();
            let index = counter % (BUFFER_SIZE * 2);

            interleaved_buffer[index] = sample;

            counter += 1;

            if counter % (BUFFER_SIZE * 2) == 0 {
                s_audio.send(interleaved_buffer.clone()).unwrap();
            }
        }
    });

    stream.start().unwrap();

    event_loop.run(move |event, _, control_flow| {
        if let Event::RedrawRequested(_) = event {
            let fft_results_l = fft_handler_l.read_results();
            let fft_results_r = fft_handler_r.read_results();

            graph_handler.update_and_draw(pixels.frame_mut(), &fft_results_l, &fft_results_r);

            if pixels.render().is_err() {
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
                _ = pixels.resize_surface(size.width, size.height);
            }
        }

        if let Event::MainEventsCleared = event {
            window_handler.window.request_redraw();
        }
    });
}
