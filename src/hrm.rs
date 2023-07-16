mod audio;
mod fft_handler;
mod graph_handler;
mod grid;
mod window_handler;
use crate::audio::spawn_audio;
use crate::graph_handler::GraphHandler;
use crossbeam_channel as channel;
use pixels::Error;
use std::sync::{Arc, Mutex};
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

    let _stream_handle = spawn_audio(
        BUFFER_SIZE,
        "./src/for_sam.wav".into(),
        audio_channel_sender,
        audio_receiver,
        fft_sender_l,
        fft_sender_r,
    );

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
                _ = graph_handler.resize_surface(size.width, size.height);
            }
        }

        if let Event::MainEventsCleared = event {
            window_handler.window.request_redraw();
        }
    });
}
