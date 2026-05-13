# 03. dsp-core 仕様（Phase 4c）

## 目的

`crates/dsp-core/` の Rust モジュール群に Phase 4c で追加する API / 内部状態 / テストを定義する。Phase 1 / 2 / 3 / 4a / 4b で確立した既存モジュールの責務（`KarplusStrong` / `VoicePool` / `Engine` / `ModalBodyResonator` / `LossFilter` / `SoftClip` / `SustainState` / `SmoothedValue` / `XorShift32` / `HoldStack` / `ParamDescriptor` / `FractionalDelay` (Thiran) / `VoiceState` / `Lfo` / `Dispersion`）はすべて維持し、本書では **Phase 4c で追加・変更する箇所のみ** 記述する。

## 他文書との関係

- 上流: [`pre-research.md`](./pre-research.md)（§3 Multi-string / §4 Hertz hammer / §5 Sympathetic / §6 B(note) LUT / §7 Modal Body M=16）、[`01-overview.md`](./01-overview.md)（D68-D85）、[`02-architecture.md`](./02-architecture.md)（dsp-core 層責務）
- 下流: [`04-wasm-audio-spec.md`](./04-wasm-audio-spec.md)（C ABI 不変）、[`07-implementation-checklist.md`](./07-implementation-checklist.md)
- 並行: Phase 4b [`03-dsp-core-spec.md`](../2026-05-09-005-phase4b/03-dsp-core-spec.md) — 既存 API スタイルの参照

## モジュール一覧（Phase 4c 後）

```
crates/dsp-core/src/
├── dispersion.rs           (Phase 4b 同等、compute_dispersion_a1 のみ Phase 4c で B 引数を呼出側で LUT 値に変更)
├── engine.rs               (Phase 4c で ResonanceBus 統合 D76 + handle_midi_cc(CC_SUSTAIN_PEDAL) 拡張での feedback_gain 切替 D77、既存 process ブロック関数の per-sample loop に 3 行追加)
├── fractional_delay.rs     (Phase 3 同等、変更なし)
├── hold_stack.rs           (Phase 2 同等、変更なし)
├── karplus_strong.rs       (Phase 4c で [StringState; 3] + n_strings_active 追加、note_on の multi-string 化 + Hertz hammer raised cosine 化、process_sample の N 弦並列化)
├── lfo.rs                  (Phase 4a 同等、変更なし)
├── lib.rs                  (Phase 4c で `pub mod resonance_bus;` 追加)
├── loss_filter.rs          (Phase 3 同等、変更なし)
├── modal_body.rs           (Phase 4b 同等、Step 14 で M=16 採用時のみ拡張)
├── note_allocator.rs       (Phase 2 同等、変更なし)
├── params.rs               (生成、Phase 4c で INHARMONICITY_B_CURVE_PIANO + UNISON_DETUNE_CENTS_PIANO + SYMPATHETIC_AMOUNT_PIANO 出力)
├── resonance_bus.rs        (Phase 4c 新規 — Global sympathetic resonance bus D76)
├── rng.rs                  (Phase 1 同等、変更なし)
├── smoothing.rs            (Phase 3 同等、変更なし)
├── soft_clip.rs            (Phase 3 同等、変更なし)
├── sustain_state.rs        (Phase 3 同等、Phase 4c で set_active 拡張なし)
├── traits.rs               (Voice trait 同等、Phase 4c で変更なし)
├── voice.rs                (Phase 4b 同等、変更なし)
├── voice_pool.rs           (Phase 4c で sum_voice_outputs / inject_feedback メソッド追加)
└── voice_state.rs          (Phase 3 同等、変更なし)
```

## 1. `karplus_strong.rs` の拡張（Phase 4c の核）

### 1.1 構造体の変更

```rust
const MAX_STRINGS_PER_VOICE: usize = 3;

/// Phase 4c: 1 voice 内の各弦の状態（Multi-string per voice）
#[derive(Debug, Clone, Copy)]
struct StringState {
    /// 弦個別の write 位置（detune で length が異なるため弦別管理）
    write_idx: usize,
    /// 弦個別の length（detune 後の値、fractional 含む）
    length: f32,
    /// 弦個別の integer length（read 用、floor(length)）
    length_int: usize,
    /// 弦個別の fractional 部（Thiran 用、length - length_int）
    fractional: f32,
    /// 弦個別の Thiran allpass 状態
    thiran: ThiranState,
    /// 弦個別の dispersion cascade (M=8 段、Piano kind のみ active)
    dispersion_stages: [DispersionStage; 8],
}

impl StringState {
    pub fn new() -> Self {
        Self {
            write_idx: 0,
            length: 0.0,
            length_int: 0,
            fractional: 0.0,
            thiran: ThiranState::new(),
            dispersion_stages: [DispersionStage::new(); 8],
        }
    }

    pub fn reset(&mut self) {
        self.write_idx = 0;
        self.length = 0.0;
        self.length_int = 0;
        self.fractional = 0.0;
        self.thiran.reset();
        for stage in self.dispersion_stages.iter_mut() {
            stage.reset();
        }
    }
}

pub struct KarplusStrong {
    // === Phase 4c で新規追加 ===
    /// 弦個別の buffer（案 1: 独立 buffer、D71）
    string_buffers: [Vec<f32>; MAX_STRINGS_PER_VOICE],
    /// 弦個別の状態（Thiran + dispersion）
    string_states: [StringState; MAX_STRINGS_PER_VOICE],
    /// active な弦数 (1, 2, or 3、note_on 時に確定)
    n_strings_active: usize,
    /// 楽器プリセット由来パラメータ（`set_instrument_params` で受領、note_on 時に参照）
    unison_detune_cents: f32,
    inharmonicity_b: f32,
    hammer_cutoff_low_hz: f32,
    hammer_cutoff_high_hz: f32,
    /// Sympathetic bus からの注入値（process_sample の 1 sample 内で消費）
    bus_feedback_pending: f32,

    // === Phase 4b 既存 (全弦共有) ===
    brightness_lpf: BrightnessLpf,
    loss_filter: LossFilter,
    damping: SmoothedValue,
    sample_rate: f32,
    rng: XorShift32,
    dispersion_active: bool,

    // === Phase 1〜4a 既存 ===
    note_id: Option<u8>,
    active: bool,
    age: u32,
    pick_position: SmoothedValue,
    pitch_bend_factor: f32,
    lfo_pitch_factor: f32,
    lfo_brightness_offset: f32,
    // ... (他は Phase 4b と同じ)
}
```

**Phase 4b 既存の `buffer: Vec<f32>` / `dispersion_stages: [DispersionStage; 8]` / `thiran: ThiranCoeffs` は削除**し、`string_buffers[0]` / `string_states[0].dispersion_stages` / `string_states[0].thiran` に統合する（中央弦 = 既存の単弦経路）。これにより `n_strings_active = 1` の経路が Phase 4b と機能的に同型になり、F61-a (`test_default_n_strings_1_matches_phase4a`) で byte 一致を機械保証できる。

### 1.2 `prepare()` の拡張

```rust
impl KarplusStrong {
    pub fn prepare(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        let max_len = (sample_rate / 27.5).ceil() as usize + 1;  // A0 = 27.5 Hz

        // Phase 4c: 弦個別 buffer を 3 本確保（heap 確保はここ 1 箇所のみ）
        for buf in self.string_buffers.iter_mut() {
            buf.resize(max_len, 0.0);  // 案 1 (D71)、案 2 採用時はサイズ調整
        }

        for state in self.string_states.iter_mut() {
            state.reset();
        }

        self.n_strings_active = 1;  // Default: 1 弦
        self.dispersion_active = false;

        // Phase 4b 既存処理
        self.brightness_lpf.reset();
        self.loss_filter.reset();
        self.damping.reset(0.996);
        // ... (Phase 1〜4b 既存)
    }
}
```

### 1.3 `n_strings(midi)` 関数

```rust
/// MIDI ノート番号から弦数を決定（D69、鍵盤位置依存）
/// - A0..A1 (21..=33): 1 弦
/// - A#1..B2 (34..=47): 2 弦
/// - C3..C8 (48..=108): 3 弦
#[inline]
fn n_strings(midi: u8) -> usize {
    match midi {
        21..=33 => 1,
        34..=47 => 2,
        _ => 3,
    }
}

/// 弦インデックスから detune 量を返す（D72）
/// - 3 弦: [0.0, -1.5, +1.5] cents
/// - 2 弦: [0.0, +1.5] cents
/// - 1 弦: [0.0]
#[inline]
fn string_detune_cents(string_idx: usize, n_strings: usize, base_cents: f32) -> f32 {
    match (n_strings, string_idx) {
        (1, 0) => 0.0,
        (2, 0) => 0.0,
        (2, 1) => base_cents,
        (3, 0) => 0.0,
        (3, 1) => -base_cents,
        (3, 2) => base_cents,
        _ => 0.0,
    }
}
```

### 1.4 `note_on_internal()` の拡張（公開 API は現行維持、楽器パラメータは別経路で渡す）

**重要**: 現行 (Phase 4b) の公開 API は `KarplusStrong::note_on(freq_hz, velocity)` および `note_on_with_id(midi_note, freq_hz, velocity)` で（`crates/dsp-core/src/karplus_strong.rs:172` 参照）、Voice trait 互換性のためこの 3 引数シグネチャは Phase 4c でも維持する。

Phase 4c で追加する楽器パラメータ（`unison_detune_cents` / `inharmonicity_b_curve` / `hammer_cutoff_low_hz` / `hammer_cutoff_high_hz`）は、`note_on` の引数に渡すのではなく、**`note_on` 直前に `set_instrument_params(...)` を呼んで `KarplusStrong` の内部フィールドに保持**する。Engine から VoicePool 経由でこのパターンを実現する（§5）。

