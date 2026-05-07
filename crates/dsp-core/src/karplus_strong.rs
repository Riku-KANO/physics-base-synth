use crate::fractional_delay::ThiranCoeffs;
use crate::loss_filter::LossFilter;
use crate::params::{BRIGHTNESS_DEFAULT, DAMPING_DEFAULT, PICK_POSITION_DEFAULT};
use crate::rng::XorShift32;
use crate::smoothing::SmoothedValue;

const NOTE_OFF_DAMPING: f32 = 0.95;
const ENERGY_RISE: f32 = 0.001;
const ENERGY_DECAY: f32 = 0.999;
const ENERGY_THRESHOLD: f32 = 1.0e-9;
const MIN_FREQ_HZ: f32 = 27.5;
/// Phase 3 D36 案 D 採用後は Thiran 1 点読みなので margin 0 でも動くが、
/// Pitch Bend での length 過剰時の境界保護として 1 サンプル余裕を残す。
/// 旧 Lagrange 実装のため 3 だった margin は Thiran 切替で 1 に縮小。
pub(crate) const FRACTIONAL_DELAY_BUFFER_MARGIN: usize = 1;

pub struct KarplusStrong {
    buffer: Vec<f32>,
    write_index: usize,
    /// 整数部のディレイ長
    length_int: usize,
    /// note_on 時にキャッシュした分数部の補間係数 (D26)。Phase 3 D36 案 D 採用で
    /// `LagrangeCoeffs` を `ThiranCoeffs` に置換 (A4 0.0002% 精度、|H(ω)|=1 allpass)。
    thiran: ThiranCoeffs,
    /// Phase 3 D33: 弦の周波数依存損失を再現する 1 段 FIR (1+ρ·z⁻¹)/(1+ρ)
    loss_filter: LossFilter,
    /// Phase 3 D34: ピック位置 β ∈ [0.05, 0.5]。SmoothedValue 化せず、
    /// 次回 note_on で励振 shaping に反映（process 中の動的変更は連打で追従）。
    pick_position: f32,
    damping: SmoothedValue,
    brightness: SmoothedValue,
    /// Phase 3 D39: Pitch Bend 適用後の length 目標 (5 ms tau で SmoothedValue)
    length_target: SmoothedValue,
    /// process_sample 内の length 再分解 skip 判定用 (R26 対策)
    cached_length: f32,
    /// Pitch Bend 0 のときの adjusted_length (D37 補正済み)
    base_length: f32,
    /// 現在の Pitch Bend 値 (clamp(-2.0, 2.0))
    pitch_bend_semitones: f32,
    last_filter_out: f32,
    energy: f32,
    active: bool,
    rng: XorShift32,
    sample_rate: f32,
    note_off_target_damping: f32,
    /// 現在発音中の MIDI ノート番号。voice stealing の same-note-replace 判定に使用
    current_note: Option<u8>,
    /// 最後の note_on からの経過サンプル数。voice stealing の oldest 判定に使用
    age_samples: u32,
}

