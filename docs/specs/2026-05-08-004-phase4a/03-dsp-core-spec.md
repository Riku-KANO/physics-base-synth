# 03. dsp-core 仕様（Phase 4a）

## 目的

`crates/dsp-core/` の Rust モジュール群に Phase 4a で追加する API / 内部状態 / テストを定義する。Phase 1 / 2 / 3 で確立した既存モジュールの責務（`KarplusStrong` / `VoicePool` / `Engine` / `ModalBodyResonator` / `LossFilter` / `SoftClip` / `SustainState` / `SmoothedValue` / `XorShift32` / `HoldStack` / `ParamDescriptor` / `FractionalDelay` (Thiran) / `VoiceState`）はすべて維持し、本書では **Phase 4a で追加・変更する箇所のみ** 記述する。

## 他文書との関係

- 上流: [`pre-research.md`](./pre-research.md)（§3 LFO 設計 / §4 Mod Wheel / §7 多楽器 Modal 係数 / §8 既存負債）、[`01-overview.md`](./01-overview.md)（D44-D55）、[`02-architecture.md`](./02-architecture.md)（dsp-core 層責務）
- 下流: [`04-wasm-audio-spec.md`](./04-wasm-audio-spec.md)（C ABI 4 関数追加）、[`07-implementation-checklist.md`](./07-implementation-checklist.md)
- 並行: Phase 3 [`03-dsp-core-spec.md`](../2026-05-07-003-phase3/03-dsp-core-spec.md) — 既存 API スタイルの参照

## モジュール一覧（Phase 4a 後）

```
crates/dsp-core/src/
├── engine.rs               (Phase 3 + Phase 4a で lfo / mod_wheel / current_instrument / lfo_*_depth フィールド追加)
├── fractional_delay.rs     (Phase 3 同等、変更なし)
├── hold_stack.rs           (Phase 2 同等、変更なし)
├── instrument.rs           (Phase 4a 新規 — InstrumentKind enum、または params.rs に統合)
├── karplus_strong.rs       (Phase 4a で lfo_pitch_offset / lfo_brightness_offset フィールド追加、excitation_snapshot を #[cfg(test)] 化)
├── lfo.rs                  (Phase 4a 新規 — Lfo 型定義 D46/D47)
├── lib.rs                  (Phase 4a で `pub mod lfo;` 追加)
├── loss_filter.rs          (Phase 3 同等、変更なし)
├── modal_body.rs           (Phase 4a で set_instrument(kind, sample_rate) メソッド追加)
├── note_allocator.rs       (Phase 2 同等、変更なし)
├── params.rs               (生成、Phase 4a で BODY_MODES_<INSTRUMENT>_L/R 12 配列 + STEREO_SPREAD_<INSTRUMENT> 6 値 + InstrumentKind enum 出力)
├── rng.rs                  (Phase 1 同等、変更なし)
├── smoothing.rs            (Phase 3 同等、変更なし)
├── soft_clip.rs            (Phase 3 同等、変更なし)
├── sustain_state.rs        (Phase 3 同等、変更なし)
├── traits.rs               (Phase 3 同等、変更なし)
├── voice.rs                (Phase 4a で set_lfo_pitch_offset / set_lfo_brightness_offset 委譲を追加)
└── voice_pool.rs           (Phase 4a で set_lfo_pitch_offset / set_lfo_brightness_offset 追加)
```

## Lfo (`lfo.rs`) — Phase 4a 新規（D46 / D47）

### 構造体定義

