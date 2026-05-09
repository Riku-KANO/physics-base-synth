# 03. dsp-core 仕様（Phase 4b）

## 目的

`crates/dsp-core/` の Rust モジュール群に Phase 4b で追加する API / 内部状態 / テストを定義する。Phase 1 / 2 / 3 / 4a で確立した既存モジュールの責務（`KarplusStrong` / `VoicePool` / `Engine` / `ModalBodyResonator` / `LossFilter` / `SoftClip` / `SustainState` / `SmoothedValue` / `XorShift32` / `HoldStack` / `ParamDescriptor` / `FractionalDelay` (Thiran) / `VoiceState` / `Lfo`）はすべて維持し、本書では **Phase 4b で追加・変更する箇所のみ** 記述する。

## 他文書との関係

- 上流: [`pre-research.md`](./pre-research.md)（§2 物理モデル / §3 B 係数 / §4 Stretching all-pass / §5 KS 組込み / §6 Hammer model / §7 Piano Modal Body）、[`01-overview.md`](./01-overview.md)（D56-D67）、[`02-architecture.md`](./02-architecture.md)（dsp-core 層責務）
- 下流: [`04-wasm-audio-spec.md`](./04-wasm-audio-spec.md)（C ABI 値域拡張のみ）、[`07-implementation-checklist.md`](./07-implementation-checklist.md)
- 並行: Phase 4a [`03-dsp-core-spec.md`](../2026-05-08-004-phase4a/03-dsp-core-spec.md) — 既存 API スタイルの参照

## モジュール一覧（Phase 4b 後）

```
crates/dsp-core/src/
├── dispersion.rs           (Phase 4b 新規 — Stretching all-pass cascade D57/D59)
├── engine.rs               (Phase 4b で apply_instrument 末尾に set_dispersion_active 呼出を追加 D67。D63 の 5 ms fade-out 当初提案は SmoothedValue 同期 set_target の実現不能性により撤回し、Phase 4a D53 即時 release を継承)
├── fractional_delay.rs     (Phase 3 同等、変更なし)
├── hold_stack.rs           (Phase 2 同等、変更なし)
├── karplus_strong.rs       (Phase 4b で dispersion_stages / dispersion_active フィールド追加、note_on で hammer 経路分岐 + dispersion 係数算出、process_sample で cascade 適用)
├── lfo.rs                  (Phase 4a 同等、変更なし)
├── lib.rs                  (Phase 4b で `pub mod dispersion;` 追加)
├── loss_filter.rs          (Phase 3 同等、変更なし)
├── modal_body.rs           (Phase 4a 同等、Piano kind は params.rs の値で動作、コード変更なし)
├── note_allocator.rs       (Phase 2 同等、変更なし)
├── params.rs               (生成、Phase 4b で InstrumentKind::Piano=7 + BODY_MODES_PIANO_L/R + STEREO_SPREAD_PIANO + INHARMONICITY_B_PIANO + HAMMER_CUTOFF_LOW_PIANO + HAMMER_CUTOFF_HIGH_PIANO 出力)
├── rng.rs                  (Phase 1 同等、変更なし)
├── smoothing.rs            (Phase 3 同等、変更なし)
├── soft_clip.rs            (Phase 3 同等、変更なし)
├── sustain_state.rs        (Phase 3 同等、変更なし)
├── traits.rs               (Phase 4b で Voice trait に set_dispersion_active(bool) 追加)
├── voice.rs                (Phase 4b で set_dispersion_active 委譲を追加)
└── voice_pool.rs           (Phase 4b で set_dispersion_active(bool) を全 voice fan-out)
```

## Dispersion (`dispersion.rs`) — Phase 4b 新規（D57 / D58 / D59）

### 構造体定義

```rust
//! Stretching / Dispersion all-pass cascade (Phase 4b D57 / D58 / D59)
//!
//! ピアノ stiff string の inharmonicity (`f_n = n·f_0·√(1+B·n²)`) を、
//! M 段の 1 次 allpass cascade で再現する。係数 a1 は Rauhala-Välimäki 2006 の
//! closed-form 式（Faust `piano_dispersion_filter` の Rust 移植）で算出。
//!
//! 配置: `KarplusStrong::process_sample` の `buffer[read_z]` 値を 8 段に通してから
//! 既存 Thiran allpass に渡す。`KarplusStrong::note_on` で `compute_dispersion_a1`
//! を呼び、各 stage の a1 + 状態を初期化する。
//!
//! Phase 4a 互換性: `dispersion_active = false` の楽器（Default 〜 Sitar）では
//! `process_sample` で skip、CPU 影響ゼロ。`Engine::apply_instrument(Piano)` で
//! 全 voice に `set_dispersion_active(true)` を fan-out。

#![allow(clippy::approx_constant)]

/// Phase 4b D57: Dispersion all-pass の段数（M=8 固定、Faust 標準）。
/// 増減する場合は `KarplusStrong::dispersion_stages` の配列長と同期させること。
pub const DISPERSION_STAGES: usize = 8;

/// Phase 4b D59: Rauhala-Välimäki 2006 closed-form の magic constants。
/// 文献値、`approx_constant` lint は module-level allow で抑止。
const K1: f32 = -0.00179;
const K2: f32 = -0.0233;
const K3: f32 = -2.93;
const M1: f32 = 0.0126;
const M2: f32 = 0.0606;
const M3: f32 = -0.00825;
const M4: f32 = 1.97;

/// 1 段の dispersion allpass。`H(z) = (a1 + z⁻¹)/(1 + a1·z⁻¹)`。
/// `KarplusStrong::dispersion_stages: [DispersionStage; 8]` で inline 保持。
#[derive(Debug, Clone, Copy)]
pub struct DispersionStage {
    pub a1: f32,
    pub z1_in: f32,
    pub z1_out: f32,
}

impl DispersionStage {
    pub const fn new() -> Self {
        Self {
            a1: 0.0,
            z1_in: 0.0,
            z1_out: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.z1_in = 0.0;
        self.z1_out = 0.0;
    }

    /// 1 サンプル処理: `y = a1·x + z1_in - a1·z1_out`、状態更新。
    /// `KarplusStrong::process_sample` のホットパスで 8 段直列呼出される前提のため
    /// `#[inline(always)]` で関数呼出オーバーヘッドを除去。
    #[inline(always)]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.a1 * x + self.z1_in - self.a1 * self.z1_out;
        self.z1_in = x;
        self.z1_out = y;
        y
    }
}