#### Voice trait API（Phase 4b 同等、変更なし）

```rust
pub trait Voice {
    fn prepare(&mut self, sample_rate: f32, max_block_size: usize);
    fn note_on(&mut self, freq_hz: f32, velocity: f32);
    fn note_off(&mut self);
    fn process_sample(&mut self) -> f32;
    fn is_active(&self) -> bool;
    // ...（Phase 4b と同じ）
}
```

#### KarplusStrong の Phase 4c 拡張メソッド（trait 外、KarplusStrong 直接呼出用）

```rust
impl KarplusStrong {
    /// Phase 4c: 楽器パラメータを内部フィールドに保持する。
    /// `note_on_with_id` の直前で呼び、次の note_on でこれらの値が使われる。
    ///
    /// 非 Piano では `b_curve_zero` 関数ポインタを渡し inharmonicity_b=0 で
    /// dispersion を実質 disable する（dispersion_active=false 経路と二重保証）。
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

    pub fn note_on_with_id(&mut self, midi_note: u8, freq_hz: f32, velocity: f32) {
        self.note_on_internal(Some(midi_note), freq_hz, velocity);
    }

    fn note_on_internal(&mut self, note_id: Option<u8>, freq_hz: f32, velocity: f32) {
        let f_0_base = freq_hz.max(1.0);

        // Phase 4c: 弦数を MIDI ノートから決定（D69、鍵盤位置依存）
        // 非 Piano (`dispersion_active = false`) では常に 1 弦（Phase 4a / 4b 互換、D83）
        self.n_strings_active = if self.dispersion_active {
            note_id.map_or(1, n_strings)
        } else {
            1
        };

        for string_idx in 0..self.n_strings_active {
            let detune = string_detune_cents(
                string_idx,
                self.n_strings_active,
                self.unison_detune_cents,
            );
            let f_0_string = f_0_base * 2.0_f32.powf(detune / 1200.0);

            // Phase 4c: 弦個別 dispersion a1 算出（B は set_instrument_params で受領済の LUT 値）
            // 戻り値 tuple は (a1, gd_per_stage)、Phase 4b 同型（karplus_strong.rs:201）
            let (a1, gd_per_stage) = if self.dispersion_active {
                compute_dispersion_a1(
                    DISPERSION_STAGES as u32,
                    self.inharmonicity_b,
                    f_0_string,
                    self.sample_rate,
                )
            } else {
                (0.0_f32, 0.0_f32)
            };

            // Phase 4c: 弦個別 length / fractional 算出
            let raw_len = self.sample_rate / f_0_string;
            let brightness_tau_g = self.compute_brightness_group_delay();  // Phase 3 既存
            let dispersion_tau_g = if self.dispersion_active {
                (DISPERSION_STAGES as f32) * gd_per_stage
            } else {
                0.0
            };
            let total_compensation = brightness_tau_g + dispersion_tau_g;
            let max_len_usize = self.string_buffers[string_idx]
                .len()
                .saturating_sub(FRACTIONAL_DELAY_BUFFER_MARGIN);
            let adjusted = (raw_len - total_compensation).max(3.0);
            let length_int = (adjusted.floor() as usize).clamp(3, max_len_usize);
            let fractional = (adjusted - length_int as f32).clamp(0.0, 1.0);

            let state = &mut self.string_states[string_idx];
            state.length = adjusted;
            state.length_int = length_int;
            state.fractional = fractional;
            state.thiran.set_fractional(fractional);
            state.thiran.reset();
            state.write_idx = 0;
            for stage in state.dispersion_stages.iter_mut() {
                stage.a1 = a1;
                stage.z1_in = 0.0;
                stage.z1_out = 0.0;
            }

            self.loss_filter_per_string_or_shared(f_0_string);  // 全弦共有なら 1 回だけ呼ぶ実装で OK

            // Phase 4c: 弦個別 buffer の Hertz hammer or pluck 初期化
            self.init_excitation_for_string(string_idx, velocity);
        }

        self.note_id = note_id;
        self.active = true;
        self.age = 0;
        self.bus_feedback_pending = 0.0;  // Sympathetic bus inject 用バッファ
        self.pitch_bend_factor = 1.0;
        self.lfo_pitch_factor = 1.0;
        self.lfo_brightness_offset = 0.0;
    }

    fn init_excitation_for_string(&mut self, string_idx: usize, velocity: f32) {
        if self.dispersion_active {
            self.init_hammer_impulse_for_string(string_idx, velocity);
        } else {
            self.init_pluck_excitation_for_string(string_idx, velocity);
        }
    }

    /// Hertz law raised cosine hammer impulse 初期化 (D74 / D75)
    /// cutoff の上下限は `set_instrument_params` 経由で self に保持済。
    fn init_hammer_impulse_for_string(&mut self, string_idx: usize, velocity: f32) {
        let cutoff_low_hz = self.hammer_cutoff_low_hz;
        let cutoff_high_hz = self.hammer_cutoff_high_hz;
        let buf = &mut self.string_buffers[string_idx];
        let state = &self.string_states[string_idx];
        let len_int = state.length_int.min(buf.len());

        // 1) パラメータ計算 (D75)
        let t_c_ms = 4.0 - 2.8 * velocity;  // 1.2 .. 4.0 ms
        let t_c_samples = ((t_c_ms * 0.001 * self.sample_rate) as usize)
            .min(len_int)
            .max(1);
        let f_c_hz = cutoff_low_hz + velocity * (cutoff_high_hz - cutoff_low_hz);
        let amplitude = velocity.sqrt();

        // 2) buffer を zero clear
        for v in buf.iter_mut() {
            *v = 0.0;
        }

        // 3) raised cosine 半周期 (sin²) で接触時間を表現
        let pi = core::f32::consts::PI;
        for i in 0..t_c_samples {
            let phi = (i as f32 / t_c_samples as f32) * pi;
            buf[i] = amplitude * (phi.sin() * phi.sin());
        }

        // 4) velocity LPF を buffer 全体に適用 (1pole IIR)
        let alpha = compute_lpf_alpha(f_c_hz, self.sample_rate);
        let mut z = 0.0_f32;
        for v in buf[..len_int].iter_mut() {
            z = alpha * (*v) + (1.0 - alpha) * z;
            *v = z;
        }
    }

    fn init_pluck_excitation_for_string(&mut self, string_idx: usize, velocity: f32) {
        // Phase 4a / 4b の pluck 経路 (noise burst + pick comb)
        // string_idx = 0 のみ active (n_strings_active = 1 のため)
        let buf = &mut self.string_buffers[string_idx];
        let state = &self.string_states[string_idx];
        let len_int = state.length_int.min(buf.len());
        for i in 0..len_int {
            buf[i] = self.rng.next_unit_bipolar() * velocity;
        }
        // Pick position comb は Phase 4a と同じ実装、ここでは省略
    }
}
```

### 1.5 `process_sample()` の N 弦並列化

```rust
impl KarplusStrong {
    pub fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let damping = self.damping.next_sample();
        let mut sum_strings = 0.0_f32;

        // Phase 4c: N 弦の並列処理
        for string_idx in 0..self.n_strings_active {
            let state_idx = string_idx;
            let buf = &mut self.string_buffers[state_idx];
            let state = &mut self.string_states[state_idx];

            if state.length_int < 3 || state.length_int >= buf.len() {
                continue;
            }

            // read 位置（write_idx - length_int、ring buffer）
            let read_z = (state.write_idx + buf.len() - state.length_int) % buf.len();
            let mut x = buf[read_z];

            // Phase 4b: dispersion cascade（弦個別、Piano のみ実行）
            if self.dispersion_active {
                for stage in state.dispersion_stages.iter_mut() {
                    let y = stage.a1 * (x - stage.z1_out) + stage.z1_in;
                    stage.z1_in = x;
                    stage.z1_out = y;
                    x = y;
                }
            }

            // Phase 4b: Thiran allpass（弦個別、Phase 3 D36）
            let y = state.thiran.process(x);

            // Phase 4b: damping + write back（Phase 4c: bus_feedback_pending を加算）
            let new_value = y * damping + self.bus_feedback_pending;
            buf[state.write_idx] = new_value + 1e-25 - 1e-25;  // denormal flush (D6)
            state.write_idx = (state.write_idx + 1) % buf.len();

            sum_strings += y;
        }

        // Sympathetic bus からの注入値は 1 sample で消費（次の sample で Engine が再設定）
        self.bus_feedback_pending = 0.0;

        // 全弦共有: brightness + loss filter
        let brightness_offset = self.lfo_brightness_offset;
        let out = self.brightness_lpf.process(sum_strings, brightness_offset);
        let out = self.loss_filter.process(out);

        self.age = self.age.saturating_add(1);
        out
    }

    /// Phase 4c: Sympathetic bus からの inject 値を保持（次の process_sample で消費）
    ///
    /// Engine から per-sample で呼ばれ、`bus_out_prev × feedback_gain` を設定する。
    /// Default kind では Engine 側で 0 を渡す（feedback_gain=0 で乗算結果が 0）。
    pub fn inject_feedback(&mut self, value: f32) {
        self.bus_feedback_pending = value;
    }
}
```

### 1.6 状態リセット

```rust
impl KarplusStrong {
    pub fn note_off(&mut self) {
        self.active = false;
    }

    pub fn reset(&mut self) {
        self.active = false;
        for buf in self.string_buffers.iter_mut() {
            buf.iter_mut().for_each(|v| *v = 0.0);
        }
        for state in self.string_states.iter_mut() {
            state.reset();
        }
        self.n_strings_active = 1;
        self.dispersion_active = false;
        // Phase 4b 既存
        self.brightness_lpf.reset();
        self.loss_filter.reset();
        self.damping.reset(0.996);
    }

    /// Phase 4b D67 と同型
    pub fn set_dispersion_active(&mut self, active: bool) {
        self.dispersion_active = active;
    }
}
```