```rust
//! Lfo (Phase 4a D46 / D47)
//!
//! グローバル LFO（Engine 内 1 個）。Sine / Triangle 切替、レンジ 0.1〜8.0 Hz、
//! SmoothedValue tau=0.05s で rate 平滑化（クリック防止）。
//! denormal flush は phase が [0, 1) で常に有限のため不要。
//!
//! 配置: `Engine::process` の per-sample loop 冒頭で `process_sample()` を呼び、
//! 戻り値 ∈ [-1, 1] を destinations の offset として伝播。

use crate::smoothing::SmoothedValue;

/// LFO 波形種。
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LfoWaveform {
    Sine = 0,
    Triangle = 1,
}

impl LfoWaveform {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Sine),
            1 => Some(Self::Triangle),
            _ => None,
        }
    }
}

pub const LFO_RATE_DEFAULT: f32 = 5.0;
pub const LFO_RATE_MIN: f32 = 0.1;
pub const LFO_RATE_MAX: f32 = 8.0;
const LFO_RATE_TAU: f32 = 0.05;

pub struct Lfo {
    /// 0..1 で正規化された phase。`process_sample` で += rate / sample_rate。
    phase: f32,
    /// SmoothedValue 化された rate (Hz)。target は `set_rate` で更新、`process_sample` で 1 サンプル毎に next_sample。
    rate_hz: SmoothedValue,
    waveform: LfoWaveform,
    sample_rate: f32,
}

impl Lfo {
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            rate_hz: SmoothedValue::new(LFO_RATE_DEFAULT),
            waveform: LfoWaveform::Sine,
            sample_rate: 48000.0,
        }
    }

    pub fn prepare(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.rate_hz.set_time_constant(sample_rate, LFO_RATE_TAU);
        self.reset();
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.rate_hz.set_immediate(LFO_RATE_DEFAULT);
        self.waveform = LfoWaveform::Sine;
    }

    pub fn set_rate(&mut self, hz: f32) {
        let v = hz.clamp(LFO_RATE_MIN, LFO_RATE_MAX);
        self.rate_hz.set_target(v);
    }

    pub fn set_waveform(&mut self, kind: LfoWaveform) {
        self.waveform = kind;
    }

    /// 1 サンプル進めて [-1, 1] の LFO 値を返す。
    /// Sine: `f32::sin(2π · phase)`、Triangle: `4·|phase − 0.5| − 1`。
    #[inline(always)]
    pub fn process_sample(&mut self) -> f32 {
        let rate = self.rate_hz.next_sample();
        self.phase += rate / self.sample_rate;
        // `phase < 1.0` の防御的 clamp は不要（`rate / sample_rate` は最大でも 8/48000 = 0.000167）。
        // 1 sample で 1.0 を跨ぐことはあり得ない、ただし周回時は減算で [0, 1) へ戻す。
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        match self.waveform {
            LfoWaveform::Sine => {
                use core::f32::consts::TAU;
                (TAU * self.phase).sin()
            }
            LfoWaveform::Triangle => {
                // [-1, 1] の対称三角波。phase=0 で -1、phase=0.5 で 1、phase=1 で -1。
                let centered = self.phase - 0.5;
                4.0 * centered.abs() - 1.0
            }
        }
    }

    #[doc(hidden)]
    pub fn phase(&self) -> f32 {
        self.phase
    }

    #[doc(hidden)]
    pub fn rate_target(&self) -> f32 {
        self.rate_hz.target()
    }
}

impl Default for Lfo {
    fn default() -> Self {
        Self::new()
    }
}
```

### Lfo テスト方針

`crates/dsp-core/tests/lfo_tests.rs` に新規追加:

| テスト名 | 検証内容 |
|---|---|
| `test_lfo_sine_range` | 1 秒 (48000 sample) 走らせて出力が [-1, 1] に収まること、min/max が ±1 に十分近い |
| `test_lfo_triangle_range` | 同上、triangle で min/max ±1 |
| `test_lfo_zero_at_init` | `process_sample` 初回呼出で sine = 0、triangle = -1 (phase=0) |
| `test_lfo_period_matches_rate` | rate=5Hz、9600 sample (0.2 秒) で位相が 1.0 に達する（1 周期分） |
| `test_lfo_rate_smoothing` | rate を 1Hz → 8Hz に変更、SmoothedValue tau=0.05s で 50ms 後に 8Hz target 到達 |
| `test_lfo_waveform_switch_no_click` | sine → triangle 切替時に出力連続性を簡易確認（switch 直後の値と 1 sample 前の値の差 < 0.5） |
| `test_lfo_no_alloc_in_process` | 1000 サンプル処理で `process_sample` のヒープ確保ゼロ（capacity 不変、Phase 3 D4 維持） |
| `test_lfo_phase_wraps` | 10 秒走らせて phase が [0, 1) で wrap している（NaN / 無限大なし） |

## InstrumentKind enum と多楽器 Modal 係数

### enum 定義（`params.rs` 生成、または手書き `instrument.rs`）

`gen-params.mjs` で生成する設計を採用（drift 防止）:

```rust
// 生成: params.rs（ParamId と同様の repr(u32) パターン）
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
            _ => None,
        }
    }
}

pub const INSTRUMENT_KIND_COUNT: usize = 7;
```

### 多楽器 Modal 係数定数

Phase 3 の `BODY_MODES_L` / `BODY_MODES_R` を温存しつつ（Default として参照）、楽器ごとに 12 配列を追加:

