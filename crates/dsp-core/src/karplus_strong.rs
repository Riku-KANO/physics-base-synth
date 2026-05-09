use crate::dispersion::{compute_dispersion_a1, DispersionStage, DISPERSION_STAGES};
use crate::fractional_delay::ThiranCoeffs;
use crate::loss_filter::LossFilter;
use crate::params::{
    BRIGHTNESS_DEFAULT, DAMPING_DEFAULT, INHARMONICITY_B_PIANO, PICK_POSITION_DEFAULT,
};
use crate::rng::XorShift32;
use crate::smoothing::SmoothedValue;

const NOTE_OFF_DAMPING: f32 = 0.95;
const ENERGY_RISE: f32 = 0.001;
const ENERGY_DECAY: f32 = 0.999;
const ENERGY_THRESHOLD: f32 = 1.0e-9;
const MIN_FREQ_HZ: f32 = 27.5;
/// Pitch Bend で length が過剰になったときの境界保護に 1 sample 余裕を残す。
pub(crate) const FRACTIONAL_DELAY_BUFFER_MARGIN: usize = 1;

pub struct KarplusStrong {
    buffer: Vec<f32>,
    write_index: usize,
    /// 整数部のディレイ長
    length_int: usize,
    /// note_on 時にキャッシュした分数部の補間係数 (D26)
    thiran: ThiranCoeffs,
    /// 弦の周波数依存損失 (1+ρ·z⁻¹)/(1+ρ)
    loss_filter: LossFilter,
    /// ピック位置 β ∈ [0.05, 0.5]。次回 note_on の励振 shaping で反映
    pick_position: f32,
    damping: SmoothedValue,
    brightness: SmoothedValue,
    /// Pitch Bend 適用後の length 目標 (5 ms tau で SmoothedValue)
    length_target: SmoothedValue,
    /// process_sample 内の length 再分解 skip 判定用
    cached_length: f32,
    /// Pitch Bend 0 のときの adjusted_length (brightness 群遅延補正済み)
    base_length: f32,
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
    /// Phase 4a D48: LFO Pitch factor (Engine 側で `exp2(-semitones/12)` 計算済、毎 sample 更新)。
    /// `process_sample` で `length_target.next_sample() * lfo_pitch_factor` で動的 length。
    /// 初期値 1.0 = pitch offset 0 と等価 (Phase 3 互換)。
    lfo_pitch_factor: f32,
    /// Phase 4a D48: LFO Brightness offset (毎 sample 更新)。
    /// `process_sample` で `(brightness + offset).clamp(0, 1)` として適用。
    /// 初期値 0.0 = brightness offset なし (Phase 3 互換)。
    lfo_brightness_offset: f32,
    /// Phase 4b D57: Piano kind での Stretching all-pass cascade (M=8 段、heap 確保ゼロ)。
    /// `dispersion_active = false` の楽器では `process_sample` で skip。
    /// 各段の a1 は `note_on` 時に `compute_dispersion_a1` で算出して全段共通で代入。
    dispersion_stages: [DispersionStage; DISPERSION_STAGES],
    /// Phase 4b D67: `Engine::apply_instrument(Piano)` で true、他 7 楽器 (Default 含む) で false。
    /// `process_sample` ホットパスでは bool 1 つの分岐のみ、Phase 4a 互換性確保。
    dispersion_active: bool,
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
            lfo_pitch_factor: 1.0,
            lfo_brightness_offset: 0.0,
            dispersion_stages: [DispersionStage::new(); DISPERSION_STAGES],
            dispersion_active: false,
        }
    }

    /// Phase 4a D48: LFO Pitch factor を毎 sample 更新 (VoicePool fan-out 経由)。
    /// Engine 側で `exp2(-semitones/12)` 計算済の値を受け取る (per voice exp2 を回避)。
    #[inline(always)]
    pub fn set_lfo_pitch_factor(&mut self, factor: f32) {
        self.lfo_pitch_factor = factor;
    }

    /// Phase 4a D48: LFO Brightness offset を毎 sample 更新 (VoicePool fan-out 経由)。
    #[inline(always)]
    pub fn set_lfo_brightness_offset(&mut self, offset: f32) {
        self.lfo_brightness_offset = offset;
    }

    /// Phase 4b D67: 楽器切替で全 voice に dispersion_active を設定。
    /// `Engine::apply_instrument` から `pool.set_dispersion_active(active)` 経由で呼ばれる。
    /// flag の bool 切替のみで heap 操作なし、`apply_instrument` での alloc 0 保証。
    /// `active = false` のときは念のため状態を reset（次に Piano に切り替えたとき
    /// 古い z1 が残らないよう）。
    #[inline(always)]
    pub fn set_dispersion_active(&mut self, active: bool) {
        self.dispersion_active = active;
        if !active {
            for stage in self.dispersion_stages.iter_mut() {
                stage.reset();
            }
        }
    }

    /// テスト専用: dispersion 状態の検証用 read-only access。
    #[doc(hidden)]
    pub fn dispersion_active(&self) -> bool {
        self.dispersion_active
    }

    #[doc(hidden)]
    pub fn dispersion_stage_a1(&self, idx: usize) -> f32 {
        self.dispersion_stages[idx].a1
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

    /// β は [0.05, 0.5] へ clamp。process 中の変更は次回 note_on で反映 (D34)。
    pub fn set_pick_position(&mut self, beta: f32) {
        self.pick_position = beta.clamp(0.05, 0.5);
    }

    /// trait `Voice` 互換用 (note_id 不明、`current_note = None` で励振)。
    pub fn note_on(&mut self, freq_hz: f32, velocity: f32) {
        self.note_on_internal(None, freq_hz, velocity);
    }

    /// VoicePool 経由のメイン経路。`current_note = Some(midi_note)` で励振。
    pub fn note_on_with_id(&mut self, midi_note: u8, freq_hz: f32, velocity: f32) {
        self.note_on_internal(Some(midi_note), freq_hz, velocity);
    }

    /// `note_id` を `Option<u8>` で受けるのは `Some(0)` と `None` の取り違えを設計レベルで排除するため。
    fn note_on_internal(&mut self, note_id: Option<u8>, freq_hz: f32, velocity: f32) {
        let raw_len = self.sample_rate / freq_hz.max(1.0);
        let max_len_usize = self
            .buffer
            .len()
            .saturating_sub(FRACTIONAL_DELAY_BUFFER_MARGIN);
        // Brightness LPF (1 段 IIR) の τ_g(b) = (1-b)/b 群遅延がピッチを下方偏移させる
        // ため、note_on 時に raw_length から差し引いて補正する (b=0.5 で 1 sample、b=1.0 で 0)。
        let brightness = self.brightness.target();
        let brightness_tau_g = if brightness > 0.001 {
            ((1.0 - brightness) / brightness).clamp(0.0, raw_len - 3.0)
        } else {
            0.0
        };

        // Phase 4b D60: Dispersion cascade の群遅延補正。Piano kind では各段の a1 を
        // 算出 + 状態クリアし、M·polydel(a1) を adjusted_length から差し引く。
        // 非 Piano (`dispersion_active = false`) では 0 加算で Phase 4a と完全互換。
        let dispersion_tau_g = if self.dispersion_active {
            let (a1, gd_per_stage) = compute_dispersion_a1(
                DISPERSION_STAGES as u32,
                INHARMONICITY_B_PIANO,
                freq_hz,
                self.sample_rate,
            );
            for stage in self.dispersion_stages.iter_mut() {
                stage.a1 = a1;
                stage.z1_in = 0.0;
                stage.z1_out = 0.0;
            }
            (DISPERSION_STAGES as f32) * gd_per_stage
        } else {
            0.0
        };

        let total_compensation = brightness_tau_g + dispersion_tau_g;
        let adjusted = (raw_len - total_compensation).max(3.0);
        let len_int = (adjusted.floor() as usize).clamp(3, max_len_usize);
        let len_frac = (adjusted - len_int as f32).clamp(0.0, 1.0);

        self.length_int = len_int;
        self.thiran.set_fractional(len_frac);
        // Thiran は IIR、note_on 連打で前 note の状態を引き継ぐと過渡応答が暴れる。
        self.thiran.reset();
        self.loss_filter.set_for_frequency(freq_hz);

        // Pick position 励振 shaping: noise burst を `buffer[i] -= buffer[i - K]` で
        // in-place comb 整形。K = round(β · length_int)、length_int-1 へ clamp。
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

        self.base_length = adjusted;
        self.pitch_bend_semitones = 0.0;
        self.length_target.set_immediate(adjusted);
        self.cached_length = adjusted;
    }

    /// length_target = base_length × 2^(-semitones/12) を 5 ms tau で滑らかに追従 (D39)。
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

    /// テスト専用: 任意の length_int で励振 (K=0 分岐の到達確認用)。
    /// 公開 β min は 0.05、length_int=9 + β=0.05 で K=round(0.45)=0 を踏める。
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

    #[doc(hidden)]
    pub fn buffer_capacity(&self) -> usize {
        self.buffer.len()
    }

    /// テスト用: 励振直後の buffer の先頭 `length_int` を読む。alloc を含むので production 経路では使わない。
    #[cfg(test)]
    pub(crate) fn excitation_snapshot(&self) -> Vec<f32> {
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
        // Phase 4a: LFO 適用値を初期状態へ (Phase 3 互換)
        self.lfo_pitch_factor = 1.0;
        self.lfo_brightness_offset = 0.0;
        // Phase 4b D67: dispersion を完全初期化 (Default kind に戻る前提)
        self.dispersion_active = false;
        for stage in self.dispersion_stages.iter_mut() {
            *stage = DispersionStage::new();
        }
    }

    #[inline(always)]
    pub fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let buf_len = self.buffer.len();

        // 定常時は length 再分解と Thiran 係数再計算を skip (差分 < 1e-5)。
        // Phase 4a D48: LFO Pitch factor を実効 length に乗算 (factor は Engine 側で exp2 済)。
        let base_target = self.length_target.next_sample();
        let effective_length = base_target * self.lfo_pitch_factor;
        if (effective_length - self.cached_length).abs() > 1e-5 {
            let max_len = (buf_len - FRACTIONAL_DELAY_BUFFER_MARGIN) as f32;
            let clamped = effective_length.clamp(3.0, max_len);
            self.length_int = clamped as usize;
            let frac = clamped - self.length_int as f32;
            self.thiran.set_fractional(frac);
            self.cached_length = effective_length;
        }

        // Pitch Bend で length_int が動的に変わるため、剰余は `% buf_len` のみ。
        // `% length_int` だと write/read で異なる剰余系になり buffer の論理長が破綻する。
        let read_z = (self.write_index + buf_len - self.length_int) % buf_len;

        let read_value = self.thiran.process(self.buffer[read_z]);

        // Phase 4a D48: brightness LPF に LFO offset を加算してから clamp。
        let b = (self.brightness.next_sample() + self.lfo_brightness_offset).clamp(0.0, 1.0);
        let filtered = b * read_value + (1.0 - b) * self.last_filter_out;
        self.last_filter_out = filtered;

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

#[cfg(test)]
mod excitation_tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48_000.0;

    fn fresh(beta: f32) -> KarplusStrong {
        let mut v = KarplusStrong::new();
        v.prepare(SAMPLE_RATE, 128);
        v.set_pick_position(beta);
        v
    }

    fn rms(samples: &[f32]) -> f32 {
        let sq: f64 = samples.iter().map(|x| (*x as f64).powi(2)).sum();
        (sq / samples.len() as f64).sqrt() as f32
    }

    fn autocorr_normalized(samples: &[f32], lag: usize) -> f32 {
        if lag >= samples.len() {
            return 0.0;
        }
        let mut sum_xy = 0.0_f64;
        let mut sum_xx = 0.0_f64;
        for i in 0..samples.len() - lag {
            sum_xy += samples[i] as f64 * samples[i + lag] as f64;
            sum_xx += (samples[i] as f64).powi(2);
        }
        if sum_xx > 0.0 {
            (sum_xy / sum_xx) as f32
        } else {
            0.0
        }
    }

    #[test]
    fn test_pick_min_beta_minimal_shape() {
        let mut v_low = fresh(0.05);
        v_low.note_on(440.0, 0.8);
        let buf_low = v_low.excitation_snapshot();

        let mut v_high = fresh(0.5);
        v_high.note_on(440.0, 0.8);
        let buf_high = v_high.excitation_snapshot();

        let rms_low = rms(&buf_low);
        let rms_high = rms(&buf_high);
        println!(
            "rms_low(β=0.05)={:.4}, rms_high(β=0.5)={:.4}",
            rms_low, rms_high
        );
        assert!(buf_low.len() == buf_high.len());
        assert!(rms_low > 0.0 && rms_high > 0.0);
        let mut differs = false;
        for (a, b) in buf_low.iter().zip(buf_high.iter()) {
            if (a - b).abs() > 1e-6 {
                differs = true;
                break;
            }
        }
        assert!(differs, "β=0.05 vs β=0.5 で励振 buffer が同一");
    }

    #[test]
    fn test_pick_position_node_at_beta_half() {
        let mut v = fresh(0.5);
        v.note_on(440.0, 0.8);
        let buf = v.excitation_snapshot();
        let l = buf.len();

        let mut v_ref = fresh(0.05);
        v_ref.note_on(440.0, 0.8);
        let buf_ref = v_ref.excitation_snapshot();

        let k_high = ((0.5 * l as f32).round()).clamp(0.0, (l - 1) as f32) as usize;
        let ac_at_k = autocorr_normalized(&buf, k_high);
        let ac_at_k_ref = autocorr_normalized(&buf_ref, k_high);
        println!(
            "β=0.5 ac[K={}]={:.4}, β=0.05 ac[K={}]={:.4}",
            k_high, ac_at_k, k_high, ac_at_k_ref
        );
        assert!(
            ac_at_k < -0.3,
            "β=0.5 anti-correlation at K should be strong (< -0.3): got {:.4}",
            ac_at_k
        );
        assert!(
            ac_at_k < ac_at_k_ref,
            "β=0.5 anti-correlation should be more negative than β=0.05"
        );
    }

    #[test]
    fn test_pick_position_attenuates_kth_harmonic() {
        for k in 2..=4 {
            let beta = 1.0 / k as f32;
            let mut v = fresh(beta);
            v.note_on(440.0, 0.8);
            let buf = v.excitation_snapshot();
            let l = buf.len();
            let lag = ((beta * l as f32).round()).clamp(0.0, (l - 1) as f32) as usize;

            let mut v_ref = fresh(0.05);
            v_ref.note_on(440.0, 0.8);
            let buf_ref = v_ref.excitation_snapshot();

            let ac = autocorr_normalized(&buf, lag);
            let ac_ref = autocorr_normalized(&buf_ref, lag);
            println!(
                "k={} β={:.3} ac[K={}]={:.4} ref={:.4}",
                k, beta, lag, ac, ac_ref
            );
            assert!(
                ac < ac_ref,
                "k={}: β=1/k anti-correlation should be more negative than β=0.05: got {:.4} ref={:.4}",
                k,
                ac,
                ac_ref
            );
        }
    }

    #[test]
    fn test_pick_internal_k_zero_branch() {
        let mut v = KarplusStrong::new();
        v.prepare(SAMPLE_RATE, 128);
        v.set_brightness(1.0);
        v.note_on_with_length_for_test(9, 0.05, 0.8);
        assert!(v.is_active());
        let buf = v.excitation_snapshot();
        assert_eq!(buf.len(), 9);
        let max_abs = buf.iter().map(|x| x.abs()).fold(0.0_f32, f32::max);
        assert!(
            max_abs > 0.0 && max_abs <= 0.8 + 1e-6,
            "noise burst out of range: {}",
            max_abs
        );
    }
}