## 2. `dispersion.rs` の B 引数化（戻り値は Phase 4b 同型の tuple を維持）

**重要**: 現行 (Phase 4b) の `compute_dispersion_a1` は `(a1, gd_per_stage)` の **tuple を返す**（`crates/dsp-core/src/karplus_strong.rs:201` で `let (a1, gd_per_stage) = compute_dispersion_a1(...)` として使用）。Phase 4c でもこの **tuple 戻り値を維持**し、呼出側で `length_compensation = M·gd_per_stage` を算出する流れを引き継ぐ（D78）。

```rust
/// Rauhala-Välimäki 2006 closed-form a1 計算 (Phase 4b と完全同型、B 引数経由)
///
/// 戻り値: `(a1, gd_per_stage)` — a1 は all-pass 係数、gd_per_stage は基音における
/// 1 段あたりの群遅延（sample 単位）。呼出側で `M * gd_per_stage` を length 補正に使う。
pub fn compute_dispersion_a1(m: u32, b: f32, f_0: f32, fs: f32) -> (f32, f32) {
    // Phase 4b §4.2 の式をそのまま使用、変更なし
    let trt = 2.0_f32.powf(1.0 / 12.0);
    let bc = b.max(1.0e-6);
    let log_bc = bc.ln();
    let ikey = ((f_0 * trt) / 27.5).ln() / trt.ln();

    let k1 = -0.00179_f32;
    let k2 = -0.0233_f32;
    let k3 = -2.93_f32;
    let kd = (k1 * log_bc * log_bc + k2 * log_bc + k3).exp();

    let m1 = 0.0126_f32;
    let m2 = 0.0606_f32;
    let m3 = -0.00825_f32;
    let m4 = 1.97_f32;
    let m_log = (m as f32).ln();
    let cd = ((m1 * m_log + m2) * log_bc + m3 * m_log + m4).exp();

    let d = (cd - ikey * kd).exp();
    let a1 = ((1.0 - d) / (1.0 + d)).clamp(-0.999, 0.999);

    // 基音における 1 段あたりの群遅延（Phase 4b と同じ polydel(a1) - polydel(1/a1)）
    let wt = 2.0 * core::f32::consts::PI * f_0 / fs;
    let polydel = |a: f32| -> f32 { (wt.sin() / (a + wt.cos())).atan() / wt };
    let gd_per_stage = polydel(a1) - polydel(1.0 / a1);

    (a1, gd_per_stage)
}

/// Phase 4b の固定 INHARMONICITY_B_PIANO = 7.5e-4 const は Phase 4c 互換性のため残置
/// （新規呼出は b_curve_piano(midi) で LUT 値を渡す）
pub const INHARMONICITY_B_PIANO: f32 = 7.5e-4;

/// Phase 4c: MIDI ノートを 21..=108 に clamp してから LUT を引く（D78 / D79）。
/// 範囲外（< 21 / > 108）は端値で fallback、Engine から受ける `u8` 全域に対応。
#[inline]
pub fn b_curve_piano(midi: u8) -> f32 {
    let clamped = midi.clamp(21, 108);
    let idx = (clamped - 21) as usize;  // 0..=87
    crate::params::INHARMONICITY_B_CURVE_PIANO[idx]
}

/// Phase 4c: 非 Piano 楽器用の B 関数ポインタ。常に 0 を返し dispersion を実質 disable。
#[inline]
pub fn b_curve_zero(_midi: u8) -> f32 {
    0.0
}
```

## 3. `resonance_bus.rs`（Phase 4c 新規）

### 3.1 構造体

```rust
use crate::smoothing::SmoothedValue;
use crate::loss_filter::BrightnessLpf;  // 名称は Phase 3 と同じ、流用

const BUS_DELAY_MS: f32 = 2.0;

pub struct ResonanceBus {
    /// Bus 用の delay line（短い lossy feedback、fundamental を持たない）
    buffer: Vec<f32>,
    /// Bus 内部の LPF（高域減衰）
    lpf: BrightnessLpf,
    /// Bus → voice への feedback gain (Piano kind + Sustain ON で 0.03〜0.05)
    feedback_gain: SmoothedValue,
    /// 現在の write 位置
    write_idx: usize,
    sample_rate: f32,
}

impl ResonanceBus {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            lpf: BrightnessLpf::new(),
            feedback_gain: SmoothedValue::new(0.0, 0.02),  // τ=0.02s
            write_idx: 0,
            sample_rate: 48000.0,
        }
    }

    pub fn prepare(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        let len = (BUS_DELAY_MS * 0.001 * sample_rate).ceil() as usize + 1;
        self.buffer.resize(len, 0.0);
        self.lpf.reset();
        self.feedback_gain.reset(0.0);
        self.write_idx = 0;
    }

    pub fn reset(&mut self) {
        for v in self.buffer.iter_mut() {
            *v = 0.0;
        }
        self.lpf.reset();
        self.feedback_gain.reset(0.0);
        self.write_idx = 0;
    }

    /// Sustain ON / OFF で feedback_gain を切替（D77、Piano kind のみ非ゼロ）
    pub fn set_feedback_gain_target(&mut self, target: f32) {
        self.feedback_gain.set_target(target.clamp(0.0, 0.05));
    }

    /// 1 sample 処理: bus_in を delay line に書き込み、bus_out を返す
    ///
    /// 戻り値の bus_out は呼出側で feedback_gain と乗算して各 voice に inject する
    pub fn process(&mut self, bus_in: f32) -> f32 {
        let len = self.buffer.len();
        if len == 0 {
            return 0.0;
        }

        // read 位置（write の 1 sample 後 = 最も古い値）
        let read_idx = (self.write_idx + 1) % len;
        let read_value = self.buffer[read_idx];

        // LPF で高域減衰
        let filtered = self.lpf.process(read_value, 0.0);

        // bus_in + filtered (lossy feedback) を delay line に書き込み
        let new_value = bus_in + filtered * 0.95;  // 0.95 は bus 内部の減衰係数
        self.buffer[self.write_idx] = new_value + 1e-25 - 1e-25;  // denormal flush
        self.write_idx = (self.write_idx + 1) % len;

        filtered
    }

    /// SmoothedValue を 1 sample 進める（外部で呼ぶ、`Engine::process` の per-sample loop 冒頭で）
    pub fn next_feedback_gain(&mut self) -> f32 {
        self.feedback_gain.next_sample()
    }
}
```

### 3.2 数値安定性の保証

- `feedback_gain` を `[0.0, 0.05]` に clamp（pre-research §5.4）
- bus 内部の `0.95` 減衰係数で全体 gain product < 1 を保証
- LPF は `BrightnessLpf` を流用（Phase 3 D36 と同型、cutoff 8 kHz 固定）

### 3.3 サイズ計算

- `buffer`: 2 ms × 48 kHz × 4 byte = 384 byte
- `lpf`: 16 byte
- `feedback_gain`: 8 byte
- 合計 +416 byte

## 4. `engine.rs` の拡張

### 4.1 構造体

Phase 4b 既存フィールド名（`current_instrument` / `mod_wheel: SmoothedValue` / `current_damping: f32` 等）と関数 `soft_clip()` (`crates/dsp-core/src/soft_clip.rs`、SoftClip 型ではなく関数) を維持し、Phase 4c で追加するフィールドのみ示す。

```rust
pub struct Engine {
    // === Phase 4c で新規追加 ===
    resonance_bus: ResonanceBus,
    bus_out_prev: f32,                              // 前 sample の bus_out を保持、次 sample で voice に inject

    /// Piano プリセット内パラメータ（D72 / D77 / D78、apply_instrument で切替）
    unison_detune_cents: f32,
    sympathetic_amount: f32,
    inharmonicity_b_for_note: fn(u8) -> f32,        // 楽器ごとに切替（Piano → b_curve_piano、他 → b_curve_zero）
    hammer_cutoff_low_hz: f32,
    hammer_cutoff_high_hz: f32,

    // === Phase 1〜4b 既存（現行 engine.rs のフィールド名を使用） ===
    pool: VoicePool<{ POLYPHONY }>,
    modal_body: ModalBodyResonator,
    sustain_state: SustainState,
    hold_stack: HoldStack,
    lfo: Lfo,                                       // Phase 4a
    mod_wheel: SmoothedValue,                       // Phase 4a (旧表記 `mod_wheel_value` ではない)
    current_instrument: InstrumentKind,             // Phase 4a (旧表記 `instrument_kind` ではない)
    current_damping: f32,                           // Phase 1
    output_gain: SmoothedValue,
    body_wet: SmoothedValue,
    channel_volume: SmoothedValue,
    lfo_pitch_depth: SmoothedValue,
    lfo_brightness_depth: SmoothedValue,
    lfo_volume_depth: SmoothedValue,
    voice_state_buffer: [u8; 33],
    voice_state_sample_counter: u32,
    mode: SynthMode,
    pick_position: f32,
    stereo_spread: f32,
    sample_rate: f32,
    // ... (他は Phase 4b と同じ engine.rs 既存フィールド)
}
```

> 注: 出力段の非線形は `crate::soft_clip::soft_clip(x: f32) -> f32` の **関数呼出**（Phase 3 既存、`SoftClip` 型のメソッドではない）。Engine が `soft_clip` フィールドを持つ仕様にはしない。

### 4.2 `apply_instrument()` の拡張

