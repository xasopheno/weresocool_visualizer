use std::convert::TryInto;
#[derive(Clone, Copy, Debug, Default)]
pub struct Cell {
    alive: bool,
    heat: f32,
    activated_this_turn: bool,
    decay: f32,
}

impl Cell {
    fn new(alive: bool, heat: f32) -> Self {
        Self {
            alive,
            heat,
            activated_this_turn: false,
            decay: 0.98,
        }
    }

    fn cool_off(&mut self) {
        self.heat *= self.decay;
    }

    fn reset_heat(&mut self) {
        self.heat = 1.0;
    }

    fn update_state(&mut self, alive: bool) {
        if alive {
            self.alive = true;
            self.reset_heat();
            self.activated_this_turn = true;
        }
    }
}
pub struct Grid {
    cells: Vec<Cell>,
    width: usize,
    height: usize,
}

impl Grid {
    #[allow(dead_code)]
    fn new_empty(width: usize, height: usize) -> Self {
        assert!(width != 0 && height != 0);
        let size = width.checked_mul(height).expect("too big");

        Self {
            cells: vec![Cell::new(false, 0.0); size],
            width,
            height,
        }
    }

    #[allow(dead_code)]
    pub fn new_bargraph(width: usize, height: usize) -> Self {
        let mut result = Self::new_empty(width, height);
        result.fill_bargraph((0..width).map(|_x| 0.0).collect::<Vec<f32>>().as_slice());
        result
    }

    fn fill_bargraph(&mut self, heights: &[f32]) {
        let bar_width = self.width / heights.len();

        let fade_distance: usize = 50;

        for (bar_idx, &bar_height) in heights.iter().enumerate() {
            let grid_height = (bar_height * self.height as f32).round() as usize;
            let grid_height = std::cmp::min(grid_height, self.height);

            let fade_factor = if bar_idx < fade_distance {
                (bar_idx as f32 / fade_distance as f32).sqrt()
            } else if bar_idx >= heights.len() - fade_distance {
                ((heights.len() - bar_idx) as f32 / fade_distance as f32).sqrt()
            } else {
                1.0
            };

            let faded_grid_height = (grid_height as f32 * fade_factor).round() as usize;

            for bar_x in bar_idx * bar_width..(bar_idx + 1) * bar_width {
                for y in 0..self.height {
                    let idx = bar_x + y * self.width;
                    let alive = y > (self.height - faded_grid_height);
                    self.cells[idx].update_state(alive);
                }
            }
        }

        for cell in &mut self.cells {
            cell.cool_off();
        }
    }

    #[allow(dead_code)]
    pub fn update_bargraph(&mut self, new_heights_l: &[f32], new_heights_r: &[f32]) {
        for cell in &mut self.cells {
            cell.alive = false;
            cell.activated_this_turn = false;
        }

        self.fill_bargraph(&[new_heights_l, new_heights_r].concat());
    }

    #[allow(dead_code)]
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

    fn _grid_idx<I: TryInto<usize>>(&self, x: I, y: I) -> Option<usize> {
        match (x.try_into(), y.try_into()) {
            (Ok(x), Ok(y)) if x < self.width && y < self.height => Some(x + y * self.width),
            _ => None,
        }
    }
}