impl Default for DispersionStage {
    fn default() -> Self {
        Self::new()
    }
}

/// Phase 4b D59: Rauhala-Välimäki 2006 closed-form で a1 + 群遅延を算出。
///
/// # 引数
/// - `m`: 段数（典型 8、`DISPERSION_STAGES`）
/// - `b`: inharmonicity coefficient（典型 1e-4〜1e-1、Phase 4b は Piano 固定 7.5e-4）
/// - `f0`: 基音周波数 (Hz)
/// - `fs`: サンプリングレート (Hz)
///
/// # 戻り値
/// - `(a1, group_delay_per_stage)`: a1 は各段共通、group_delay_per_stage は基音 f0 における 1 段の群遅延（sample 単位、`adjusted_length` 補正に使用）
///
/// # 数値安定性
/// - B → 0 で a1 → 0（allpass = passthrough）
/// - B 大 → a1 大、|a1| < 1.0 で極が単位円内
/// - 念のため `a1.clamp(-0.999, 0.999)` で安全側に制限（C8 / 高 B 値での発散防止）
pub fn compute_dispersion_a1(m: u32, b: f32, f0: f32, fs: f32) -> (f32, f32) {
    use core::f32::consts::PI;

    let m_f32 = m as f32;
    let trt = 2.0_f32.powf(1.0 / 12.0);
    let bc = b.max(1.0e-6);
    let log_bc = bc.ln();

    // 鍵盤位置 Ikey(f0) = log_(2^(1/12))(f0 · 2^(1/12) / 27.5)
    // A0 = 27.5 Hz を 0 とする半音単位インデックス（A4 = 48）
    let ikey = ((f0 * trt) / 27.5_f32).ln() / trt.ln();

    // kd = exp(k1 · log²(B) + k2 · log(B) + k3)
    let kd = (K1 * log_bc * log_bc + K2 * log_bc + K3).exp();

    // Cd = exp((m1 · log(M) + m2) · log(B) + m3 · log(M) + m4)
    let m_log = m_f32.ln();
    let cd = ((M1 * m_log + M2) * log_bc + M3 * m_log + M4).exp();

    // D = exp(Cd - Ikey · kd)
    let d = (cd - ikey * kd).exp();

    // a1 = (1 - D) / (1 + D)
    let a1 = ((1.0 - d) / (1.0 + d)).clamp(-0.999, 0.999);

    // 群遅延（基音 f0 における 1 段の delay）
    // polydel(a) = atan(sin(wT) / (a + cos(wT))) / wT
    let wt = 2.0 * PI * f0 / fs;
    let sin_wt = wt.sin();
    let cos_wt = wt.cos();
    let polydel = |a: f32| -> f32 { (sin_wt / (a + cos_wt)).atan() / wt };
    let group_delay_per_stage = polydel(a1) - polydel(1.0 / a1);

    (a1, group_delay_per_stage)
}
```

### Dispersion テスト方針

`crates/dsp-core/tests/dispersion_tests.rs` に新規追加:

| テスト名 | 検証内容 |
|---|---|
| `test_dispersion_a1_in_safe_range` | `compute_dispersion_a1(8, 7.5e-4, 440.0, 48000.0)` の `a1.abs() < 1.0`（極の単位円内安定性） |
| `test_dispersion_a1_increases_with_b` | B が小さい→大きいと `|a1|` が単調増加（B=1e-6 → 1e-2 で `|a1|` 増加） |
| `test_dispersion_b_zero_limit` | B=1e-6（事実上ゼロ）で `a1.abs() < 0.05`（passthrough 近似） |
| `test_dispersion_a1_keyboard_dependence` | 同 B での low note (A0, 27.5 Hz) と high note (C8, 4186 Hz) で a1 が異なる（`Ikey(f0)` 補正が効いている） |
| `test_dispersion_stage_reset` | `process` で状態更新後、`reset()` で z1_in = z1_out = 0 |
| `test_dispersion_stage_passthrough_when_a1_zero` | `a1 = 0.0` で `process(x) == x`（z1_out = 0 後の最初のサンプル）、続くサンプルでも DC でゲイン 1 |
| `test_dispersion_cascade_8_stages_stable` | M=8 段カスケード（同 a1）で 1024 サンプル走らせて出力が有限（NaN / Inf なし）、|y| < 100 |
| `test_dispersion_group_delay_positive` | A4 / B=7.5e-4 / M=8 で `group_delay_per_stage > 0`（dispersion で位相遅延が生じる） |

## InstrumentKind::Piano enum 拡張（D62）

`gen-params.mjs` 拡張で生成される params.rs に Piano kind を追加:

```rust
// 生成: params.rs（Phase 4a の 7 値 + Piano）
#[repr(u32)]
#[non_exhaustive]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum InstrumentKind {
    Default = 0,         // Phase 3 既存ギターボディ係数（後方互換）
    GuitarClassical = 1,
    Ukulele = 2,
    Mandolin = 3,
    Bass = 4,
    GuitarSteel = 5,
    Sitar = 6,
    Piano = 7,           // Phase 4b 新規
}