**現行 (Phase 4b) 実装** (`crates/dsp-core/src/engine.rs:314-326`) は楽器切替時に以下を実行する:
1. `pool.all_notes_off()` (Phase 4a D53 即時 release)
2. `hold_stack.clear()`
3. `sustain_state.reset()` ← **Sustain ペダルもリセット**
4. `current_instrument = kind` / `stereo_spread` / `modal_body` 切替
5. `pool.set_dispersion_active(matches!(kind, Piano))` (Phase 4b D67)

Phase 4c では **この既存挙動 (`sustain_state.reset()` も含む) を完全継承** する。`sustain_state.reset()` 後は当然 `is_active() == false` なので、bus の feedback_gain は **無条件で 0 にリセット** する（D77 / 設計上の整合）。

```rust
impl Engine {
    pub fn apply_instrument(&mut self, kind: InstrumentKind) {
        // === Phase 4a / 4b 既存（engine.rs:314-325 と同型、変更なし） ===
        self.pool.all_notes_off();
        self.hold_stack.clear();
        self.sustain_state.reset();              // ← 楽器切替時に Sustain もリセット
        self.current_instrument = kind;
        self.stereo_spread = stereo_spread_for_instrument(kind);
        self.modal_body.set_instrument(kind, self.sample_rate);

        let is_piano = matches!(kind, InstrumentKind::Piano);
        self.pool.set_dispersion_active(is_piano);  // Phase 4b D67

        // === Phase 4c 追加: Piano プリセット内パラメータの切替（D72 / D77 / D78） ===
        let (detune, sympathetic, b_curve, cutoff_low, cutoff_high) = if is_piano {
            (
                params::UNISON_DETUNE_CENTS_PIANO,
                params::SYMPATHETIC_AMOUNT_PIANO,
                b_curve_piano as fn(u8) -> f32,
                params::HAMMER_CUTOFF_LOW_PIANO,
                params::HAMMER_CUTOFF_HIGH_PIANO,
            )
        } else {
            (0.0, 0.0, b_curve_zero as fn(u8) -> f32, 0.0, 0.0)
        };
        self.unison_detune_cents = detune;
        self.sympathetic_amount = sympathetic;
        self.inharmonicity_b_for_note = b_curve;
        self.hammer_cutoff_low_hz = cutoff_low;
        self.hammer_cutoff_high_hz = cutoff_high;

        // 全 voice に楽器パラメータを fan-out。`inharmonicity_b` は note 依存のため
        // プレースホルダ 0 で OK（note_on 直前に VoicePool::note_on_with_piano_params が
        // 割当先 voice にだけ正しい LUT 値を再設定する）。
        self.pool.set_piano_params(detune, 0.0, cutoff_low, cutoff_high);

        // Sustain は `sustain_state.reset()` 済で必ず inactive、よって bus feedback_gain も
        // 無条件で 0 ターゲット。CC#64 ON が再送されると `handle_midi_cc(CC_SUSTAIN_PEDAL, ≥0.5)` 経由で
        // `sympathetic * FEEDBACK_GAIN_MAX` に動的に切替（§4.5）。
        self.resonance_bus.set_feedback_gain_target(0.0);

        // Phase 4c: 楽器切替で bus buffer / bus_out_prev も完全リセット。
        // §4.4 の bus_mix gate (feedback_gain=0 で 0) で出力寄与は止まるが、bus 内部の
        // delay line に残留した dry を切ることでデバッグ容易性と「楽器切替で全 voice 即時 release」
        // (Phase 4a D53) との一貫性を保つ。
        self.resonance_bus.reset();
        self.bus_out_prev = 0.0;
    }
}
```

> **重要**: 「楽器切替で Sustain ペダル状態を引き継ぐ」設計を採るなら `sustain_state.reset()` を含め設計再考が必要（CC#64 値の保持、bus feedback_gain の引継ぎ条件の見直し、Mono / Poly モードとの干渉）。Phase 4c では **Phase 4a D53 / 4b D67 を継承し、切替時は Sustain もリセット** という方針を採る（Phase 4d で必要なら再評価）。

### 4.3 `note_on()` の Multi-string + B(note) 連携（既存 Mono / Sustain ロジックを維持）

**現行 (Phase 4b) 実装** (`crates/dsp-core/src/engine.rs:129-147`) は以下の責務を持つ:
1. `sustain_state.clear_pending(midi_note)` (Phase 3 D40 P1-3、再打鍵時の pending 解消)
2. Mono mode: 直前 top の voice を release + `hold_stack.push_unique(midi_note)` (Phase 2 既存)
3. `trigger_voice(midi_note, velocity)` — `pool.note_on(midi, freq, velocity)` + `pool.set_damping_voice(assigned, current_damping)`

Phase 4c では **公開 `Engine::note_on` の構造を完全継承** し、内部の `trigger_voice` が呼ぶ `pool.note_on(...)` を **`pool.note_on_with_piano_params(...)` に差し替える** のみとする。`sustain_state.clear_pending` / Mono 分岐 / `hold_stack` 操作はすべて Phase 4b と同型。

```rust
impl Engine {
    /// Phase 4b と同型。Mono / Sustain ロジックは現行 engine.rs:129 をそのまま継承し、
    /// 末尾で trigger_voice を呼ぶ（trigger_voice の中身だけ Phase 4c で差し替え）。
    pub fn note_on(&mut self, midi_note: u8, velocity: f32) {
        // Phase 3 D40 P1-3: Sustain pending bit を再打鍵時に消す（Phase 4b 同型）
        self.sustain_state.clear_pending(midi_note);

        if matches!(self.mode, SynthMode::Mono) {
            // Phase 2 既存: 直前 top の release + push_unique
            if let Some(prev) = self.hold_stack.top() {
                if prev != midi_note {
                    self.pool.note_off(prev);
                }
            }
            self.hold_stack.push_unique(midi_note);
        }

        self.trigger_voice(midi_note, velocity);  // ← 中身が Phase 4c で差し替わる
    }

    /// Phase 4c: 内部の voice 割当 + 発音経路を VoicePool::note_on_with_piano_params に差し替え。
    /// Mono / Sustain / hold_stack 等の上位責務は `note_on()` 側で完結する。
    ///
    /// 現行 (Phase 4b) `trigger_voice` (engine.rs:122) は:
    ///   let freq = midi_to_freq(midi_note);
    ///   let assigned = self.pool.note_on(midi_note, freq, velocity);
    ///   self.pool.set_damping_voice(assigned, self.current_damping);
    ///
    /// Phase 4c では:
    ///   1) B(note) を関数ポインタ経由で lookup
    ///   2) freq を midi_to_freq + pitch_bend 反映で算出
    ///   3) pool.note_on_with_piano_params で Piano パラメータ込みの note_on
    ///   4) set_damping_voice で割当先 voice にだけ damping を適用（Phase 4b 同型）
    fn trigger_voice(&mut self, midi_note: u8, velocity: f32) {
        let inharmonicity_b = (self.inharmonicity_b_for_note)(midi_note);
        let freq = midi_to_freq(midi_note);

        let assigned = self.pool.note_on_with_piano_params(
            midi_note, freq, velocity,
            self.unison_detune_cents,
            inharmonicity_b,
            self.hammer_cutoff_low_hz,
            self.hammer_cutoff_high_hz,
        );

        // Phase 4b D11 / D33: 割当先 voice にだけ damping を適用（release 中 voice を巻き戻さない）
        self.pool.set_damping_voice(assigned, self.current_damping);
    }
}
```

> **note_off / Mono の note revive 経路への影響**: Phase 2 の Mono mode で `note_off` 時に新しい top に発音復帰する経路（`engine.rs:167-170`）も `trigger_voice(top, MONO_REVIVE_VELOCITY)` 経由で同じ Phase 4c 差し替えを受ける（Piano kind + Mono の組合せでも `n_strings(top_midi)` が正しく決定）。`note_off` 本体は Phase 4b と同型で変更なし。

### 4.4 `Engine::process(...)` の Sympathetic bus 統合（per-sample loop 内挿入、既存責務を完全維持）

**重要**: Engine は per-sample 関数ではなく **ブロック処理関数** `fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32])`（`crates/dsp-core/src/engine.rs:445`）。本仕様ではこの関数を新設せず、**既存の per-sample loop 内** に Phase 4c の経路を挿入する。voice_state 書き込みは loop 外（既存 `voice_state_sample_counter` の stride 経路）を維持。

VoicePool の `voices` は private（`crates/dsp-core/src/voice_pool.rs:8`）、また既存 `pool.process_sample()` は `poly_scale = 1/√N` を最後に掛ける（D20、`voice_pool.rs:149`）。Phase 4c では voice 配列に直接触らず、VoicePool に新メソッド `process_sample_with_feedback(bus_out_prev, feedback_gain) -> f32` を追加してこの両方を保つ（§5.3）。

#### Phase 4b 既存 `process` (engine.rs:445-492) との差分（Phase 4c で書き換える行のみ）

