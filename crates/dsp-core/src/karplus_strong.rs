use crate::dispersion::{compute_dispersion_a1, DispersionStage, DISPERSION_STAGES};
use crate::fractional_delay::ThiranCoeffs;
use crate::loss_filter::LossFilter;
use crate::params::{
    BRIGHTNESS_DEFAULT, DAMPING_DEFAULT, HAMMER_CUTOFF_HIGH_PIANO, HAMMER_CUTOFF_LOW_PIANO,
    INHARMONICITY_B_PIANO, PICK_POSITION_DEFAULT,
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

/// Phase 4c D70 / D71: 1 voice あたりの最大弦数 (Piano は 1/2/3、他は 1)。
pub const MAX_STRINGS_PER_VOICE: usize = 3;

/// Phase 4c D69: MIDI ノート番号から弦数を決定する (鍵盤位置依存)。
/// - A0..=A1 (21..=33): 1 弦 (低音域、太い銅線 1 本構成のため unison なし)
/// - A#1..=B2 (34..=47): 2 弦 (中低域)
/// - C3..=C8 (48..=108): 3 弦 (中高域、tri-string unison)
///
/// 範囲外 (`midi < 21` or `midi > 108`) は端側で fallback する (低音域 → 1 弦、
/// 高音域 → 3 弦)。`Engine::note_on` は `u8` で受け取るため未定義動作を発生させない。
#[inline]
pub fn n_strings(midi: u8) -> usize {
    match midi {
        ..=33 => 1,
        34..=47 => 2,
        _ => 3,
    }
}

/// Phase 4c D72: 弦インデックスから unison detune 量 (cents) を返す。
/// - 3 弦: [0.0, -base, +base]  (中央 + 左右ペア)
/// - 2 弦: [0.0, +base]          (中央 + 片側)
/// - 1 弦: [0.0]
///
/// `base` は Piano プリセットの `unison_detune_cents` (典型 1.5、D72)。
/// 中央弦は常に detune = 0 として「Phase 4b 同等の center pitch」を維持。
#[inline]
pub fn string_detune_cents(string_idx: usize, n_strings_total: usize, base_cents: f32) -> f32 {
    match (n_strings_total, string_idx) {
        (1, 0) => 0.0,
        (2, 0) => 0.0,
        (2, 1) => base_cents,
        (3, 0) => 0.0,
        (3, 1) => -base_cents,
        (3, 2) => base_cents,
        _ => 0.0,
    }
}

/// Phase 4c D70: 1 voice 内の各弦の独立状態。`KarplusStrong::string_states: [StringState; 3]`
/// で inline 保持する。Phase 4b までは voice 単位で持っていた `write_index` / `length_int` /
/// `thiran` / `dispersion_stages` を弦別管理に分解した形 (中央弦 = string_states[0])。
///
/// 名称: 仕様書 03 章 §1.1 は `thiran: ThiranState` と書いているが、実体の Rust 型は
/// Phase 3 以来 `ThiranCoeffs` (`fractional_delay.rs:54`)。型は変えずフィールド名のみ
/// `thiran` で揃える。
#[derive(Debug, Clone, Copy)]
pub(crate) struct StringState {
    pub write_idx: usize,
    pub length_int: usize,
    pub thiran: ThiranCoeffs,
    pub dispersion_stages: [DispersionStage; DISPERSION_STAGES],
}

impl StringState {
    pub const fn new() -> Self {
        Self {
            write_idx: 0,
            length_int: 0,
            thiran: ThiranCoeffs::new(),
            dispersion_stages: [DispersionStage::new(); DISPERSION_STAGES],
        }
    }

    pub fn reset(&mut self) {
        self.write_idx = 0;
        self.length_int = 0;
        self.thiran.reset();
        for stage in self.dispersion_stages.iter_mut() {
            stage.reset();
        }
    }
}

pub struct KarplusStrong {
    /// Phase 4c D71: 弦個別 buffer (案 1、独立 Vec)。`prepare()` で 3 本を一括確保、
    /// `process_sample` ホットパスで alloc ゼロ (Phase 1 D4 維持)。Step 4 時点では
    /// `n_strings_active = 1` で `string_buffers[0]` のみ駆動し Phase 4b 同等挙動。
    string_buffers: [Vec<f32>; MAX_STRINGS_PER_VOICE],
    /// Phase 4c D70: 弦個別状態 (write_idx / length_int / thiran / dispersion_stages)。
    /// Step 4 時点では `string_states[0]` のみ active、`[1..]` は inline 配列で stack 上に存在
    /// するが note_on / process_sample で参照されない。
    string_states: [StringState; MAX_STRINGS_PER_VOICE],
    /// Phase 4c D69 / D70: 現在 active な弦数 (1, 2, or 3)。note_on 時に確定。
    /// Step 4 時点では常に 1 (Phase 4b byte compatibility)、Step 5 で `n_strings(midi)` 連動。
    n_strings_active: usize,
    /// Phase 4c D72: 楽器プリセット由来の unison detune (cents)。`set_instrument_params` 経由で
    /// Engine→VoicePool→KarplusStrong に伝搬。Step 4 では保存のみ、Step 5 で note_on に反映。
    unison_detune_cents: f32,
    /// Phase 4c D78: note_on 直前に Engine が `b_curve_piano(midi)` で lookup した値。
    /// `set_instrument_params` 経由で受領、`note_on_internal` で `compute_dispersion_a1` の B 引数に渡す。
    inharmonicity_b: f32,
    /// Phase 4c D75: Hertz hammer の cutoff 上下限 (Step 6 で活用)。Step 4 では保存のみ。
    hammer_cutoff_low_hz: f32,
    hammer_cutoff_high_hz: f32,
    /// Phase 4c D76 / D77: Sympathetic bus からの注入値。`Engine::process` per-sample loop で
    /// `inject_feedback(bus_out_prev × feedback_gain)` を呼んだあと `process_sample` 末尾で消費。
    /// Step 4 では常に 0、Step 7 で `process_sample` の damping write-back に加算。
    bus_feedback_pending: f32,
    /// 弦の周波数依存損失 (1+ρ·z⁻¹)/(1+ρ)。全弦共有 (Phase 4b 同等)。
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
    /// Phase 4b D67: `Engine::apply_instrument(Piano)` で true、他 7 楽器 (Default 含む) で false。
    /// `process_sample` ホットパスでは bool 1 つの分岐のみ、Phase 4a 互換性確保。
    dispersion_active: bool,
}

impl KarplusStrong {
    pub fn new() -> Self {
        Self {
            string_buffers: [const { Vec::new() }; MAX_STRINGS_PER_VOICE],
            string_states: [StringState::new(); MAX_STRINGS_PER_VOICE],
            n_strings_active: 1,
            unison_detune_cents: 0.0,
            inharmonicity_b: INHARMONICITY_B_PIANO,
            hammer_cutoff_low_hz: HAMMER_CUTOFF_LOW_PIANO,
            hammer_cutoff_high_hz: HAMMER_CUTOFF_HIGH_PIANO,
            bus_feedback_pending: 0.0,
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
    /// `active = false` のときは念のため全弦の dispersion 状態を reset。
    #[inline(always)]
    pub fn set_dispersion_active(&mut self, active: bool) {
        self.dispersion_active = active;
        if !active {
            for state in self.string_states.iter_mut() {
                for stage in state.dispersion_stages.iter_mut() {
                    stage.reset();
                }
            }
        }
    }

    /// Phase 4c D72 / D75 / D78: 楽器パラメータを KarplusStrong 内部に保持する。
    /// `VoicePool::note_on_with_piano_params` が `note_on_with_id` の直前に呼ぶことで
    /// `note_on_internal` から self.unison_detune_cents / inharmonicity_b / hammer_cutoff_* を
    /// 参照できる。Voice trait の note_on(freq, vel) シグネチャを Phase 4b と完全同型に保つ
    /// ための間接経路 (D81 の C ABI 維持と整合)。
    pub fn set_instrument_params(
        &mut self,
        unison_detune_cents: f32,
        inharmonicity_b: f32,
        hammer_cutoff_low_hz: f32,
        hammer_cutoff_high_hz: f32,
    ) {
        self.unison_detune_cents = unison_detune_cents;
        self.inharmonicity_b = inharmonicity_b;
        self.hammer_cutoff_low_hz = hammer_cutoff_low_hz;
        self.hammer_cutoff_high_hz = hammer_cutoff_high_hz;
    }

    /// Phase 4c D76 / D77: Sympathetic bus からの注入値を 1 sample 分保持する。
    /// `Engine::process` per-sample loop が `VoicePool::process_sample_with_feedback` 経由で
    /// `bus_out_prev × feedback_gain` を渡す。`process_sample` 末尾で消費 (Step 7)。
    /// Step 4 時点では Step 7 まで `process_sample` が値を参照しないため副作用なし。
    #[inline(always)]
    pub fn inject_feedback(&mut self, value: f32) {
        self.bus_feedback_pending = value;
    }

    /// テスト専用: dispersion 状態の検証用 read-only access。
    /// `tests/` 配下の integration test は rlib 経由で参照するため `#[cfg(test)]` だと
    /// 見えず、`#[doc(hidden)]` で公開しつつ docs.rs から隠す手法を採る (`buffer_capacity`
    /// と同じ Phase 4a 既存パターン)。
    #[doc(hidden)]
    pub fn dispersion_active(&self) -> bool {
        self.dispersion_active
    }

    /// テスト専用: 中央弦 (string_states[0]) の dispersion stage a1 を読む。
    /// Phase 4b までは voice 単位で 1 cascade だったため `dispersion_stages[idx]` を直接
    /// 読んでいたが、Phase 4c で弦別になったため string 0 経由で取得する。
    #[doc(hidden)]
    pub fn dispersion_stage_a1(&self, idx: usize) -> f32 {
        self.string_states[0].dispersion_stages[idx].a1
    }

    /// Phase 4c D70: 現在 active な弦数。test-only accessor (03 章 §7.5)。
    /// Step 4 時点では常に 1、Step 5 で `n_strings(midi)` 連動。
    #[doc(hidden)]
    pub fn n_strings_active(&self) -> usize {
        self.n_strings_active
    }

    /// Phase 4c D78: 直近の note_on で受領した B(note) 値。test-only accessor。
    #[doc(hidden)]
    pub fn inharmonicity_b(&self) -> f32 {
        self.inharmonicity_b
    }

    /// Phase 4c D72: 楽器プリセットの unison detune (cents)。test-only accessor。
    #[doc(hidden)]
    pub fn unison_detune_cents(&self) -> f32 {
        self.unison_detune_cents
    }

    pub fn prepare(&mut self, sample_rate: f32, _max_block_size: usize) {
        self.sample_rate = sample_rate;
        let max_buffer_len =
            (sample_rate / MIN_FREQ_HZ).ceil() as usize + FRACTIONAL_DELAY_BUFFER_MARGIN;

        // Phase 4c D71: 3 弦分の buffer を一括確保 (案 1)。Step 4 では string 0 のみ駆動だが
        // Step 5 以降で 2/3 弦が動的に有効化されるため、prepare で全 3 本を確保しておく。
        for buf in self.string_buffers.iter_mut() {
            *buf = vec![0.0; max_buffer_len];
        }
        for state in self.string_states.iter_mut() {
            state.reset();
        }
        self.n_strings_active = 1;
        self.bus_feedback_pending = 0.0;

        self.damping.set_time_constant(sample_rate, 0.02);
        self.brightness.set_time_constant(sample_rate, 0.02);
        self.length_target.set_time_constant(sample_rate, 0.005); // Phase 3 D39: 5ms tau

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
    ///
    /// Phase 4c Step 5: `dispersion_active && note_id.is_some()` のとき `n_strings(midi)` で
    /// 1/2/3 弦に分岐 (D69)。各弦に `string_detune_cents` を適用し、弦個別の
    /// `f_0_string = f_0_base × 2^(detune/1200)` と dispersion 係数を算出する (D72)。
    /// それ以外の経路 (非 Piano / Voice trait `note_on` の note_id=None) では
    /// `n_strings_active = 1` 固定で Phase 4a / 4b と byte 一致 (D83 / F61-a)。
    ///
    /// `state.write_idx` は Phase 4b 同型 `= len_int` を維持する (仕様書 §1.4 のサンプルは
    /// `write_idx = 0` と書いているが、励振 buffer は `buf[0..len_int]` に配置されており、
    /// `read_z = (write_idx + buf_len - length_int) % buf_len = 0` で第 1 sample を読むには
    /// `write_idx = len_int` が必須。`write_idx = 0` だと初回 read が `buf_len - length_int`
    /// になり、励振を踏まない経路となって Phase 4a HEAD byte 一致が破綻する)。
    fn note_on_internal(&mut self, note_id: Option<u8>, freq_hz: f32, velocity: f32) {
        let f_0_base = freq_hz.max(1.0);

        // Phase 4c D69 / D70: 弦数を MIDI ノートから決定。Voice trait 経由 (note_id=None) や
        // 非 Piano (`dispersion_active=false`) は常に 1 弦 (D83 互換性継承)。
        let n_strings_total = match (self.dispersion_active, note_id) {
            (true, Some(midi)) => n_strings(midi),
            _ => 1,
        };
        self.n_strings_active = n_strings_total;

        // Brightness LPF (1 段 IIR) の τ_g(b) = (1-b)/b 群遅延がピッチを下方偏移させる
        // ため、note_on 時に raw_length から差し引いて補正する (b=0.5 で 1 sample、b=1.0 で 0)。
        // 弦個別 detune は ±1.5 cents 程度で raw_len の差は 0.1% 未満、brightness_tau_g の
        // clamp 上限差は実質的に影響しないため voice 共通で 1 回だけ計算する。
        let raw_len_base = self.sample_rate / f_0_base;
        let brightness = self.brightness.target();
        let brightness_tau_g = if brightness > 0.001 {
            ((1.0 - brightness) / brightness).clamp(0.0, raw_len_base - 3.0)
        } else {
            0.0
        };

        let max_len_usize = self.string_buffers[0]
            .len()
            .saturating_sub(FRACTIONAL_DELAY_BUFFER_MARGIN);

        // 中央弦 (string_idx = 0) の adjusted_length を `base_length` / `length_target` に採用
        // して Pitch Bend / LFO Pitch を駆動する (Phase 3 / 4a 既存仕様継承)。
        let mut center_adjusted = raw_len_base.max(3.0);

        for string_idx in 0..n_strings_total {
            let detune_cents =
                string_detune_cents(string_idx, n_strings_total, self.unison_detune_cents);
            let f_0_string = if detune_cents.abs() < 1e-9 {
                f_0_base
            } else {
                f_0_base * 2.0_f32.powf(detune_cents / 1200.0)
            };
            let raw_len_string = self.sample_rate / f_0_string;

            // Phase 4b D60 + Phase 4c D78: 弦個別の dispersion a1 算出。
            // `self.inharmonicity_b` は `set_instrument_params` 経由で Engine から受領した
            // B(note) LUT 値。Step 4 時点ではデフォルト `INHARMONICITY_B_PIANO` (7.5e-4) と
            // 一致するため、`note_id = None` 経路は Phase 4b と byte 一致継承。
            let dispersion_tau_g = if self.dispersion_active {
                let (a1, gd_per_stage) = compute_dispersion_a1(
                    DISPERSION_STAGES as u32,
                    self.inharmonicity_b,
                    f_0_string,
                    self.sample_rate,
                );
                for stage in self.string_states[string_idx].dispersion_stages.iter_mut() {
                    stage.a1 = a1;
                    stage.z1_in = 0.0;
                    stage.z1_out = 0.0;
                }
                (DISPERSION_STAGES as f32) * gd_per_stage
            } else {
                // dispersion_active=false の弦は a1=0 で passthrough 動作にしておく。
                for stage in self.string_states[string_idx].dispersion_stages.iter_mut() {
                    stage.a1 = 0.0;
                    stage.z1_in = 0.0;
                    stage.z1_out = 0.0;
                }
                0.0
            };

            let total_compensation = brightness_tau_g + dispersion_tau_g;
            let adjusted = (raw_len_string - total_compensation).max(3.0);
            let len_int = (adjusted.floor() as usize).clamp(3, max_len_usize);
            let len_frac = (adjusted - len_int as f32).clamp(0.0, 1.0);

            if string_idx == 0 {
                center_adjusted = adjusted;
            }

            {
                let state = &mut self.string_states[string_idx];
                state.length_int = len_int;
                state.thiran.set_fractional(len_frac);
                // Thiran は IIR、note_on 連打で前 note の状態を引き継ぐと過渡応答が暴れる。
                state.thiran.reset();
                state.write_idx = len_int;
            }

            // Phase 4b D61: buffer 初期化を pluck (Phase 1〜4a 既存) / hammer (Piano kind) で分岐。
            // Phase 4c Step 5 時点では Phase 4b の Commuted impulse + velocity LPF を全弦に
            // 適用する (Step 6 で Hertz raised cosine impulse に差し替え)。
            let buf = &mut self.string_buffers[string_idx];
            for v in buf.iter_mut() {
                *v = 0.0;
            }
            if self.dispersion_active {
                // === Hammer 経路 (Step 6 で Hertz raised cosine に差し替え予定) ===
                buf[0] = velocity;
                let cutoff_hz = self.hammer_cutoff_low_hz
                    + velocity.clamp(0.0, 1.0)
                        * (self.hammer_cutoff_high_hz - self.hammer_cutoff_low_hz);
                let alpha = (1.0
                    - (-2.0 * core::f32::consts::PI * cutoff_hz / self.sample_rate).exp())
                .clamp(0.001, 0.999);
                let mut z = 0.0_f32;
                for sample in buf.iter_mut().take(len_int) {
                    z = alpha * (*sample) + (1.0 - alpha) * z;
                    *sample = z;
                }
            } else {
                // === Pluck 経路 (Phase 1〜4a 既存、Default + 6 楽器、n_strings_total=1 のみ通る) ===
                for sample in buf.iter_mut().take(len_int) {
                    *sample = self.rng.next_unit_bipolar() * velocity;
                }
                let k = (self.pick_position * len_int as f32)
                    .round()
                    .clamp(0.0, len_int.saturating_sub(1) as f32) as usize;
                if k > 0 {
                    for i in (k..len_int).rev() {
                        buf[i] -= buf[i - k];
                    }
                }
            }
        }

        self.loss_filter.set_for_frequency(f_0_base);

        self.last_filter_out = 0.0;
        self.energy = velocity * velocity;
        self.active = true;
        self.age_samples = 0;
        self.current_note = note_id;
        self.bus_feedback_pending = 0.0;

        self.base_length = center_adjusted;
        self.pitch_bend_semitones = 0.0;
        self.length_target.set_immediate(center_adjusted);
        self.cached_length = center_adjusted;
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
        let max_len = (self.string_buffers[0].len() - FRACTIONAL_DELAY_BUFFER_MARGIN) as f32;
        self.length_target.set_target(target.clamp(3.0, max_len));
    }

    /// テスト専用: 任意の length_int で励振 (K=0 分岐の到達確認用)。
    /// 公開 β min は 0.05、length_int=9 + β=0.05 で K=round(0.45)=0 を踏める。
    #[doc(hidden)]
    pub fn note_on_with_length_for_test(&mut self, length_int: usize, beta: f32, velocity: f32) {
        debug_assert!(length_int >= 3);
        debug_assert!(self.string_buffers[0].len() > length_int);
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
        self.string_states[0].length_int
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
        self.string_buffers[0].len()
    }

    /// テスト用: 励振直後の string 0 buffer の先頭 `length_int` を読む。alloc を含むので
    /// production 経路では使わない。
    #[cfg(test)]
    pub(crate) fn excitation_snapshot(&self) -> Vec<f32> {
        self.string_buffers[0][..self.string_states[0].length_int].to_vec()
    }

    pub fn reset(&mut self) {
        for buf in self.string_buffers.iter_mut() {
            for v in buf.iter_mut() {
                *v = 0.0;
            }
        }
        for state in self.string_states.iter_mut() {
            state.reset();
        }
        self.n_strings_active = 1;
        self.bus_feedback_pending = 0.0;
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
    }

    #[inline(always)]
    pub fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        // Step 4 では Phase 4b 経路を維持するため string 0 のみ駆動。
        // Step 7 で `for string_idx in 0..self.n_strings_active` 並列ループへ拡張。
        let buf = &mut self.string_buffers[0];
        let state = &mut self.string_states[0];
        let buf_len = buf.len();

        // 定常時は length 再分解と Thiran 係数再計算を skip (差分 < 1e-5)。
        // Phase 4a D48: LFO Pitch factor を実効 length に乗算 (factor は Engine 側で exp2 済)。
        let base_target = self.length_target.next_sample();
        let effective_length = base_target * self.lfo_pitch_factor;
        if (effective_length - self.cached_length).abs() > 1e-5 {
            let max_len = (buf_len - FRACTIONAL_DELAY_BUFFER_MARGIN) as f32;
            let clamped = effective_length.clamp(3.0, max_len);
            state.length_int = clamped as usize;
            let frac = clamped - state.length_int as f32;
            state.thiran.set_fractional(frac);
            self.cached_length = effective_length;
        }

        // Pitch Bend で length_int が動的に変わるため、剰余は `% buf_len` のみ。
        // `% length_int` だと write/read で異なる剰余系になり buffer の論理長が破綻する。
        let read_z = (state.write_idx + buf_len - state.length_int) % buf_len;

        // Phase 4b D60: Dispersion cascade を Thiran の前段に挿入。
        // `dispersion_active = false` 経路は Phase 4a と完全一致 (D67 互換性核心)。
        let read_value = if self.dispersion_active {
            let mut x = buf[read_z];
            for stage in state.dispersion_stages.iter_mut() {
                x = stage.process(x);
            }
            state.thiran.process(x)
        } else {
            state.thiran.process(buf[read_z])
        };

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

        buf[state.write_idx] = damped;
        let next_write = state.write_idx + 1;
        state.write_idx = if next_write == buf_len { 0 } else { next_write };

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

    /// Phase 4b D61: dispersion_active=true で hammer 経路 (Commuted impulse + velocity LPF)
    /// が使われる。pluck 経路の noise burst では隣接 sample 間の符号反転が頻発するが、
    /// hammer 経路では impulse + LPF 平滑化のため buffer 前半が単調減衰となる。
    #[test]
    fn test_note_on_with_dispersion_active_uses_hammer_excitation() {
        let mut v = KarplusStrong::new();
        v.prepare(SAMPLE_RATE, 128);
        v.set_dispersion_active(true);
        v.note_on(440.0, 0.8);

        let snapshot = v.excitation_snapshot();
        // hammer 経路: buffer[0] が impulse の平滑化結果として最大、続く buffer[i] が
        // 1pole LPF の応答で単調減衰。velocity=0.8 で cutoff = 800 + 0.8*4700 = 4560 Hz
        // (Phase 4c で HIGH 5500 に拡張)、alpha = 1 - exp(-2π·4560/48000) ≈ 0.450、
        // y[1] = α·0 + (1-α)·y[0] ≈ 0.550·y[0]
        assert!(
            snapshot[0].abs() > 0.0,
            "buffer[0] must carry impulse energy, got {}",
            snapshot[0]
        );
        let r = snapshot[1] / snapshot[0];
        assert!(
            (0.3..=0.95).contains(&r),
            "1pole LPF decay ratio buffer[1]/buffer[0] should be in [0.3, 0.95], got {}",
            r
        );
        // 単調減衰 (符号変化なし) を最低 4 sample まで確認
        for i in 1..4.min(snapshot.len()) {
            assert!(
                snapshot[i].signum() == snapshot[0].signum() || snapshot[i].abs() < 1.0e-9,
                "hammer LPF should produce monotonic decay without sign change, buf[{}]={}",
                i,
                snapshot[i]
            );
        }
    }

    #[test]
    fn test_note_on_with_dispersion_inactive_uses_pluck_excitation() {
        let mut v = KarplusStrong::new();
        v.prepare(SAMPLE_RATE, 128);
        assert!(!v.dispersion_active());
        v.note_on(440.0, 0.8);

        let snapshot = v.excitation_snapshot();
        // pluck 経路: noise burst なので隣接 sample で頻繁に符号変化が起きる
        let mut sign_changes = 0;
        for i in 1..snapshot.len() {
            if snapshot[i].signum() != snapshot[i - 1].signum() {
                sign_changes += 1;
            }
        }
        // Pluck noise burst: 全 sample のうち 1/4 以上で符号変化があるはず
        assert!(
            sign_changes >= snapshot.len() / 4,
            "pluck noise should have many sign changes, got {} of {}",
            sign_changes,
            snapshot.len()
        );
    }

    #[test]
    fn test_hammer_velocity_affects_brightness() {
        // velocity が高いほど cutoff が高く (= 1pole LPF が浅く) なり、buffer の高域成分が増える。
        // 高域成分の指標として、隣接 sample 差分の RMS を使う (差分は HPF 等価)。
        fn diff_rms(buf: &[f32]) -> f32 {
            let mut sum = 0.0_f64;
            for i in 1..buf.len() {
                let d = (buf[i] - buf[i - 1]) as f64;
                sum += d * d;
            }
            (sum / (buf.len() - 1) as f64).sqrt() as f32
        }

        let mut v_soft = KarplusStrong::new();
        v_soft.prepare(SAMPLE_RATE, 128);
        v_soft.set_dispersion_active(true);
        v_soft.note_on(440.0, 0.1);
        let buf_soft = v_soft.excitation_snapshot();

        let mut v_hard = KarplusStrong::new();
        v_hard.prepare(SAMPLE_RATE, 128);
        v_hard.set_dispersion_active(true);
        v_hard.note_on(440.0, 1.0);
        let buf_hard = v_hard.excitation_snapshot();

        let rms_soft = diff_rms(&buf_soft);
        let rms_hard = diff_rms(&buf_hard);
        // hard の方が cutoff が高い (= 高域多い) ので diff_rms は大きい
        assert!(
            rms_hard > rms_soft,
            "hard velocity should produce higher diff_rms, soft={}, hard={}",
            rms_soft,
            rms_hard
        );
    }
}