```rust
// 生成: params.rs（pre-research §7.2 の参考値、Phase 3 と同形式の #[rustfmt::skip] 1 行）

#[rustfmt::skip]
pub const BODY_MODES_DEFAULT_L: [BodyMode; 8] = BODY_MODES_L;  // Phase 3 既存値の alias
#[rustfmt::skip]
pub const BODY_MODES_DEFAULT_R: [BodyMode; 8] = BODY_MODES_R;

#[rustfmt::skip]
pub const BODY_MODES_GUITAR_CLASSICAL_L: [BodyMode; 8] = [
    BodyMode { freq: 105.0, q: 30.0, gain: 1.0 }, BodyMode { freq: 200.0, q: 25.0, gain: 0.8 }, BodyMode { freq: 280.0, q: 20.0, gain: 0.5 }, BodyMode { freq: 420.0, q: 35.0, gain: 0.4 }, BodyMode { freq: 580.0, q: 40.0, gain: 0.35 }, BodyMode { freq: 850.0, q: 45.0, gain: 0.25 }, BodyMode { freq: 1400.0, q: 50.0, gain: 0.2 }, BodyMode { freq: 2300.0, q: 60.0, gain: 0.15 },
];
#[rustfmt::skip]
pub const BODY_MODES_GUITAR_CLASSICAL_R: [BodyMode; 8] = [ /* L 値に gen-params.mjs の applyStereoSpread(modes, 0.05) を適用したもの */ ];

#[rustfmt::skip]
pub const BODY_MODES_UKULELE_L: [BodyMode; 8] = [
    BodyMode { freq: 200.0, q: 18.0, gain: 0.9 }, BodyMode { freq: 380.0, q: 20.0, gain: 0.7 }, BodyMode { freq: 540.0, q: 22.0, gain: 0.45 }, BodyMode { freq: 780.0, q: 28.0, gain: 0.35 }, BodyMode { freq: 1100.0, q: 32.0, gain: 0.3 }, BodyMode { freq: 1600.0, q: 38.0, gain: 0.22 }, BodyMode { freq: 2200.0, q: 42.0, gain: 0.18 }, BodyMode { freq: 3100.0, q: 50.0, gain: 0.12 },
];
// ... 他 5 楽器も同形式

pub const STEREO_SPREAD_DEFAULT: f32 = 0.05;             // Phase 3 既存値
pub const STEREO_SPREAD_GUITAR_CLASSICAL: f32 = 0.05;
pub const STEREO_SPREAD_UKULELE: f32 = 0.04;
pub const STEREO_SPREAD_MANDOLIN: f32 = 0.06;
pub const STEREO_SPREAD_BASS: f32 = 0.03;
pub const STEREO_SPREAD_GUITAR_STEEL: f32 = 0.05;
pub const STEREO_SPREAD_SITAR: f32 = 0.08;
```

### `body_modes_for_instrument(kind) -> (&[BodyMode; 8], &[BodyMode; 8])`

楽器選択を係数配列にマップする手書きヘルパ（`params.rs` と同居）:

```rust
pub fn body_modes_for_instrument(
    kind: InstrumentKind,
) -> (&'static [BodyMode; 8], &'static [BodyMode; 8]) {
    match kind {
        InstrumentKind::Default => (&BODY_MODES_DEFAULT_L, &BODY_MODES_DEFAULT_R),
        InstrumentKind::GuitarClassical => {
            (&BODY_MODES_GUITAR_CLASSICAL_L, &BODY_MODES_GUITAR_CLASSICAL_R)
        }
        InstrumentKind::Ukulele => (&BODY_MODES_UKULELE_L, &BODY_MODES_UKULELE_R),
        InstrumentKind::Mandolin => (&BODY_MODES_MANDOLIN_L, &BODY_MODES_MANDOLIN_R),
        InstrumentKind::Bass => (&BODY_MODES_BASS_L, &BODY_MODES_BASS_R),
        InstrumentKind::GuitarSteel => (&BODY_MODES_GUITAR_STEEL_L, &BODY_MODES_GUITAR_STEEL_R),
        InstrumentKind::Sitar => (&BODY_MODES_SITAR_L, &BODY_MODES_SITAR_R),
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
    }
}
```

`params.json` の `instruments` セクションが gen-params.mjs に与えられ、`applyStereoSpread(modes, spread)` で L/R 両方を生成。`STEREO_SPREAD_*` 定数も生成。手書きヘルパ `body_modes_for_instrument` / `stereo_spread_for_instrument` は `params.rs` 末尾に固定文として generator に出させる（drift 防止）。

## ModalBodyResonator の拡張（D52 / D53 / D54）

`crates/dsp-core/src/modal_body.rs` に `set_instrument` メソッドを追加:

```rust
impl ModalBodyResonator {
    /// Phase 4a D52 / D53: 楽器切替で Modal 係数を新セットに差し替え、状態クリア。
    /// `Engine::apply_instrument` から呼ばれる。
    pub fn set_instrument(&mut self, kind: InstrumentKind, sample_rate: f32) {
        let (l_modes, r_modes) = body_modes_for_instrument(kind);
        for i in 0..NUM_MODES {
            self.coeffs_l[i] = calc_coeffs(l_modes[i], sample_rate);
            self.coeffs_r[i] = calc_coeffs(r_modes[i], sample_rate);
        }
        self.reset();  // z1 / z2 状態をクリア（楽器切替で過去の共鳴を引きずらない）
    }
}
```

`prepare(sample_rate)` も Phase 3 既存の `BODY_MODES_L/R` を呼ぶのではなく、現在の `current_instrument` から決まる係数を使う設計に変更。**`prepare` の引数に `kind: InstrumentKind` を追加するか、`Engine` 側で `prepare` 後に `set_instrument(kind, sr)` を呼ぶ**かを選択。**後者を採用**（`prepare` シグネチャを変えると `AudioProcessor` trait 互換に影響、Engine 側で順次呼出する方が安全）。

```rust
// modal_body.rs（Phase 4a 後の prepare、変更なし、Phase 3 既存値で初期化）
pub fn prepare(&mut self, sample_rate: f32) {
    self.sample_rate = sample_rate;
    for i in 0..NUM_MODES {
        // Phase 3 互換のため Default の L/R で初期化
        self.coeffs_l[i] = calc_coeffs(BODY_MODES_DEFAULT_L[i], sample_rate);
        self.coeffs_r[i] = calc_coeffs(BODY_MODES_DEFAULT_R[i], sample_rate);
    }
    self.reset();
}
```

