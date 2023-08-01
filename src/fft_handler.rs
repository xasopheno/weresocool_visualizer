use crossbeam_channel::{unbounded, Sender};
use std::sync::Arc;
use weresocool_fft::WscFFT;

pub struct FFTHandler {
    num_results: usize,
    read_fn: Box<dyn Fn() -> Vec<f32>>,
    sender: Arc<Sender<Vec<f32>>>,
}

impl FFTHandler {
    pub fn new(config: &crate::core::WereSoCoolSpectrumConfig) -> Self {
        let (s_fft, r_fft) = unbounded();
        let (read_fn, _) = WscFFT::spawn(
            config.visual_buffer_size,
            config.ring_buffer_size,
            config.sample_rate,
            r_fft,
        );
        FFTHandler {
            num_results: config.visual_buffer_size / config.fft_div,
            read_fn: Box::new(read_fn),
            sender: Arc::new(s_fft),
        }
    }

    pub fn read_results(&self) -> Vec<f32> {
        let results = (self.read_fn)();
        results[2..self.num_results].to_vec()
    }

    pub fn get_sender(&self) -> Arc<Sender<Vec<f32>>> {
        Arc::clone(&self.sender)
    }
}