impl InstrumentKind {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Default),
            1 => Some(Self::GuitarClassical),
            2 => Some(Self::Ukulele),
            3 => Some(Self::Mandolin),
            4 => Some(Self::Bass),
            5 => Some(Self::GuitarSteel),
            6 => Some(Self::Sitar),
            7 => Some(Self::Piano),       // Phase 4b 新規
            _ => None,
        }
    }
}

pub const INSTRUMENT_KIND_COUNT: usize = 8;  // Phase 4a 7 → Phase 4b 8
```

## Piano Modal 係数定数（D62）

`gen-params.mjs` 拡張で生成、Phase 4a の 14 配列 + 7 stereo_spread に Piano の 2 配列 + 1 stereo_spread + 3 Piano 専用フィールドを追加:

```rust
// 生成: params.rs（pre-research §7.2 の文献値、Phase 4a と同形式の #[rustfmt::skip] 1 行）

#[rustfmt::skip]
pub const BODY_MODES_PIANO_L: [BodyMode; 8] = [
    BodyMode { freq: 55.0,   q: 10.0, gain: 1.0  },  // soundboard mode 1 (Conklin 1996, 49-60 Hz)
    BodyMode { freq: 110.0,  q: 12.0, gain: 0.85 },
    BodyMode { freq: 175.0,  q: 15.0, gain: 0.7  },
    BodyMode { freq: 280.0,  q: 18.0, gain: 0.55 },
    BodyMode { freq: 460.0,  q: 22.0, gain: 0.45 },
    BodyMode { freq: 750.0,  q: 28.0, gain: 0.35 },
    BodyMode { freq: 1300.0, q: 35.0, gain: 0.28 },
    BodyMode { freq: 2200.0, q: 40.0, gain: 0.22 },
];

#[rustfmt::skip]
pub const BODY_MODES_PIANO_R: [BodyMode; 8] = [
    /* applyStereoSpread(L, 0.05) で gen-params.mjs が生成 */
];

pub const STEREO_SPREAD_PIANO: f32 = 0.05;

// Phase 4b D58 / D61: Piano 専用フィールド
pub const INHARMONICITY_B_PIANO: f32 = 7.5e-4;       // A4 基準、Steinway B 計測値
pub const HAMMER_CUTOFF_LOW_PIANO: f32 = 800.0;      // velocity=0 での LPF cutoff (Hz)
pub const HAMMER_CUTOFF_HIGH_PIANO: f32 = 4000.0;    // velocity=1 での LPF cutoff (Hz)
```

`body_modes_for_instrument` / `stereo_spread_for_instrument` も Piano 分岐を追加:

```rust
#[rustfmt::skip]
pub fn body_modes_for_instrument(
    kind: InstrumentKind,
) -> (&'static [BodyMode; 8], &'static [BodyMode; 8]) {
    match kind {
        InstrumentKind::Default => (&BODY_MODES_DEFAULT_L, &BODY_MODES_DEFAULT_R),
        InstrumentKind::GuitarClassical => (&BODY_MODES_GUITAR_CLASSICAL_L, &BODY_MODES_GUITAR_CLASSICAL_R),
        InstrumentKind::Ukulele => (&BODY_MODES_UKULELE_L, &BODY_MODES_UKULELE_R),
        InstrumentKind::Mandolin => (&BODY_MODES_MANDOLIN_L, &BODY_MODES_MANDOLIN_R),
        InstrumentKind::Bass => (&BODY_MODES_BASS_L, &BODY_MODES_BASS_R),
        InstrumentKind::GuitarSteel => (&BODY_MODES_GUITAR_STEEL_L, &BODY_MODES_GUITAR_STEEL_R),
        InstrumentKind::Sitar => (&BODY_MODES_SITAR_L, &BODY_MODES_SITAR_R),
        InstrumentKind::Piano => (&BODY_MODES_PIANO_L, &BODY_MODES_PIANO_R),    // Phase 4b 新規
    }
}

