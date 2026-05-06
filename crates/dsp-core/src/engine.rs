use crate::hold_stack::HoldStack;
use crate::params::{ParamId, OUTPUT_GAIN_DEFAULT};
use crate::smoothing::SmoothedValue;
use crate::traits::AudioProcessor;
use crate::voice_pool::{VoicePool, POLYPHONY};

/// mono モード復帰時のデフォルト velocity。Phase 3 で「note_off されたキーの velocity を保持」へ拡張候補。
const MONO_REVIVE_VELOCITY: f32 = 0.8;

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
    /// mono モード時の押下中ノート履歴 (D29)。poly モードでは参照しない。
    hold_stack: HoldStack,
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
            hold_stack: HoldStack::new(),
            current_damping: ParamId::Damping.descriptor().default,
        }
    }

    /// 新規ノートを発音し、割当先ボイスのみ damping をユーザー値に復元する。
    /// `set_damping_voice` を fan-out にすると release 中ボイスを 0.95 → current_damping に
    /// 巻き戻して再生を「復活」させてしまうため、必ず assigned index にだけ適用する。
    fn trigger_voice(&mut self, midi_note: u8, velocity: f32) {
        let freq = midi_to_freq(midi_note);
        let assigned = self.pool.note_on(midi_note, freq, velocity);
        self.pool.set_damping_voice(assigned, self.current_damping);
    }

    /// poly: VoicePool に直接発火。mono: 直前 top をリリースしてから push + 新規発音。
    pub fn note_on(&mut self, midi_note: u8, velocity: f32) {
        if matches!(self.mode, SynthMode::Mono) {
            // 直前 top のボイスをリリース (mono は 1 音のみ鳴らす建前だが、
            // 短い release tail はクリック対策で残す)
            if let Some(prev) = self.hold_stack.top() {
                if prev != midi_note {
                    self.pool.note_off(prev);
                }
            }
            self.hold_stack.push(midi_note);
        }
        self.trigger_voice(midi_note, velocity);
    }

    /// poly: 該当ボイスを release (damping を 0.95 に加速)。
    /// mono: hold_stack から削除し、現ボイスを release。新しい top があれば top に発音復帰。
    pub fn note_off(&mut self, midi_note: u8) {
        match self.mode {
            SynthMode::Poly => {
                self.pool.note_off(midi_note);
            }
            SynthMode::Mono => {
                let prev_top = self.hold_stack.top();
                self.hold_stack.remove(midi_note);
                self.pool.note_off(midi_note);
                let new_top = self.hold_stack.top();
                // top が変わった場合のみ復帰発音 (中間キー解放では再 trigger しない、クリック対策)
                if new_top != prev_top {
                    if let Some(top) = new_top {
                        self.trigger_voice(top, MONO_REVIVE_VELOCITY);
                    }
                }
            }
        }
    }

    pub fn set_param(&mut self, id: u32, value: f32) {
        match ParamId::from_u32(id) {
            Some(ParamId::Damping) => {
                let v = ParamId::Damping.descriptor().clamp(value);
                self.current_damping = v;
                self.pool.set_damping(v);
            }
            Some(ParamId::Brightness) => {
                self.pool
                    .set_brightness(ParamId::Brightness.descriptor().clamp(value));
            }
            Some(ParamId::OutputGain) => {
                self.output_gain
                    .set_target(ParamId::OutputGain.descriptor().clamp(value));
            }
            None => {}
        }
    }

    pub fn set_mode(&mut self, mode: SynthMode) {
        self.mode = mode;
        // モード切替時は履歴を破棄する。進行中のボイスは VoicePool 側で自然減衰させる。
        self.hold_stack.clear();
    }

    pub fn mode(&self) -> SynthMode {
        self.mode
    }

    pub fn current_damping(&self) -> f32 {
        self.current_damping
    }

    /// D22: C ABI 非公開。
    #[doc(hidden)]
    pub fn active_voice_count(&self) -> usize {
        self.pool.active_count()
    }

    #[doc(hidden)]
    pub fn voice_index_for_note(&self, midi_note: u8) -> Option<usize> {
        self.pool.voice_index_for_note(midi_note)
    }

    /// release 中ボイスの damping_target 等の検証用。
    #[doc(hidden)]
    pub fn pool(&self) -> &VoicePool<POLYPHONY> {
        &self.pool
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
        self.output_gain
            .set_time_constant(sample_rate, ParamId::OutputGain.descriptor().smoothing_tau);
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
        self.hold_stack.clear();
    }
}

#[inline]
pub fn midi_to_freq(midi_note: u8) -> f32 {
    440.0 * 2f32.powf((midi_note as f32 - 69.0) / 12.0)
}
