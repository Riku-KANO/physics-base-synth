pub struct SmoothedValue {
    current: f32,
    target: f32,
    coeff: f32,
}

impl SmoothedValue {
    pub fn new(initial: f32) -> Self {
        Self {
            current: initial,
            target: initial,
            coeff: 0.0,
        }
    }

    pub fn set_time_constant(&mut self, sample_rate: f32, tau_seconds: f32) {
        self.coeff = 1.0 - (-1.0 / (sample_rate * tau_seconds.max(1e-6))).exp();
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    pub fn set_immediate(&mut self, value: f32) {
        self.current = value;
        self.target = value;
    }

    #[inline]
    pub fn next_sample(&mut self) -> f32 {
        self.current += self.coeff * (self.target - self.current);
        self.current
    }

    pub fn current(&self) -> f32 {
        self.current
    }

    pub fn target(&self) -> f32 {
        self.target
    }
}