## Engine の Phase 4a 拡張

### フィールド追加

```rust
pub struct Engine {
    // Phase 3 既存
    sample_rate: f32,
    pool: VoicePool<POLYPHONY>,
    output_gain: SmoothedValue,
    modal_body: ModalBodyResonator,
    body_wet: SmoothedValue,
    mode: SynthMode,
    hold_stack: HoldStack,
    current_damping: f32,
    pick_position: f32,
    channel_volume: SmoothedValue,
    sustain_state: SustainState,
    voice_state_buffer: [u8; 33],
    voice_state_sample_counter: u32,

    // Phase 4a 新規
    /// D46: グローバル LFO 1 個
    lfo: Lfo,
    /// D49: Mod Wheel (CC#1) を SmoothedValue で保持
    mod_wheel: SmoothedValue,
    /// D48: LFO Pitch destination 深さ ∈ [0, 1]
    lfo_pitch_depth: SmoothedValue,
    /// D48: LFO Brightness destination 深さ ∈ [0, 1]
    lfo_brightness_depth: SmoothedValue,
    /// D48: LFO Volume destination 深さ ∈ [0, 1]
    lfo_volume_depth: SmoothedValue,
    /// D52 / D53: 現在の楽器選択
    current_instrument: InstrumentKind,
    /// D54: 楽器ごとの stereo_spread を反映する保持値（ModalBodyResonator へは
    /// `prepare` / `set_instrument` 内で計算される BODY_MODES_*_R が直接反映、本フィールドは
    /// `Engine::stereo_spread()` 公開 API 用の参照値）
    stereo_spread: f32,
}
```

`mod_wheel` のデフォルトは 0.0（Phase 3 互換、LFO 効果ゼロ）。`lfo_*_depth` はそれぞれデフォルト 0.0（明示設定なしでは LFO 効果ゼロ）。

### 定数追加

```rust
const MOD_WHEEL_DEFAULT: f32 = 0.0;
const MOD_WHEEL_TAU: f32 = 0.05;

const LFO_DEPTH_DEFAULT: f32 = 0.0;
const LFO_DEPTH_TAU: f32 = 0.05;

/// D48: LFO Pitch destination の深さスケール (depth=1.0 で ±0.5 半音)
const LFO_PITCH_SCALE_SEMITONES: f32 = 0.5;
/// D48: LFO Brightness destination の深さスケール (depth=1.0 で ±0.5 brightness offset)
const LFO_BRIGHTNESS_SCALE: f32 = 0.5;
/// D48: LFO Volume destination の深さスケール (depth=1.0 で ±0.5 volume multiplier offset、0.5〜1.5 倍)
const LFO_VOLUME_SCALE: f32 = 0.5;
```

### コンストラクタと prepare

```rust
impl Engine {
    pub fn new() -> Self {
        Self {
            // ...Phase 3 既存...
            lfo: Lfo::new(),
            mod_wheel: SmoothedValue::new(MOD_WHEEL_DEFAULT),
            lfo_pitch_depth: SmoothedValue::new(LFO_DEPTH_DEFAULT),
            lfo_brightness_depth: SmoothedValue::new(LFO_DEPTH_DEFAULT),
            lfo_volume_depth: SmoothedValue::new(LFO_DEPTH_DEFAULT),
            current_instrument: InstrumentKind::Default,
            stereo_spread: STEREO_SPREAD_DEFAULT,
        }
    }
}

impl AudioProcessor for Engine {
    fn prepare(&mut self, sample_rate: f32, max_block_size: usize) {
        // ...Phase 3 既存...
        self.lfo.prepare(sample_rate);
        self.mod_wheel.set_time_constant(sample_rate, MOD_WHEEL_TAU);
        self.lfo_pitch_depth.set_time_constant(sample_rate, LFO_DEPTH_TAU);
        self.lfo_brightness_depth.set_time_constant(sample_rate, LFO_DEPTH_TAU);
        self.lfo_volume_depth.set_time_constant(sample_rate, LFO_DEPTH_TAU);
    }

    fn reset(&mut self) {
        // ...Phase 3 既存...
        self.lfo.reset();
        self.mod_wheel.set_immediate(MOD_WHEEL_DEFAULT);
        self.lfo_pitch_depth.set_immediate(LFO_DEPTH_DEFAULT);
        self.lfo_brightness_depth.set_immediate(LFO_DEPTH_DEFAULT);
        self.lfo_volume_depth.set_immediate(LFO_DEPTH_DEFAULT);
        // 楽器選択も Default に戻す（reset の意味として「初期状態へ復帰」）
        self.current_instrument = InstrumentKind::Default;
        self.stereo_spread = STEREO_SPREAD_DEFAULT;
        self.modal_body.set_instrument(InstrumentKind::Default, self.sample_rate);
    }
}
```

### `Engine::handle_midi_cc` の CC#1 分岐有効化（D49）

