use crate::params::{ParamId, OUTPUT_GAIN_DEFAULT, PARAM_DESCRIPTORS};
use crate::smoothing::SmoothedValue;
use crate::traits::AudioProcessor;
use crate::voice_pool::{VoicePool, POLYPHONY};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynthMode {
    Poly,
    Mono,
}

pub struct Engine {
    sample_rate: f32,
    pool: VoicePool<POLYPHONY>,
    output_gain: SmoothedValue,
    mode: SynthMode,
    /// ユーザー設定の damping。note_off は damping target を 0.95 に上書きするので、
    /// 新規 / 再励振したボイスのみ復元するために保持する (set_damping_voice 用)。
    current_damping: f32,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            sample_rate: 44100.0,
            pool: VoicePool::new(),
            output_gain: SmoothedValue::new(OUTPUT_GAIN_DEFAULT),
            mode: SynthMode::Poly,
            current_damping: PARAM_DESCRIPTORS[ParamId::Damping as usize].default,
        }
    }

    /// note_on を VoicePool に委譲し、割当先ボイスのみ damping を復元する。
    /// hold stack の連携は Step 13 で完成、現状は poly / mono どちらも同じ経路。
    pub fn note_on(&mut self, midi_note: u8, velocity: f32) {
        let freq = midi_to_freq(midi_note);
        let assigned = self.pool.note_on(midi_note, freq, velocity);
        self.pool.set_damping_voice(assigned, self.current_damping);
    }

    /// note_off を VoicePool に委譲する。Step 13 で mono モードでの hold stack 復帰を追加。
    pub fn note_off(&mut self, midi_note: u8) {
        self.pool.note_off(midi_note);
    }

    pub fn set_param(&mut self, id: u32, value: f32) {
        match ParamId::from_u32(id) {
            Some(ParamId::Damping) => {
                let v = PARAM_DESCRIPTORS[ParamId::Damping as usize].clamp(value);
                self.current_damping = v;
                self.pool.set_damping(v);
            }
            Some(ParamId::Brightness) => {
                let v = PARAM_DESCRIPTORS[ParamId::Brightness as usize].clamp(value);
                self.pool.set_brightness(v);
            }
            Some(ParamId::OutputGain) => {
                let v = PARAM_DESCRIPTORS[ParamId::OutputGain as usize].clamp(value);
                self.output_gain.set_target(v);
            }
            None => {}
        }
    }

    pub fn set_mode(&mut self, mode: SynthMode) {
        self.mode = mode;
    }

    pub fn mode(&self) -> SynthMode {
        self.mode
    }

    pub fn current_damping(&self) -> f32 {
        self.current_damping
    }

    /// テスト・診断用: アクティブなボイス数 (D22 で C ABI 非公開)。
    #[doc(hidden)]
    pub fn active_voice_count(&self) -> usize {
        self.pool.active_count()
    }

    /// テスト用: 該当 midi_note のボイス index を取得。
    #[doc(hidden)]
    pub fn voice_index_for_note(&self, midi_note: u8) -> Option<usize> {
        self.pool.voice_index_for_note(midi_note)
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
        self.pool.prepare(sample_rate, max_block_size);
        self.output_gain.set_time_constant(
            sample_rate,
            PARAM_DESCRIPTORS[ParamId::OutputGain as usize].smoothing_tau,
        );
        self.pool.set_damping(self.current_damping);
    }

    fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32]) {
        debug_assert_eq!(output_l.len(), output_r.len());
        for i in 0..output_l.len() {
            let raw = self.pool.process_sample();
            let g = self.output_gain.next_sample();
            let s = raw * g;
            output_l[i] = s;
            output_r[i] = s;
        }
    }

    fn reset(&mut self) {
        self.pool.reset();
        self.pool.set_damping(self.current_damping);
        self.output_gain.set_immediate(OUTPUT_GAIN_DEFAULT);
    }
}

#[inline]
pub fn midi_to_freq(midi_note: u8) -> f32 {
    440.0 * 2f32.powf((midi_note as f32 - 69.0) / 12.0)
}