impl KarplusStrong {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            write_index: 0,
            length_int: 0,
            thiran: ThiranCoeffs::new(),
            loss_filter: LossFilter::new(),
            pick_position: PICK_POSITION_DEFAULT,
            damping: SmoothedValue::new(DAMPING_DEFAULT),
            brightness: SmoothedValue::new(BRIGHTNESS_DEFAULT),
            length_target: SmoothedValue::new(0.0),
            cached_length: 0.0,
            base_length: 0.0,
            pitch_bend_semitones: 0.0,
            last_filter_out: 0.0,
            energy: 0.0,
            active: false,
            rng: XorShift32::new(0x1234_5678),
            sample_rate: 44100.0,
            note_off_target_damping: NOTE_OFF_DAMPING,
            current_note: None,
            age_samples: 0,
        }
    }

    pub fn prepare(&mut self, sample_rate: f32, _max_block_size: usize) {
        self.sample_rate = sample_rate;
        let max_buffer_len =
            (sample_rate / MIN_FREQ_HZ).ceil() as usize + FRACTIONAL_DELAY_BUFFER_MARGIN;
        self.buffer = vec![0.0; max_buffer_len];

        self.damping.set_time_constant(sample_rate, 0.02);
        self.brightness.set_time_constant(sample_rate, 0.02);
        self.length_target.set_time_constant(sample_rate, 0.005); // Phase 3 D39: 5ms tau

        self.write_index = 0;
        self.length_int = 0;
        self.thiran.reset();
        self.loss_filter.reset();
        self.cached_length = 0.0;
        self.base_length = 0.0;
        self.pitch_bend_semitones = 0.0;
        self.last_filter_out = 0.0;
        self.energy = 0.0;
        self.active = false;
        self.current_note = None;
        self.age_samples = 0;
    }

    pub fn set_seed(&mut self, seed: u32) {
        self.rng = XorShift32::new(seed);
    }

    /// Phase 3 D34: 次回 note_on で適用される pick position β を更新。
    /// β は [0.05, 0.5] へ clamp。process 中の音色変化は次回 note_on で反映される。
    pub fn set_pick_position(&mut self, beta: f32) {
        self.pick_position = beta.clamp(0.05, 0.5);
    }

    /// trait `Voice` 互換用 (note_id 不明、`current_note = None` で励振)。
    /// 内部実装は `note_on_internal(None, ...)` に委譲。
    pub fn note_on(&mut self, freq_hz: f32, velocity: f32) {
        self.note_on_internal(None, freq_hz, velocity);
    }

    /// VoicePool 経由のメイン経路。`current_note = Some(midi_note)` で励振。
    pub fn note_on_with_id(&mut self, midi_note: u8, freq_hz: f32, velocity: f32) {
        self.note_on_internal(Some(midi_note), freq_hz, velocity);
    }

    /// 共通実装。`note_id` の取り扱いは引数の `Option<u8>` に従う。
    /// `Some(0)` と `None` を取り違えるバグを設計レベルで排除（P1 対策）。
    fn note_on_internal(&mut self, note_id: Option<u8>, freq_hz: f32, velocity: f32) {
        let raw_len = self.sample_rate / freq_hz.max(1.0);
        let max_len_usize = self
            .buffer
            .len()
            .saturating_sub(FRACTIONAL_DELAY_BUFFER_MARGIN);
        // Phase 3 D37: Brightness LPF の 1 段 IIR は τ_g(b) = (1-b)/b の群遅延を持ち、
        // ピッチを下方偏移させる (b=0.5 で 1 sample、b=1.0 で 0)。note_on 時に
        // adjusted_length = raw_length - τ_g で補正する。
        let brightness = self.brightness.target();
        let tau_g = if brightness > 0.001 {
            ((1.0 - brightness) / brightness).clamp(0.0, raw_len - 3.0)
        } else {
            0.0
        };
        let adjusted = (raw_len - tau_g).max(3.0);
        let len_int = (adjusted.floor() as usize).clamp(3, max_len_usize);
        let len_frac = (adjusted - len_int as f32).clamp(0.0, 1.0);

        self.length_int = len_int;
        self.thiran.set_fractional(len_frac);
        // Thiran は IIR で内部状態を保つため、note_on 連打時に前 note の状態が
        // 引き継がれると過渡応答が暴れる。新規励振では state をクリア。
        self.thiran.reset();
        // Phase 3 D33: 周波数依存式で ρ を再算出。state z1 はリセットしない（過渡応答を引き継いでも実害なし）
        self.loss_filter.set_for_frequency(freq_hz);

        // Phase 3 D34: Pick position 励振 shaping。
        // 1. buffer 全体ゼロクリア → 先頭 length_int に noise burst をロード
        // 2. K = round(β · length_int)、length_int-1 へ clamp
        // 3. K > 0 なら降順ループで buffer[i] -= buffer[i - K] を in-place 適用
        for v in self.buffer.iter_mut() {
            *v = 0.0;
        }
        for i in 0..len_int {
            self.buffer[i] = self.rng.next_unit_bipolar() * velocity;
        }
        let k = (self.pick_position * len_int as f32)
            .round()
            .clamp(0.0, len_int.saturating_sub(1) as f32) as usize;
        if k > 0 {
            for i in (k..len_int).rev() {
                self.buffer[i] -= self.buffer[i - k];
            }
        }

        self.write_index = len_int;
        self.last_filter_out = 0.0;
        self.energy = velocity * velocity;
        self.active = true;
        self.age_samples = 0;
        self.current_note = note_id;

        // Phase 3 D39: Pitch Bend baseline。base_length は補正済 adjusted、
        // length_target を即座に同値で set_immediate（既存 SmoothedValue API、P3 対策）
        self.base_length = adjusted;
        self.pitch_bend_semitones = 0.0;
        self.length_target.set_immediate(adjusted);
        self.cached_length = adjusted;
    }

    /// Phase 3 D39: Pitch Bend を半音単位でセット (±2 にクランプ)。
    /// length_target = base_length × 2^(-semitones/12) を SmoothedValue で 5 ms tau で遷移。
    pub fn set_pitch_bend(&mut self, semitones: f32) {
        let clamped = semitones.clamp(-2.0, 2.0);
        self.pitch_bend_semitones = clamped;
        if !self.active || self.base_length < 3.0 {
            return;
        }
        let factor = 2.0_f32.powf(-clamped / 12.0);
        let target = self.base_length * factor;
        let max_len = (self.buffer.len() - FRACTIONAL_DELAY_BUFFER_MARGIN) as f32;
        self.length_target.set_target(target.clamp(3.0, max_len));
    }

    /// テスト専用: 任意の length_int で励振する経路（K=0 分岐の到達確認用）。
    /// 公開 API の β min は 0.05、length_int 最小は 3 のため通常は K ≥ 1 だが、
    /// length_int=9 + β=0.05 で積 = 0.45 → round = 0 を踏める。
    /// integration test (tests/) からも呼べるように `#[cfg(test)]` ではなく
    /// `#[doc(hidden)]` のみで露出する。
    #[doc(hidden)]
    pub fn note_on_with_length_for_test(&mut self, length_int: usize, beta: f32, velocity: f32) {
        debug_assert!(length_int >= 3);
        debug_assert!(self.buffer.len() > length_int);
        let prev_pick = self.pick_position;
        self.pick_position = beta.clamp(0.0, 1.0); // テスト用に下限緩和
        let freq = self.sample_rate / length_int as f32;
        self.note_on_internal(None, freq, velocity);
        self.pick_position = prev_pick;
    }

    pub fn note_off(&mut self) {
        self.damping.set_target(self.note_off_target_damping);
        self.current_note = None;
    }

    pub fn set_damping(&mut self, value: f32) {
        self.damping.set_target(value);
    }

    pub fn set_brightness(&mut self, value: f32) {
        self.brightness.set_target(value);
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn length_int(&self) -> usize {
        self.length_int
    }

    pub fn note_id(&self) -> Option<u8> {
        self.current_note
    }

    pub fn age_samples(&self) -> u32 {
        self.age_samples
    }

    pub fn energy(&self) -> f32 {
        self.energy
    }

    /// テスト用: damping target を直接読む (release 中ボイスが誤復活していないかの検証)
    #[doc(hidden)]
    pub fn damping_target(&self) -> f32 {
        self.damping.target()
    }

    /// テスト用: buffer 容量を直接読む（pick shaping の no-alloc 検証）
    #[doc(hidden)]
    pub fn buffer_capacity(&self) -> usize {
        self.buffer.len()
    }

    /// テスト用: 励振直後の buffer 内容（length_int 分）を読む（pick shaping 効果の検証）
    #[doc(hidden)]
    pub fn excitation_snapshot(&self) -> Vec<f32> {
        self.buffer[..self.length_int].to_vec()
    }

    pub fn reset(&mut self) {
        for v in self.buffer.iter_mut() {
            *v = 0.0;
        }
        self.write_index = 0;
        self.length_int = 0;
        self.thiran.reset();
        self.loss_filter.reset();
        self.cached_length = 0.0;
        self.base_length = 0.0;
        self.pitch_bend_semitones = 0.0;
        self.length_target.set_immediate(0.0);
        self.last_filter_out = 0.0;
        self.energy = 0.0;
        self.active = false;
        self.current_note = None;
        self.age_samples = 0;
    }

    #[inline(always)]
    pub fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let buf_len = self.buffer.len();

        // Phase 3 D39: Pitch Bend で length_target が変動する場合のみ length 再分解。
        // R26 対策: 定常時 (差分 < 1e-5) は再計算 skip。
        let new_len = self.length_target.next_sample();
        if (new_len - self.cached_length).abs() > 1e-5 {
            let max_len = (buf_len - FRACTIONAL_DELAY_BUFFER_MARGIN) as f32;
            let clamped = new_len.clamp(3.0, max_len);
            self.length_int = clamped as usize;
            let frac = clamped - self.length_int as f32;
            self.thiran.set_fractional(frac);
            self.cached_length = new_len;
        }

        // Thiran は 1 点読み取り。
        // ring buffer 不変条件: write_index / read 位置とも `% buf_len`、`% length_int` 不可。
        let read_z = (self.write_index + buf_len - self.length_int) % buf_len;

        let read_value = self.thiran.process(self.buffer[read_z]);

        let b = self.brightness.next_sample();
        let filtered = b * read_value + (1.0 - b) * self.last_filter_out;
        self.last_filter_out = filtered;

        // Phase 3 D33: brightness LPF 直後・damping 前に loss filter
        let loss_out = self.loss_filter.process_sample(filtered);

        let d = self.damping.next_sample();
        let mut damped = d * loss_out;

        // denormal flush (D6)
        damped += 1.0e-25;
        damped -= 1.0e-25;

        self.buffer[self.write_index] = damped;
        let next_write = self.write_index + 1;
        self.write_index = if next_write == buf_len { 0 } else { next_write };

        self.energy = self.energy * ENERGY_DECAY + damped * damped * ENERGY_RISE;
        if self.energy < ENERGY_THRESHOLD {
            self.active = false;
        }

        self.age_samples = self.age_samples.saturating_add(1);

        read_value
    }
}

impl Default for KarplusStrong {
    fn default() -> Self {
        Self::new()
    }
}
