pub struct XorShift32 {
    state: u32,
}

impl XorShift32 {
    pub fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0xDEAD_BEEF } else { seed },
        }
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    #[inline]
    pub fn next_unit_bipolar(&mut self) -> f32 {
        let r = self.next_u32() as f32 / u32::MAX as f32;
        r * 2.0 - 1.0
    }
}