pub fn stereo_spread_for_instrument(kind: InstrumentKind) -> f32 {
    match kind {
        InstrumentKind::Default => STEREO_SPREAD_DEFAULT,
        InstrumentKind::GuitarClassical => STEREO_SPREAD_GUITAR_CLASSICAL,
        InstrumentKind::Ukulele => STEREO_SPREAD_UKULELE,
        InstrumentKind::Mandolin => STEREO_SPREAD_MANDOLIN,
        InstrumentKind::Bass => STEREO_SPREAD_BASS,
        InstrumentKind::GuitarSteel => STEREO_SPREAD_GUITAR_STEEL,
        InstrumentKind::Sitar => STEREO_SPREAD_SITAR,
        InstrumentKind::Piano => STEREO_SPREAD_PIANO,                            // Phase 4b 新規
    }
}
```

## KarplusStrong の Phase 4b 拡張

### フィールド追加

```rust
use crate::dispersion::{compute_dispersion_a1, DispersionStage, DISPERSION_STAGES};
use crate::params::{
    INHARMONICITY_B_PIANO, HAMMER_CUTOFF_LOW_PIANO, HAMMER_CUTOFF_HIGH_PIANO,
    BRIGHTNESS_DEFAULT, DAMPING_DEFAULT, PICK_POSITION_DEFAULT,
};

pub struct KarplusStrong {
    // ...Phase 1〜4a 既存 fields...

    // Phase 4b D57 新規
    /// Piano kind での Stretching all-pass cascade（M=8 段、heap 確保ゼロ）。
    /// `dispersion_active = false` の楽器では `process_sample` で skip。
    /// 各段の a1 は `note_on` 時に Engine 側 (Piano プリセット) から渡された
    /// inharmonicity_b と freq_hz から `compute_dispersion_a1` で算出。
    dispersion_stages: [DispersionStage; DISPERSION_STAGES],
    /// `Engine::apply_instrument(Piano)` で true、他 7 楽器 (Default 含む) で false。
    /// `process_sample` ホットパスでは bool 1 つの分岐のみ、Phase 4a 互換性確保。
    dispersion_active: bool,
}

impl KarplusStrong {
    pub fn new() -> Self {
        Self {
            // ...Phase 1〜4a 既存...
            dispersion_stages: [DispersionStage::new(); DISPERSION_STAGES],
            dispersion_active: false,    // Phase 4a 互換性のためデフォルト false
        }
    }

