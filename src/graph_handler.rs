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
        config: &crate::core::WereSoCoolSpectrumConfig,
        window: &winit::window::Window,
    ) -> Result<Self, Error> {
        let width = window.inner_size().width as u32;
        let height = window.inner_size().height as u32;

        let surface_texture = SurfaceTexture::new(width, height, window);
        let pixels = Pixels::new(width, height as u32, surface_texture)?;
        let grid = Grid::new_bargraph(width as usize, height as usize);
        let fft_handler_l = FFTHandler::new(config);
        let fft_handler_r = FFTHandler::new(config);
        Ok(GraphHandler {
            width: config.width as usize,
            height: config.height as usize,
            grid,
            pixels,
            fft_handler_l,
            fft_handler_r,
        })
    }

    pub fn update_and_draw(&mut self) -> Result<(), Error> {
        let mut fft_results_l = self.fft_handler_l.read_results();
        let fft_results_r = self.fft_handler_r.read_results();

        // Skip updating and drawing if the FFT results are all zero
        if fft_results_l
            .iter()
            .chain(fft_results_r.iter())
            .all(|&val| val == 0.0)
            && fft_results_r.iter().all(|&val| val == 0.0)
        {
            return Ok(());
        }

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
