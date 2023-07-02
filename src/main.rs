#![deny(clippy::all)]
#![forbid(unsafe_code)]

use error_iter::ErrorIter as _;
use log::{debug, error};
use pixels::{Error, Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

const WIDTH: u32 = 4000;
const HEIGHT: u32 = 3000;

fn main() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();

    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        let scaled_size = LogicalSize::new(WIDTH as f64 * 3.0, HEIGHT as f64 * 3.0);
        WindowBuilder::new()
            .with_title("Conway's Game of Life")
            .with_inner_size(scaled_size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH, HEIGHT, surface_texture)?
    };

    let mut life = Grid::new_alternating(WIDTH as usize, HEIGHT as usize);

    event_loop.run(move |event, _, control_flow| {
        // The one and only event that winit_input_helper doesn't have for us...
        if let Event::RedrawRequested(_) = event {
            life.draw(pixels.frame_mut());
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
            life.update();
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

const BIRTH_RULE: [bool; 9] = [false, false, false, true, false, false, false, false, false];
const SURVIVE_RULE: [bool; 9] = [false, false, true, true, false, false, false, false, false];
const INITIAL_FILL: f32 = 0.3;

#[derive(Clone, Copy, Debug, Default)]
struct Cell {
    alive: bool,
    heat: u8,
}

impl Cell {
    fn new(alive: bool) -> Self {
        Self { alive, heat: 0 }
    }

    #[must_use]
    fn update_neibs(self, heat: u8) -> Self {
        self.next_state(heat)
    }

    #[must_use]
    fn next_state(mut self, heat: u8) -> Self {
        self.heat = heat;
        self
    }

    // fn set_alivet(&mut self, alive: bool) {
    // *self = self.next_state(alive);
    // }

    fn cool_off(&mut self, decay: f32) {
        if !self.alive {
            let heat = (self.heat as f32 * decay).clamp(0.0, 255.0);
            assert!(heat.is_finite());
            self.heat = heat as u8;
        }
    }
}

#[derive(Clone, Debug)]
struct Grid {
    cells: Vec<Cell>,
    width: usize,
    height: usize,
    // Should always be the same size as `cells`. When updating, we read from
    // `cells` and write to `scratch_cells`, then swap. Otherwise it's not in
    // use, and `cells` should be updated directly.
    scratch_cells: Vec<Cell>,
}

impl Grid {
    fn new_empty(width: usize, height: usize) -> Self {
        assert!(width != 0 && height != 0);
        let size = width.checked_mul(height).expect("too big");
        Self {
            cells: vec![Cell::default(); size],
            scratch_cells: vec![Cell::default(); size],
            width,
            height,
        }
    }

    fn new_alternating(width: usize, height: usize) -> Self {
        let mut result = Self::new_empty(width, height);
        result.fill_alternating();
        result
    }

    fn fill_alternating(&mut self) {
        let values = (0..10).map(|x| x as f64 * 0.01);
        let grid = (0..10).into_iter().map(|x| x as f32).collect::<Vec<_>>();

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = x + y * self.width;
                // Alternate based on row

                let grid_idx = (y % grid.len()) as usize;
                self.cells[idx] = Cell::new(f32::sin(y as f32 * x as f32 * 300.0) > 0.0);
            }
        }
    }

    fn update(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = x + y * self.width;
                let next = self.cells[idx];
                // Write into scratch_cells, since we're still reading from `self.cells`
                self.scratch_cells[idx] = next;
            }
        }
        std::mem::swap(&mut self.scratch_cells, &mut self.cells);
    }

    fn draw(&self, screen: &mut [u8]) {
        debug_assert_eq!(screen.len(), 4 * self.cells.len());
        for (c, pix) in self.cells.iter().zip(screen.chunks_exact_mut(4)) {
            let color = if c.alive {
                [0, 0xff, 0xff, 0xff]
            } else {
                [0, 0, c.heat, 0xff]
            };
            pix.copy_from_slice(&color);
        }
    }

    fn grid_idx<I: std::convert::TryInto<usize>>(&self, x: I, y: I) -> Option<usize> {
        match (x.try_into(), y.try_into()) {
            (Ok(x), Ok(y)) if x < self.width && y < self.height => Some(x + y * self.width),
            _ => None,
        }
    }
}
