use crossbeam_channel as channel;
use weresocool_fft::WscFFT;

pub struct FFTHandler {
    buffer_size: usize,
    num_results: usize,
    read_fn: Box<dyn Fn() -> Vec<f32>>,
}

impl FFTHandler {
    pub fn new(buffer_size: usize, num_results: usize, r_fft: channel::Receiver<Vec<f32>>) -> Self {
        let (read_fn, _) = WscFFT::spawn(buffer_size, r_fft);
        FFTHandler {
            buffer_size,
            num_results,
            read_fn: Box::new(read_fn),
        }
    }

    pub fn read_results(&self) -> Vec<f32> {
        let results = (self.read_fn)();
        results[2..self.num_results].to_vec()
    }
}
