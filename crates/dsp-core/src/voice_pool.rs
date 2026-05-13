use crate::karplus_strong::KarplusStrong;
use crate::note_allocator::select_voice_for_steal;
use crate::params::ParamId;

/// D12: N=8 固定。const generic で API は将来 N=4 / N=16 への切替を許容する形を維持
pub const POLYPHONY: usize = 8;

pub struct VoicePool<const N: usize> {
    voices: [KarplusStrong; N],
    sample_rate: f32,
    /// 1/sqrt(N) スケール (D20)。N から `new()` で計算してホットパスでの除算を避ける。
    /// `f32::sqrt` は const 関数ではないためコンパイル時計算は不可。
    poly_scale: f32,
}

impl<const N: usize> VoicePool<N> {
    pub fn new() -> Self {
        Self {
            voices: core::array::from_fn(|i| {
                let mut ks = KarplusStrong::new();
                // 各ボイスに固有のシードを与え、励振ノイズの相関を排除
                ks.set_seed(0x1234_5678 ^ ((i as u32).wrapping_mul(0x9E37_79B9)));
                ks
            }),
            sample_rate: 44100.0,
            poly_scale: 1.0 / (N as f32).sqrt(),
        }
    }

    pub fn prepare(&mut self, sample_rate: f32, max_block_size: usize) {
        self.sample_rate = sample_rate;
        for v in self.voices.iter_mut() {
            v.prepare(sample_rate, max_block_size);
        }
    }

    fn find_voice_index(&self, midi_note: u8) -> Option<usize> {
        self.voices
            .iter()
            .position(|v| v.note_id() == Some(midi_note))
    }

    /// note_on を 4 段フォールバックでボイスに割り当てる (D13)。戻り値は割当先 index。
    /// 1. same-note-replace: 同じ midi_note を発音中のボイスがあれば再励振
    /// 2. 空きボイス検索: 非アクティブのうち最若番に割当
    /// 3. voice stealing: select_voice_for_steal で energy 閾値以下のうち最古
    pub fn note_on(&mut self, midi_note: u8, freq_hz: f32, velocity: f32) -> usize {
        let i = self.allocate_voice(midi_note);
        self.voices[i].note_on_with_id(midi_note, freq_hz, velocity);
        i
    }

    /// Phase 4c D70 / D78: Engine が B(note) LUT 値 + 楽器パラメータを保持して
    /// `set_instrument_params` 直後に `note_on_with_id` を呼ぶ経路。`Engine::trigger_voice`
    /// から呼び出される (D81 で C ABI を増やさないため、VoicePool 内に閉じた拡張)。
    ///
    /// Phase 4c の 4 楽器パラメータをそのまま受け取る関係で引数が 8 個になる。新しい
    /// struct を切り出すと Engine 側のコール経路まで波及するため、ここでは spec 通りの
    /// シグネチャを維持して clippy::too_many_arguments のみローカル抑止する。
    #[allow(clippy::too_many_arguments)]
    pub fn note_on_with_piano_params(
        &mut self,
        midi_note: u8,
        freq_hz: f32,
        velocity: f32,
        unison_detune_cents: f32,
        inharmonicity_b: f32,
        hammer_cutoff_low_hz: f32,
        hammer_cutoff_high_hz: f32,
    ) -> usize {
        let i = self.allocate_voice(midi_note);
        // 割当先 voice にだけ Piano パラメータ + B(note) を渡す (他 voice は
        // `apply_instrument` 時の値が `set_piano_params` で fan-out 済)。
        self.voices[i].set_instrument_params(
            unison_detune_cents,
            inharmonicity_b,
            hammer_cutoff_low_hz,
            hammer_cutoff_high_hz,
        );
        self.voices[i].note_on_with_id(midi_note, freq_hz, velocity);
        i
    }

    /// 既存 `note_on` の 3 段フォールバック (same-note replace / free voice / steal) を
    /// 共通化する private helper。Phase 4c で `note_on_with_piano_params` から再利用する。
    fn allocate_voice(&mut self, midi_note: u8) -> usize {
        if let Some(i) = self.find_voice_index(midi_note) {
            return i;
        }
        if let Some(i) = self.voices.iter().position(|v| !v.is_active()) {
            return i;
        }
        let i = select_voice_for_steal(&self.voices);
        debug_assert!(i < N);
        i
    }

