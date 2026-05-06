use crate::karplus_strong::KarplusStrong;
use crate::params::{
    ParamId, BRIGHTNESS_MAX, BRIGHTNESS_MIN, DAMPING_DEFAULT, DAMPING_MAX, DAMPING_MIN,
    OUTPUT_GAIN_DEFAULT, OUTPUT_GAIN_MAX, OUTPUT_GAIN_MIN,
};
use crate::smoothing::SmoothedValue;
use crate::traits::AudioProcessor;

pub struct Engine {
    sample_rate: f32,
    voice: KarplusStrong,
    output_gain: SmoothedValue,
    current_note: Option<u8>,
    current_damping: f32,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            sample_rate: 44100.0,
            voice: KarplusStrong::new(),
            output_gain: SmoothedValue::new(OUTPUT_GAIN_DEFAULT),
            current_note: None,
            current_damping: DAMPING_DEFAULT,
        }
    }

    pub fn note_on(&mut self, midi_note: u8, velocity: f32) {
        let freq = midi_to_freq(midi_note);
        self.voice.note_on(freq, velocity);
        self.voice.set_damping(self.current_damping);
        self.current_note = Some(midi_note);
    }

    pub fn note_off(&mut self, midi_note: u8) {
        if self.current_note == Some(midi_note) {
            self.voice.note_off();
            self.current_note = None;
        }
    }

    pub fn set_param(&mut self, id: u32, value: f32) {
        match ParamId::from_u32(id) {
            Some(ParamId::Damping) => {
                let v = value.clamp(DAMPING_MIN, DAMPING_MAX);
                self.current_damping = v;
                self.voice.set_damping(v);
            }
            Some(ParamId::Brightness) => {
                self.voice
                    .set_brightness(value.clamp(BRIGHTNESS_MIN, BRIGHTNESS_MAX));
            }
            Some(ParamId::OutputGain) => {
                self.output_gain
                    .set_target(value.clamp(OUTPUT_GAIN_MIN, OUTPUT_GAIN_MAX));
            }
            None => {}
        }
    }

    pub fn current_note(&self) -> Option<u8> {
        self.current_note
    }

    pub fn current_damping(&self) -> f32 {
        self.current_damping
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioProcessor for Engine {
    fn prepare(&mut self, sample_rate: f32, max_block_size: usize) {
        self.sample_rate = sample_rate;
        self.voice.prepare(sample_rate, max_block_size);
        self.output_gain.set_time_constant(sample_rate, 0.01);
        self.voice.set_damping(self.current_damping);
    }

    fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32]) {
        debug_assert_eq!(output_l.len(), output_r.len());
        for i in 0..output_l.len() {
            let raw = self.voice.process_sample();
            let g = self.output_gain.next_sample();
            let s = raw * g;
            output_l[i] = s;
            output_r[i] = s;
        }
    }

    fn reset(&mut self) {
        self.voice.reset();
        self.voice.set_damping(self.current_damping);
        self.output_gain.set_immediate(OUTPUT_GAIN_DEFAULT);
        self.current_note = None;
    }
}

#[inline]
pub fn midi_to_freq(midi_note: u8) -> f32 {
    440.0 * 2f32.powf((midi_note as f32 - 69.0) / 12.0)
}