```rust
fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32]) {
    debug_assert_eq!(output_l.len(), output_r.len());
    let n = output_l.len();
    for i in 0..n {
        // Phase 4a 既存: LFO / Mod Wheel / pitch / brightness / volume 経路はそのまま
        let lfo_value = self.lfo.process_sample();
        let mod_wheel_v = self.mod_wheel.next_sample();

        let pitch_offset_semitones = lfo_value
            * self.lfo_pitch_depth.next_sample()
            * mod_wheel_v
            * LFO_PITCH_SCALE_SEMITONES;
        let pitch_factor = (-pitch_offset_semitones / 12.0).exp2();
        self.pool.set_lfo_pitch_factor(pitch_factor);

        let brightness_offset = lfo_value
            * self.lfo_brightness_depth.next_sample()
            * mod_wheel_v
            * LFO_BRIGHTNESS_SCALE;
        self.pool.set_lfo_brightness_offset(brightness_offset);

        let volume_multiplier = 1.0
            + lfo_value * self.lfo_volume_depth.next_sample() * mod_wheel_v * LFO_VOLUME_SCALE;

        // === Phase 4c 差分 (a): bus feedback_gain を進めて、voice 注入経路つきの sum を取得 ===
        let feedback_gain = self.resonance_bus.next_feedback_gain();
        let dry = self.pool.process_sample_with_feedback(self.bus_out_prev, feedback_gain);
        //    ↑ Phase 4b の `let dry = self.pool.process_sample();` をこの 1 行に置換。
        //      poly_scale = 1/√N も内部で適用済なので Phase 4b と返却値の意味は同型。

        // === Phase 4c 差分 (b): bus を進めて bus_out_prev を更新 ===
        // bus 自体は常に dry で駆動して状態を進めるが、出力への寄与は後段 (c) の bus_mix で gate する。
        let bus_out = self.resonance_bus.process(dry);
        self.bus_out_prev = bus_out;

        // === Phase 4c 差分 (c): bus_out の modal_body 入力ミックスを feedback_gain でゲート ===
        // bus_mix = feedback_gain / FEEDBACK_GAIN_MAX で 0..=1 に正規化された強度比を作る。
        // ・Default 等の非 Piano: handle_midi_cc / apply_instrument 経路で feedback_gain は常に 0 →
        //   bus_mix = 0 → bus_out * BUS_DIRECT_MIX_GAIN * bus_mix = 0 で modal_body 入力に寄与せず、
        //   Phase 4a HEAD / Phase 4b 7 楽器との byte 一致 (D83) を保証する。
        // ・Piano + Sustain OFF: feedback_gain.target=0 で SmoothedValue が 0 へ収束 → bus_mix も同期して 0 に減衰。
        // ・Piano + Sustain ON: feedback_gain = sympathetic_amount * FEEDBACK_GAIN_MAX →
        //   bus_mix = sympathetic_amount (≤ 1)、bus_out 寄与が滑らかに立ち上がる。
        let bus_mix = feedback_gain * (1.0 / FEEDBACK_GAIN_MAX);
        let (body_l, body_r) = self.modal_body.process_sample(dry + bus_out * BUS_DIRECT_MIX_GAIN * bus_mix);
        //    ↑ Phase 4b の `self.modal_body.process_sample(dry)` を上の式に変更。
        //      bus_mix=0 のとき `dry + 0.0` = `dry` で Phase 4b と完全に同型 (byte 一致)。

        // === Phase 4b 既存: bodyWet / channel_volume / LFO volume / soft_clip 経路を完全維持 ===
        let wet = self.body_wet.next_sample();
        let dry_amount = 1.0 - wet;
        let mixed_l = dry_amount * dry + wet * body_l;
        let mixed_r = dry_amount * dry + wet * body_r;
        let combined = self.output_gain.next_sample()
            * self.channel_volume.next_sample()
            * volume_multiplier;
        output_l[i] = soft_clip(mixed_l * combined);   // 関数呼出（SoftClip 型ではない）
        output_r[i] = soft_clip(mixed_r * combined);
    }

    // Phase 4b 既存: voice_state 書き込みの stride 制御も完全維持
    self.voice_state_sample_counter = self.voice_state_sample_counter.saturating_add(n as u32);
    if self.voice_state_sample_counter >= VOICE_STATE_WRITE_STRIDE {
        self.voice_state_sample_counter = 0;
        self.write_voice_state();
    }
}
```

Phase 4c で **書き換える行**（per-sample loop 内）:
- `dry = pool.process_sample()` → `dry = pool.process_sample_with_feedback(bus_out_prev, feedback_gain)`（feedback_gain 取得行込みで 2 行追加）
- `bus_out = resonance_bus.process(dry); bus_out_prev = bus_out;`（2 行追加）
- `modal_body.process_sample(dry)` → `let bus_mix = feedback_gain / FEEDBACK_GAIN_MAX; modal_body.process_sample(dry + bus_out * BUS_DIRECT_MIX_GAIN * bus_mix)`

**重要 (byte 一致の中核)**: bus 自体は dry で常に駆動されるが、出力への寄与は `bus_mix = feedback_gain / FEEDBACK_GAIN_MAX` で gate される。Default kind や Piano + Sustain OFF では `feedback_gain` が 0 (SmoothedValue で滑らかに 0 へ収束) → `bus_mix = 0` → modal_body 入力は `dry + 0.0` で **Phase 4b と完全に同型**。voice 側への `inject = bus_out_prev * feedback_gain` も `feedback_gain=0` で 0 になるため、Default kind の出力は Phase 4a HEAD と byte 一致継承される（D83）。bus 内部の状態だけは進行するが、`apply_instrument` / CC#123 で `resonance_bus.reset()` + `bus_out_prev = 0.0` を実行することで残留 bus を断つ（§4.2 / §4.5）。

それ以外（LFO / Mod Wheel / pitch_factor / brightness_offset / volume_multiplier / bodyWet / channel_volume / soft_clip 関数 / voice_state stride）は **Phase 4b 既存コードを完全維持**、これによりリグレッションリスクを最小化する。

### 4.5 Sustain（CC#64）経路の拡張（D77）

**重要**: 現行 (Phase 4b) は `Engine::set_sustain(on)` という独立メソッドを持たず、`handle_midi_cc(CC_SUSTAIN_PEDAL, v)` 内で完結している（`engine.rs:222-226`）。**`SustainState::set_active(on)` は pending release bitmap (`u128`) を返す関数** で、その戻り値を `Engine::release_pending(...)` に渡して保留 note_off を確定 release するパターンが Phase 3 D40 で確立されている（同 P1 系列の機能）。Phase 4c でこのパターンを **絶対に落とさない** こと（落とすと Sustain OFF 時に保留 note_off が解放されなくなる）。

Phase 4c では新メソッド `Engine::set_sustain` を導入せず、**既存 `handle_midi_cc(CC_SUSTAIN_PEDAL, v)` 経路を拡張** して bus feedback_gain も同時に切替える形を採る:

```rust
impl Engine {
    pub fn handle_midi_cc(&mut self, cc: u8, value_normalized: f32) {
        let v = value_normalized.clamp(0.0, 1.0);
        match cc {
            CC_MOD_WHEEL => { /* Phase 4a 既存、変更なし */ }
            CC_CHANNEL_VOLUME => { /* Phase 4a 既存、変更なし */ }
            CC_SUSTAIN_PEDAL => {
                let on = v >= 0.5;
                // === Phase 3 D40 既存 (engine.rs:224-225)、絶対に落とさない ===
                let released = self.sustain_state.set_active(on);
                self.release_pending(released);

                // === Phase 4c 追加 (D77): bus feedback_gain も同時に切替 ===
                let target_gain = if matches!(self.current_instrument, InstrumentKind::Piano) && on {
                    self.sympathetic_amount * FEEDBACK_GAIN_MAX
                } else {
                    0.0
                };
                self.resonance_bus.set_feedback_gain_target(target_gain);
            }
            CC_ALL_NOTES_OFF => {
                // Phase 4b 既存 (engine.rs:227-232) + Phase 4c 追加: bus 状態も完全リセット
                self.pool.all_notes_off();
                self.hold_stack.clear();
                self.sustain_state.reset();
                self.resonance_bus.set_feedback_gain_target(0.0);  // Phase 4c 追加
                self.resonance_bus.reset();                         // Phase 4c 追加（残留 bus を切る）
                self.bus_out_prev = 0.0;                            // Phase 4c 追加
            }
            _ => {}
        }
    }
}
```

> もし将来 Mono+Sustain 本実装などで `Engine::set_sustain(on)` を独立メソッドとして切り出すなら、戻り値経路 `let released = self.sustain_state.set_active(on); self.release_pending(released);` を **必ずペアで実装** する。Phase 4c では新メソッドを増やさず、CC#64 経路の拡張で完結させる方針。

### 4.6 `Engine::prepare` / `Engine::reset` の拡張

```rust
impl Engine {
    pub fn prepare(&mut self, sample_rate: f32) {
        // Phase 1〜4b 既存
        self.pool.prepare(sample_rate);
        self.modal_body.prepare(sample_rate);
        // ...

        // Phase 4c 追加
        self.resonance_bus.prepare(sample_rate);
        self.bus_out_prev = 0.0;
    }

    pub fn reset(&mut self) {
        // Phase 1〜4b 既存 (engine.rs:494-519 と同型)
        self.pool.reset();
        self.modal_body.reset();
        self.sustain_state.reset();
        self.hold_stack.clear();
        // ... (output_gain / body_wet / channel_volume / LFO / Mod Wheel / 楽器 reset)

        // Phase 4c 追加: bus も完全リセット
        self.resonance_bus.reset();
        self.bus_out_prev = 0.0;
    }
}
```

> 注: `Engine::reset` は C ABI `synth_reset` から呼ばれる（04 章）。`apply_instrument` / `CC#123` / `synth_reset` の 3 経路すべてで bus 状態を確実にクリアすることで、Default kind の byte 一致テスト (F61-a) を「実行順序によらず常に成立」させる。

## 5. `voice_pool.rs` の拡張

`VoicePool::voices` は **private 維持**（`crates/dsp-core/src/voice_pool.rs:8` 参照）。Phase 4c の Sympathetic / Multi-string 操作はすべて VoicePool 内に閉じたメソッドで実装し、Engine からは voice 配列に触らない。

### 5.1 楽器パラメータ fan-out

```rust
impl<const N: usize> VoicePool<N> {
    /// Phase 4c: 楽器切替時に全 voice の `set_instrument_params` を呼ぶ。
    /// 各 voice の `inharmonicity_b` は note_on 時に midi から再 lookup するため、
    /// ここでは 0 を渡す（dispersion_active=false なら影響ゼロ）。
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
}
```

