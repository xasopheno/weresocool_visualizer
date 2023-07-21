use crate::grid::*;
use winit::platform::macos::WindowBuilderExtMacOS;
use winit::{dpi::LogicalSize, event_loop::EventLoop, window::WindowBuilder};

pub struct WindowHandler {
    width: u32,
    height: u32,
    pub window: winit::window::Window,
}

impl WindowHandler {
    pub fn new(width: u32, height: u32, event_loop: &EventLoop<()>) -> Self {
        let monitor = event_loop.primary_monitor().unwrap();
        let monitor_size = monitor.size();
        let scale_factor = monitor.scale_factor();
        dbg!(scale_factor);
        // let actual_width = monitor_size.width / 2;
        dbg!(monitor_size);
        // let logical_size = LogicalSize::new(0.9 * actual_width as f64, height as f64);
        let logical_size = LogicalSize::new(
            monitor_size.width as f64 / scale_factor * 0.5,
            monitor_size.height as f64 / scale_factor / 15.0,
        );

        let window = WindowBuilder::new()
            .with_title("weresoFFT")
            // .with_decorations(false)
            .with_titlebar_hidden(true)
            .with_active(false)
            // .with_has_shadow(true)
            // .with_inner_size(size)
            // .with_position(winit::dpi::PhysicalPosition::new(0, 0))
            // .with_titlebar_buttons_hidden(false)
            .build(&event_loop)
            .unwrap();

        window.set_window_level(winit::window::WindowLevel::AlwaysOnTop);
        window.set_inner_size(logical_size);
        window.set_outer_position(winit::dpi::PhysicalPosition::new(
            0.5 * monitor_size.width as f32,
            0.0 * monitor_size.height as f32,
            // 0.05 * monitor_size.width as f32,
            // 0.05 * monitor_size.height as f32,
        ));
        // window.set_outer_position(winit::dpi::LogicalPosition::new(0.0, 0.0));

        WindowHandler {
            width,
            height,
            window,
        }
    }

    pub fn inner_size(&self) -> (u32, u32) {
        let size = self.window.inner_size();
        (size.width, size.height)
    }
}
