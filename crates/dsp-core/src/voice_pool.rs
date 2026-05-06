use crate::karplus_strong::KarplusStrong;
use crate::note_allocator::{select_voice_for_steal, StealResult};
use crate::params::PARAM_DESCRIPTORS;

/// D12: N=8 固定。const generic で API は将来 N=4 / N=16 への切替を許容する形を維持
pub const POLYPHONY: usize = 8;

/// 1/sqrt(N) スケール (D20)。POLYPHONY=8 用にコンパイル時定数で計算しておく。
const POLY_SCALE: f32 = 0.353_553_4; // 1.0 / 8.0_f32.sqrt() ≈ 1/2.8284

pub struct VoicePool<const N: usize> {
    voices: [KarplusStrong; N],
    sample_rate: f32,
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
        }
    }

    pub fn prepare(&mut self, sample_rate: f32, max_block_size: usize) {
        self.sample_rate = sample_rate;
        for v in self.voices.iter_mut() {
            v.prepare(sample_rate, max_block_size);
        }
    }

    /// note_on を 4 段フォールバックでボイスに割り当てる (D13)。戻り値は割当先 index。
    /// 1. same-note-replace: 同じ midi_note を発音中のボイスがあれば再励振
    /// 2. 空きボイス検索: 非アクティブのうち最若番に割当
    /// 3. voice stealing: select_voice_for_steal で energy 閾値以下のうち最古
    pub fn note_on(&mut self, midi_note: u8, freq_hz: f32, velocity: f32) -> usize {
        for (i, v) in self.voices.iter_mut().enumerate() {
            if v.note_id() == Some(midi_note) {
                v.note_on_with_id(midi_note, freq_hz, velocity);
                return i;
            }
        }
        for (i, v) in self.voices.iter_mut().enumerate() {
            if !v.is_active() {
                v.note_on_with_id(midi_note, freq_hz, velocity);
                return i;
            }
        }
        let StealResult::Index(i) = select_voice_for_steal(&self.voices);
        debug_assert!(i < N);
        self.voices[i].note_on_with_id(midi_note, freq_hz, velocity);
        i
    }

    /// 指定 index のボイスのみに damping を設定 (Engine::note_on から呼ぶ)。
    /// fan-out 版 set_damping は release 中ボイスを 0.95 → current_damping に巻き戻して
    /// 再生を「復活」させてしまうため、新規 / 再励振したボイスにのみ適用する用途に分離。
    pub fn set_damping_voice(&mut self, index: usize, value: f32) {
        if let Some(v) = self.voices.get_mut(index) {
            let clamped = PARAM_DESCRIPTORS[0].clamp(value);
            v.set_damping(clamped);
        }
    }

    /// 該当 midi_note を発音中のボイスに note_off を発火 (poly モード用)。
    pub fn note_off(&mut self, midi_note: u8) {
        for v in self.voices.iter_mut() {
            if v.note_id() == Some(midi_note) {
                v.note_off();
            }
        }
    }

    /// 全ボイスへ damping を fan-out (Engine::set_param から呼ぶ)。
    pub fn set_damping(&mut self, value: f32) {
        let clamped = PARAM_DESCRIPTORS[0].clamp(value);
        for v in self.voices.iter_mut() {
            v.set_damping(clamped);
        }
    }

    pub fn set_brightness(&mut self, value: f32) {
        let clamped = PARAM_DESCRIPTORS[1].clamp(value);
        for v in self.voices.iter_mut() {
            v.set_brightness(clamped);
        }
    }

    pub fn reset(&mut self) {
        for v in self.voices.iter_mut() {
            v.reset();
        }
    }

    /// 全ボイスを process_sample して累積し、1/sqrt(N) スケールで返す (D20)。
    #[inline]
    pub fn process_sample(&mut self) -> f32 {
        let mut sum = 0.0_f32;
        for v in self.voices.iter_mut() {
            sum += v.process_sample();
        }
        sum * POLY_SCALE
    }

    /// アクティブなボイス数 (テスト・診断用、C ABI 非公開、D22)。
    pub fn active_count(&self) -> usize {
        self.voices.iter().filter(|v| v.is_active()).count()
    }

    /// テスト用: 該当 midi_note を発音中のボイスを探す。
    #[doc(hidden)]
    pub fn voice_index_for_note(&self, midi_note: u8) -> Option<usize> {
        self.voices.iter().position(|v| v.note_id() == Some(midi_note))
    }

    /// テスト用: 指定 index のボイスを参照する。
    #[doc(hidden)]
    pub fn voice(&self, index: usize) -> Option<&KarplusStrong> {
        self.voices.get(index)
    }

    /// テスト用: voice の length_int (alloc 不変検証用)。
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