### 5.2 Piano パラメータ込みの note_on 経路（既存 `note_on` を内部で再利用）

```rust
impl<const N: usize> VoicePool<N> {
    /// Phase 4c: Engine から呼ぶメイン経路。
    /// 1) 既存 `note_on` (`voice_pool.rs:47-60`) の 3 段フォールバック (same-note replace
    ///    / free voice / steal) で voice を割り当て
    /// 2) 割り当てた voice にだけ `set_instrument_params` で Piano パラメータ + B(note) を渡す
    ///    （他 voice は既に Engine::apply_instrument 時の値が保持されているため不要）
    /// 3) `note_on_with_id` で発音
    ///
    /// 戻り値は割当先 index（Phase 2 D13 既存 `note_on` と同じ）。
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
        // 既存ロジックと同じ 3 段フォールバック (same-note / free / steal) で voice を選ぶ
        let i = self.allocate_voice(midi_note);

        // 割当先のみ Piano パラメータを更新（midi 依存の inharmonicity_b は注 1 参照）
        self.voices[i].set_instrument_params(
            unison_detune_cents,
            inharmonicity_b,
            hammer_cutoff_low_hz,
            hammer_cutoff_high_hz,
        );
        self.voices[i].note_on_with_id(midi_note, freq_hz, velocity);
        i
    }

    /// 既存 `note_on(midi, freq, velocity)` (`voice_pool.rs:47-60`) の voice 割当ロジック
    /// （same-note replace / free voice / steal の 3 段フォールバック）を共通化するため
    /// プライベートヘルパへ分離。現行コードでは inline されているが Phase 4c で再利用する。
    fn allocate_voice(&mut self, midi_note: u8) -> usize {
        if let Some(i) = self.find_voice_index(midi_note) {
            return i;
        }
        if let Some(i) = self.voices.iter().position(|v| !v.is_active()) {
            return i;
        }
        select_voice_for_steal(&self.voices)
    }
}
```

> 注 1: Phase 4c の `inharmonicity_b` は MIDI ノート依存（B(note) LUT）であり、voice 割当時点で初めて確定する。Engine 側で `(self.inharmonicity_b_for_note)(midi_note)` を計算してから本メソッドに渡すことで、voice には正しい B 値が note_on 直前に到達する。

### 5.3 Sympathetic bus と連動した `process_sample_with_feedback`

```rust
impl<const N: usize> VoicePool<N> {
    /// Phase 4c: bus_out_prev × feedback_gain を各 voice に inject してから process_sample。
    /// 戻り値は Phase 2 D20 と同型の `sum * poly_scale`。
    ///
    /// feedback_gain = 0 のとき各 voice には 0 が inject され、Phase 4a / 4b の voice 出力と同型になる。
    /// これにより F65-a (`test_engine_inject_zero_when_feedback_gain_zero`) が機械保証される。
    #[inline(always)]
    pub fn process_sample_with_feedback(
        &mut self,
        bus_out_prev: f32,
        feedback_gain: f32,
    ) -> f32 {
        let inject = bus_out_prev * feedback_gain;
        let mut sum = 0.0_f32;
        for v in self.voices.iter_mut() {
            v.inject_feedback(inject);
            sum += v.process_sample();
        }
        sum * self.poly_scale  // Phase 2 D20 1/√N スケール維持
    }
}
```

Phase 4b 既存の `process_sample()` (sympathetic を使わない経路) は **削除せず残す**。Engine 側で kind に応じて呼び分けるのではなく、**常に `process_sample_with_feedback(bus_out_prev, feedback_gain)` を呼び、Default kind では `feedback_gain = 0` で動的に Phase 4b 経路と等価になる**設計（Phase 4a / 4b 互換性を機械保証）。Phase 4b 既存メソッドはテスト互換のため残置。

## 6. `params.rs` の生成内容（Phase 4c 拡張）

`scripts/gen-params.mjs` で出力する `crates/dsp-core/src/params.rs` の Phase 4c 追加部分:

```rust
// === Phase 4c で生成される const (auto-generated) ===

/// Piano の unison detuning 量 (cents、D72)
pub const UNISON_DETUNE_CENTS_PIANO: f32 = 1.5;

/// Piano の sympathetic resonance amount (0.0..=1.0、D77)
pub const SYMPATHETIC_AMOUNT_PIANO: f32 = 1.0;  // ペダル ON 時に full 効果

/// Hammer cutoff Hz の上下限（D75）
pub const HAMMER_CUTOFF_LOW_PIANO: f32 = 800.0;
pub const HAMMER_CUTOFF_HIGH_PIANO: f32 = 5500.0;  // Phase 4b の 4000 Hz から拡張

/// 88 鍵 × f32 LUT (A0=21 から C8=108、D78 / D79)
/// 値は Step 4 で概数を埋める、Step 18-19 で聴感調整
#[rustfmt::skip]
pub const INHARMONICITY_B_CURVE_PIANO: [f32; 88] = [
    // A0 (21) -> C8 (108)
    3.1e-4, 2.9e-4, 2.7e-4, 2.5e-4, 2.3e-4, 2.2e-4, 2.1e-4, 2.0e-4,  // A0..F1
    2.0e-4, 2.0e-4, 2.0e-4, 2.0e-4, 2.0e-4, 2.0e-4, 2.0e-4, 2.0e-4,  // F#1..C#3
    2.1e-4, 2.1e-4, 2.2e-4, 2.3e-4, 2.4e-4, 2.5e-4, 2.7e-4, 2.9e-4,  // D3..A3
    3.1e-4, 3.4e-4, 3.7e-4, 4.1e-4, 4.5e-4, 5.0e-4, 5.5e-4, 6.1e-4,  // A#3..F4
    6.7e-4, 7.5e-4, 8.3e-4, 9.2e-4, 1.0e-3, 1.1e-3, 1.3e-3, 1.4e-3,  // F#4..C#5
    1.6e-3, 1.8e-3, 2.0e-3, 2.3e-3, 2.6e-3, 2.9e-3, 3.2e-3, 3.6e-3,  // D5..A5
    4.0e-3, 4.5e-3, 5.0e-3, 5.6e-3, 6.3e-3, 7.0e-3, 7.9e-3, 8.8e-3,  // A#5..F6
    9.9e-3, 1.1e-2, 1.2e-2, 1.4e-2, 1.6e-2, 1.8e-2, 2.0e-2, 2.2e-2,  // F#6..C#7
    2.5e-2, 2.8e-2, 3.2e-2, 3.5e-2, 4.0e-2, 4.5e-2, 5.0e-2, 5.6e-2,  // D7..A7
    6.3e-2, 7.1e-2, 8.0e-2, 9.0e-2, 1.0e-1, 1.1e-1, 1.3e-1, 1.4e-1,  // A#7..F8 (B7 まで実 88、F#8 以降は range 外、padding)
    1.6e-1, 1.8e-1, 2.0e-1, 2.2e-1, 2.5e-1, 2.8e-1, 3.2e-1, 3.6e-1,  // padding 値、実用 88 = A0..C8 (21..=108) でしか参照されない
];

// 既存 Phase 4b const は維持（互換性）
pub const INHARMONICITY_B_PIANO: f32 = 7.5e-4;
```

## 7. テスト一覧（Phase 4c 新規、F-tag マスタは [`06-build-and-verify.md` §検証項目](./06-build-and-verify.md#検証項目f-tag) と一致）

Phase 4c の F-tag 採番は **06 章をマスタ** とし、本章のテストはその割当に従う。F59〜F70 の 12 件、サブタグ込みで 30 件超のテストを 3 つの新規ファイル + 既存ファイル拡張で実装する。

### 7.1 Multi-string テスト (`tests/multi_string_tests.rs`)

| F-tag | テスト名 | 内容 |
|---|---|---|
| F59-a | `test_n_strings_for_midi` | `n_strings(21)=1, n_strings(33)=1, n_strings(34)=2, n_strings(47)=2, n_strings(48)=3, n_strings(108)=3`、および **範囲外 `n_strings(20) = 1` / `n_strings(127) = 3` の clamp 動作**（§1.3 / §6.4 参照） |
| F59-b | `test_string_detune_cents_3_strings` | 3 弦時に `[0.0, -1.5, +1.5]` 返却 |
| F59-c | `test_string_detune_cents_2_strings` | 2 弦時に `[0.0, +1.5]` |
| F59-d | `test_string_detune_cents_1_string` | 1 弦時に `[0.0]` |
| F60-a | `test_piano_n_strings_3_at_c4` | Piano kind で C4 (60) note_on → `n_strings_active = 3` |
| F60-b | `test_piano_n_strings_2_at_b2` | Piano kind で B2 (47) note_on → `n_strings_active = 2` |
| F60-c | `test_piano_n_strings_1_at_a1` | Piano kind で A1 (33) note_on → `n_strings_active = 1` |
| F60-d | `test_default_kind_always_1_string` | Default kind で C4 note_on → `n_strings_active = 1` |
| F61-a | `test_default_n_strings_1_matches_phase4a` | Default kind で 256 frame × 2ch 出力が Phase 4a HEAD fixture と ε=1e-6 バイト一致（D83、Phase 4b 互換性継承） |
| F61-b | `test_piano_n_strings_diverges_from_phase4b_fixed_b` | Piano kind で出力が Phase 4b（固定 B=7.5e-4、n_strings=1）と意図的に異なる（負のテスト） |
| F61-c | `test_guitar_classical_phase4b_byte_match` | Guitar Classical kind で Phase 4b 出力と byte 一致（dispersion_active=false 経路） |
| F61-d | `test_all_non_piano_kinds_n_strings_1` | Default / Guitar / Ukulele / Mandolin / Bass / GuitarSteel / Sitar の 7 種で `n_strings_active = 1` 維持 |
| F61-e | `test_default_kind_bus_direct_mix_is_zero` | Default kind + CC#64 ON でも `bus_mix = feedback_gain / FEEDBACK_GAIN_MAX = 0` で modal_body 入力に bus が寄与せず、出力が Phase 4a HEAD fixture と byte 一致継続。Piano → Default 切替後に `resonance_bus.reset()` + `bus_out_prev = 0.0` で bus 残留も無いことを `engine.resonance_feedback_target_for_test() == 0` で確認 |
| F62-a | `test_string_detune_produces_beating` | 3 弦で 1.5 cents detune した出力に beating（振幅変調）が観測される |
| F62-b | `test_string_independent_dispersion_a1` | 各弦の dispersion a1 が detune で異なる f_0 を反映 |
| F62-c | `test_two_stage_decay_observation` | 1 秒持続音の前半 (0-200ms) と後半 (500-1000ms) で減衰率が異なる |
| F63 | `test_no_allocation_in_process_multi_string` | `process_sample` で N=3 弦 active 時に alloc ゼロ（Phase 1 D4 維持） |