    /// Phase 4b D67: 楽器切替で全 voice に dispersion_active を設定。
    /// `Engine::apply_instrument` から `pool.set_dispersion_active(active)` 経由で呼ばれる。
    /// flag の bool 切替のみで heap 操作なし、`apply_instrument` での alloc 0 保証。
    #[inline(always)]
    pub fn set_dispersion_active(&mut self, active: bool) {
        self.dispersion_active = active;
        if !active {
            // 念のため状態をクリア（次に Piano に切り替えたとき
            // 古い z1 が残らないよう）
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
}
```

### `note_on_internal` の拡張（D60 / D61）

```rust
fn note_on_internal(&mut self, note_id: Option<u8>, freq_hz: f32, velocity: f32) {
    let raw_len = self.sample_rate / freq_hz.max(1.0);
    let max_len_usize = self.buffer.len().saturating_sub(FRACTIONAL_DELAY_BUFFER_MARGIN);

    // Phase 3 D37 既存: Brightness LPF 群遅延補正
    let brightness = self.brightness.target();
    let brightness_tau_g = if brightness > 0.001 {
        ((1.0 - brightness) / brightness).clamp(0.0, raw_len - 3.0)
    } else {
        0.0
    };

    // Phase 4b D60 新規: Dispersion cascade の群遅延補正
    let dispersion_tau_g = if self.dispersion_active {
        let (a1, gd_per_stage) = compute_dispersion_a1(
            DISPERSION_STAGES as u32,
            INHARMONICITY_B_PIANO,
            freq_hz,
            self.sample_rate,
        );
        // 各段同一 a1 で初期化、状態クリア
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
    self.thiran.reset();
    self.loss_filter.set_for_frequency(freq_hz);

    // Phase 4b D61 新規: buffer 初期化を pluck / hammer で分岐
    if self.dispersion_active {
        // === Hammer 経路 (Piano kind) ===
        // 1) 単位 impulse を buffer[0] に配置、それ以外は 0
        for v in self.buffer.iter_mut() {
            *v = 0.0;
        }
        self.buffer[0] = velocity;

        // 2) Velocity-dependent 1pole IIR LPF を buffer[..len_int] に適用
        //    cutoff = HAMMER_CUTOFF_LOW + velocity * (HAMMER_CUTOFF_HIGH - HAMMER_CUTOFF_LOW)
        //    velocity=0.1: cutoff ≈ 1120 Hz (dim)
        //    velocity=1.0: cutoff = 4000 Hz (bright)
        let cutoff_hz = HAMMER_CUTOFF_LOW_PIANO
            + velocity.clamp(0.0, 1.0) * (HAMMER_CUTOFF_HIGH_PIANO - HAMMER_CUTOFF_LOW_PIANO);
        // 1pole IIR: y[n] = α·x[n] + (1-α)·y[n-1]、α = 1 - exp(-2π·fc / fs)
        let alpha = 1.0 - (-2.0 * core::f32::consts::PI * cutoff_hz / self.sample_rate).exp();
        let mut z = 0.0_f32;
        for i in 0..len_int {
            z = alpha * self.buffer[i] + (1.0 - alpha) * z;
            self.buffer[i] = z;
        }
        // Pick position は適用しない（hammer は固定位置、ピアノ物理として整合）
    } else {
        // === Pluck 経路 (Phase 1〜4a 既存、Default + 6 楽器) ===
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
```

### `process_sample` の拡張（D60）

Phase 4a 既存の per-sample 処理に dispersion cascade を 1 段追加:

```rust
#[inline(always)]
pub fn process_sample(&mut self) -> f32 {
    if !self.active {
        return 0.0;
    }

    let buf_len = self.buffer.len();

    // Phase 4a 既存: length 再計算 skip ロジック
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

    let read_z = (self.write_index + buf_len - self.length_int) % buf_len;

    // Phase 4b D60 新規: Dispersion cascade を Thiran の前段に挿入
    let read_value = if self.dispersion_active {
        let mut x = self.buffer[read_z];
        // 8 段直列、`#[inline(always)]` で関数呼出オーバーヘッドなし
        for stage in self.dispersion_stages.iter_mut() {
            x = stage.process(x);
        }
        self.thiran.process(x)
    } else {
        // Phase 1〜4a 既存経路（dispersion なし）
        self.thiran.process(self.buffer[read_z])
    };

    // Phase 4a 既存: brightness LPF + LFO offset 加算
    let b = (self.brightness.next_sample() + self.lfo_brightness_offset).clamp(0.0, 1.0);
    let filtered = b * read_value + (1.0 - b) * self.last_filter_out;
    self.last_filter_out = filtered;

    // Phase 3 既存: loss filter + damping + denormal flush + ring write
    let loss_out = self.loss_filter.process_sample(filtered);
    let d = self.damping.next_sample();
    let mut damped = d * loss_out;
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
```

### `reset` の拡張

```rust
pub fn reset(&mut self) {
    // ...Phase 1〜4a 既存...

    // Phase 4b 追加: dispersion 状態を初期化
    self.dispersion_active = false;  // Default 楽器に戻す前提
    for stage in self.dispersion_stages.iter_mut() {
        *stage = DispersionStage::new();
    }
}
```

## VoicePool / Voice trait の拡張（D67）

### `voice_pool.rs`

```rust
impl<const N: usize> VoicePool<N> {
    /// Phase 4b D67: 全 voice に dispersion_active を fan-out。
    /// `Engine::apply_instrument(Piano)` で true、他 7 楽器で false。
    pub fn set_dispersion_active(&mut self, active: bool) {
        for v in &mut self.voices {
            v.set_dispersion_active(active);
        }
    }
}
```

### `traits.rs` (Voice trait)

```rust
pub trait Voice {
    // ...Phase 1〜4a 既存メソッド...

    // Phase 4b D67 新規
    fn set_dispersion_active(&mut self, active: bool);
}
```

### `voice.rs` (KarplusStrong 向け委譲)

```rust
impl Voice for KarplusStrong {
    // ...Phase 1〜4a 既存実装...

    fn set_dispersion_active(&mut self, active: bool) {
        KarplusStrong::set_dispersion_active(self, active)
    }
}
```

### `note_allocator.rs` の MockVoice（test only）

`#[cfg(test)] mod` 内の `MockVoice` impl にも `set_dispersion_active` の空実装を追加（Phase 4a で `set_lfo_pitch_factor` 等を追加した経緯と同じ、E0046 not all trait items implemented を回避）:

```rust
// crates/dsp-core/src/note_allocator.rs（テスト内 MockVoice）
#[cfg(test)]
impl Voice for MockVoice {
    // ...Phase 4a 既存...
    fn set_dispersion_active(&mut self, _active: bool) {}
}
```

## Engine の Phase 4b 拡張（D63 改訂）

### `apply_instrument` は Phase 4a の即時 release を継承（D63 改訂後）

**仕様変更（指摘事項 #3）**: 当初 D63 で「5 ms fade-out」を提案していたが、`SmoothedValue::set_target` は target 代入のみで `current` は `next_sample()` でしか進まないため、同じ同期メソッド内で `set_target(0.0)` → `set_target(prev_value)` を実行しても **fade-out は発生しない**。状態機械（`PendingInstrumentChange`）を導入する案も検討したが、Phase 4b の主目的（ピアノ音色）に対する実装複雑度が大きいため、**D63 を改訂して Phase 4a D53「即時 release」を継承**する。fade-out / cross-fade は Phase 4c 送り。

Phase 4a 既存に **`pool.set_dispersion_active(piano)` 呼出のみ追加**:

```rust
impl Engine {
    /// Phase 4a D52 / D53 + Phase 4b D67 (改訂後 D63): 楽器プリセット切替。
    /// **Phase 4a D53 を継承**し、即時 `pool.all_notes_off()` で release（fade-out なし）。
    /// 演奏中の音切れは UI 側で「楽器切替時は音が切れます」を告知（Phase 4a と同じ）。
    /// Phase 4b 新規追加は `pool.set_dispersion_active(piano)` の 1 行のみ（D67）。
    /// fade-out / cross-fade は Phase 4c 以降の UX 改善で再評価。
    pub fn apply_instrument(&mut self, kind: InstrumentKind) {
        // Phase 4a 既存処理（即時 release、Phase 4b で変更なし）
        self.pool.all_notes_off();
        self.hold_stack.clear();
        self.sustain_state.reset();
        self.current_instrument = kind;
        self.stereo_spread = stereo_spread_for_instrument(kind);
        self.modal_body.set_instrument(kind, self.sample_rate);

        // Phase 4b D67 新規: dispersion_active を全 voice に fan-out
        let dispersion_active = matches!(kind, InstrumentKind::Piano);
        self.pool.set_dispersion_active(dispersion_active);
    }
}
```

**重要（D63 改訂後）**:
- **即時 release**: Phase 4a D53 を完全継承、Modal 係数差し替えと voice 内 KS は同 thread / 同 process call 内で同期実行。Phase 4a と同じ実装パターン
- **pop noise 軽減なし**: 楽器切替時の Body z1/z2 不連続による pop noise は Phase 4a と同レベルで残る。UX 改善（fade-out / cross-fade）は Phase 4c 以降
- **`pool.set_dispersion_active(piano)` の 1 行のみ追加**: 既存 7 楽器 → Piano への切替で `true`、Piano → Default 等で `false`。`matches!(kind, InstrumentKind::Piano)` で楽器に応じて自動切替、漏れなし

### `reset` の拡張

```rust
fn reset(&mut self) {
    // ...Phase 4a 既存...

    // Phase 4b 追加: dispersion_active も Default kind に戻る
    self.modal_body.set_instrument(InstrumentKind::Default, self.sample_rate);
    self.pool.set_dispersion_active(false);
}
```

## ModalBodyResonator は変更なし

Phase 4a 既存の `set_instrument(kind, sample_rate)` は、`body_modes_for_instrument(InstrumentKind::Piano)` で `BODY_MODES_PIANO_L/R` を返す（params.rs の `gen-params.mjs` 拡張で出力）ため、`modal_body.rs` 自体のコード変更は不要。`Engine::apply_instrument` から呼ばれる経路も Phase 4a と同一。

## 統合フロー（Engine::process per sample、Phase 4b 拡張版）

```text
Engine::process (per sample, n iterations):
  ┌─ Phase 4a 既存（D46-D49 LFO/Mod Wheel）─────────────┐
  │ 1. lfo_value = lfo.process_sample()                  │
  │ 2. mod_wheel_v = mod_wheel.next_sample()             │
  │ 3-4. Pitch destination: pool.set_lfo_pitch_factor   │
  │ 5-6. Brightness destination: pool.set_lfo_brightness_offset │
  │ 7. volume_multiplier = ...                           │
  └─────────────────────────────────────────────────────┘
  ┌─ Phase 4a 既存パス（KS + Body + dry/wet）──────────┐
  │ 8. dry = pool.process_sample()                       │
  │     ├─ for each voice (active):                       │
  │     │   ├─ KarplusStrong::process_sample():           │
  │     │   │   ├─ length_target.next_sample()            │
  │     │   │   ├─ effective_length = base × lfo_pitch_factor │
  │     │   │   ├─ if diff > 1e-5: 再計算 length_int      │
  │     │   │   ├─ read_z = (write_index + buf_len - length_int) % buf_len│
  │     │   │   │                                          │
  │     │   │   ├─ Phase 4b D60 新規: ────────────────┐ │
  │     │   │   │   if dispersion_active:                 │ │
  │     │   │   │     x = buffer[read_z]                  │ │
  │     │   │   │     for stage in dispersion_stages:    │ │
  │     │   │   │       x = stage.process(x) (8 段)       │ │
  │     │   │   │     read_value = thiran.process(x)      │ │
  │     │   │   │   else:                                  │ │
  │     │   │   │     read_value = thiran.process(buffer[read_z])│ │
  │     │   │   ├─────────────────────────────────────┘ │
  │     │   │   │                                          │
  │     │   │   ├─ brightness LPF (LFO offset 加算)       │
  │     │   │   ├─ loss filter                            │
  │     │   │   ├─ damping multiply                       │
  │     │   │   ├─ ring buffer write                      │
  │     │   │   └─ denormal flush                         │
  │     │   └─ amp scaling, energy update                  │
  │     └─ pool sum × 1/sqrt(N)                           │
  │ 9. (body_l, body_r) = modal_body.process_sample(dry) │
  │ 10. wet mix → mixed_l, mixed_r                       │
  │ 11. combined = output_gain × channel_volume ×        │
  │                volume_multiplier                      │
  │      ↑ Phase 4b: D63 改訂後 Phase 4a 同等            │
  │        (apply_instrument は即時 release、fade-out なし)│
  │ 12. soft_clip(mixed_l × combined) → output_l[i]      │
  │ 13. soft_clip(mixed_r × combined) → output_r[i]      │
  └─────────────────────────────────────────────────────┘
  ┌─ Phase 3 既存: Voice State stride push ────────────┐
  │ 14. voice_state_sample_counter += n                  │
  │ 15. if counter >= 1024: write_voice_state(); reset   │
  └─────────────────────────────────────────────────────┘
```

## ring buffer 不変条件（Phase 1〜4a から不変、Phase 4b でも厳守）

- `write_index = (write_index + 1) % buf_len`、`% length_int` ではない
- read 位置も `% buf_len` で計算
- `length_int` の動的変更（Pitch Bend + LFO Pitch + Phase 4b dispersion 群遅延補正で変動）でも buffer.len() は不変（`Engine::prepare` で確保された最大長を保持）

## テスト方針

### `crates/dsp-core/tests/dispersion_tests.rs` (新規、Dispersion 単体)

| テスト名 | 検証内容 |
|---|---|
| `test_dispersion_a1_in_safe_range` | M=8, B=7.5e-4, f0=440, fs=48000 で `a1.abs() < 1.0` |
| `test_dispersion_a1_increases_with_b` | B=1e-6 → 1e-2 で `a1` の絶対値増加 |
| `test_dispersion_b_zero_limit` | B=1e-6 で `a1.abs() < 0.05`（passthrough 近似） |
| `test_dispersion_a1_keyboard_dependence` | A0 (27.5) と C8 (4186) で a1 が異なる |
| `test_dispersion_stage_reset` | process 後の reset で z1_in = z1_out = 0 |
| `test_dispersion_stage_passthrough_when_a1_zero` | a1=0 で `process(x) == x`（z 状態 0 から） |
| `test_dispersion_cascade_8_stages_stable` | M=8 cascade を 1024 サンプル走らせて NaN/Inf なし、|y| < 100 |
| `test_dispersion_group_delay_positive` | A4 / B=7.5e-4 で `gd_per_stage > 0` |

### `crates/dsp-core/tests/instrument_tests.rs` (Phase 4a 既存に追加)

| テスト名 | 検証内容 |
|---|---|
| `test_apply_instrument_piano_enables_dispersion` | `apply_instrument(Piano)` 後、全 voice の `dispersion_active() == true` |
| `test_apply_instrument_default_disables_dispersion` | `apply_instrument(Piano)` → `apply_instrument(Default)` で全 voice の `dispersion_active() == false` |
| `test_piano_modal_coeffs_match_params` | `modal_body.coeff_l_b0(0)` が `BODY_MODES_PIANO_L[0]` ベースの計算値と一致 |
| `test_apply_instrument_piano_no_alloc` | `apply_instrument(Piano)` を 100 連打で WASM heap 不変（dispersion_stages は inline 配列） |

### `crates/dsp-core/tests/karplus_strong_dispersion_tests.rs` (新規)

| テスト名 | 検証内容 |
|---|---|
| `test_note_on_with_dispersion_active_uses_hammer_excitation` | `dispersion_active = true` 状態で note_on 後、buffer[0] が単位 impulse 由来（pluck noise burst でない、autocorr 確認） |
| `test_note_on_with_dispersion_inactive_uses_pluck_excitation` | `dispersion_active = false` で従来 pluck noise burst（Phase 4a 互換） |
| `test_dispersion_a1_set_in_note_on` | `note_on` 後、`dispersion_stage_a1(0)` が `compute_dispersion_a1(8, B, f0, fs).0` と一致 |
| `test_hammer_velocity_affects_brightness` | velocity=0.1 と velocity=1.0 で buffer の高域成分（前半 sample の RMS / 後半 RMS 比）が異なる |
| `test_dispersion_disabled_matches_phase4a` | **D67 互換性核心テスト**: 同じ条件 (Default kind / Mod Wheel=0 / LFO depth=0 / 全パラメータ Phase 4a 既定値) で 256 サンプル process した出力が、`dispersion_active = false` 経路で **Phase 4a と ε=1e-6 でバイト一致**（Phase 4a 出力は `karplus_strong.rs` の Phase 4a 版固定の golden 値 or 直接比較） |

### `crates/dsp-core/tests/no_alloc_tests.rs` (Phase 4a 既存に追加)

| テスト名 | 検証内容 |
|---|---|
| `test_no_allocation_with_piano_kind` | 8 voice + Piano kind active + LFO + Mod Wheel + Pitch Bend + 楽器切替 (Piano ↔ Default) で voice buffer / LFO 状態 / dispersion_stages capacity 不変 |

### `crates/dsp-core/tests/dsp_core_tests.rs` (Phase 4a 既存に追加、F50 = 旧 F46 拡張)

| テスト名 | 検証内容 |
|---|---|
| `test_engine_process_block_timing_phase4b_piano` | `#[cfg(not(debug_assertions))]` で release 限定、8 voice 全 active + Pitch Bend + CC#7 + Mod Wheel = 1.0 + LFO depths 全 1.0 + Piano kind の最悪ケース、1000 回 process の avg < 1.7 ms |
| `test_engine_process_block_timing_phase4b_other_instruments` | Piano 以外 (Default 含む 7 楽器) で avg < 1.0 ms（Phase 4a の 0.023 ms と同等、dispersion skip で CPU 影響ゼロ） |

### `crates/dsp-core/tests/modal_body_tests.rs` (Phase 4a 既存に追加)

| テスト名 | 検証内容 |
|---|---|
| `test_piano_modal_first_mode_at_55hz` | Piano kind の `BODY_MODES_PIANO_L[0].freq == 55.0` |
| `test_piano_stereo_spread_default` | `STEREO_SPREAD_PIANO == 0.05` |

## 性能考慮

### Dispersion cascade の per-sample コスト

`#[inline(always)]` 付き 1 段 allpass（Phase 4a Thiran と同型）:

| 演算 | 演算数 |
|---|---|
| 1 段 dispersion stage process | 4 演算 (a1·x + z1_in - a1·z1_out + 状態更新 2) |
| 8 段 cascade per voice | 32 演算 |
| 8 voice × 8 段 cascade | 256 演算/sample |
| 128 frames/process で | **+32768 演算/process** |

WASM 1 GHz 仮定で +32768 / 1e9 = **+0.0328 ms/process**。Phase 4a 実測 0.023 ms に加算で 0.056 ms（target 1.7 ms の 3.3%）。

**他楽器演奏時 (`dispersion_active = false`)**: per-voice の `if self.dispersion_active` 分岐 1 つのみ追加、加算演算 ~1 / sample × 8 voice × 128 = 1024 演算/process = +0.001 ms/process（誤差範囲）。

### Hammer LPF の per-event コスト

`note_on_internal` の 1pole IIR ループ:

| 演算 | 演算数 |
|---|---|
| `(-2π·fc/fs).exp()` | 7 演算 (1 回) |
| 1pole IIR per sample | 3 演算 (a·x + (1-a)·y_prev + 代入) |
| len_int 個 (典型 100〜500) | 300〜1500 演算 |
| **合計 per note_on** | **~700 演算（A4 freq @ 48kHz、len_int=109）** |

per-sample 換算で無視できる（note_on は連打でも数十 Hz、process 1024 sample stride の per-sample 換算で +1 演算未満）。

### `apply_instrument` の per-event コスト（D63 改訂後）

Phase 4a 既存処理 + dispersion 切替:

| 演算 | 演算数 |
|---|---|
| Phase 4a 既存 (`pool.all_notes_off` + biquad 16 係数 calc + reset) | ~600 演算 |
| Phase 4b 追加 (`pool.set_dispersion_active` × 8 voice) | 8 演算 (bool 代入のみ) |
| **合計 per event** | **~608 演算** |

per-sample 換算で無視できる（楽器切替は秒オーダーの操作）。**当初 D63 で `output_gain.set_target(0.0)` / 復帰の 4 演算を加えていたが、SmoothedValue 同期 set_target で fade-out が実現不能なため撤回（指摘事項 #3）。Phase 4b 改訂後は `output_gain` を触らない**。

## サブモジュール責務一覧

| ファイル | 責務 | Phase 4b 変更 |
|---|---|---|
| `dispersion.rs` | Stretching all-pass cascade（DispersionStage + closed-form 係数） | **新規** |
| `engine.rs` | Engine 全体 | `apply_instrument` 末尾に `pool.set_dispersion_active(matches!(kind, Piano))` を追加、`reset` で dispersion 初期化（Phase 4a D53 即時 release を継承、当初の D63 5 ms fade-out 提案は SmoothedValue 同期 set_target の実現不能性により撤回） |
| `voice_pool.rs` | 8 音 voice 管理 | `set_dispersion_active(active)` 追加 |
| `karplus_strong.rs` | KS 単音 | `dispersion_stages` / `dispersion_active` フィールド追加、`note_on` で hammer 経路 + dispersion a1 算出、`process_sample` で cascade 適用 |
| `voice.rs` | Voice trait 委譲 | `set_dispersion_active` 委譲 |
| `traits.rs` | Voice trait 定義 | `set_dispersion_active(bool)` 追加 |
| `note_allocator.rs` | MockVoice (test) | `set_dispersion_active` の空実装追加（trait 拡張対応） |
| `modal_body.rs` | Modal Body Resonator | **変更なし**（Phase 4a の `set_instrument(kind, sr)` が params.rs の Piano 値を自動取得） |
| `params.rs` (生成) | ParamDescriptor / BodyMode / 楽器係数 | `InstrumentKind::Piano = 7`、`BODY_MODES_PIANO_L/R`、`STEREO_SPREAD_PIANO`、`INHARMONICITY_B_PIANO`、`HAMMER_CUTOFF_LOW_PIANO`、`HAMMER_CUTOFF_HIGH_PIANO`、`body_modes_for_instrument` / `stereo_spread_for_instrument` の Piano 分岐 |
| `lib.rs` | モジュール公開 | `pub mod dispersion;` + `pub use dispersion::{DispersionStage, compute_dispersion_a1, DISPERSION_STAGES};` 追加 |
