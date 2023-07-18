use crate::graph_handler::GraphHandler;
use crate::window_handler::WindowHandler;
use crossbeam_channel as channel;
use pixels::Error;
use std::sync::{Arc, Mutex};
use winit::{
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
};
use winit_input_helper::WinitInputHelper;

pub struct WereSoCoolSpectrumConfig {
    pub width: u32,
    pub height: u32,
    pub logical_width: u32,
    pub logical_height: u32,
    pub buffer_size: usize,
    pub fft_div: usize,
}

impl WereSoCoolSpectrumConfig {
    pub fn new() -> Self {
        Self {
            width: 1024 * 2,
            height: 1024 / 7,
            logical_width: 1024,
            logical_height: 512 / 9,
            buffer_size: 1024 * 2,
            fft_div: 16,
        }
    }
}

pub struct WereSoCoolSpectrumCore {
    window: Arc<Mutex<winit::window::Window>>,
    graph_handler: Arc<Mutex<GraphHandler>>,
    event_loop: Option<EventLoop<()>>,
}

impl WereSoCoolSpectrumCore {
    pub fn new(
        config: &WereSoCoolSpectrumConfig,
    ) -> Result<
        (
            Self,
            (
                Arc<channel::Sender<Vec<f32>>>,
                Arc<channel::Sender<Vec<f32>>>,
            ),
        ),
        Error,
    > {
        let event_loop = EventLoop::new();

        let window_handler =
            WindowHandler::new(config.logical_width, config.logical_height, &event_loop);

        let graph_handler = GraphHandler::new(
            config.width as usize,
            config.height as usize,
            config.buffer_size as usize,
            config.buffer_size / config.fft_div,
            &window_handler.window,
        )?;

        let (fft_sender_l, fft_sender_r) = graph_handler.get_fft_senders();

        let window = Arc::new(Mutex::new(window_handler.window));
        let graph_handler = Arc::new(Mutex::new(graph_handler));

        Ok((
            Self {
                window,
                graph_handler,
                event_loop: Some(event_loop),
            },
            (fft_sender_l, fft_sender_r),
        ))
    }

    pub fn run(&mut self) -> Result<(), Error> {
        let mut input = WinitInputHelper::new();

        let window = Arc::clone(&self.window);
        let graph_handler = Arc::clone(&self.graph_handler);
        let event_loop = self.event_loop.take().unwrap();

        event_loop.run(move |event, _, control_flow| {
            if let Event::RedrawRequested(_) = event {
                let mut graph_handler_guard = graph_handler.lock().unwrap();
                if let Err(_) = graph_handler_guard.update_and_draw() {
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
                    let _ = graph_handler
                        .lock()
                        .unwrap()
                        .resize_surface(size.width, size.height);
                }
            }

            if let Event::MainEventsCleared = event {
                window.lock().unwrap().request_redraw();
            }
        });
    }
}