    /// 指定 index のボイスのみに damping を設定 (Engine::note_on から呼ぶ)。
    /// fan-out 版 set_damping は release 中ボイスを 0.95 → current_damping に巻き戻して
    /// 再生を「復活」させてしまうため、新規 / 再励振したボイスにのみ適用する用途に分離。
    pub fn set_damping_voice(&mut self, index: usize, value: f32) {
        if let Some(v) = self.voices.get_mut(index) {
            v.set_damping(ParamId::Damping.descriptor().clamp(value));
        }
    }

    /// 該当 midi_note を発音中のボイスに note_off を発火 (poly モード用)。
    pub fn note_off(&mut self, midi_note: u8) {
        if let Some(i) = self.find_voice_index(midi_note) {
            self.voices[i].note_off();
        }
    }

    /// 全 active voice を一斉 release (CC#123 All Notes Off 用)。
    /// 128 個の MIDI note を線形検索する代わりに 8 voice を直接 release する。
    pub fn all_notes_off(&mut self) {
        for v in self.voices.iter_mut() {
            if v.is_active() {
                v.note_off();
            }
        }
    }

    /// 全ボイスへ damping を fan-out (Engine::set_param から呼ぶ)。
    pub fn set_damping(&mut self, value: f32) {
        let clamped = ParamId::Damping.descriptor().clamp(value);
        for v in self.voices.iter_mut() {
            v.set_damping(clamped);
        }
    }

    pub fn set_brightness(&mut self, value: f32) {
        let clamped = ParamId::Brightness.descriptor().clamp(value);
        for v in self.voices.iter_mut() {
            v.set_brightness(clamped);
        }
    }

    /// 各 voice は内部で `[0.05, 0.5]` に clamp する。次回 note_on で反映 (D34)。
    pub fn set_pick_position(&mut self, value: f32) {
        for v in self.voices.iter_mut() {
            v.set_pick_position(value);
        }
    }

    /// 各 voice は内部で `[-2.0, 2.0]` に clamp する (D39)。
    pub fn set_pitch_bend(&mut self, semitones: f32) {
        for v in self.voices.iter_mut() {
            v.set_pitch_bend(semitones);
        }
    }

    /// Phase 4a D48: LFO Pitch factor (Engine 側で exp2 済) を全 voice に fan-out。
    /// per sample 呼出される。Engine 側で 1 回 exp2 計算した値を全 voice に配るので
    /// per voice exp2 を回避できる。
    #[inline(always)]
    pub fn set_lfo_pitch_factor(&mut self, factor: f32) {
        for v in self.voices.iter_mut() {
            v.set_lfo_pitch_factor(factor);
        }
    }

    /// Phase 4a D48: LFO Brightness offset を全 voice に fan-out。
    #[inline(always)]
    pub fn set_lfo_brightness_offset(&mut self, offset: f32) {
        for v in self.voices.iter_mut() {
            v.set_lfo_brightness_offset(offset);
        }
    }

    /// Phase 4b D67: 楽器切替で全 voice に dispersion_active を fan-out。
    /// `Engine::apply_instrument(Piano)` で true、他 7 楽器で false。
    pub fn set_dispersion_active(&mut self, active: bool) {
        for v in self.voices.iter_mut() {
            v.set_dispersion_active(active);
        }
    }

    /// Phase 4c D72 / D75 / D78: 楽器プリセットの per-voice パラメータを全 voice に fan-out。
    /// `Engine::apply_instrument` から呼ばれ、各 voice の `set_instrument_params` を
    /// `note_on` 前段で更新しておくことで、`note_on_with_piano_params` での 1 voice 上書きと
    /// 整合する。`inharmonicity_b` は note 依存 (B(note) LUT) のため fan-out では 0 を渡し、
    /// `note_on_with_piano_params` 内で割当 voice にだけ正しい LUT 値を再設定する。
    pub fn set_piano_params(
        &mut self,
        unison_detune_cents: f32,
        inharmonicity_b: f32,
        hammer_cutoff_low_hz: f32,
        hammer_cutoff_high_hz: f32,
    ) {
        for v in self.voices.iter_mut() {
            v.set_instrument_params(
                unison_detune_cents,
                inharmonicity_b,
                hammer_cutoff_low_hz,
                hammer_cutoff_high_hz,
            );
        }
    }