### 7.2 Sympathetic resonance テスト (`tests/sympathetic_tests.rs`)

ResonanceBus は **process() が feedback_gain と独立** に lossy delay+LPF で bus_out を返す設計（§3.1 / §3.2）。voice への注入が無効化されるのは **`Engine` 経由で `bus_out × feedback_gain` を inject する経路に feedback_gain=0 が乗ったとき** であり、bus 単体テスト（F64）と Engine 統合テスト（F65）でテスト対象を明確に分ける。

| F-tag | テスト名 | 内容 |
|---|---|---|
| F64-a | `test_resonance_bus_process_returns_filtered_signal` | bus.process(impulse) は LPF + lossy delay 後の有限信号を返す（feedback_gain と独立、入力が非ゼロなら出力も非ゼロ） |
| F64-b | `test_resonance_bus_decay_after_impulse` | bus_in にインパルスを 1 sample 入れた後、ゼロ入力を継続すると数十 sample で振幅が `1e-6` 以下に減衰 |
| F64-c | `test_resonance_bus_stability_1024_samples` | feedback_gain は bus 内部とは無関係、`BUS_INTERNAL_DECAY = 0.95` で 1024 sample 連続インパルス入力しても max amplitude < 10.0 |
| F64-d | `test_resonance_bus_lpf_attenuation` | 4 kHz と 200 Hz の正弦波を bus.process に入れ、低域出力 / 高域出力比 > 2.0 |
| F65-a | `test_engine_inject_zero_when_feedback_gain_zero` | Default kind + Sustain ON で voice への `inject_feedback` 引数が 0（feedback_gain=0 で乗算結果が 0、§4.4 経路） |
| F65-b | `test_engine_sustain_on_activates_sympathetic_piano` | Piano kind + Sustain ON で `resonance_bus.next_feedback_gain()` が target に向けて > 0 に推移（SmoothedValue 経由） |
| F65-c | `test_engine_sustain_on_no_sympathetic_default` | Default kind + Sustain ON で `next_feedback_gain()` が 0 維持 |
| F65-d | `test_engine_sustain_off_zeroes_sympathetic` | Piano kind で Sustain OFF → 数 sample 後に `next_feedback_gain() ≈ 0` |
| F65-e | `test_engine_apply_instrument_resets_sympathetic` | Piano → Default 切替で `next_feedback_gain()` が 0 へ収束 |
| F65-f | `test_no_allocation_in_resonance_bus_process` | bus.process(0.5) で alloc ゼロ |
| F65-g | `test_engine_bus_mix_zero_when_feedback_gain_zero` | Default kind + Sustain ON、または Piano kind + Sustain OFF（SmoothedValue 収束後）で **`bus_mix = feedback_gain / FEEDBACK_GAIN_MAX = 0`**。modal_body 入力は `dry + bus_out * BUS_DIRECT_MIX_GAIN * 0 = dry` で Phase 4b 同型 (§4.4 / D83 補強) |
| F65-h | `test_engine_apply_instrument_resets_bus_buffer` | Piano kind で数 sample 発音させて bus に状態を蓄積 → `apply_instrument(Default)` 呼出 → `resonance_bus.reset()` + `bus_out_prev = 0.0` で bus buffer と前 sample 値がクリアされていることを `#[doc(hidden)]` accessor で観測（§4.2） |
| F65-i | `test_engine_all_notes_off_resets_bus_buffer` | 同上、`handle_midi_cc(CC_ALL_NOTES_OFF, 1.0)` でも bus が完全リセットされる（§4.5） |

### 7.3 Hertz hammer テスト (`tests/hammer_hertz_tests.rs`)

| F-tag | テスト名 | 内容 |
|---|---|---|
| F66-a | `test_hammer_t_c_decreases_with_velocity` | velocity=0.1 で t_c=3.72 ms, velocity=1.0 で t_c=1.2 ms（線形補間） |
| F66-b | `test_hammer_f_c_increases_with_velocity` | velocity=0.1 で f_c=1270 Hz, velocity=1.0 で f_c=5500 Hz |
| F66-c | `test_hammer_amplitude_sqrt_velocity` | velocity=0.25 で amp=0.5、velocity=1.0 で amp=1.0 |
| F66-d | `test_hammer_raised_cosine_shape` | buffer[0..t_c_samples] が sin² で形成、ピークは中央 (i = t_c/2 近傍) |
| F66-e | `test_hammer_velocity_affects_brightness` | velocity=0.1 と 1.0 で出力スペクトル centroid が顕著に異なる（centroid_v10 > centroid_v01 × 1.5） |
| F66-f | `test_hammer_pluck_path_for_default` | Default kind の note_on で pluck 経路（noise burst）が走る、hammer 経路は走らない |

### 7.4 B(note) LUT テスト (Multi-string テストファイルか hammer テストファイルに同居)

| F-tag | テスト名 | 内容 |
|---|---|---|
| F67-a | `test_b_curve_length_88` | `INHARMONICITY_B_CURVE_PIANO.len() == 88` |
| F67-b | `test_b_curve_lookup_a0` | `b_curve_piano(21) ≈ 3.1e-4`（A0、低音） |
| F67-c | `test_b_curve_lookup_a4` | `b_curve_piano(69) ≈ 7.5e-4`（A4 = MIDI 69、index = 48、Phase 4b 値と近似一致） |
| F67-d | `test_b_curve_lookup_c8` | `b_curve_piano(108) >= 0.05`（C8、高音） |
| F67-e | `test_b_curve_monotonic_increase_above_a3` | A3 (MIDI 57) 以上で LUT 値が単調増加 |
| F67-f | `test_b_curve_clamps_out_of_range` | `b_curve_piano(0) == LUT[0]`、`b_curve_piano(127) == LUT[87]`（MIDI clamp、§1.3 / §6.4） |
| F67-g | `test_b_curve_used_in_note_on_piano` | Piano kind の note_on で `set_instrument_params` 経由で渡される `inharmonicity_b` 引数が `b_curve_piano(midi)` と一致 |
| F67-h | `test_b_curve_not_used_for_default` | Default kind では `inharmonicity_b = 0`（b_curve 関数ポインタが zero 返却） |

### 7.5 `Engine::apply_instrument` 経路の Piano 内部状態検証 (`tests/instrument_tests.rs` 拡張、dsp-core 内部)

F68 は **dsp-core 内部の `Engine::apply_instrument(InstrumentKind::Piano)` 経路** を検証する（wasm-audio 側の `synth_apply_instrument(handle, 7)` は薄いラッパなので、内部状態の検証は dsp-core で完結させ、wasm-audio 側には Phase 4c で追加テストを置かない (D81)）。

#### Phase 4c で追加する test-only accessor (`#[doc(hidden)] pub fn ..._for_test()`)

現行 `voice_pool.rs:165-177` の `voice_index_for_note` / `voice` / `voice_length_int` パターンを踏襲し、Phase 4c では以下の internal accessor を **`#[doc(hidden)]` 付きで pub に公開** する。C ABI / 公開ドキュメントには露出しない（CLAUDE.md の制約と整合）。

