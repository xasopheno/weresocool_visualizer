use crate::grid::Grid;
pub struct GraphHandler {
    width: usize,
    height: usize,
    grid: Grid,
}

impl GraphHandler {
    pub fn new(width: usize, height: usize) -> Self {
        let grid = Grid::new_bargraph(width, height);
        GraphHandler {
            width,
            height,
            grid,
        }
    }

    pub fn update_and_draw(&mut self, pixels: &mut [u8], l: &[f32], r: &[f32]) {
        self.grid.update_bargraph(l, r);
        self.grid.draw(pixels);
    }
}
