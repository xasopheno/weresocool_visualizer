use crossbeam_channel::{unbounded, Receiver, Sender};
use std::sync::Arc;
use weresocool_fft::WscFFT;

pub struct FFTHandler {
    buffer_size: usize,
    num_results: usize,
    read_fn: Box<dyn Fn() -> Vec<f32>>,
    sender: Arc<Sender<Vec<f32>>>,
}

impl FFTHandler {
    pub fn new(buffer_size: usize, num_results: usize) -> Self {
        let (s_fft, r_fft) = unbounded();
        let (read_fn, _) = WscFFT::spawn(buffer_size, r_fft);
        FFTHandler {
            buffer_size,
            num_results,
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