| accessor | 場所 | 戻り値 | 用途 |
|---|---|---|---|
| `SustainState::is_active_for_test(&self) -> bool` | `sustain_state.rs` | `self.active` を読むだけ | F65-b/c/d / F68-c で sustain ON/OFF 状態を観測。現行は `pub active: bool` をフィールド公開しているが、`#[doc(hidden)]` 関数経由に統一して将来 active を private 化できる余地を残す |
| `ResonanceBus::feedback_gain_target_for_test(&self) -> f32` | `resonance_bus.rs` (新規) | `self.feedback_gain.target()` を返す | F65-a/b/c/d/e / F68-c で target を観測（`SmoothedValue::target()` を呼ぶ。SmoothedValue 側にも `#[doc(hidden)] pub fn target(&self) -> f32` がなければ追加） |
| `ResonanceBus::next_feedback_gain_for_test(&mut self) -> f32` | `resonance_bus.rs` (新規) | `self.feedback_gain.next_sample()` | F65-b で「数 sample 後に > 0」を確認するための per-sample 進行 |
| `VoicePool::voice_n_strings_active_for_test(&self, idx: usize) -> Option<usize>` | `voice_pool.rs` | `voices.get(idx).map(\|v\| v.n_strings_active())` | F68-a/b で割当 voice の弦数を観測 |
| `VoicePool::voice_inharmonicity_b_for_test(&self, idx: usize) -> Option<f32>` | `voice_pool.rs` | `voices.get(idx).map(\|v\| v.inharmonicity_b())` | F67-g / F68-a/b で `set_instrument_params` 後の B 値を観測 |
| `VoicePool::voice_unison_detune_cents_for_test(&self, idx: usize) -> Option<f32>` | `voice_pool.rs` | 同上 | F68-a/b で detune 値を観測 |
| `VoicePool::voice_dispersion_active_for_test(&self, idx: usize) -> Option<bool>` | `voice_pool.rs` | 同上 | F68-a/b で dispersion_active を観測 |
| `KarplusStrong::n_strings_active(&self) -> usize` | `karplus_strong.rs` | `self.n_strings_active` | 上記 VoicePool accessor のバックエンド。`#[doc(hidden)]` の getter として追加 |
| `KarplusStrong::inharmonicity_b(&self) -> f32` | 同上 | `self.inharmonicity_b` | 同上 |
| `KarplusStrong::unison_detune_cents(&self) -> f32` | 同上 | `self.unison_detune_cents` | 同上 |
| `KarplusStrong::is_dispersion_active(&self) -> bool` | 同上 | `self.dispersion_active` | 同上 |
| `Engine::sustain_active_for_test(&self) -> bool` | `engine.rs` | `self.sustain_state.is_active_for_test()` への薄いラッパ | F68-c で Engine 経由で簡潔にチェック |
| `Engine::resonance_feedback_target_for_test(&self) -> f32` | 同上 | `self.resonance_bus.feedback_gain_target_for_test()` | F65/F68 で Engine 経由でチェック |
| `Engine::voice_n_strings_active_for_test(&self, midi: u8) -> Option<usize>` | 同上 | `voice_index_for_note(midi)` で割当先 index を取得し、`pool.voice_n_strings_active_for_test(i)` | F68-a/b で「note_on(60) 後に割当 voice の n_strings_active = 3」を観測 |

これらはすべて **`#[doc(hidden)] pub fn ..._for_test`** とし、ドキュメント化されない一方で `crates/dsp-core/tests/` 配下から呼べる（Rust の private 可視性の制約上、`#[cfg(test)]` のみだと integration test から見えないため、`#[doc(hidden)]` で public + `for_test` suffix で意図を明示する Phase 4b 既存パターンを踏襲、`voice_pool.rs:165` の `voice_index_for_note` 参照）。

| F-tag | テスト名 | 内容 |
|---|---|---|
| F68-a | `test_apply_instrument_piano_activates_all_features` | `engine.apply_instrument(Piano)` → `engine.note_on(60, 0.8)` 後、`engine.voice_n_strings_active_for_test(60) == Some(3)`、割当 voice の `voice_inharmonicity_b_for_test ≈ b_curve_piano(60)`、`voice_unison_detune_cents_for_test == 1.5`、`voice_dispersion_active_for_test == Some(true)`（accessor は §7.5 の test-only リスト参照） |
| F68-b | `test_apply_instrument_default_deactivates_all_features` | `engine.apply_instrument(Default)` → `engine.note_on(60, 0.8)` 後、`voice_n_strings_active_for_test(60) == Some(1)` + `voice_dispersion_active_for_test == Some(false)` + `voice_inharmonicity_b_for_test == 0.0` + `voice_unison_detune_cents_for_test == 0.0` |
| F68-c | `test_apply_instrument_piano_resets_sustain_and_bus_gain` | `engine.handle_midi_cc(CC_SUSTAIN_PEDAL, 1.0)` → `engine.apply_instrument(Piano)` 後、`engine.sustain_active_for_test() == false` かつ `engine.resonance_feedback_target_for_test() == 0.0`（apply_instrument の `sustain_state.reset()` + `resonance_bus.set_feedback_gain_target(0.0)` 経路） |
| F68-d | `test_apply_instrument_piano_preset_byte_diverges_from_phase4b` | Piano kind 出力が Phase 4b と byte 不一致（F61-b と独立、別の波形条件で確認） |

## 8. 数値定数一覧（Phase 4c で追加）

| 定数 | 値 | 単位 | 由来 |
|---|---|---|---|
| `MAX_STRINGS_PER_VOICE` | 3 | — | D69 (Steinway D 標準) |
| `UNISON_DETUNE_CENTS_PIANO` | 1.5 | cents | D72 (Weinreich 典型値) |
| `SYMPATHETIC_AMOUNT_PIANO` | 1.0 | (0.0..=1.0) | D77 (ペダル ON で full 効果、内部で × 0.05 = 最大 feedback_gain) |
| `HAMMER_CUTOFF_LOW_PIANO` | 800.0 | Hz | D75 (Phase 4b 同値) |
| `HAMMER_CUTOFF_HIGH_PIANO` | 5500.0 | Hz | D75 (Phase 4b の 4000 → 5500 拡張) |
| `BUS_DELAY_MS` | 2.0 | ms | D76 (pre-research §5.3) |
| `BUS_INTERNAL_DECAY` | 0.95 | — | bus 内部の lossy feedback 係数 (pre-research §5.4) |
| `FEEDBACK_GAIN_MAX` | 0.05 | — | bus → voice の最大 feedback gain (pre-research §5.4) |
| `HAMMER_AMPLITUDE_GAMMA` | 0.5 (= velocity^0.5) | — | perceptual loudness 補正 (D75) |
| `HAMMER_T_C_BASE_MS` | 4.0 | ms | velocity=0 時の接触時間 (D75) |
| `HAMMER_T_C_SLOPE_MS` | 2.8 | ms/(velocity unit) | t_c の velocity slope |
| `HAMMER_F_C_BASE_HZ` | 800.0 | Hz | velocity=0 時の cutoff |
| `HAMMER_F_C_SLOPE_HZ` | 4700.0 | Hz/(velocity unit) | f_c の velocity slope |
| `INHARMONICITY_B_CURVE_PIANO[88]` | 概数（Step 4 確定） | — | D78、index = `midi.clamp(21, 108) - 21`、範囲外 MIDI は端値 fallback |
| `BUS_DIRECT_MIX_GAIN` | 0.5 | — | `Engine::process` の per-sample loop 内で bus_out を modal_body 入力に重ねる係数（§4.4） |
| `bus_mix` (動的) | `feedback_gain / FEEDBACK_GAIN_MAX` ∈ [0, 1] | — | bus_out の直接ミックスゲート係数（§4.4）。`feedback_gain` SmoothedValue に同期して滑らかに 0→sympathetic_amount まで上昇。Default kind や Piano + Sustain OFF では常に 0 で modal_body 入力に bus が寄与せず byte 一致を保証 |
| `FEEDBACK_GAIN_MAX` (再掲) | 0.05 | — | apply_instrument で `sympathetic_amount × FEEDBACK_GAIN_MAX` が target になる上限値 |

## 9. メモリ確保ゼロの保証

Phase 1 D4「`process` ホットパスでヒープ確保ゼロ」は Phase 4c でも完全維持:

1. `KarplusStrong::prepare` で `string_buffers` × 3 を `Vec::resize` で一括確保
2. `string_states: [StringState; 3]` は inline 配列で stack/struct 内、heap なし
3. `n_strings_active: usize` も inline、heap なし
4. `process_sample` 内では `for string_idx in 0..self.n_strings_active` のみ、新規 `Vec`/`Box`/`String` なし
5. `ResonanceBus::prepare` で `buffer` を `Vec::resize` で確保、process で alloc なし
6. テスト: `test_no_allocation_in_process_multi_string` (F63) と `test_no_allocation_in_resonance_bus_process` (F65-f) で機械保証

## 10. 既存 Phase 4b モジュールへの影響

| モジュール | Phase 4c での影響 |
|---|---|
| `lfo.rs` | 影響なし、Phase 4a 同等 |
| `loss_filter.rs` | 影響なし、Phase 3 同等 |
| `modal_body.rs` | Phase 4b 同等、Step 14 で M=16 採用時のみ M_MAX を 8 → 16 化 |
| `soft_clip.rs` | 影響なし、Phase 3 同等 |
| `sustain_state.rs` | `set_active` 拡張なし、Engine 側で resonance_bus を追加で操作 |
| `note_allocator.rs` | 影響なし、Phase 2 同等 |
| `hold_stack.rs` | 影響なし、Phase 2 同等 |
| `voice_state.rs` | 影響なし、Phase 3 同等 |
| `voice.rs` | Phase 4b 同等、`KarplusStrong` の note_on シグネチャ拡張に追随 |
| `traits.rs` | Voice trait に変更なし、`KarplusStrong::note_on` の引数増加は内部のみ |
| `fractional_delay.rs` | Phase 3 同等、`ThiranState` は弦個別になっただけで実装不変 |
| `dispersion.rs` | `compute_dispersion_a1` シグネチャ不変、呼出側で B 値を LUT 経由に変更 |

---

## まとめ

Phase 4c の dsp-core 仕様は **Multi-string per voice の追加 (1/2/3 弦並列、案 α + 案 1)**、**Hertz law raised cosine hammer（接触時間 + cutoff + amp パラメータ式）**、**Global sympathetic resonance bus（新規モジュール）**、**88 鍵 B(note) LUT** の 4 本柱で構成。Phase 4b の `KarplusStrong` / `Engine` / `dispersion.rs` / `params.rs` は拡張、`fractional_delay.rs` / `lfo.rs` / `loss_filter.rs` / `modal_body.rs` / `soft_clip.rs` は不変。新規モジュール `resonance_bus.rs` を追加、新規テストファイル 3 ファイル (multi_string / sympathetic / hammer_hertz) + 既存拡張で合計 Phase 4c 新規テスト ~30 件。`process` ホットパスの alloc ゼロ + Phase 4a HEAD byte 一致 (`n_strings = 1`) は完全継承（D83）。
