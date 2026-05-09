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
        if let Some(i) = self.find_voice_index(midi_note) {
            self.voices[i].note_on_with_id(midi_note, freq_hz, velocity);
            return i;
        }
        if let Some(i) = self.voices.iter().position(|v| !v.is_active()) {
            self.voices[i].note_on_with_id(midi_note, freq_hz, velocity);
            return i;
        }
        let i = select_voice_for_steal(&self.voices);
        debug_assert!(i < N);
        self.voices[i].note_on_with_id(midi_note, freq_hz, velocity);
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