```rust
pub fn handle_midi_cc(&mut self, cc: u8, value_normalized: f32) {
    let v = value_normalized.clamp(0.0, 1.0);
    match cc {
        CC_MOD_WHEEL => {
            // Phase 4a D49: Mod Wheel を LFO depth の master 乗数として保持。
            // Phase 3 では no-op だった経路を有効化。
            self.mod_wheel.set_target(v);
        }
        CC_CHANNEL_VOLUME => {
            self.channel_volume.set_target(v);
        }
        CC_SUSTAIN_PEDAL => {
            let released = self.sustain_state.set_active(v >= 0.5);
            self.release_pending(released);
        }
        CC_ALL_NOTES_OFF => {
            self.pool.all_notes_off();
            self.hold_stack.clear();
            self.sustain_state.reset();
        }
        _ => {}
    }
}
```

### `Engine::apply_instrument(kind)`（D52 / D53）

```rust
impl Engine {
    /// Phase 4a D52 / D53: 楽器プリセット切替。
    /// 全 voice 即時 release → Modal 係数差し替え → reset。
    pub fn apply_instrument(&mut self, kind: InstrumentKind) {
        // 演奏中の音は即時 release（fade-out なし、UX 注意点として UI 側で告知）
        self.pool.all_notes_off();
        self.hold_stack.clear();
        self.sustain_state.reset();

        self.current_instrument = kind;
        self.stereo_spread = stereo_spread_for_instrument(kind);
        self.modal_body.set_instrument(kind, self.sample_rate);
    }

    #[doc(hidden)]
    pub fn current_instrument(&self) -> InstrumentKind {
        self.current_instrument
    }

    #[doc(hidden)]
    pub fn stereo_spread(&self) -> f32 {
        self.stereo_spread
    }
}
```

### LFO setter 群

```rust
impl Engine {
    /// Phase 4a D46: LFO レート設定 (0.1〜8.0 Hz)。
    pub fn lfo_set_rate(&mut self, hz: f32) {
        self.lfo.set_rate(hz);
    }

    /// Phase 4a D47: LFO 波形設定。
    pub fn lfo_set_waveform(&mut self, kind: LfoWaveform) {
        self.lfo.set_waveform(kind);
    }

    /// Phase 4a D48: LFO destination depth 設定。
    /// `dest`: 0=Pitch, 1=Brightness, 2=Volume
    pub fn lfo_set_depth(&mut self, dest: LfoDestination, depth: f32) {
        let v = depth.clamp(0.0, 1.0);
        match dest {
            LfoDestination::Pitch => self.lfo_pitch_depth.set_target(v),
            LfoDestination::Brightness => self.lfo_brightness_depth.set_target(v),
            LfoDestination::Volume => self.lfo_volume_depth.set_target(v),
        }
    }
}

// `lfo.rs` または `instrument.rs` に追加
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LfoDestination {
    Pitch = 0,
    Brightness = 1,
    Volume = 2,
}

impl LfoDestination {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Pitch),
            1 => Some(Self::Brightness),
            2 => Some(Self::Volume),
            _ => None,
        }
    }
}
```

### `Engine::process` の per-sample loop 拡張（D46-D49）

```rust
fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32]) {
    debug_assert_eq!(output_l.len(), output_r.len());
    let n = output_l.len();
    for i in 0..n {
        // Phase 4a D46-D49: LFO 値を取得し、Mod Wheel で master 制御。
        let lfo_value = self.lfo.process_sample();  // ∈ [-1, 1]
        let mod_wheel_v = self.mod_wheel.next_sample();  // ∈ [0, 1]

        // D48 Pitch destination: per voice の length_target SmoothedValue に offset 加算。
        // 16 voice 分の SmoothedValue を毎 sample 更新するのは非効率なので、
        // 「LFO offset」を VoicePool に push する API で実装（後述）。
        let pitch_offset = lfo_value * self.lfo_pitch_depth.next_sample() * mod_wheel_v
            * LFO_PITCH_SCALE_SEMITONES;
        self.pool.set_lfo_pitch_offset(pitch_offset);

        // D48 Brightness destination
        let brightness_offset = lfo_value * self.lfo_brightness_depth.next_sample() * mod_wheel_v
            * LFO_BRIGHTNESS_SCALE;
        self.pool.set_lfo_brightness_offset(brightness_offset);

        // D48 Volume destination: Engine 単位で適用 (per voice 不要)
        let volume_multiplier = 1.0 + lfo_value * self.lfo_volume_depth.next_sample()
            * mod_wheel_v * LFO_VOLUME_SCALE;

        // Phase 3 既存パス
        let dry = self.pool.process_sample();
        let (body_l, body_r) = self.modal_body.process_sample(dry);
        let wet = self.body_wet.next_sample();
        let dry_amount = 1.0 - wet;
        let mixed_l = dry_amount * dry + wet * body_l;
        let mixed_r = dry_amount * dry + wet * body_r;

        // D38b + Phase 4a D48: final = output_gain × channel_volume × volume_multiplier
        let combined = self.output_gain.next_sample()
            * self.channel_volume.next_sample()
            * volume_multiplier;

        output_l[i] = soft_clip(mixed_l * combined);
        output_r[i] = soft_clip(mixed_r * combined);
    }
    // Voice State 書き込みは Phase 3 同等
    self.voice_state_sample_counter = self.voice_state_sample_counter.saturating_add(n as u32);
    if self.voice_state_sample_counter >= VOICE_STATE_WRITE_STRIDE {
        self.voice_state_sample_counter = 0;
        self.write_voice_state();
    }
}
```

