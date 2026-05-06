use crate::karplus_strong::KarplusStrong;
use crate::traits::Voice;

impl Voice for KarplusStrong {
    fn note_on(&mut self, freq_hz: f32, velocity: f32) {
        KarplusStrong::note_on(self, freq_hz, velocity);
    }

    fn note_off(&mut self) {
        KarplusStrong::note_off(self);
    }

    fn process_sample(&mut self) -> f32 {
        KarplusStrong::process_sample(self)
    }

    fn is_active(&self) -> bool {
        KarplusStrong::is_active(self)
    }
}
