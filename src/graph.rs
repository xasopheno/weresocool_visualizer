use std::convert::TryInto;
use weresocool_fft::*;
#[derive(Clone, Copy, Debug, Default)]
pub struct Cell {
    alive: bool,
    heat: f32,
    activated_this_turn: bool,
}

impl Cell {
    fn new(alive: bool, heat: f32) -> Self {
        Self {
            alive,
            heat,
            activated_this_turn: false,
        }
    }

    fn cool_off(&mut self, decay: f32) {
        self.heat *= decay;
    }

    fn reset_heat(&mut self) {
        self.heat = 1.0;
    }
}
pub struct Grid {
    cells: Vec<Cell>,
    width: usize,
    height: usize,
    decay: f32,
}

impl Grid {
    fn new_empty(width: usize, height: usize) -> Self {
        assert!(width != 0 && height != 0);
        let size = width.checked_mul(height).expect("too big");

        Self {
            cells: vec![Cell::new(false, 0.0); size],
            width,
            height,
            decay: 0.90,
        }
    }

    pub fn new_bargraph(width: usize, height: usize) -> Self {
        let mut result = Self::new_empty(width, height);
        result.fill_bargraph(
            (0..1024)
                .map(|x| f32::sin(x as f32))
                .collect::<Vec<f32>>()
                .as_slice(),
        );
        result
    }

    fn fill_bargraph(&mut self, heights: &[f32]) {
        let bar_width = self.width / heights.len();

        for (bar_idx, &bar_height) in heights.iter().enumerate() {
            let grid_height = (bar_height * self.height as f32).round() as usize;
            let grid_height = std::cmp::min(grid_height, self.height);

            for bar_x in bar_idx * bar_width..(bar_idx + 1) * bar_width {
                for y in 0..self.height {
                    let idx = bar_x + y * self.width;
                    let alive = y > (self.height - grid_height);

                    // Update only the cells that are alive
                    if alive {
                        self.cells[idx].alive = true;
                        self.cells[idx].reset_heat();
                        self.cells[idx].activated_this_turn = true;
                    }
                }
            }
        }

        // Now cool off all cells
        for cell in &mut self.cells {
            cell.cool_off(self.decay);
        }
    }

    pub fn update_bargraph(&mut self, new_heights: &[f32]) {
        // First, reset all cells
        for cell in &mut self.cells {
            cell.alive = false;
            cell.activated_this_turn = false;
        }

        // Then fill the bargraph and cool off cells
        self.fill_bargraph(new_heights);
    }

    pub fn draw(&mut self, screen: &mut [u8]) {
        debug_assert_eq!(screen.len(), 4 * self.cells.len());
        for (c, pix) in self.cells.iter().zip(screen.chunks_exact_mut(4)) {
            let color = if c.alive {
                [144u8, 100u8, 223u8, (c.heat * 255.0) as u8]
            } else {
                [193u8, 140u8, 183u8, (c.heat * 255.0) as u8]
            };
            pix.copy_from_slice(&color);
        }
    }

    fn grid_idx<I: TryInto<usize>>(&self, x: I, y: I) -> Option<usize> {
        match (x.try_into(), y.try_into()) {
            (Ok(x), Ok(y)) if x < self.width && y < self.height => Some(x + y * self.width),
            _ => None,
        }
    }
}