    pub fn reset(&mut self) {
        for v in self.voices.iter_mut() {
            v.reset();
        }
    }

    /// 全ボイスを process_sample して累積し、1/sqrt(N) スケールで返す (D20)。
    #[inline(always)]
    pub fn process_sample(&mut self) -> f32 {
        let mut sum = 0.0_f32;
        for v in self.voices.iter_mut() {
            sum += v.process_sample();
        }
        sum * self.poly_scale
    }

    /// Phase 4c D76 / D77: Sympathetic bus からの注入経路。
    /// `inject = bus_out_prev × feedback_gain` を各 voice の `inject_feedback` に渡してから
    /// 通常の `process_sample` を実行、合算して `poly_scale = 1/√N` を掛けて返す
    /// (Phase 2 D20 / `process_sample` と同型のスケール)。
    ///
    /// `feedback_gain = 0` (Default kind / Piano + Sustain OFF) のとき `inject = 0` で
    /// 各 voice 側の `bus_feedback_pending` が 0、`process_sample` 内の damping write-back に
    /// 0 が加算されるため Phase 4a / 4b の voice 出力と byte 一致継承 (F65-a / D83)。
    #[inline(always)]
    pub fn process_sample_with_feedback(&mut self, bus_out_prev: f32, feedback_gain: f32) -> f32 {
        let inject = bus_out_prev * feedback_gain;
        let mut sum = 0.0_f32;
        for v in self.voices.iter_mut() {
            v.inject_feedback(inject);
            sum += v.process_sample();
        }
        sum * self.poly_scale
    }

    /// アクティブなボイス数 (テスト・診断用、C ABI 非公開、D22)。
    pub fn active_count(&self) -> usize {
        self.voices.iter().filter(|v| v.is_active()).count()
    }

    #[doc(hidden)]
    pub fn voice_index_for_note(&self, midi_note: u8) -> Option<usize> {
        self.find_voice_index(midi_note)
    }

    #[doc(hidden)]
    pub fn voice(&self, index: usize) -> Option<&KarplusStrong> {
        self.voices.get(index)
    }

    #[doc(hidden)]
    pub fn voice_length_int(&self, index: usize) -> Option<usize> {
        self.voices.get(index).map(|v| v.length_int())
    }

    /// Phase 4c test-only: 割当 voice の弦数を観測する (F60-a..d / F68-a / F68-b)。
    #[doc(hidden)]
    pub fn voice_n_strings_active_for_test(&self, index: usize) -> Option<usize> {
        self.voices.get(index).map(|v| v.n_strings_active())
    }

    /// Phase 4c test-only: 割当 voice の inharmonicity_b を観測する (F67-g / F68-a / F68-b)。
    #[doc(hidden)]
    pub fn voice_inharmonicity_b_for_test(&self, index: usize) -> Option<f32> {
        self.voices.get(index).map(|v| v.inharmonicity_b())
    }

    /// Phase 4c test-only: 割当 voice の unison_detune_cents を観測する (F68-a / F68-b)。
    #[doc(hidden)]
    pub fn voice_unison_detune_cents_for_test(&self, index: usize) -> Option<f32> {
        self.voices.get(index).map(|v| v.unison_detune_cents())
    }

    /// Phase 4c test-only: 割当 voice の dispersion_active を観測する (F68-a / F68-b)。
    #[doc(hidden)]
    pub fn voice_dispersion_active_for_test(&self, index: usize) -> Option<bool> {
        self.voices.get(index).map(|v| v.dispersion_active())
    }
}

impl<const N: usize> Default for VoicePool<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poly_scale_matches_inverse_sqrt_n() {
        // const generic を変えても 1/sqrt(N) が追従することを確認 (gain ハードコードのバグ防止)。
        let p4: VoicePool<4> = VoicePool::new();
        let p8: VoicePool<POLYPHONY> = VoicePool::new();
        let p16: VoicePool<16> = VoicePool::new();
        assert!((p4.poly_scale - 0.5).abs() < 1.0e-6);
        assert!((p8.poly_scale - 1.0 / 8.0_f32.sqrt()).abs() < 1.0e-6);
        assert!((p16.poly_scale - 0.25).abs() < 1.0e-6);
    }
}
