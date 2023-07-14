use crate::fft_handler::FFTHandler;
use crate::grid::Grid;
use crossbeam_channel as channel;
use pixels::{Error, Pixels, SurfaceTexture};
use std::sync::Arc;
pub struct GraphHandler {
    width: usize,
    height: usize,
    grid: Grid,
    pixels: Pixels,
    fft_handler_l: FFTHandler,
    fft_handler_r: FFTHandler,
}

impl GraphHandler {
    pub fn new(
        width: usize,
        height: usize,
        buffer_size: usize,
        num_results: usize,
        window: &winit::window::Window,
    ) -> Result<Self, Error> {
        let surface_texture = SurfaceTexture::new(width as u32, height as u32, window);
        let pixels = Pixels::new(width as u32, height as u32, surface_texture)?;
        let grid = Grid::new_bargraph(width, height);
        let fft_handler_l = FFTHandler::new(buffer_size, num_results);
        let fft_handler_r = FFTHandler::new(buffer_size, num_results);
        Ok(GraphHandler {
            width,
            height,
            grid,
            pixels,
            fft_handler_l,
            fft_handler_r,
        })
    }

    pub fn update_and_draw(&mut self) -> Result<(), Error> {
        let mut fft_results_l = self.fft_handler_l.read_results();
        let fft_results_r = self.fft_handler_r.read_results();
        fft_results_l.reverse();
        self.grid.update_bargraph(&fft_results_l, &fft_results_r);
        self.grid.draw(self.pixels.frame_mut());
        _ = self.pixels.render();
        Ok(())
    }

    pub fn get_fft_senders(
        &self,
    ) -> (
        Arc<channel::Sender<Vec<f32>>>,
        Arc<channel::Sender<Vec<f32>>>,
    ) {
        (
            self.fft_handler_l.get_sender(),
            self.fft_handler_r.get_sender(),
        )
    }

    pub fn resize_surface(&mut self, width: u32, height: u32) -> Result<(), Error> {
        self.pixels.resize_surface(width, height).unwrap();
        Ok(())
    }
}
