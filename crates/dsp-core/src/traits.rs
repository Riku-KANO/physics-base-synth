pub trait AudioProcessor {
    fn prepare(&mut self, sample_rate: f32, max_block_size: usize);
    fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32]);
    fn reset(&mut self);
}

pub trait Voice {
    fn note_on(&mut self, freq_hz: f32, velocity: f32);
    fn note_off(&mut self);
    fn process_sample(&mut self) -> f32;
    fn is_active(&self) -> bool;
}