## VoicePool の Phase 4a 拡張

```rust
impl<const N: usize> VoicePool<N> {
    /// Phase 4a D48: LFO Pitch offset を全 voice に fan-out。
    /// per sample 呼出されるため、fan-out 内では SmoothedValue ではなく
    /// 単純 f32 として保持し、Voice の per-sample 計算で読まれる設計。
    pub fn set_lfo_pitch_offset(&mut self, semitones: f32) {
        for v in &mut self.voices {
            v.set_lfo_pitch_offset(semitones);
        }
    }

    /// Phase 4a D48: LFO Brightness offset を全 voice に fan-out。
    pub fn set_lfo_brightness_offset(&mut self, offset: f32) {
        for v in &mut self.voices {
            v.set_lfo_brightness_offset(offset);
        }
    }
}
```

`Voice` trait と `voice.rs` の委譲も同形式で追加。

## KarplusStrong の Phase 4a 拡張

### フィールド追加

```rust
pub struct KarplusStrong {
    // ...Phase 3 既存...

    // Phase 4a D48 新規
    /// LFO からの pitch offset (半音単位、毎 sample 更新)。
    /// `process_sample` で `length_target.target()` に加算して動的 length を計算。
    lfo_pitch_offset_semitones: f32,
    /// LFO からの brightness offset。
    /// `process_sample` で `brightness.next_sample()` に加算して動的値を計算。
    lfo_brightness_offset: f32,
}

impl KarplusStrong {
    pub fn new() -> Self {
        Self {
            // ...Phase 3 既存...
            lfo_pitch_offset_semitones: 0.0,
            lfo_brightness_offset: 0.0,
        }
    }

    /// Phase 4a D48: LFO Pitch offset を毎 sample 更新（VoicePool fan-out 経由）。
    #[inline(always)]
    pub fn set_lfo_pitch_offset(&mut self, semitones: f32) {
        self.lfo_pitch_offset_semitones = semitones;
    }

    #[inline(always)]
    pub fn set_lfo_brightness_offset(&mut self, offset: f32) {
        self.lfo_brightness_offset = offset;
    }
}
```

### `process_sample` 内での適用

Phase 3 既存の length 再計算は `length_target.next_sample()` の差分で skip 判定。Phase 4a で LFO offset を加味する設計:

```rust
// karplus_strong.rs::process_sample（一部抜粋）
pub fn process_sample(&mut self) -> f32 {
    if !self.active && self.energy < ENERGY_THRESHOLD {
        return 0.0;
    }
    self.age_samples = self.age_samples.saturating_add(1);

    // Phase 4a D48: LFO Pitch offset を加算した実効 length target を毎 sample 計算
    // base_length は brightness 補正済み (D37)、pitch_bend は SmoothedValue で滑らか
    let base_target = self.length_target.next_sample();
    // LFO pitch offset は 2^(-offset/12) で長さ係数化
    let lfo_factor = (-self.lfo_pitch_offset_semitones / 12.0).exp2();
    let effective_length = base_target * lfo_factor;

    // 差分が閾値超過時のみ length_int / length_frac を再計算 (R26 対策、Phase 3 既存)
    if (effective_length - self.cached_length).abs() > 1e-5 {
        let buf_max = (self.buffer.len() - FRACTIONAL_DELAY_BUFFER_MARGIN) as f32;
        let clamped = effective_length.clamp(3.0, buf_max);
        self.cached_length = clamped;
        let len_int = clamped.floor() as usize;
        let len_frac = clamped - len_int as f32;
        self.length_int = len_int;
        self.thiran.set_fractional(len_frac);
    }

    // Phase 3 既存: ring buffer read / fractional delay / brightness LPF / loss filter / damping / write
    // ただし brightness の値読み込みで LFO offset を加算
    let brightness_v = (self.brightness.next_sample() + self.lfo_brightness_offset).clamp(0.0, 1.0);
    // 以降の brightness LPF 計算にこの brightness_v を使用

    // ...（Phase 3 既存処理を継承）...
}
```

### `excitation_snapshot` を `#[cfg(test)]` でガード（D45）

Phase 3 既存:
```rust
// karplus_strong.rs（Phase 3）
#[doc(hidden)]
pub fn excitation_snapshot(&self) -> Vec<f32> {
    self.buffer.iter().take(self.length_int).copied().collect()
}
```

Phase 4a で `#[cfg(test)]` ガード:
```rust
// karplus_strong.rs（Phase 4a）
#[cfg(test)]
pub fn excitation_snapshot(&self) -> Vec<f32> {
    self.buffer.iter().take(self.length_int).copied().collect()
}
```

これで production builder からは完全に除外。`#[doc(hidden)]` は `#[cfg(test)]` の場合自動的に non-public のため不要。**呼び出し元（test ファイル）は `#[cfg(test)]` 環境下なので問題なし**。`pnpm build` (release) では `cargo build --target wasm32-unknown-unknown --release` で `#[cfg(test)]` がオフになり、コードは完全削除される。

