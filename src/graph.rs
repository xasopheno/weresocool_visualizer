use crossbeam_channel as channel;
use error_iter::ErrorIter as _;
use log::error;
use pixels::{Error, Pixels, SurfaceTexture};
use portaudio as pa;
use rodio::Source;
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

#[derive(Clone, Copy, Debug, Default)]
pub struct Cell {
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
pub struct Grid {
    cells: Vec<Cell>,
    width: usize,
    height: usize,
    time: std::time::SystemTime,
    // Should always be the same size as `cells`. When updating, we read from
    // `cells` and write to `scratch_cells`, then swap. Otherwise it's not in
    // use, and `cells` should be updated directly.
    scratch_cells: Vec<Cell>,
}

impl Grid {
    fn new_empty(width: usize, height: usize) -> Self {
        assert!(width != 0 && height != 0);
        let size = width.checked_mul(height).expect("too big");
        let now = std::time::SystemTime::now();

        Self {
            cells: vec![Cell::default(); size],
            scratch_cells: vec![Cell::default(); size],
            width,
            height,
            time: now,
        }
    }

    pub fn new_bargraph(width: usize, height: usize) -> Self {
        let mut result = Self::new_empty(width, height);
        let heights: Vec<f32> = (0..1024).map(|x| f32::sin(x as f32)).collect();
        result.fill_bargraph(&heights);
        result
    }

    pub fn update_bargraph(&mut self, new_heights: &[f32]) {
        // Clear the existing grid first
        self.cells = vec![Cell::default(); self.width * self.height];

        // Now fill the grid with the new bar graph
        self.fill_bargraph(new_heights);
    }

    fn fill_bargraph(&mut self, heights: &[f32]) {
        // assert!(
        // heights.len() <= 512 as usize,
        // "Too many heights provided for the width of the grid"
        // );

        // The width of each bar, assuming the number of bars is less than or equal to the width of the grid
        let bar_width = self.width / heights.len();

        for (bar_idx, &bar_height) in heights.iter().enumerate() {
            let grid_height = (bar_height * self.height as f32).round() as usize;
            let grid_height = std::cmp::min(grid_height, self.height);

            for bar_x in bar_idx * bar_width..(bar_idx + 1) * bar_width {
                for y in 0..self.height {
                    let idx = bar_x + y * self.width;
                    // Set the cell to alive if its y-coordinate is less than the bar height
                    self.cells[idx] = Cell::new(y > self.height - grid_height);
                }
            }
        }

        // Fill remaining cells with false if there are fewer bars than the width of the grid
        for x in heights.len() * bar_width..self.width {
            for y in 0..self.height {
                let idx = x + y * self.width;
                self.cells[idx] = Cell::new(false);
            }
        }
    }

    fn process_buffer(buffer: &mut Vec<f32>) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let mut complex_buffer = f32_to_complex(&buffer);

        fft_in_place(&mut complex_buffer);

        let magnitude_buffer: Vec<_> = complex_buffer.iter().map(|c| c.norm()).collect();

        buffer.clear();

        Ok(magnitude_buffer)
    }

    pub fn update(&mut self, new_heights: &[f32]) {
        // let now = std::time::SystemTime::now();
        // let since_the_epoch = now.duration_since(self.time).unwrap();
        // let t = since_the_epoch.as_secs_f32();
        // let new_heights: Vec<f32> = (0..COUNT)
        // .map(|x| f32::sin((t + x as f32 * 8.0 * 8.0 * 8.0) / 2.0))
        // .collect();

        // Update the bar graph with the new set of heights
        self.update_bargraph(&new_heights);

        // Swap the buffers
        std::mem::swap(&mut self.scratch_cells, &mut self.cells);
    }
    pub fn draw(&self, screen: &mut [u8]) {
        debug_assert_eq!(screen.len(), 4 * self.cells.len());
        for (c, pix) in self.cells.iter().zip(screen.chunks_exact_mut(4)) {
            let color = if c.alive {
                [144u8, 100u8, 223u8, 0xFF]
            } else {
                [10u8, 5u8, 38u8, 0xFF]
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
