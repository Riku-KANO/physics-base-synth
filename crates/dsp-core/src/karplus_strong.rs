use crate::params::{BRIGHTNESS_DEFAULT, DAMPING_DEFAULT};
use crate::rng::XorShift32;
use crate::smoothing::SmoothedValue;

const NOTE_OFF_DAMPING: f32 = 0.95;
const ENERGY_RISE: f32 = 0.001;
const ENERGY_DECAY: f32 = 0.999;
const ENERGY_THRESHOLD: f32 = 1.0e-9;
const MIN_FREQ_HZ: f32 = 27.5;

pub struct KarplusStrong {
    buffer: Vec<f32>,
    write_index: usize,
    length: usize,
    damping: SmoothedValue,
    brightness: SmoothedValue,
    last_filter_out: f32,
    energy: f32,
    active: bool,
    rng: XorShift32,
    sample_rate: f32,
    note_off_target_damping: f32,
}

impl KarplusStrong {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            write_index: 0,
            length: 0,
            damping: SmoothedValue::new(DAMPING_DEFAULT),
            brightness: SmoothedValue::new(BRIGHTNESS_DEFAULT),
            last_filter_out: 0.0,
            energy: 0.0,
            active: false,
            rng: XorShift32::new(0x1234_5678),
            sample_rate: 44100.0,
            note_off_target_damping: NOTE_OFF_DAMPING,
        }
    }

    pub fn prepare(&mut self, sample_rate: f32, _max_block_size: usize) {
        self.sample_rate = sample_rate;
        let max_buffer_len = (sample_rate / MIN_FREQ_HZ).ceil() as usize;
        self.buffer = vec![0.0; max_buffer_len];

        self.damping.set_time_constant(sample_rate, 0.02);
        self.brightness.set_time_constant(sample_rate, 0.02);

        self.write_index = 0;
        self.length = 0;
        self.last_filter_out = 0.0;
        self.energy = 0.0;
        self.active = false;
    }

    pub fn note_on(&mut self, freq_hz: f32, velocity: f32) {
        let raw_len = (self.sample_rate / freq_hz.max(1.0)).round() as usize;
        let len = raw_len.clamp(2, self.buffer.len());
        self.length = len;

        for i in 0..len {
            self.buffer[i] = self.rng.next_unit_bipolar() * velocity;
        }

        self.write_index = 0;
        self.last_filter_out = 0.0;
        self.energy = velocity * velocity;
        self.active = true;
    }

    pub fn note_off(&mut self) {
        self.damping.set_target(self.note_off_target_damping);
    }

    pub fn set_damping(&mut self, value: f32) {
        self.damping.set_target(value);
    }

    pub fn set_brightness(&mut self, value: f32) {
        self.brightness.set_target(value);
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn reset(&mut self) {
        for v in self.buffer.iter_mut() {
            *v = 0.0;
        }
        self.write_index = 0;
        self.length = 0;
        self.last_filter_out = 0.0;
        self.energy = 0.0;
        self.active = false;
    }

    #[inline]
    pub fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let read_index = (self.write_index + 1) % self.length;
        let current = self.buffer[self.write_index];
        let next = self.buffer[read_index];

        let b = self.brightness.next_sample();
        let avg = 0.5 * (current + next);
        let filtered = b * avg + (1.0 - b) * self.last_filter_out;
        self.last_filter_out = filtered;

        let d = self.damping.next_sample();
        let mut damped = d * filtered;

        // denormal flush: subnormal な値が連続するとIntel系CPUで処理が極端に遅延するため、
        // DC injection で強制的にゼロ近傍へ吸収する（detail: 01-overview.md 設計判断 D6）
        damped += 1.0e-25;
        damped -= 1.0e-25;

        self.buffer[self.write_index] = damped;
        self.write_index = read_index;

        self.energy = self.energy * ENERGY_DECAY + damped * damped * ENERGY_RISE;
        if self.energy < ENERGY_THRESHOLD {
            self.active = false;
        }

        current
    }
}

impl Default for KarplusStrong {
    fn default() -> Self {
        Self::new()
    }
}