## Lfo の denormal 対策

`Lfo::process_sample` の出力は phase ∈ [0, 1) で sin / triangle 計算後も `[-1, 1]` の有限値、denormal は発生しない。**追加の `+1e-25 -1e-25` は不要**（Phase 3 の `ModalBodyResonator` / `KarplusStrong` の denormal flush で十分）。

## 統合フロー（Engine::process per sample）

```text
Engine::process (per sample, n iterations):
  ┌─ Phase 4a 新規 ─────────────────────────────────────┐
  │ 1. lfo_value = self.lfo.process_sample()            │
  │ 2. mod_wheel_v = self.mod_wheel.next_sample()       │
  │ 3. pitch_offset = lfo_value × lfo_pitch_depth ×     │
  │                   mod_wheel_v × 0.5                  │
  │ 4. self.pool.set_lfo_pitch_offset(pitch_offset)     │  ← per sample fan-out
  │ 5. brightness_offset = ... × LFO_BRIGHTNESS_SCALE   │
  │ 6. self.pool.set_lfo_brightness_offset(...)         │
  │ 7. volume_multiplier = 1 + ... × LFO_VOLUME_SCALE   │
  └─────────────────────────────────────────────────────┘
  ┌─ Phase 3 既存パス ──────────────────────────────────┐
  │ 8. dry = self.pool.process_sample()                  │
  │     ├─ for each voice (active):                       │
  │     │     ├─ KarplusStrong::process_sample():         │
  │     │     │     ├─ length_target.next_sample()        │
  │     │     │     ├─ lfo_factor = exp2(-lfo_off/12)    │  ← Phase 4a 拡張
  │     │     │     ├─ effective_length = base × factor   │
  │     │     │     ├─ if diff > 1e-5: 再計算 length_int  │
  │     │     │     │                                      │
  │     │     │     ├─ ring buffer read + Thiran apply    │
  │     │     │     ├─ brightness LPF (LFO offset 加算)   │  ← Phase 4a 拡張
  │     │     │     ├─ loss filter                        │
  │     │     │     ├─ damping multiply                   │
  │     │     │     ├─ ring buffer write                  │
  │     │     │     └─ denormal flush                     │
  │     │     └─ amp scaling, energy update                │
  │     └─ pool sum × 1/sqrt(N)                           │
  │ 9. (body_l, body_r) = modal_body.process_sample(dry)  │
  │ 10. wet mix → mixed_l, mixed_r                        │
  │ 11. combined = output_gain × channel_volume ×         │
  │                volume_multiplier                       │  ← Phase 4a 拡張
  │ 12. soft_clip(mixed_l × combined) → output_l[i]       │
  │ 13. soft_clip(mixed_r × combined) → output_r[i]       │
  └─────────────────────────────────────────────────────┘
  ┌─ Phase 3 既存: Voice State stride push ────────────┐
  │ 14. voice_state_sample_counter += n                 │
  │ 15. if counter >= 1024: write_voice_state(); reset  │
  └─────────────────────────────────────────────────────┘
```

## ring buffer 不変条件（Phase 2 / 3 から不変、Phase 4a でも厳守）

- `write_index = (write_index + 1) % buf_len` であり `% length_int` ではないこと
- read 位置も `% buf_len` で計算
- `length_int` の動的変更（Pitch Bend + LFO Pitch offset で変動）でも buffer.len() は不変（`Engine::prepare` で確保された最大長を保持）

## テスト方針

### `crates/dsp-core/tests/lfo_tests.rs` (新規、Lfo 単体)

| テスト名 | 検証内容 |
|---|---|
| `test_lfo_sine_range` | sine 波で min/max が ±1 に収束 |
| `test_lfo_triangle_range` | triangle 波で min/max が ±1 |
| `test_lfo_zero_at_init` | 初回 sample で sine = 0、triangle = -1 |
| `test_lfo_period_matches_rate` | rate=5Hz、9600 sample 後に 1 周期完了 |
| `test_lfo_rate_smoothing` | rate を 1Hz → 8Hz、tau=0.05s で 50ms 後に target 到達 |
| `test_lfo_no_alloc_in_process` | `process_sample` のヒープ確保ゼロ |

### `crates/dsp-core/tests/instrument_tests.rs` (新規、楽器切替)

| テスト名 | 検証内容 |
|---|---|
| `test_apply_instrument_changes_modal_coeffs` | `apply_instrument(Ukulele)` で `modal_body.coeffs_l[0]` が GuitarClassical と異なる |
| `test_apply_instrument_releases_all_voices` | 8 voice active → `apply_instrument` → `pool.active_count() == 0` |
| `test_apply_instrument_clears_sustain_state` | sustain pending あり → `apply_instrument` → pending bitmap = 0 |
| `test_apply_instrument_resets_modal_state` | active modal output → `apply_instrument` → `process_sample(0.0)` が 0.0 を返す |
| `test_apply_instrument_no_alloc` | apply_instrument 100 連打で WASM heap 不変 |
| `test_stereo_spread_per_instrument` | `stereo_spread()` が楽器ごとに異なる値を返す |
| `test_default_instrument_matches_phase3_modes` | Default kind の係数が Phase 3 の `BODY_MODES_L/R` と完全一致 |

