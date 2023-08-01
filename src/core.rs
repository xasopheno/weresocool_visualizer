use crate::graph_handler::GraphHandler;
use crate::graph_handler::Senders;
use crate::window_handler::WindowHandler;
use pixels::Error;
use std::sync::{Arc, Mutex};
use winit::{
    dpi::PhysicalPosition,
    event::{DeviceEvent, ElementState, Event, MouseButton, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};
use winit_input_helper::WinitInputHelper;

pub struct WereSoCoolSpectrumConfig {
    pub width: u32,
    pub height: u32,
    pub logical_width: u32,
    pub logical_height: u32,
    pub visual_buffer_size: usize,
    pub audio_buffer_size: usize,
    pub ring_buffer_size: usize,
    pub fft_div: usize,
    pub sample_rate: usize,
}

impl WereSoCoolSpectrumConfig {
    #[allow(dead_code)]
    pub fn new(buffer_size: usize) -> Self {
        // Self {
        // width: 1024 * 2,
        // height: 1024 / 7,
        // logical_width: 1024,
        // logical_height: 512 / 9,
        // visual_buffer_size: 1024 * 2,
        // audio_buffer_size: 1024 * 2 * 6,
        // ring_buffer_size: 10,
        // fft_div: 12,
        // sample_rate: 48_000,
        // }

        Self {
            width: 1024 * 2,
            height: 1024,
            logical_width: 1024,
            logical_height: 1024,
            visual_buffer_size: 1024 * 2,
            audio_buffer_size: buffer_size,
            ring_buffer_size: 10,
            fft_div: 12,
            sample_rate: 48_000,
        }
    }
}

pub struct WereSoCoolSpectrum {
    window: Arc<Mutex<winit::window::Window>>,
    graph_handler: Arc<Mutex<GraphHandler>>,
    event_loop: Option<EventLoop<()>>,
    pub scale_factor: f64,
}

impl WereSoCoolSpectrum {
    #[allow(dead_code)]
    pub fn new(config: &WereSoCoolSpectrumConfig) -> Result<(Self, Senders), Error> {
        let event_loop = EventLoop::new();

        let window_handler = WindowHandler::new(&event_loop);

        let graph_handler = GraphHandler::new(
            config,
            &window_handler.window,
            window_handler.width,
            window_handler.height,
        )?;

        let (fft_sender_l, fft_sender_r) = graph_handler.get_fft_senders();

        let window = Arc::new(Mutex::new(window_handler.window));
        let graph_handler = Arc::new(Mutex::new(graph_handler));

        Ok((
            Self {
                window,
                graph_handler,
                event_loop: Some(event_loop),
                scale_factor: window_handler.scale_factor,
            },
            (fft_sender_l, fft_sender_r),
        ))
    }

    #[allow(dead_code)]
    pub fn run(&mut self) -> Result<(), Error> {
        let mut input = WinitInputHelper::new();
        let mut is_dragging = false;
        let mut _prev_mouse_position: Option<PhysicalPosition<f64>> = None;

        let window = Arc::clone(&self.window);
        let graph_handler = Arc::clone(&self.graph_handler);
        let event_loop = self.event_loop.take().unwrap();

        event_loop.run(move |event, _, control_flow| {
            if let Event::RedrawRequested(_) = event {
                let mut graph_handler_guard = graph_handler.lock().unwrap();
                if graph_handler_guard.update_and_draw().is_err() {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }

            if let Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } = event
            {
                if button == MouseButton::Left {
                    is_dragging = state == ElementState::Pressed;
                }
            }

            if let Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } = event
            {
                if is_dragging {
                    if let Ok(locked_window) = window.lock() {
                        if let Ok(mut outer_position) = locked_window.outer_position() {
                            outer_position.x += delta.0 as i32 * 2;
                            outer_position.y += delta.1 as i32 * 2;
                            locked_window.set_outer_position(outer_position);
                        }
                    }
                }
            }

            if input.update(&event) {
                if input.key_pressed(VirtualKeyCode::Escape)
                    || input.close_requested()
                    || input.destroyed()
                {
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
