# 03. Phase 3 dsp-core クレート仕様

## 目的

Phase 3 で `dsp-core` に追加する **4 モジュール**（`modal_body.rs` / `loss_filter.rs` / `sustain_state.rs` / `soft_clip.rs`、Pick position は専用モジュールなし・`KarplusStrong::note_on` 内の励振 shaping）と、既存モジュールへの変更（`fractional_delay.rs` への `ThiranCoeffs` 追加 + `FractionalDelay` enum での Lagrange/Thiran 統合、`voice.rs` の `Voice` trait に `set_pitch_bend` 1 メソッド追加（Mod Wheel `set_mod_depth` は Phase 4 送り）、`smoothing.rs` は **完全維持**（既存 `set_immediate` を流用、新メソッド追加なし）、`karplus_strong.rs` の Loss filter / Brightness 群遅延補正 / Pitch Bend / Pick position 励振 shaping 統合、`engine.rs` の Modal Body / Sustain / Voice State / MIDI CC dispatch (CC#7/#64/#123) / Channel Volume / Soft clip 統合、`voice_pool.rs` の Pitch Bend / Pick position fan-out）を定義する。Phase 1 / 2 で確立したリアルタイム制約とテスト方針は継承する。

## 他文書との関係

- 上流: [`02-architecture.md`](./02-architecture.md)（モノレポ構成、メモリレイアウト変更、ParamDescriptor codegen 拡張）
- 並列: [`04-wasm-audio-spec.md`](./04-wasm-audio-spec.md)（dsp-core を呼ぶ側の C ABI）
- 参考: [`pre-research.md`](./pre-research.md) §2〜§8（Modal Body / Extended KS / Thiran / brightness 補正 / MIDI CC / UI / soft clip）
- Phase 2 参照: [Phase 2 03 章](../2026-05-07-002-phase2/03-dsp-core-spec.md)（VoicePool / FractionalDelay (Lagrange) / NoteAllocator / HoldStack / SynthMode の既存 API、ParamDescriptor 構造、テスト方針）— **本書で明示的に変更しない部分はすべて Phase 2 の記述を継承**

## クレート設定

[Phase 2 03 章 §クレート設定](../2026-05-07-002-phase2/03-dsp-core-spec.md#クレート設定) を **完全維持**。`Cargo.toml` の依存ゼロ、`crate-type = ["rlib"]` は Phase 3 でも変更なし（D23 継承）。Phase 3 で追加する 4 モジュール（`modal_body` / `loss_filter` / `sustain_state` / `soft_clip`、Pick position は専用モジュールなし・`KarplusStrong::note_on` 内の励振 shaping）はすべて自前で実装し、`microfft` などの外部 crate を追加しない。

## モジュール一覧（Phase 3）

| ファイル | Phase 2 | Phase 3 |
|---|---|---|
| `lib.rs` | 11 モジュール宣言 | 4 モジュール宣言を追加（合計 15） |
| `traits.rs` | `AudioProcessor`、`Voice` (7 メソッド) | `Voice` trait に 1 メソッド追加（D39、`set_pitch_bend`、Mod Wheel は Phase 4 送り）|
| `params.rs` | コード生成出力 | **生成出力に Body Mode 関連が追加**（D32、`BODY_MODES_L/R` / `STEREO_SPREAD`） |
| `smoothing.rs` | `SmoothedValue` | **完全維持**（既存 `set_immediate(value)` を Pitch Bend SmoothedValue の note_on 時初期化に流用、新メソッド追加なし、`crates/dsp-core/src/smoothing.rs:20` 参照） |
| `rng.rs` | `XorShift32` | **完全維持** |
| `karplus_strong.rs` | KS Lagrange 統合 | Loss filter / Brightness 補正 / Pitch Bend / Pick position 励振 shaping 統合 |
| `voice.rs` | `Voice` trait の `KarplusStrong` 委譲 | `set_pitch_bend` / `set_pick_position` (inherent) の委譲を追記 |
| `engine.rs` | VoicePool / HoldStack / SynthMode | Modal Body / Sustain / Voice State / MIDI CC dispatch / Soft clip 統合 |
| `voice_pool.rs` | `VoicePool<8>` / voice stealing | Pitch Bend / Pick position fan-out、`voice_state(&self)` 公開 API 化（D41） |
| `fractional_delay.rs` | `LagrangeCoeffs` (`new` / `apply` / `Default`) | **`ThiranCoeffs` を追加** + **`LagrangeCoeffs::set_fractional(&mut self, d: f32)` を追加**（中身は `*self = Self::new(d)`、enum 経由で再計算するために必要）+ **`FractionalDelay` enum で Lagrange/Thiran を統合**（`set_fractional` / `apply` / `reset` / `new_lagrange` / `new_thiran`、D36 試作用） |
| `note_allocator.rs` | voice stealing 戦略 | **完全維持** |
| `hold_stack.rs` | `LinearStack<u8, 16>` | **完全維持** |
| **`modal_body.rs`** | — | **新規**: `ModalBodyResonator` (M=8 並列 bandpass biquad、stereo) |
| **`loss_filter.rs`** | — | **新規**: One-zero loss filter (D33) |
| **`sustain_state.rs`** | — | **新規**: Sustain Pedal 状態管理 (D40) |
| **`soft_clip.rs`** | — | **新規**: 区間関数型 soft clip (D43) |
| ~~`pick_position.rs`~~ | — | **作らない**（励振 shaping で実装、`KarplusStrong::note_on` 内、D34 設計変更版） |

### `lib.rs` の更新

```rust
pub mod engine;
pub mod fractional_delay;
pub mod hold_stack;
pub mod karplus_strong;
pub mod loss_filter;        // Phase 3 新規
pub mod modal_body;         // Phase 3 新規
pub mod note_allocator;
pub mod params;
pub mod rng;
pub mod smoothing;
pub mod soft_clip;          // Phase 3 新規
pub mod sustain_state;      // Phase 3 新規
pub mod traits;
pub mod voice;
pub mod voice_pool;
// Note: pick_position は専用モジュールではなく、KarplusStrong::note_on 内の励振 shaping として実装（D34）
```

## params.json の Phase 3 拡張

### 拡張内容（02 章 §params.json から再掲）

```json
{
  "params": [
    { "id": 0, "name": "Damping",      "min": 0.90, "max": 0.9999, "default": 0.996, "smoothing_tau": 0.02 },
    { "id": 1, "name": "Brightness",   "min": 0.0,  "max": 1.0,    "default": 0.5,   "smoothing_tau": 0.02 },
    { "id": 2, "name": "OutputGain",   "min": 0.0,  "max": 1.5,    "default": 0.8,   "smoothing_tau": 0.01 },
    { "id": 3, "name": "PickPosition", "min": 0.05, "max": 0.5,    "default": 0.125, "smoothing_tau": 0.05 },
    { "id": 4, "name": "BodyWet",      "min": 0.0,  "max": 1.0,    "default": 0.5,   "smoothing_tau": 0.02 }
  ],
  "body_modes": [
    { "freq": 105.0,  "q": 30.0, "gain": 1.0  },
    { "freq": 200.0,  "q": 25.0, "gain": 0.8  },
    { "freq": 280.0,  "q": 20.0, "gain": 0.5  },
    { "freq": 420.0,  "q": 35.0, "gain": 0.4  },
    { "freq": 580.0,  "q": 40.0, "gain": 0.35 },
    { "freq": 850.0,  "q": 45.0, "gain": 0.25 },
    { "freq": 1400.0, "q": 50.0, "gain": 0.2  },
    { "freq": 2300.0, "q": 60.0, "gain": 0.15 }
  ],
  "stereo_spread": 0.05
}
```

### `params.rs` 生成出力の Phase 3 追加分

Phase 2 の生成出力（[Phase 2 03 章 §params.json から params.rs への生成出力例](../2026-05-07-002-phase2/03-dsp-core-spec.md)）に以下を追加:

```rust
// AUTO-GENERATED FROM params.json — DO NOT EDIT

#[derive(Debug, Clone, Copy)]
pub struct BodyMode {
    pub freq: f32,
    pub q: f32,
    pub gain: f32,
}

pub const STEREO_SPREAD: f32 = 0.05;

pub const BODY_MODES_L: [BodyMode; 8] = [
    BodyMode { freq: 105.0,  q: 30.0, gain: 1.0  },
    BodyMode { freq: 200.0,  q: 25.0, gain: 0.8  },
    // ... 8 モード ...
];

pub const BODY_MODES_R: [BodyMode; 8] = [
    // 各モードの freq / q / gain を ±5% 揺らした値（gen-params.mjs の applyStereoSpread 出力）
    BodyMode { freq: 110.25, q: 28.5, gain: 1.05 },
    BodyMode { freq: 190.0,  q: 26.25, gain: 0.76 },
    // ...
];

// Phase 3 新規パラメータの descriptor も Phase 2 同様に追加
pub const PICK_POSITION_DESCRIPTOR: ParamDescriptor = ParamDescriptor {
    id: 3, name: "PickPosition", min: 0.05, max: 0.5, default: 0.125, smoothing_tau: 0.05,
};
pub const BODY_WET_DESCRIPTOR: ParamDescriptor = ParamDescriptor {
    id: 4, name: "BodyWet", min: 0.0, max: 1.0, default: 0.5, smoothing_tau: 0.02,
};

#[repr(u32)]
#[non_exhaustive]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ParamId {
    Damping = 0,
    Brightness = 1,
    OutputGain = 2,
    PickPosition = 3,    // Phase 3
    BodyWet = 4,         // Phase 3
}

pub const PARAM_DESCRIPTORS: [ParamDescriptor; 5] = [
    DAMPING_DESCRIPTOR,
    BRIGHTNESS_DESCRIPTOR,
    OUTPUT_GAIN_DESCRIPTOR,
    PICK_POSITION_DESCRIPTOR,
    BODY_WET_DESCRIPTOR,
];
```

### `applyStereoSpread` の式（`gen-params.mjs` の純粋関数）

```javascript
function applyStereoSpread(modes, spread) {
  return modes.map((m, i) => ({
    freq: m.freq * (i % 2 === 0 ? (1 + spread) : (1 - spread)),
    q:    m.q    * (i % 2 === 0 ? (1 - spread) : (1 + spread)),
    gain: m.gain * (1 + spread),  // 全モード一律 +5%
  }));
}
```

偶数 index と奇数 index で freq / q を反転させ、左右の chorus 的広がりを生成。`stereo_spread = 0.05` で控えめ、聴感上は 1 系統の Body と聞こえつつ widening が出る。

## 公開 trait の拡張

### `traits.rs` の Phase 3 版

```rust
pub trait AudioProcessor {
    fn prepare(&mut self, sample_rate: f32, max_block_size: usize);
    fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32]);
    fn reset(&mut self);
}

pub trait Voice {
    fn note_on(&mut self, freq_hz: f32, velocity: f32);
    fn note_off(&mut self);
    fn process_sample(&mut self) -> f32;
    fn is_active(&self) -> bool;
    fn note_id(&self) -> Option<u8>;     // Phase 2
    fn age(&self) -> u32;                 // Phase 2
    fn amplitude(&self) -> f32;           // Phase 2

    // Phase 3 で追加（D39）
    /// Pitch Bend をセット（半音単位、±2 まで）。SmoothedValue で 5 ms tau の遷移を内部で管理
    fn set_pitch_bend(&mut self, semitones: f32);
}
```

> `set_pitch_bend` は **全 active voice に fan-out**（VoicePool の責務）。Voice trait の対象は KarplusStrong 以外（Phase 4 で追加されうる ピアノ / ウクレレ等）でも同じシグネチャで透過的に扱える。
>
> **Mod Wheel (CC#1) は Phase 3 では非対応**: LFO の rate / 波形 / 配分 / 深さの仕様確定が Phase 3 スコープ外と判断、Phase 4 で `set_mod_depth` および LFO 仕様を併せて確定する。

## ModalBodyResonator (`modal_body.rs`)

### 役割（D30 / D31 / D32）

楽器ボディ共鳴を 8 つの並列 biquad（共鳴 IIR）で再現。`Engine::process` で `pool.process_sample()` 後・`output_gain` 前に挿入。stereo は左右で独立（係数は左右でわずかに異なる）。

### 構造体定義

```rust
use crate::params::{BodyMode, BODY_MODES_L, BODY_MODES_R};

const NUM_MODES: usize = 8;

#[derive(Debug, Clone, Copy)]
struct ModeCoeffs {
    b0: f32,
    b2: f32,    // bandpass: b1 = 0, b2 = -b0
    a1: f32,
    a2: f32,
}

#[derive(Debug, Clone, Copy)]
struct ModeState {
    z1: f32,
    z2: f32,
}

pub struct ModalBodyResonator {
    coeffs_l: [ModeCoeffs; NUM_MODES],
    coeffs_r: [ModeCoeffs; NUM_MODES],
    states_l: [ModeState; NUM_MODES],
    states_r: [ModeState; NUM_MODES],
    sample_rate: f32,
}

impl ModalBodyResonator {
    pub fn new() -> Self {
        const ZERO_C: ModeCoeffs = ModeCoeffs { b0: 0.0, b2: 0.0, a1: 0.0, a2: 0.0 };
        const ZERO_S: ModeState = ModeState { z1: 0.0, z2: 0.0 };
        Self {
            coeffs_l: [ZERO_C; NUM_MODES],
            coeffs_r: [ZERO_C; NUM_MODES],
            states_l: [ZERO_S; NUM_MODES],
            states_r: [ZERO_S; NUM_MODES],
            sample_rate: 48000.0,
        }
    }

    pub fn prepare(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        for i in 0..NUM_MODES {
            self.coeffs_l[i] = Self::calc_coeffs(BODY_MODES_L[i], sample_rate);
            self.coeffs_r[i] = Self::calc_coeffs(BODY_MODES_R[i], sample_rate);
        }
        self.reset();
    }

    pub fn reset(&mut self) {
        for i in 0..NUM_MODES {
            self.states_l[i] = ModeState { z1: 0.0, z2: 0.0 };
            self.states_r[i] = ModeState { z1: 0.0, z2: 0.0 };
        }
    }

    /// RBJ "Audio EQ Cookbook" の bandpass biquad（constant peak gain Q）形:
    ///   H(z) = (b0 + b2·z⁻²) / (1 + a1·z⁻¹ + a2·z⁻²)
    /// DC ゲイン = 0（body resonance に妥当）、ピーク at mode.freq でゲイン = mode.gain
    fn calc_coeffs(mode: BodyMode, sr: f32) -> ModeCoeffs {
        let omega = 2.0 * core::f32::consts::PI * mode.freq / sr;
        let cos_w = omega.cos();
        let sin_w = omega.sin();
        let alpha = sin_w / (2.0 * mode.q);
        let a0 = 1.0 + alpha;
        let inv_a0 = 1.0 / a0;
        ModeCoeffs {
            b0: alpha * mode.gain * inv_a0,
            b2: -alpha * mode.gain * inv_a0,
            a1: -2.0 * cos_w * inv_a0,
            a2: (1.0 - alpha) * inv_a0,
        }
    }

    /// Stereo process: 1 サンプル入力に対し左右 2 サンプル出力（並列加算、Direct Form II Transposed）
    #[inline(always)]
    pub fn process_sample(&mut self, x: f32) -> (f32, f32) {
        let mut y_l = 0.0_f32;
        let mut y_r = 0.0_f32;
        for i in 0..NUM_MODES {
            // 左 ch (DF-II Transposed: y = b0·x + z1; z1 = z2 - a1·y; z2 = b2·x - a2·y)
            let c = self.coeffs_l[i];
            let s = &mut self.states_l[i];
            let y = c.b0 * x + s.z1;
            s.z1 = s.z2 - c.a1 * y;
            s.z2 = c.b2 * x - c.a2 * y;
            y_l += y;

            // 右 ch
            let c = self.coeffs_r[i];
            let s = &mut self.states_r[i];
            let y = c.b0 * x + s.z1;
            s.z1 = s.z2 - c.a1 * y;
            s.z2 = c.b2 * x - c.a2 * y;
            y_r += y;
        }
        // denormal flush（R24 対策）
        (y_l + 1e-25 - 1e-25, y_r + 1e-25 - 1e-25)
    }
}

impl Default for ModalBodyResonator {
    fn default() -> Self { Self::new() }
}
```

### biquad 係数の根拠（bandpass、constant peak gain Q）

RBJ "Audio EQ Cookbook" の bandpass 形（peak gain = Q ではなく **peak gain = mode.gain** に正規化）:

```
ω = 2π · f / Fs
α = sin(ω) / (2Q)
a0_raw = 1 + α
a1_raw = -2·cos(ω)
a2_raw = 1 - α
b0_raw =  α · gain
b1_raw =  0
b2_raw = -α · gain
（全係数を a0_raw で正規化）
```

特性:
- **DC ゲイン**: `H(1) = (b0 + b2) / (1 + a1 + a2) = 0`（DC を通さない、body resonance に物理的に妥当）
- **Nyquist ゲイン**: `H(-1) = 0`
- **ピークゲイン**: `f = mode.freq` で `|H(e^(jω))| = mode.gain`（厳密、constant peak gain Q 形の特性）
- **−3dB 帯域幅**: `BW = freq / Q`

注意: 旧版仕様書の resonator `b0 = (1-a2)·gain` は DC/低域でゲインが膨張する（特に低 f・高 Q で顕著）誤りだったため、bandpass 形に置換した。Phase 1 [§3.3 Modal Synthesis](../2026-05-06-001-mvp/pre-research.md) の `exp(-decay·t) sin(2πft+φ)` の離散化は本来 bandpass 形と等価。

### テスト方針

テストは **(a) 単体 biquad の係数検証** と **(b) aggregate の ModalBodyResonator** に分け、隣接モードの寄与でピーク値が揺れるリスクを避ける。

#### (a) 単体 biquad のテスト（`tests/modal_body_biquad_tests.rs` 新規）

`ModeCoeffs::calc_coeffs(mode, sr)` の出力を**単一モード**で 1 段だけ走らせて係数仕様を保証する:

- `test_single_biquad_dc_blocking`: 1 段だけの biquad に DC 入力 1.0、定常出力 < 0.001（H(1) = 0 を直接検証）
- `test_single_biquad_peak_at_freq`: 1 段だけ走らせ、`f = mode.freq` の sin 入力（振幅 1.0）に対し定常出力 RMS が **`mode.gain / sqrt(2)` ± 5%**（ピーク正規化検証、隣接モードの干渉なし）
- `test_single_biquad_bandwidth`: -3 dB 帯域幅が概ね `freq / Q`（±20%、bandpass の選択性検証）

#### (b) aggregate ModalBodyResonator のテスト（`tests/modal_body_tests.rs` 新規）

複数モードを並列加算した後の挙動を**緩い許容範囲**で検証する:

- `test_modal_body_dc_blocking`: DC 入力 1.0 を 1 秒入力した後の定常出力 RMS < 0.001（全モードが H(1)=0 のため）
- `test_modal_body_peak_at_modes`: 各モード周波数の sin 入力に対し、定常出力 RMS がそのモードの `mode.gain` の **0.5 〜 1.5 倍**の範囲（隣接モードの寄与を見込んだ広い許容、aggregate 構造に対し brittle にしない）
- `test_modal_body_inter_mode_attenuation`: 全モード周波数の中点（例: 105Hz と 200Hz の中点 ~152Hz）で出力 RMS が **任意の `mode.gain` の最大値より低い**（モード間で減衰するという定性的性質のみ確認）
- `test_modal_body_stereo_spread`: 同入力に対し左右出力の RMS 差が 3〜10%（STEREO_SPREAD ±5% で偶奇 index 反転、許容 [3%, 10%]）
- `test_modal_body_no_alloc_in_process`: `prepare` 後の `process_sample` 1000 回呼び出しで length 不変
- `test_modal_body_reset_clears_state`: 励振後 reset で z1/z2 が 0 に戻る

> **テスト分離の理由**: ピーク正規化（mode.gain ± 5%）は単一 biquad で厳密に保証できるが、aggregate では隣接モードの寄与で ±50% 程度揺れる可能性がある（特に 105Hz と 200Hz のような近接モード）。aggregate テストでピーク値を厳密にチェックすると、係数初期値の微調整で頻繁に test fail する brittle なテストになる。単体テストで係数仕様を、aggregate テストで定性的性質を、それぞれ別の責務として検証する。

## LossFilter (`loss_filter.rs`)

### 役割（D33）

弦の周波数依存損失を `(1 + ρ·z⁻¹)/(1 + ρ)` の 1 段 FIR で再現。`KarplusStrong::process_sample` の brightness LPF 直後・damping 乗算前に挿入。

### 構造体定義

```rust
pub struct LossFilter {
    rho: f32,         // 0..0.5 推奨
    norm: f32,        // = 1.0 / (1.0 + rho)
    z1: f32,
}

impl LossFilter {
    pub const RHO_BASE: f32 = 0.05;

    pub fn new() -> Self {
        Self { rho: Self::RHO_BASE, norm: 1.0 / (1.0 + Self::RHO_BASE), z1: 0.0 }
    }

    /// note_on 時に呼ぶ。周波数依存式 ρ = ρ_base · clamp(freq/220, 0.5, 2.0)
    pub fn set_for_frequency(&mut self, freq_hz: f32) {
        let scale = (freq_hz / 220.0).clamp(0.5, 2.0);
        self.rho = (Self::RHO_BASE * scale).clamp(0.0, 0.5);
        self.norm = 1.0 / (1.0 + self.rho);
    }

    pub fn reset(&mut self) {
        self.z1 = 0.0;
    }

    #[inline(always)]
    pub fn process_sample(&mut self, x: f32) -> f32 {
        let y = (x + self.rho * self.z1) * self.norm;
        self.z1 = x;
        y
    }
}

impl Default for LossFilter {
    fn default() -> Self { Self::new() }
}
```

### テスト方針

- `test_loss_filter_dc_gain`: DC 入力 1.0 に対し定常出力が 1.0 ± 0.001（DC ゲイン保存）
- `test_loss_filter_nyquist_attenuation`: Nyquist 入力（交互 ±1.0）に対し定常出力 RMS が `(1-rho)/(1+rho)` に近いこと
- `test_loss_filter_high_freq_more_loss`: A4 (440Hz) と A6 (1760Hz) で `set_for_frequency` 後、ρ_A6 > ρ_A4 を確認

## Pick position（励振 shaping、専用モジュールなし）

### 役割（D34、設計変更版）

ピック位置 β の効果は **`note_on` 時の励振 shaping** で実装。`KarplusStrong::note_on` 内で生成した noise burst を `noise[n] − noise[n−K]` の comb 整形を経て delay buffer にロードする。物理的にはピック位置 β·L の節を持つ進行波の重ね合わせと等価で、Smith *PASP* "Plucked String" の標準的アプローチ。

**旧版仕様（`pick_position.rs` で feedback loop 内に 1-tap comb 挿入）からの変更理由**:
1. feedback loop 内の comb は **物理的な pick position ではなく、強い周波数依存 loss filter**として機能し、ループゲインと減衰特性を変える。loop gain 安定性議論を再開する必要があり、Step 1 の Thiran allpass 試作と干渉する
2. 出力経路に出すには PICK_DELAY_MAX = 1024 が必要（A1 + β=0.5 で K = 437、旧仕様 256 では A2 ですら届かない）。8 voice × 1024 × f32 = 32 KB のメモリ追加で予算超過リスク
3. 励振 shaping なら追加バッファ不要、process 内コスト 0、物理的に正統

### 実装イメージ（`KarplusStrong::note_on` 内）

```rust
pub fn note_on(&mut self, freq_hz: f32, velocity: f32) {
    // ... length / loss_filter / brightness 補正の計算（既存） ...

    let k = (self.pick_position * self.length_int as f32).round() as usize;
    let k = k.clamp(0, self.length_int.saturating_sub(1));

    // 1. buffer 全体ゼロクリア（Lagrange 補間余裕領域も含む）
    for s in self.buffer.iter_mut() { *s = 0.0; }

    // 2. noise burst を生成して `length_int` サンプル分一時保持
    //    - スタックの固定配列ではなく、buffer 自体を使うことで追加 alloc ゼロ
    //    - まず buffer の先頭 length_int に noise をロード
    for i in 0..self.length_int {
        self.buffer[i] = (self.rng.next_f32() - 0.5) * 2.0 * velocity;
    }

    // 3. comb 整形を後ろから適用（K だけ遡って差を取る、in-place）
    //    `buffer[i] -= buffer[i - K]` を i = length_int-1 から K まで降順に適用すれば、
    //    上書き元（buffer[i-K]）はまだ整形前の値が残っている
    if k > 0 {
        for i in (k..self.length_int).rev() {
            self.buffer[i] -= self.buffer[i - k];
        }
    }

    // 4. write_index = length_int から開始（既存の Phase 2 パターン継承）
    self.write_index = self.length_int;

    // 5. その他の励振後初期化（loss_filter / energy / age）
    self.loss_filter.reset();
    self.length_target.set_target(adjusted_length);
    self.age_samples = 0;
    // ...
}
```

### 仕様

- **K の範囲**: K = round(β · length_int)、`β ∈ [0.05, 0.5]`、length_int ∈ [約 11 (C8), 1746 (27.5Hz)]
  - 最小 K: β=0.05 + length_int=11 → K=1
  - 最大 K: β=0.5 + length_int=1746 → K=873（buffer 内に収まる、追加メモリ不要）
- **K=0 の扱い**: β·length_int < 0.5 でラウンド結果が 0 になる場合、comb shaping を skip（loss filter / damping のみで KS 動作）
- **動的更新**: `pick_position` パラメータは Engine が `f32` で保持（SmoothedValue 不要）。process 中の変更は **次回 `note_on` で反映**。連打すれば追従する
- **fractional β**: Phase 4 送り（連続変更を滑らかにするのは Phase 4）
- **process 内コスト**: 0（励振時のみ +length_int 演算）
- **メモリコスト**: 0（既存 buffer を使い回し）

### テスト方針

専用モジュールがないため、`tests/karplus_strong_pick_tests.rs`（新規）に統合:

- `test_pick_min_beta_minimal_shape`: β=0.05（公開 API の最小値、K=1 〜 数 sample）で comb shape の効果が **最小限**であることを確認（β=0.5 比でスペクトル変化が小さい、β=0.05 のテストとして外部から到達可能）
- `test_pick_position_node_at_beta_half`: β=0.5 で 2 倍音（偶数倍音の最低次）の振幅が他の倍音の 0.1 倍以下（FFT で確認）
- `test_pick_position_attenuates_kth_harmonic`: β=1/k（k=2,3,4）で k 番目倍音の振幅が顕著に低下
- `test_pick_position_no_extra_alloc`: `Engine::prepare` 後に β を変えて `note_on` 連打、buffer.len() 不変
- `test_pick_internal_k_zero_branch` (`#[cfg(test)]` 内部テスト or test-only constructor): 内部的に β·length_int < 0.5 で K = round → 0 となる境界の分岐パスをテスト。**Rust の `f32::round` は half-away-from-zero（0.5 → 1.0）** なので、K=0 を確実に踏ませるには `length_int ≤ 9 + β=0.05`（積 = 0.45 → round = 0）を使う。`length_int = 10 + β = 0.05` だと積 = 0.5 → round = 1 になり K=0 にならないので注意。**外部 API の β min は 0.05** で C8 (length_int ≈ 11) では K=1 が下限になるため K=0 分岐は通常実行で踏めない。テストは test-only API（`#[cfg(test)] fn note_on_with_length(length: usize, beta: f32)` 等）で `length_int = 9` + `β = 0.05` を直接設定し、K=0 分岐が panic なく入力素通しで抜けることを検証する。**外部 API 制約と内部分岐検証を分離**することで、テスト到達可能性と完全網羅を両立する

## SustainState (`sustain_state.rs`)

### 役割（D40）

CC#64 の状態を保持し、note_off 時に sustain 中なら release を保留する。

### 構造体定義

```rust
pub struct SustainState {
    pub active: bool,
    pending_release: u128,    // bit i = MIDI note i が pending release 中
}

impl SustainState {
    pub const fn new() -> Self {
        Self { active: false, pending_release: 0 }
    }

    pub fn set_active(&mut self, active: bool) -> u128 {
        let was_active = self.active;
        self.active = active;
        if was_active && !active {
            // sustain off の瞬間に pending を全 release、bitmap を返す
            let pending = self.pending_release;
            self.pending_release = 0;
            pending
        } else {
            0
        }
    }

    /// note_off を pending として記録。sustain 中なら true を返し呼び元は release を保留する
    pub fn try_defer_note_off(&mut self, midi_note: u8) -> bool {
        if self.active && midi_note < 128 {
            self.pending_release |= 1u128 << midi_note;
            true
        } else {
            false
        }
    }

    /// note_on 時に呼ぶ。同一ノートが pending release 中だった場合、bit をクリアする。
    /// シナリオ: C4 on → Sustain on → C4 off (pending bit 60 立つ) → C4 on (same-note-replace)
    /// で再励振 → CC#64 off で「再打鍵分まで release される」バグを防ぐ（再打鍵後にまだ離していないので、
    /// 古い pending が残っていると pedal off で誤って release されてしまう）。
    pub fn clear_pending(&mut self, midi_note: u8) {
        if midi_note < 128 {
            self.pending_release &= !(1u128 << midi_note);
        }
    }

    /// 現在の pending bitmap を返す（reset せず参照のみ）。
    /// `Engine::set_mode` で「mode 切替前に pending を取り出してから reset、各 note を pool.note_off」
    /// するパターンで使用（mode 切替時の pending 即時 release、D40 拡張）。
    pub fn pending_release_bitmap(&self) -> u128 {
        self.pending_release
    }

    pub fn reset(&mut self) {
        self.active = false;
        self.pending_release = 0;
    }
}

impl Default for SustainState {
    fn default() -> Self { Self::new() }
}
```

### テスト方針

- `test_sustain_defers_note_off`: active=true で `try_defer_note_off(60)` が true を返し、pending bit が立つ
- `test_sustain_release_on_off`: pending 複数件積んだあと `set_active(false)` で全 bit が返る
- `test_sustain_passthrough_when_inactive`: active=false で `try_defer_note_off` が常に false
- `test_sustain_clear_pending_on_retrigger`: pending bit 60 が立った状態で `clear_pending(60)` を呼ぶと bit が落ち、その後 `set_active(false)` で 60 が返らない（同一ノート再打鍵シナリオ）
- `test_sustain_reset_clears_active_and_pending`: `set_active(true)` + `try_defer_note_off(60)` 後に `reset()` を呼ぶと active=false / pending=0（CC#123 All Notes Off で sustain も clear する仕様）
- `test_sustain_pending_release_bitmap_readonly`: `pending_release_bitmap()` が pending を変更せず参照のみ返す（`Engine::set_mode` で mode 切替前に pending を取り出すための API）

## SoftClip (`soft_clip.rs`)

### 役割（D43）

**区間関数型 saturator**: 安全域 (|x| ≤ 0.95) は完全 linear（誤差ゼロ）、超過分を rational mapping で `[0, 0.05)` に圧縮し、|x| → ∞ で出力 ±1.0 に厳密漸近。`tanh` 近似（旧版）は |x| → ∞ で発散するため不採用。

### 構造体定義

```rust
const SOFT_CLIP_THRESHOLD: f32 = 0.95;
const SOFT_CLIP_RANGE: f32 = 0.05;  // = 1.0 - THRESHOLD（saturate 後の超過上限）

#[inline(always)]
pub fn soft_clip(x: f32) -> f32 {
    let abs_x = x.abs();
    if abs_x <= SOFT_CLIP_THRESHOLD {
        // 安全域は完全 linear（誤差ゼロ）
        x
    } else {
        // |x| > 0.95: 超過分 e ∈ (0, ∞) を rational mapping で [0, 0.05) に圧縮
        let e = abs_x - SOFT_CLIP_THRESHOLD;
        let compressed = SOFT_CLIP_RANGE * e / (e + SOFT_CLIP_RANGE);
        x.signum() * (SOFT_CLIP_THRESHOLD + compressed)
    }
}
```

### 関数の特性

- **|x| ≤ 0.95**: `soft_clip(x) ≡ x`（厳密一致、`assert_eq!` 可能）
- **|x| > 0.95**: `signum(x) · (0.95 + 0.05·e/(e+0.05))`、ここで `e = |x| − 0.95`
  - `e = 0` → 0（threshold で連続接続）
  - `e = 0.05`（|x|=1.0）→ 0.025、出力 0.975
  - `e = 0.55`（|x|=1.5）→ ≈0.046、出力 0.996
  - `e → ∞` → 0.05（厳密上限）、出力 ±1.0 に漸近
- **微分連続**: |x|=0.95 で左右ともに `dy/dx = 1`（kink なし）
- **計算量**: abs / 比較 / signum 各 1 + 算数 4-5 = 6-7 演算 / sample
- **依存ゼロ**: `f32::tanh` 不使用、Padé 近似不要

### テスト方針

- `test_soft_clip_linear_in_safe_range`: |x| ≤ 0.95 で `soft_clip(x) == x`（厳密一致、`assert_eq!`）
- `test_soft_clip_bounded`: 任意の x（±100, ±1e6 等）で `soft_clip(x).abs() < 1.0`（厳密有界）
- `test_soft_clip_continuous_at_threshold`: x = 0.95 ± 1e-6 で連続（出力差 < 1e-6）
- `test_soft_clip_extreme`: |x| = 1e6 で `0.99 < |y| < 1.0`
- `test_soft_clip_no_alloc`: stateless なので無条件、関数として呼べることのみ確認

## Fractional delay の Phase 3 拡張: `ThiranCoeffs`

### 既存 `LagrangeCoeffs` の維持

[Phase 2 03 章 §FractionalDelay](../2026-05-07-002-phase2/03-dsp-core-spec.md) の `LagrangeCoeffs` は **完全維持**。Step 1 で D36 の試作評価を行い、案 A 採用なら `KarplusStrong` の補間層を `ThiranCoeffs` に切替、案 C 維持なら現状継続。

### `ThiranCoeffs` の追加と `FractionalDelay` enum での統一

KarplusStrong は read 側で `self.fractional_delay.apply(buf_m, buf_z, buf_p1, buf_p2)`、書き込み側で `self.fractional_delay.set_fractional(d)` を呼ぶ。Step 1 試作中は両方を切り替えるため、共通 enum でラップする。

**Phase 3 で `LagrangeCoeffs` に `set_fractional(&mut self, d: f32)` メソッドを追加**（既存 API は `new(d)` と `apply(...)` のみだったため、enum 経由で再計算するために必要）:

```rust
impl LagrangeCoeffs {
    // 既存 (Phase 2): pub fn new(d: f32) -> Self { ... }
    // 既存 (Phase 2): pub fn apply(&self, x_minus, x_zero, x_plus_1, x_plus_2) -> f32 { ... }
    // 既存 (Phase 2): impl Default { fn default() -> Self { Self::new(0.0) } }

    /// Phase 3 追加: enum 経由で d を再設定する。中身は new(d) で全係数を再計算。
    /// `note_on` / Pitch Bend 中の length 再分解時に呼ばれる。
    pub fn set_fractional(&mut self, d: f32) {
        *self = Self::new(d);
    }
}
```

```rust
// crates/dsp-core/src/fractional_delay.rs

#[derive(Debug, Clone, Copy)]
pub struct ThiranCoeffs {
    pub a1: f32,
    z1_in: f32,
    z1_out: f32,
}

impl ThiranCoeffs {
    pub fn new() -> Self {
        Self { a1: 0.0, z1_in: 0.0, z1_out: 0.0 }
    }

    /// note_on 時に 1 度だけ呼ぶ。d=0 は a₁=1.0 で極が単位円上に来る境界ケース
    /// （FIR 1-tap delay と等価だが極零相殺で実装が壊れやすい）のため、
    /// d ∈ [1e-4, 0.999] に clamp（R25 / D36）。d=1e-4 → a₁ ≈ 0.9998、
    /// d=0.999 → a₁ ≈ 5e-4、いずれも極が単位円内で安定。
    pub fn set_fractional(&mut self, d: f32) {
        let d_safe = d.clamp(1e-4, 0.999);
        self.a1 = (1.0 - d_safe) / (1.0 + d_safe);
    }

    pub fn reset(&mut self) {
        self.z1_in = 0.0;
        self.z1_out = 0.0;
    }

    /// 整数 delay D_int サンプル後の値に対し allpass を通す
    #[inline(always)]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.a1 * x + self.z1_in - self.a1 * self.z1_out;
        self.z1_in = x;
        self.z1_out = y;
        y
    }
}

impl Default for ThiranCoeffs {
    fn default() -> Self { Self::new() }
}

/// Step 1 試作期間中は Lagrange / Thiran を切り替えるための enum でラップする。
/// D36 確定後（案 A 採用なら全 Thiran、案 C 維持なら全 Lagrange）に enum を解消し、
/// `fractional_delay: LagrangeCoeffs` または `fractional_delay: ThiranCoeffs` の
/// 単一型 field に直して enum dispatch のオーバーヘッドを除去する（実装 clean-up）。
#[derive(Debug, Clone, Copy)]
pub enum FractionalDelay {
    Lagrange(LagrangeCoeffs),
    Thiran(ThiranCoeffs),
}

impl FractionalDelay {
    /// note_on / Pitch Bend 中の length 再分解時に呼ぶ
    #[inline(always)]
    pub fn set_fractional(&mut self, d: f32) {
        match self {
            Self::Lagrange(l) => l.set_fractional(d),
            Self::Thiran(t) => t.set_fractional(d),
        }
    }

    /// process_sample から呼ぶ統一 read API。
    /// Lagrange は 4 点 FIR で全引数使用、Thiran は z のみ使用（m / p1 / p2 は無視）。
    /// 命名は KarplusStrong 既存実装の `read_m / read_z / read_p1 / read_p2` に対応:
    ///   x_m  = buffer[read_m]   (z+1、時間的に新しい側、x[n - D_int + 1])
    ///   x_z  = buffer[read_z]   (主読み取り位置、x[n - D_int])
    ///   x_p1 = buffer[read_p1]  (z-1、時間的に古い側、x[n - D_int - 1])
    ///   x_p2 = buffer[read_p2]  (p1-1、x[n - D_int - 2])
    #[inline(always)]
    pub fn apply(&mut self, x_m: f32, x_z: f32, x_p1: f32, x_p2: f32) -> f32 {
        match self {
            Self::Lagrange(l) => l.apply(x_m, x_z, x_p1, x_p2),
            Self::Thiran(t) => t.process(x_z),
        }
    }

    pub fn reset(&mut self) {
        match self {
            Self::Lagrange(_) => {}  // FIR、係数のみで状態なし
            Self::Thiran(t) => t.reset(),
        }
    }

    pub fn new_lagrange() -> Self {
        // 既存 LagrangeCoeffs::new(d: f32) は引数必須、Default::default() は new(0.0) と等価
        Self::Lagrange(LagrangeCoeffs::default())
    }
    pub fn new_thiran() -> Self { Self::Thiran(ThiranCoeffs::new()) }
}
```

KarplusStrong の field は `fractional_delay: FractionalDelay`（enum、Step 1 中）で統一。`Engine::new` / `Engine::new_with_thiran` test-only constructor で `FractionalDelay::new_lagrange()` または `new_thiran()` を選択し、Step 1 試作で両方を比較する（07 章 Step 1）。

### Step 1 試作の責務

`crates/dsp-core/tests/pitch_accuracy.rs` を拡張:

1. `measure_f0` 関数を再利用（Phase 2 既存）
2. `test_pitch_a1_thiran` / `test_pitch_a2_thiran` / `test_pitch_a4_thiran` / `test_pitch_c6_thiran` / `test_pitch_c8_thiran` を追加
3. `KarplusStrong` を一時的に Thiran 化するヘルパー（compile-time feature flag は使わず、test 内で別 builder を呼ぶ形）
4. 各テストで誤差を Phase 2 比で出力（`println!`）、A1〜C6 で誤差悪化が +0.1% を超えるかチェック
5. C8 で自己発振が成立すること（`measure_f0` が成功すること）を確認

試作結果に応じて 03 章本文の D36 を後追い更新する（仕様書改訂版）。

## KarplusStrong の Phase 3 拡張

### 統合フロー（`process_sample`）

Phase 2 の処理順序に **Loss filter / Brightness 補正** を統合（Pick position は note_on 励振 shaping で実装するため process_sample 内には現れない）。

**ring buffer の不変条件**（Phase 2 既存設計と同じ、Pitch Bend で `length_int` が動的になる Phase 3 でも維持）:

- `buf_len = self.buffer.len()` は `prepare` で確保した固定容量。`(sample_rate / 27.5).ceil() + LAGRANGE_BUFFER_MARGIN` で常に一定
- `length_int ∈ [3, buf_len - LAGRANGE_BUFFER_MARGIN]` は `note_on` / `set_pitch_bend` / SmoothedValue 遷移で動的に変動
- `write_index` は `(write_index + 1) % buf_len` で進む（**`length_int` で剰余を取らない**）
- read 位置は `(write_index + buf_len - length_int) % buf_len`（4 サンプル参照は -length_int + 1, -length_int, -length_int - 1, -length_int - 2 のオフセット）
- Pitch Bend で length_int が変わると read 位置が瞬時に変化するが、buffer の実体は連続的に更新されているため出力に大きな段差は出ない（SmoothedValue 5 ms tau で更に滑らか化）

```rust
// 既存 Phase 2 実装 `crates/dsp-core/src/karplus_strong.rs:163-211` をベースに、
// Pitch Bend 用の length 再分解（手順 0）と Loss filter（手順 3-）を加えた形

pub fn process_sample(&mut self) -> f32 {
    if !self.active { return 0.0; }

    // 0. Pitch Bend 中は length_target を更新（SmoothedValue 定常時は再計算 skip、R26 対策）
    let new_len = self.length_target.next_sample();
    if (new_len - self.cached_length).abs() > 1e-5 {
        let max_len = (self.buffer.len() - LAGRANGE_BUFFER_MARGIN) as f32;
        let clamped = new_len.clamp(3.0, max_len);
        self.length_int = clamped as usize;
        self.length_frac = clamped - self.length_int as f32;
        self.fractional_delay.set_fractional(self.length_frac);  // Lagrange or Thiran
        self.cached_length = new_len;
    }

    // 1. Lagrange 4 点読み取り（既存実装と完全一致、`% buf_len` で剰余）
    //    剰余を length_int で取ると read_p1/p2 がリング上の「新しい側」に
    //    巻き込まれ、x[n - D_int - 1] / x[n - D_int - 2] を取り出せない（D27）
    //    命名は既存実装に揃える: read_z (主), read_m (時間的に新しい z+1),
    //                            read_p1 (時間的に古い z-1), read_p2 (さらに古い p1-1)
    let buf_len = self.buffer.len();
    let read_z  = (self.write_index + buf_len - self.length_int) % buf_len;
    let read_m  = if read_z  + 1 == buf_len { 0 } else { read_z  + 1 };
    let read_p1 = if read_z      == 0       { buf_len - 1 } else { read_z  - 1 };
    let read_p2 = if read_p1     == 0       { buf_len - 1 } else { read_p1 - 1 };

    let read_value = self.fractional_delay.apply(
        self.buffer[read_m],
        self.buffer[read_z],
        self.buffer[read_p1],
        self.buffer[read_p2],
    );
    // 注: `fractional_delay` は `FractionalDelay` enum（Lagrange or Thiran）。
    //     Step 1 試作では `Engine::new` で `FractionalDelay::new_lagrange()`、
    //     `Engine::new_with_thiran` (test-only) で `FractionalDelay::new_thiran()` を選択。
    //     D36 確定後（案 A 採用なら全 Thiran、案 C 維持なら全 Lagrange）は enum を
    //     解消して単一型 field に置き換え、enum dispatch を除去する

    // 2. Brightness LPF（Phase 2 既存）
    let b = self.brightness.next_sample();
    let filtered = b * read_value + (1.0 - b) * self.last_filter_out;
    self.last_filter_out = filtered;

    // 3. Loss filter（Phase 3 新規 D33、one-zero `(1+ρ·z⁻¹)/(1+ρ)`）
    let loss_out = self.loss_filter.process_sample(filtered);

    // 4. Damping 乗算（Phase 2 既存）
    let d = self.damping.next_sample();
    let mut damped = d * loss_out;

    // 5. DC injection（Phase 1 D6 継承）
    damped += 1.0e-25;
    damped -= 1.0e-25;

    // 6. delay buffer write（既存パターン、`% buf_len` 不変条件）
    self.buffer[self.write_index] = damped;

    // 7. energy 追跡（Phase 2 既存）
    self.energy = self.energy * ENERGY_DECAY + damped * damped * ENERGY_RISE;
    if self.energy < ENERGY_THRESHOLD { self.active = false; }

    // 8. write_index 更新（**`% buf_len`**、`% length_int` ではない、分岐デクリメント）
    let next_write = self.write_index + 1;
    self.write_index = if next_write == buf_len { 0 } else { next_write };

    self.age_samples = self.age_samples.saturating_add(1);

    read_value  // フィルタ前の生の delay 出力（Phase 2 と同じ、出力経路に loss/damping を入れない）
}
```

> **重要**: Phase 2 の既存実装 `crates/dsp-core/src/karplus_strong.rs:163-211` を**そのまま継承**し、手順 0（Pitch Bend length 再分解）と手順 3（Loss filter）の 2 段だけを Phase 3 で追加する。read_m / read_p1 / read_p2 の命名と添字計算は既存と完全一致させること。`% length_int` には絶対に書き換えない（Pitch Bend で length_int が変わると剰余が動き、read 位置と write 位置が異なる剰余系で計算され buffer の論理長が破綻する）。

### Pitch Bend 統合（D39）

KarplusStrong に追加:

```rust
pub struct KarplusStrong {
    // 既存（Phase 2）
    buffer: Vec<f32>,
    write_index: usize,
    length_int: usize,
    length_frac: f32,
    brightness: SmoothedValue,
    damping: SmoothedValue,
    energy: f32,
    active: bool,
    note_id: Option<u8>,
    age_samples: u32,
    last_filter_out: f32,
    rng: XorShift32,
    sample_rate: f32,

    // Phase 2 で `lagrange: LagrangeCoeffs` だった field を Phase 3 で置換:
    //   `fractional_delay: FractionalDelay`（enum で Lagrange / Thiran を統一、D36 試作用）
    fractional_delay: FractionalDelay,

    // Phase 3 追加
    base_length: f32,            // Pitch Bend 0 のときの delay 長（adjusted_length）
    length_target: SmoothedValue, // Pitch Bend 適用後の target、SmoothedValue で 5ms 遷移
    base_freq: f32,              // note_on 時の周波数（Pitch Bend 計算の基準）
    cached_length: f32,          // process_sample 内 length 再計算の skip 判定用（R26 対策）
    pitch_bend_semitones: f32,
    loss_filter: LossFilter,
    pick_position: f32,          // β ∈ [0.05, 0.5]、Engine から fan-out で更新（次回 note_on で反映）
}

impl KarplusStrong {
    /// Phase 2 既存 API。`FractionalDelay::new_lagrange()` を内包して構築。
    /// VoicePool::new() から呼ばれるデフォルト経路。
    pub fn new() -> Self { /* ... fractional_delay: FractionalDelay::new_lagrange() ... */ }

    /// Phase 3 追加。Step 1 試作で Thiran 版 KarplusStrong を構築するための test-only constructor。
    /// `Engine::new_with_thiran()` → `VoicePool::new_with_fractional_delay(FractionalDelay::new_thiran)` →
    /// `KarplusStrong::new_with_fractional_delay(FractionalDelay::new_thiran())` の経路で各 voice に Thiran を注入する。
    /// D36 確定後（案 A 採用なら全 Thiran、案 C なら Lagrange 単一）に enum を解消する際にも使えるよう、
    /// signature は FractionalDelay を直接受け取る形にしておく（呼び元が `FractionalDelay::new_thiran()`
    /// または `FractionalDelay::new_lagrange()` を渡す）
    #[doc(hidden)]
    pub fn new_with_fractional_delay(fd: FractionalDelay) -> Self { /* ... fractional_delay: fd ... */ }
}

impl KarplusStrong {
    /// Phase 2 既存実装（`crates/dsp-core/src/karplus_strong.rs`）の `note_on_with_id`
    /// に対応する Phase 3 拡張版。VoicePool::note_on(midi_note, freq, velocity) から
    /// 直接呼ばれる経路で、note id を内部に保持する責務もここで持つ。
    ///
    /// 公開 API は `note_on(freq_hz, velocity)`（trait Voice 互換、`note_id = None`）と
    /// `note_on_with_id(midi_note, freq_hz, velocity)`（VoicePool 経由、`note_id = Some(midi_note)`）
    /// の 2 つで、内部実装は **共通ヘルパ `note_on_internal(note_id: Option<u8>, freq_hz, velocity)`** に集約する
    /// （Some(0) と None を取り違えるバグを設計レベルで排除、P1 対策）。
    pub fn note_on_with_id(&mut self, midi_note: u8, freq_hz: f32, velocity: f32) {
        self.note_on_internal(Some(midi_note), freq_hz, velocity);
    }

    /// trait `Voice` 互換用（midi_note 不明、`note_id = None` で励振）。Phase 2 既存 API。
    /// VoicePool は実装上 `note_on_with_id` を呼ぶため、この関数は外部 trait 経由でのみ使われる。
    pub fn note_on(&mut self, freq_hz: f32, velocity: f32) {
        self.note_on_internal(None, freq_hz, velocity);
    }

    /// 共通実装。`note_id` の取り扱いは引数の `Option<u8>` に従う（呼び元が `Some(0)` と
    /// `None` を意図的に区別できる、`note_on_with_id(0, ...)` で誤って `Some(0)` が
    /// 「note id 不明」を意味することを防ぐ）。
    fn note_on_internal(&mut self, note_id: Option<u8>, freq_hz: f32, velocity: f32) {
        self.base_freq = freq_hz;
        let raw_length = self.sample_rate / freq_hz;
        // Brightness 群遅延補正（D37）
        let brightness = self.brightness.target();  // Phase 2 既存 SmoothedValue の getter（karplus_strong.rs:21 / smoothing.rs:31）
        let tau_g = if brightness > 0.001 { (1.0 - brightness) / brightness } else { 0.0 };
        let max_len = (self.buffer.len() - LAGRANGE_BUFFER_MARGIN) as f32;
        let adjusted_length = (raw_length - tau_g).clamp(3.0, max_len);
        self.base_length = adjusted_length;
        self.length_target.set_immediate(adjusted_length);  // 既存 SmoothedValue::set_immediate を使用、target = current = value
        self.length_int = adjusted_length as usize;
        self.length_frac = adjusted_length.fract();
        self.cached_length = adjusted_length;

        // **必須**: fractional delay 係数を即座に再計算
        //   process_sample の手順 0 は cached_length 差分が < 1e-5 なら係数再計算を skip するため、
        //   note_on 直後の初回 sample で古い係数が使われないよう、ここで明示的に set_fractional する。
        //   Lagrange と Thiran どちらの実装でも note_on 必須の手順
        self.fractional_delay.set_fractional(self.length_frac);

        // Phase 3 統合: Loss filter は周波数依存式を再計算、内部状態 z1 はリセットしない
        // （note_on 連打で前 note の loss filter 状態を引き継いでも実害なし、過渡応答が
        //  自然に更新される。状態リセットしたい場合は self.loss_filter.reset() を併用）
        self.loss_filter.set_for_frequency(freq_hz);

        // Pick position 励振 shaping (D34)
        let k = (self.pick_position * self.length_int as f32).round() as usize;
        let k = k.min(self.length_int.saturating_sub(1));

        // 1. buffer 全体ゼロクリア
        for s in self.buffer.iter_mut() { *s = 0.0; }

        // 2. noise burst を buffer の先頭 length_int サンプルにロード
        for i in 0..self.length_int {
            self.buffer[i] = (self.rng.next_f32() - 0.5) * 2.0 * velocity;
        }

        // 3. comb 整形を後ろから in-place 適用（K=0 ならスキップ）
        if k > 0 {
            for i in (k..self.length_int).rev() {
                self.buffer[i] -= self.buffer[i - k];
            }
        }

        // 4. write_index を length_int から開始（既存パターン、Phase 2 D27 継承）
        self.write_index = self.length_int;

        // 5. Voice の状態リセット
        self.energy = INITIAL_ENERGY;        // active 判定用
        self.active = true;
        self.age_samples = 0;
        self.note_id = note_id;              // ← 引数の Option<u8> をそのまま保持。Some(0) と None が区別される
        self.last_filter_out = 0.0;          // brightness LPF 状態
        // 注: fractional_delay の内部状態（Thiran allpass の z1_in / z1_out 等）は
        //     note_on 時に reset しない。前 note の状態を引き継いでも自然な過渡応答になる。
        //     ただし `synth_reset` 経由の場合は別途 reset を呼ぶ
    }

    // 注: 旧仕様で公開 `note_on` の二重定義があったが、上記 `pub fn note_on(freq_hz, velocity)`
    //     が共通ヘルパ `note_on_internal(None, ...)` を呼ぶ形に集約済み（P1 対策）。
    //     `note_on_with_id(0, ...)` を呼んで後から `note_id = None` で上書きする実装は
    //     Some(0) と None を混同するバグの温床なので採用しない。

    pub fn set_pitch_bend(&mut self, semitones: f32) {
        self.pitch_bend_semitones = semitones;
        let bent_freq = self.base_freq * 2.0_f32.powf(semitones / 12.0);
        let raw_length = self.sample_rate / bent_freq;
        let brightness = self.brightness.target();  // Phase 2 既存 SmoothedValue の getter（karplus_strong.rs:21 / smoothing.rs:31）
        let tau_g = if brightness > 0.001 { (1.0 - brightness) / brightness } else { 0.0 };
        let max_len = (self.buffer.len() - LAGRANGE_BUFFER_MARGIN) as f32;
        let adjusted_length = (raw_length - tau_g).clamp(3.0, max_len);
        // SmoothedValue で 5 ms tau の遷移、process_sample の手順 0 で
        //   cached_length との差分 > 1e-5 を検知して fractional_delay.set_fractional を呼ぶ
        //   set_pitch_bend 自体は SmoothedValue の target を更新するだけで係数は触らない
        self.length_target.set_target(adjusted_length);
    }

    /// Engine が pick_position パラメータ変更時に全 voice に push（次回 note_on で反映）
    pub fn set_pick_position(&mut self, beta: f32) {
        self.pick_position = beta.clamp(0.05, 0.5);
    }
}
```

### `process_sample` 内での length 動的更新

Pitch Bend が SmoothedValue で 5 ms 遷移するため、`process_sample` の **手順 0**（前掲 §統合フロー）で `length_target.next_sample()` を呼んで cached_length と差分を取り、`> 1e-5` の場合のみ `length_int` / `length_frac` を再分解 + `self.fractional_delay.set_fractional(self.length_frac)` で係数再計算する（R26 対策、定常時は skip）。詳細実装は §統合フロー の擬似コード参照（フィールド名は `self.fractional_delay`、別名 `fractional_delay_coeffs` ではない）。

**コスト**: SmoothedValue.next_sample = 3 演算、cached_length 差分判定 = 1 演算、再計算分岐内で length_int/frac 分解 = 2 演算 + Lagrange 係数再計算 = 12 演算 / Thiran 係数再計算 = 3 演算。Phase 2 の note_on 1 度計算（D26）から「Pitch Bend 中のみ毎サンプル再計算」に拡張。定常時は cached_length 一致で再計算 skip、Lagrange/Thiran 共通でホットパス影響軽微。

### Brightness 群遅延補正（D37）

`note_on` / `set_pitch_bend` のいずれでも `tau_g(brightness) = (1-b)/b` を計算して `adjusted_length = (raw_length - tau_g).clamp(3.0, max_len)` を使う（ring buffer + Lagrange 4 点参照の安全条件）。`self.brightness`（SmoothedValue）の **遷移中の current 値ではなく `self.brightness.target()`** を使うことで、note_on / pitch bend のタイミングで 1 度だけ補正（process 中の brightness 変化はピッチ偏移として許容、これは vibrato 効果として捉える）。

## VoicePool の Phase 3 拡張

### Constructor: FractionalDelay 注入経路（Step 1 試作用）

VoicePool::new() は Phase 2 既存 API 互換で内部的に `KarplusStrong::new()`（Lagrange）を呼ぶ。Step 1 の Thiran 試作で全 voice に Thiran を注入するため、Phase 3 で **`new_with_fractional_delay_factory`** を追加する。`#[doc(hidden)]` test-only constructor:

```rust
impl<const N: usize> VoicePool<N> {
    /// Phase 2 既存 API: 内部で `[KarplusStrong::new(); N]` 相当（実際は `core::array::from_fn`）。
    /// Lagrange ベースの fractional delay でデフォルト構築。Engine::new() → これ経由。
    pub fn new() -> Self { ... }

    /// Phase 3 追加（`#[doc(hidden)]` または `#[cfg(test)]`）: 各 voice に
    /// FractionalDelay を注入する経路。引数は `Fn() -> FractionalDelay` の factory closure
    /// （`FractionalDelay::new_thiran()` を毎回呼んで独立インスタンスを各 voice に配る）。
    /// Step 1 試作では `Engine::new_with_thiran()` → これ → 各 voice の
    /// `KarplusStrong::new_with_fractional_delay(...)` の順に伝播する。
    #[doc(hidden)]
    pub fn new_with_fractional_delay_factory<F: Fn() -> FractionalDelay>(factory: F) -> Self {
        // core::array::from_fn(|_| KarplusStrong::new_with_fractional_delay(factory()))
        // で N 個の voice を独立した FractionalDelay で構築
    }
}
```

### Pitch Bend / Mod Depth fan-out

```rust
impl<const N: usize> VoicePool<N> {
    // 既存（Phase 2）
    pub fn note_on(&mut self, midi_note: u8, freq_hz: f32, velocity: f32) -> usize { ... }
    // 引数順は Phase 2 既存 API（`crates/dsp-core/src/voice_pool.rs:47`）に合わせる。
    // 内部で各 voice の `KarplusStrong::note_on_with_id(midi_note, freq_hz, velocity)`
    // を呼び、note_id を保持させる。
    // 戻り値の usize は assigned voice index、Engine::trigger_voice はこれを使って
    // damping を assigned voice に復元する
    pub fn note_off(&mut self, midi_note: u8) { ... }
    pub fn process_sample(&mut self) -> f32 { ... }

    // Phase 3 追加
    pub fn set_pitch_bend(&mut self, semitones: f32) {
        for v in self.voices.iter_mut() {
            v.set_pitch_bend(semitones);
        }
    }

    /// Pick position β を全 voice に push（次回 note_on で反映、process 中の動的変更ではない）
    pub fn set_pick_position(&mut self, beta: f32) {
        for v in self.voices.iter_mut() {
            v.set_pick_position(beta);
        }
    }

    pub fn all_notes_off(&mut self) {
        for v in self.voices.iter_mut() {
            v.note_off();
        }
    }

    /// Voice State スナップショット（D41）
    /// active mask u8 + 8 振幅 f32 を返す。Engine が voice_state_buffer に書き込む
    pub fn voice_state(&self) -> VoiceStateSnapshot {
        let mut active_mask = 0u8;
        let mut amplitudes = [0.0f32; 8];
        for (i, v) in self.voices.iter().enumerate() {
            if v.is_active() {
                active_mask |= 1u8 << i;
            }
            amplitudes[i] = v.amplitude();
        }
        VoiceStateSnapshot { active_mask, amplitudes }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VoiceStateSnapshot {
    pub active_mask: u8,
    pub amplitudes: [f32; 8],
}
```

### 1/sqrt(N) スケール（Phase 2 維持）

D20 / D31 継承。Modal Body は VoicePool 出力の **後**に Engine が単一段で適用するため、VoicePool 内では Phase 2 のスケールロジックを変更しない。

## Engine の Phase 3 拡張

### 構造体追加

```rust
pub struct Engine {
    // 既存（Phase 2）
    pool: VoicePool<8>,
    output_gain: SmoothedValue,
    hold_stack: HoldStack,
    mode: SynthMode,
    sample_rate: f32,

    // Phase 3 追加
    modal_body: ModalBodyResonator,
    body_wet: SmoothedValue,
    pick_position: f32,           // SmoothedValue 不要（次回 note_on で反映、process 中の動的変更なし）
    channel_volume: SmoothedValue, // MIDI CC#7、UI の output_gain と直交（D38b）。デフォルト 1.0
    sustain_state: SustainState,
    voice_state_buffer: [u8; 33], // active_mask 1 byte + 8 × f32 = 32 bytes
}
```

### `process` 内 per-sample loop（Phase 3 版）

```rust
fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32]) {
    let block_size = output_l.len();
    for i in 0..block_size {
        // 1. VoicePool process（1/sqrt(N) スケール込み、Phase 2 既存）
        //    pick_position は note_on 時に K を確定するため per-sample fan-out 不要
        let dry = self.pool.process_sample();

        // 2. Modal Body Resonator（D31、bandpass biquad）
        let (body_l, body_r) = self.modal_body.process_sample(dry);

        // 3. Wet/Dry mix（D32 BodyWet）
        let wet = self.body_wet.next_sample();
        let mixed_l = dry * (1.0 - wet) + body_l * wet;
        let mixed_r = dry * (1.0 - wet) + body_r * wet;

        // 4. Output gain × Channel Volume（D38b）
        //    UI ParamSlider の output_gain と MIDI CC#7 の channel_volume が直交。
        //    デフォルトで channel_volume=1.0 なら Phase 2 と等価動作
        let g = self.output_gain.next_sample();
        let cv = self.channel_volume.next_sample();
        let scaled_l = mixed_l * g * cv;
        let scaled_r = mixed_r * g * cv;

        // 5. Soft clip（D43、|x|≤0.95 で linear、|x|→∞ で ±1.0 厳密漸近）
        output_l[i] = soft_clip(scaled_l);
        output_r[i] = soft_clip(scaled_r);
    }

    // 6. Voice State export（D41）、stride カウンタは Worklet 側で管理
    let snapshot = self.pool.voice_state();
    self.voice_state_buffer[0] = snapshot.active_mask;
    for i in 0..8 {
        let bytes = snapshot.amplitudes[i].to_le_bytes();
        self.voice_state_buffer[1 + i*4..1 + i*4 + 4].copy_from_slice(&bytes);
    }
}
```

### MIDI CC dispatch

```rust
impl Engine {
    pub fn handle_midi_cc(&mut self, cc: u8, value_normalized: f32) {
        match cc {
            7 => {
                // Channel Volume = MIDI CC#7、独立 SmoothedValue（D38b）
                // UI の OutputGain (master) と直交配置: output_gain * channel_volume が
                // 最終ゲイン。CC#7 で UI スライダーを「上書き」せず、両者の状態が独立に保たれる。
                // value_normalized ∈ [0, 1] をそのまま target に設定（デフォルト 1.0）
                self.channel_volume.set_target(value_normalized);
            }
            64 => {
                // Sustain Pedal
                let active = value_normalized >= 0.5; // 64/127 ≈ 0.504
                // 重要: set_active で active=false になった後に pending bitmap が返る、
                //       active=false 状態で note_off を呼ぶことで再 defer を回避
                let released_bitmap = self.sustain_state.set_active(active);
                if released_bitmap != 0 {
                    // sustain_state.active は既に false。pending を全 release
                    for note in 0..128u8 {
                        if released_bitmap & (1u128 << note) != 0 {
                            self.note_off(note);
                        }
                    }
                }
            }
            123 => {
                // All Notes Off: 全 voice 即時 deactivate + hold_stack + sustain_state すべて clear
                // sustain_state.reset() を忘れると、その後の CC#64 操作で古い pending が再処理されるバグ
                self.pool.all_notes_off();
                self.hold_stack.clear();
                self.sustain_state.reset();
            }
            // CC#1 (Mod Wheel) は Phase 4 送り（D39 / pre-research §6.1 注釈）
            _ => {} // 未対応 CC は無視
        }
    }

    /// note_on: Phase 2 既存実装（`crates/dsp-core/src/engine.rs:50-64`）に Sustain
    /// `clear_pending` を冒頭追加するのみで、それ以外は Phase 2 D29 の挙動を完全継承。
    pub fn note_on(&mut self, midi_note: u8, velocity: f32) {
        // Sustain 中の同一ノート再打鍵で古い pending bit をクリア（P1-3 対策）
        //   このノートに対する未確定 pending を必ず消してから新規発音させる。
        //   こうしておけば「再打鍵後にまだ離していないのに古い pending で release される」事故を防げる。
        //   再打鍵後に改めて note_off → CC#64 off と進めば、新しく立った pending bit のみで release される。
        self.sustain_state.clear_pending(midi_note);

        if matches!(self.mode, SynthMode::Mono) {
            // 直前 top のボイスをリリース（短い release tail はクリック対策で残す、Phase 2 既存）
            if let Some(prev) = self.hold_stack.top() {
                if prev != midi_note {
                    self.pool.note_off(prev);
                }
            }
            // MIDI 重複 noteOn で stale 履歴が残らないよう push_unique（Phase 2 既存 D29）
            self.hold_stack.push_unique(midi_note);
        }
        // VoicePool の引数順は (midi_note, freq, velocity) であることに注意（既存 API、`voice_pool.rs:47`）
        self.trigger_voice(midi_note, velocity);
    }

    /// trigger_voice: Phase 2 既存ヘルパ。assigned voice にのみ damping を復元する
    fn trigger_voice(&mut self, midi_note: u8, velocity: f32) {
        let freq = midi_to_freq(midi_note);
        let assigned = self.pool.note_on(midi_note, freq, velocity);  // (midi_note, freq, velocity)
        self.pool.set_damping_voice(assigned, self.current_damping);
    }

    /// note_off: Phase 2 既存実装（`engine.rs:68-86`）の挙動を完全継承し、
    /// Poly mode のみに Sustain defer を適用する。
    ///
    /// **Mono mode では Sustain CC#64 を無視する**: 実機 Mono synth でも挙動が機種で様々
    /// （Minimoog 系は Sustain で release block、近代モデルは voice 切替時に前 voice を保持等）。
    /// Mono の last-note priority と Sustain の release defer は本質的に相反するため、
    /// Phase 3 では Mono + Sustain は **Sustain を適用しない**（Mono の Phase 2 D29 既存挙動を完全継承）。
    /// Mono + Sustain の挙動は Phase 4 で再評価、必要に応じて仕様を追加する。
    pub fn note_off(&mut self, midi_note: u8) {
        match self.mode {
            SynthMode::Poly => {
                // Poly のみ Sustain で defer 可能
                if self.sustain_state.try_defer_note_off(midi_note) {
                    return; // pending bit 立てて return、sustain off で release
                }
                self.pool.note_off(midi_note);
            }
            SynthMode::Mono => {
                // Phase 2 既存ロジックを完全継承、Sustain は適用しない
                let prev_top = self.hold_stack.top();
                self.hold_stack.remove(midi_note);
                self.pool.note_off(midi_note);
                let new_top = self.hold_stack.top();
                // top が変わった場合のみ復帰発音（中間キー解放では再 trigger しない、クリック対策、prev_top != new_top ガード）
                if new_top != prev_top {
                    if let Some(top) = new_top {
                        self.trigger_voice(top, MONO_REVIVE_VELOCITY);
                    }
                }
            }
        }
    }

    pub fn handle_pitch_bend(&mut self, semitones: f32) {
        let clamped = semitones.clamp(-2.0, 2.0);
        self.pool.set_pitch_bend(clamped);
    }

    /// `synth_set_polyphony_mode` から呼ばれる。Phase 2 既存挙動（hold_stack.clear()）に
    /// **Sustain pending の即時 release** を追加する（D40 拡張）。
    ///
    /// 理由: Poly で pending を積んだ状態で Mono に切替えると、Mono は Sustain 無視のため
    /// pending が宙ぶらりんになる。CC#64 off を後で受けても Mono note_off に流れて意図と異なる
    /// release が起きる可能性がある。**mode 切替時に pending を全 release** するのが
    /// 最も明示的・予測可能。
    pub fn set_mode(&mut self, mode: SynthMode) {
        // 切替前の Sustain pending を全 release（active=true のままでも reset で全部 0 になる）
        // pending bitmap を取り出してから sustain_state.reset() し、その後 self.note_off で release
        let pending = self.sustain_state.pending_release_bitmap();
        self.sustain_state.reset();
        if pending != 0 {
            for note in 0..128u8 {
                if pending & (1u128 << note) != 0 {
                    // この時点で sustain_state.active = false なので try_defer は false を返し
                    // 通常 release 経路に流れる（mode 切替「前」の voice 状態に対する release）
                    self.pool.note_off(note);
                }
            }
        }

        self.mode = mode;
        // モード切替時は履歴を破棄する（Phase 2 既存）。進行中のボイスは VoicePool 側で自然減衰
        self.hold_stack.clear();
    }

    pub fn voice_state_ptr(&self) -> *const u8 {
        self.voice_state_buffer.as_ptr()
    }
}

// Engine の constructor: デフォルトは Lagrange、test-only で Thiran 注入経路を提供
impl Engine {
    /// Phase 2 既存 API。`VoicePool::new()` 経由で Lagrange ベースの fractional delay。
    pub fn new() -> Self {
        Self {
            pool: VoicePool::new(),
            // ... 残り全フィールド初期化（output_gain / hold_stack / mode / modal_body / etc.） ...
        }
    }

    /// Phase 3 追加（`#[doc(hidden)]` test-only constructor）。Step 1 の Thiran 試作で
    /// 各 voice に `FractionalDelay::new_thiran()` を注入する経路。
    /// `cargo test` で `Engine::new_with_thiran()` を呼んで pitch_accuracy.rs の
    /// `test_pitch_*_thiran` 系テストを実行する。
    #[doc(hidden)]
    pub fn new_with_thiran() -> Self {
        Self {
            pool: VoicePool::new_with_fractional_delay_factory(|| FractionalDelay::new_thiran()),
            // ... 残り全フィールド初期化（new() と同じ） ...
        }
    }
}
```

### Pitch Bend の Mono / Poly 共通動作

mono モードでも poly モードでも `set_pitch_bend` は全ボイスに fan-out（mono は 1 ボイスしか active でないため実質 1 件のみ更新）。Hold stack の動作には影響しない。

### Mono + Sustain の挙動仕様（明示）

| シナリオ | Phase 3 動作 |
|---|---|
| Mono + CC#64=127 + C4 on | C4 発音、stack=[C4]、Sustain は適用されない |
| Mono + CC#64=127 + C4 on + C4 off | C4 即時 release（Sustain で defer されない、stack=[]） |
| Mono + CC#64=127 + C4 on + D4 on + D4 off | C4 復帰（既存 D29 通り）、D 系の voice release も即時 |
| Mono + CC#64=0 (off) | Mono 内では何も変化なし（pending bitmap が常に空） |

Phase 4 で Mono+Sustain の挙動が必要になった場合は、(a) 「Mono mode は前 voice の release を Sustain で defer する」案、(b) 「Mono は常に Sustain 無視」案、を改めて検討する。

## テスト方針（Phase 3 新規追加分）

| テスト | 目的 | 実装ファイル |
|---|---|---|
| `test_single_biquad_dc_blocking` | 単体 biquad の DC ゲイン 0（H(1) = 0 直接検証） | `tests/modal_body_biquad_tests.rs` 新規 |
| `test_single_biquad_peak_at_freq` | 単体 biquad のピークゲイン `mode.gain` ± 5%（隣接干渉なし） | 同上 |
| `test_single_biquad_bandwidth` | -3 dB 帯域幅 ≈ `freq / Q` ± 20% | 同上 |
| `test_modal_body_dc_blocking` | aggregate DC 入力で定常出力 RMS < 0.001 | `tests/modal_body_tests.rs` 新規 |
| `test_modal_body_peak_at_modes` | 各モード周波数で `mode.gain` の 0.5〜1.5 倍（aggregate、隣接モード干渉許容） | 同上 |
| `test_modal_body_inter_mode_attenuation` | モード間中点の出力 RMS が最大 `mode.gain` 未満（定性的検証） | 同上 |
| `test_modal_body_stereo_spread` | 左右 RMS 差 3〜10% | 同上 |
| `test_modal_body_no_alloc_in_process` | `prepare` 後 process 中の length 不変 | 同上 |
| `test_modal_body_reset_clears_state` | 励振後 reset で z1/z2 が 0 | 同上 |
| `test_loss_filter_dc_gain` | DC ゲイン 1.0 保存 | `tests/loss_filter_tests.rs` 新規 |
| `test_loss_filter_nyquist_attenuation` | Nyquist で `(1-ρ)/(1+ρ)` 減衰 | 同上 |
| `test_loss_filter_high_freq_more_loss` | A6 で A4 より大きな ρ | 同上 |
| `test_pick_min_beta_minimal_shape` | β=0.05（公開 API の最小値）で comb 効果が最小限、β=0.5 比でスペクトル変化が小さい（外部から到達可能） | `tests/karplus_strong_pick_tests.rs` 新規 |
| `test_pick_internal_k_zero_branch` | 内部分岐検証: 短 length_int（≤ 9）+ β=0.05 で K = round(0.45) = 0 となる境界を `#[cfg(test)]` 内部 or test-only constructor 経由で踏む、panic なく素通し処理で抜ける | 同上 |
| `test_pick_position_node_at_beta_half` | β=0.5 で偶数倍音 < 0.1 倍 | 同上 |
| `test_pick_position_attenuates_kth_harmonic` | β=1/k で k 番目倍音減衰 | 同上 |
| `test_pick_position_no_extra_alloc` | β を変えて note_on 連打、buffer.len() 不変 | 同上 |
| `test_sustain_defers_note_off` | active=true で `try_defer_note_off(60)` が true、pending bit 60 立つ | `tests/sustain_tests.rs` 新規 |
| `test_sustain_release_on_off` | pending 複数件積んで `set_active(false)` で全 bit が返る | 同上 |
| `test_sustain_passthrough_when_inactive` | active=false で defer なし | 同上 |
| `test_sustain_clear_pending_on_retrigger` | pending bit 立った状態で `clear_pending(60)` 呼ぶと bit 落ち、`set_active(false)` で 60 が返らない（同一ノート再打鍵対策） | 同上 |
| `test_sustain_reset_clears_active_and_pending` | `reset()` で active=false / pending=0（CC#123 シナリオ用） | 同上 |
| `test_sustain_pending_release_bitmap_readonly` | `pending_release_bitmap()` が pending を変更せず参照のみ返す（mode 切替用 API） | 同上 |
| `test_soft_clip_linear_in_safe_range` | |x|≤0.95 で `assert_eq!(soft_clip(x), x)` | `tests/soft_clip_tests.rs` 新規 |
| `test_soft_clip_bounded` | 任意の x で `\|y\| < 1.0`（厳密有界） | 同上 |
| `test_soft_clip_continuous_at_threshold` | x=0.95 ± 1e-6 で連続 | 同上 |
| `test_soft_clip_extreme` | |x|=1e6 で `0.99 < \|y\| < 1.0` | 同上 |
| `test_pitch_a1_thiran` 〜 `test_pitch_c8_thiran` | Step 1 評価、A1〜C8 各音程の Thiran 精度 | `tests/pitch_accuracy.rs` 拡張 |
| `test_pitch_c8_thiran_self_oscillates` | Thiran で C8 自己発振、tail RMS > 0.01 | 同上 |
| `test_pitch_bend_smooth_transition` | Pitch Bend 5ms tau での滑らか遷移 | `tests/pitch_bend_tests.rs` 新規 |
| `test_pitch_bend_clamps_to_range` | ±2 半音超でクランプ | 同上 |
| `test_pitch_bend_ring_buffer_invariant` | Pitch Bend 中も write_index `% buf_len` 不変条件維持 | 同上 |
| `test_engine_modal_body_in_signal_chain` | dry / wet ミックスが正しく動作 | `tests/dsp_core_tests.rs` 拡張 |
| `test_engine_midi_cc_volume` | CC#7 で `channel_volume` target が変わる、`output_gain` は変わらない（D38b 直交確認） |  同上 |
| `test_engine_midi_cc_volume_multiplied_in_output` | CC#7=0.5 + OutputGain=1.0 で出力が 0.5 倍、両方 0.8 で 0.64 倍（積算確認） | 同上 |
| `test_engine_midi_cc_sustain_defers` | CC#64=127 + note_on(60) + note_off(60) で voice が active のまま（pending bit 60 立つ）、CC#64=0 で voice が release | 同上 |
| `test_engine_midi_cc_sustain_clears_pending_on_retrigger` | C4 on → CC#64=127 → C4 off (pending bit 60 立つ) → C4 on (re-strike、`clear_pending` で bit 60 落ちる) → CC#64=0 (off) で **再打鍵分はまだ離していないので release されない**（pending bitmap 空、`clear_pending` 動作確認、P1-3 対策） | 同上 |
| `test_engine_midi_cc_all_notes_off_clears_sustain` | CC#64=127 で pending を積んだ後 CC#123 を呼ぶと sustain_state も reset される、続く CC#64 操作で古い pending が再処理されない（P1-1 対策） | 同上 |
| `test_engine_mono_sustain_no_op` | Mono mode + CC#64=127 + C4 on → C4 off で Sustain は無視され voice が即時 release（Mono は Phase 2 D29 既存挙動を完全継承、Sustain は Poly のみで意味、Mono+Sustain は Phase 4 で再評価） | 同上 |
| `test_engine_mode_switch_clears_sustain` | Poly + CC#64=127 で C4 on → C4 off (pending bit 60 立つ) → `set_mode(Mono)` で **pending を全 release（C4 voice の damping 加速発火）** + `sustain_state.active=false` + `hold_stack` clear。続けて CC#64 操作しても古い pending が再処理されない（mode 切替の境界仕様、P2-1 対策） | 同上 |
| `test_engine_mode_switch_no_pending_passes_through` | Poly + Sustain なし状態で `set_mode(Mono)` を呼んでも pending が空なので no-op、Phase 2 既存の `set_mode` 挙動と等価（regression なし） | 同上 |
| `test_engine_midi_cc_unknown_ignored` | CC#1 や未対応 CC でも panic なし | 同上 |
| `test_engine_voice_state_buffer_format` | 33 byte レイアウト検証 | 同上 |
| `test_engine_brightness_pitch_correction` | 群遅延補正後の A4 ピッチが ±0.5% 以内 | `tests/pitch_accuracy.rs` 拡張 |
| `test_engine_process_block_timing` | release ビルドで 128 frame × 1000 回 process が < 1.5 ms 平均（F37） | `tests/dsp_core_tests.rs` 拡張、`#[cfg(not(debug_assertions))]` |
| `test_no_allocation_with_modal_body_and_midi_cc` | Phase 3 全機能 ON で alloc ゼロ | `tests/voice_pool_tests.rs` 拡張 |

Phase 2 の既存 41 テストは全件継続パス必須（regression なし）。Phase 3 で +30 程度を目標、合計 71 テスト程度を想定。

## リアルタイム制約遵守ルール（Phase 3 版）

[Phase 1 03 章 §リアルタイム制約遵守ルール](../2026-05-06-001-mvp/03-dsp-core-spec.md) と Phase 2 を継承。Phase 3 で追加するルール:

- **Modal Body の biquad denormal 対策**: 各 biquad の出力に `+1e-25 -1e-25` トリック（D6 継承）。`process_sample` の最後に `(y_l + 1e-25 - 1e-25, y_r + 1e-25 - 1e-25)` を返す
- **Pitch Bend の SmoothedValue 遷移中の係数再計算**: 5 ms tau なので最大 240 サンプル分（48kHz）の再計算が発生するが、Lagrange 12 演算 / Thiran 3 演算 / sample のみで予算内
- **Voice State buffer の書き込み**: process_block の終端で 1 度だけ。process 中に書き換えない（Worklet が読むタイミングと race しない設計）
- **Soft clip の区間関数型実装**: `f32::tanh` も Padé 近似も使わない（Padé は |x|→∞ で発散）。`|x| ≤ 0.95` は完全 linear、`|x| > 0.95` は `signum(x) · (0.95 + 0.05·e/(e+0.05))`、`|x|→∞` で ±1.0 厳密漸近、6-7 演算/sample
- **MIDI CC の switch dispatch**: `handle_midi_cc` は process_block 外で呼ばれることを想定（Worklet message 受信時）。process 内では呼ばない
