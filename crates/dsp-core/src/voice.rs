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

    fn note_id(&self) -> Option<u8> {
        KarplusStrong::note_id(self)
    }

    fn age(&self) -> u32 {
        KarplusStrong::age_samples(self)
    }

    fn amplitude(&self) -> f32 {
        // RMS-like 推定: energy は x² の指数移動平均なので sqrt が振幅近似
        KarplusStrong::energy(self).sqrt()
    }

    fn set_pitch_bend(&mut self, semitones: f32) {
        KarplusStrong::set_pitch_bend(self, semitones);
    }

    fn set_lfo_pitch_factor(&mut self, factor: f32) {
        KarplusStrong::set_lfo_pitch_factor(self, factor);
    }

    fn set_lfo_brightness_offset(&mut self, offset: f32) {
        KarplusStrong::set_lfo_brightness_offset(self, offset);
    }

    fn set_dispersion_active(&mut self, active: bool) {
        KarplusStrong::set_dispersion_active(self, active);
    }
}