### `crates/dsp-core/tests/lfo_destinations_tests.rs` (新規、LFO destinations 統合)

| テスト名 | 検証内容 |
|---|---|
| `test_lfo_pitch_destination_modulates_voice_length` | 1 voice active、LFO Pitch depth=1.0 + Mod Wheel=1.0 で voice の `cached_length` が周期変動 |
| `test_lfo_brightness_destination_modulates_filter` | LFO Brightness depth=1.0 で voice の `last_filter_out` の変調検知 |
| `test_lfo_volume_destination_modulates_output` | LFO Volume depth=1.0 で `output_l/r` の RMS が周期変動 |
| `test_mod_wheel_zero_disables_lfo` | LFO depth=1.0 でも Mod Wheel=0 で出力に LFO 影響なし（Phase 3 互換） |
| `test_mod_wheel_one_full_lfo` | Mod Wheel=1.0 で LFO depth がそのまま反映 |
| `test_lfo_no_alloc_in_engine_process` | LFO + 8 voice + Pitch Bend + CC#7 で `process` のヒープ確保ゼロ |

### `crates/dsp-core/tests/midi_cc_tests.rs` (Phase 3 既存に追加)

| テスト名 | 検証内容 |
|---|---|
| `test_midi_cc_mod_wheel_sets_target` | CC#1 で `mod_wheel.target() == value` |
| `test_midi_cc_mod_wheel_clamps_to_range` | CC#1 value=1.5 / -0.5 で 0..1 に clamp |

### `tests/no_alloc_tests.rs` (Phase 3 既存に追加)

| テスト名 | 検証内容 |
|---|---|
| `test_no_allocation_with_lfo_and_instrument` | 8 voice + LFO active + Mod Wheel + 楽器切替（preserve voice 1 回 + apply 1 回）で voice buffer / LFO 状態 capacity 不変 |

## 性能考慮

### LFO 計算の per-sample コスト

| 演算 | 演算数 |
|---|---|
| `lfo.process_sample()` (sine) | rate next_sample (3) + phase update (3) + sin (5) ≈ 11 |
| `lfo.process_sample()` (triangle) | rate next_sample (3) + phase update (3) + abs + mul (3) ≈ 9 |
| Engine 内 LFO 適用（offsets 計算） | next_sample × 3 (9) + 乗算 (6) ≈ 15 |
| VoicePool fan-out (set_lfo_pitch_offset × 8 voice) | 8 |
| KarplusStrong 内の LFO factor 計算 (exp2) | 7 (per voice) × 8 = 56 |
| **合計 (sine, 8 voice)** | **11 + 15 + 8 + 56 = 90 演算/sample** |

WASM 1 GHz 仮定で +90 / 1e9 × 128 = +0.012 ms/process。pre-research §9.3 想定 +0.0036 ms より大きいが、許容範囲（Phase 3 想定 1.95 ms に対し +0.6%）。triangle 波形なら -2 演算で +0.011 ms。

### 楽器切替の per-event コスト

`apply_instrument` は `pool.all_notes_off()` (64 sample 即時 release) + 16 biquad 係数の `calc_coeffs` (8 mode × 2 ch × 約 30 演算 = 480 演算) + state reset で **総コスト ~600 演算**。1 回の event なので per-sample 換算で無視できる。

## サブモジュール責務一覧

| ファイル | 責務 | Phase 4a 変更 |
|---|---|---|
| `lfo.rs` | LFO（phase + rate SmoothedValue + waveform） | **新規** |
| `engine.rs` | Engine 全体 | LFO + Mod Wheel + 楽器選択フィールド追加、`apply_instrument` / `lfo_set_*` メソッド追加、`process` の per-sample loop に LFO 適用、`handle_midi_cc` の CC#1 分岐有効化 |
| `voice_pool.rs` | 8 音 voice 管理 | `set_lfo_pitch_offset` / `set_lfo_brightness_offset` 追加 |
| `karplus_strong.rs` | KS 単音 | LFO offset フィールド追加、`process_sample` で LFO factor を length / brightness に適用、`excitation_snapshot` を `#[cfg(test)]` でガード |
| `voice.rs` | Voice trait 委譲 | `set_lfo_pitch_offset` / `set_lfo_brightness_offset` 委譲 |
| `traits.rs` | Voice trait 定義 | `set_lfo_pitch_offset` / `set_lfo_brightness_offset` 追加 |
| `modal_body.rs` | Modal Body Resonator | `set_instrument(kind, sample_rate)` メソッド追加 |
| `params.rs` (生成) | ParamDescriptor / BodyMode / 楽器係数 | `InstrumentKind` enum、`BODY_MODES_<INSTRUMENT>_L/R` 12 配列、`STEREO_SPREAD_<INSTRUMENT>` 6 値、`body_modes_for_instrument` / `stereo_spread_for_instrument` ヘルパ |
| `lib.rs` | モジュール公開 | `pub mod lfo;` 追加、必要に応じ `pub use lfo::{Lfo, LfoWaveform, LfoDestination};` |
