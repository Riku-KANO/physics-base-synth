# 03. Phase 2 dsp-core クレート仕様

## 目的

Phase 2 で `dsp-core` に追加する 4 モジュール（`voice_pool.rs` / `fractional_delay.rs` / `note_allocator.rs` / `hold_stack.rs`）と、既存モジュールへの変更（`params.rs` のコード生成出力化、`KarplusStrong` への fractional delay 統合、`Engine` の VoicePool 化、`Voice` trait の拡張）を定義する。Phase 1 で確立したリアルタイム制約とテスト方針はそのまま継承する。

## 他文書との関係

- 上流: [`02-architecture.md`](./02-architecture.md)（モノレポ構成、メモリレイアウト変更、ParamDescriptor codegen パイプライン）
- 並列: [`04-wasm-audio-spec.md`](./04-wasm-audio-spec.md)（dsp-core を呼び出す側）
- 参考: [Phase 2 pre-research §1〜6](./pre-research.md)（ポリフォニー / Lagrange / hold stack の文献整理）
- Phase 1 参照: [Phase 1 03 章](../2026-05-06-001-mvp/03-dsp-core-spec.md)（KarplusStrong / Engine / SmoothedValue / XorShift32 の既存 API、リアルタイム制約遵守ルール、テスト方針）— **本書で明示的に変更しない部分はすべて Phase 1 の記述を継承**

## クレート設定

[Phase 1 03 章 §クレート設定](../2026-05-06-001-mvp/03-dsp-core-spec.md#クレート設定) を **完全維持**。`Cargo.toml` の依存ゼロ、`crate-type = ["rlib"]`、`std` feature flag は Phase 2 でも変更なし（D23）。`heapless` 等の外部 crate を追加せず、Phase 2 で追加する固定配列構造（VoicePool / HoldStack）はすべて自前で実装する。

## モジュール一覧（Phase 2）

| ファイル | Phase 1 | Phase 2 |
|---|---|---|
| `lib.rs` | 既存モジュール宣言 | 4 モジュール宣言を追加 |
| `traits.rs` | `AudioProcessor`、`Voice` trait | `Voice` trait に 3 メソッド追加（D19）|
| `params.rs` | 手書き enum + 範囲定数 | **コード生成出力に置換**（git commit、D25）|
| `smoothing.rs` | `SmoothedValue` | **完全維持** |
| `rng.rs` | `XorShift32` | **完全維持** |
| `karplus_strong.rs` | `KarplusStrong` | fractional delay 統合、`Voice` trait の追加メソッド対応 |
| `voice.rs` | `Voice` trait の `KarplusStrong` 向け実装 | 追加メソッドの委譲を追記 |
| `engine.rs` | 単一 voice 保持の Engine | VoicePool / HoldStack / SynthMode 化で大幅書き換え |
| **`voice_pool.rs`** | — | **新規**: `VoicePool<const N: usize>` |
| **`fractional_delay.rs`** | — | **新規**: Lagrange 3 次補間係数 |
| **`note_allocator.rs`** | — | **新規**: voice stealing 戦略 |
| **`hold_stack.rs`** | — | **新規**: `LinearStack<u8, MAX_HELD>` |

### `lib.rs` の更新

```rust
pub mod engine;
pub mod fractional_delay;
pub mod hold_stack;
pub mod karplus_strong;
pub mod note_allocator;
pub mod params;
pub mod rng;
pub mod smoothing;
pub mod traits;
pub mod voice;
pub mod voice_pool;
```

## ParamDescriptor 構造

### 構造体定義（`params.rs` 生成出力の一部）

```rust
#[derive(Debug, Clone, Copy)]
pub struct ParamDescriptor {
    pub id: u32,
    pub name: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub smoothing_tau: f32,  // SmoothedValue 設定用、秒単位
}

impl ParamDescriptor {
    pub const fn clamp(&self, value: f32) -> f32 {
        if value < self.min { self.min }
        else if value > self.max { self.max }
        else { value }
    }
}
```

`f32::clamp` ではなく自前 const fn を持つのは、Phase 2 で生成テーブルを `const PARAM_DESCRIPTORS` として埋め込むため、const context で使えるようにするため。

### `params.json` から `params.rs` への生成出力例

`scripts/gen-params.mjs` が以下のような Rust ソースを出力する想定（[02 章](./02-architecture.md#paramdescriptor-コード生成パイプライン)）。

```rust
// AUTO-GENERATED FROM params.json — DO NOT EDIT
// Run `pnpm gen:params` to regenerate.

#[derive(Debug, Clone, Copy)]
pub struct ParamDescriptor {
    pub id: u32,
    pub name: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub smoothing_tau: f32,
}

impl ParamDescriptor {
    pub const fn clamp(&self, value: f32) -> f32 {
        if value < self.min { self.min }
        else if value > self.max { self.max }
        else { value }
    }
}

#[repr(u32)]
#[non_exhaustive]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ParamId {
    Damping = 0,
    Brightness = 1,
    OutputGain = 2,
}

impl ParamId {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Damping),
            1 => Some(Self::Brightness),
            2 => Some(Self::OutputGain),
            _ => None,
        }
    }

    pub fn descriptor(&self) -> &'static ParamDescriptor {
        &PARAM_DESCRIPTORS[*self as usize]
    }
}

pub const DAMPING_DESCRIPTOR: ParamDescriptor = ParamDescriptor {
    id: 0, name: "Damping", min: 0.90, max: 0.9999, default: 0.996, smoothing_tau: 0.02,
};
pub const BRIGHTNESS_DESCRIPTOR: ParamDescriptor = ParamDescriptor {
    id: 1, name: "Brightness", min: 0.0, max: 1.0, default: 0.5, smoothing_tau: 0.02,
};
pub const OUTPUT_GAIN_DESCRIPTOR: ParamDescriptor = ParamDescriptor {
    id: 2, name: "OutputGain", min: 0.0, max: 1.5, default: 0.8, smoothing_tau: 0.01,
};

pub const PARAM_DESCRIPTORS: [ParamDescriptor; 3] = [
    DAMPING_DESCRIPTOR,
    BRIGHTNESS_DESCRIPTOR,
    OUTPUT_GAIN_DESCRIPTOR,
];

// Phase 1 互換の範囲定数（既存コードからの参照のため維持、descriptor.min / .max への移行は段階的に）
pub const DAMPING_MIN: f32 = DAMPING_DESCRIPTOR.min;
pub const DAMPING_MAX: f32 = DAMPING_DESCRIPTOR.max;
pub const DAMPING_DEFAULT: f32 = DAMPING_DESCRIPTOR.default;

pub const BRIGHTNESS_MIN: f32 = BRIGHTNESS_DESCRIPTOR.min;
pub const BRIGHTNESS_MAX: f32 = BRIGHTNESS_DESCRIPTOR.max;
pub const BRIGHTNESS_DEFAULT: f32 = BRIGHTNESS_DESCRIPTOR.default;

pub const OUTPUT_GAIN_MIN: f32 = OUTPUT_GAIN_DESCRIPTOR.min;
pub const OUTPUT_GAIN_MAX: f32 = OUTPUT_GAIN_DESCRIPTOR.max;
pub const OUTPUT_GAIN_DEFAULT: f32 = OUTPUT_GAIN_DESCRIPTOR.default;
```

> **drift 防止**: Phase 1 では `params.rs` と `messages.ts` が手動同期で drift リスクがあった。Phase 2 では `params.json` を単一ソースとし、`scripts/check-params-sync.mjs` が CI で diff を検知（[02 章](./02-architecture.md#scriptscheck-params-syncmjs-の責務)、[06 章 F14/F15](./06-build-and-verify.md)）。

### Phase 1 互換性

Phase 1 で公開された `ParamId::Damping/Brightness/OutputGain`、`from_u32`、`DAMPING_MIN/MAX/DEFAULT` 等の定数はすべて生成出力でも引き続き提供する。Phase 1 を import している既存コード（`crates/dsp-core/src/engine.rs` 等）は変更不要。

## 公開 trait の拡張

### `traits.rs` の Phase 2 版

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

    // Phase 2 で追加（D19）
    /// 現在発音中のノート番号。Phase 2 で voice stealing の same-note-replace 判定に使用
    fn note_id(&self) -> Option<u8>;
    /// このボイスが最後に note_on されてからの経過サンプル数。voice stealing の oldest 判定に使用
    fn age(&self) -> u32;
    /// 現在の振幅推定値（envelope tracking ベース）。voice stealing の quietest 判定に使用
    fn amplitude(&self) -> f32;
}
```

> `note_id` / `age` / `amplitude` は **Voice stealing 戦略の判定** のみに使う。`set_*`（damping / brightness）は inherent method のままで、VoicePool が `KarplusStrong` 固有 API として呼ぶ。Phase 3 で他楽器を追加する場合も同じ trait シグネチャで透過的に扱える。

## XorShift32 / SmoothedValue

[Phase 1 03 章 §SmoothedValue](../2026-05-06-001-mvp/03-dsp-core-spec.md#smoothedvalue) と [§XorShift32](../2026-05-06-001-mvp/03-dsp-core-spec.md#xorshift32) を **完全維持**。Phase 2 でも変更なし。各ボイスが個別に `XorShift32` インスタンスを保持する（VoicePool 構築時に異なるシードを与える、後述）。

## Fractional delay

### `fractional_delay.rs`

Lagrange 3 次補間（D14）の係数計算と補間関数を提供する。

```rust
/// Lagrange 3 次補間係数。fractional 部 d ∈ [0, 1) に対する 4 つの重み
#[derive(Debug, Clone, Copy)]
pub struct LagrangeCoeffs {
    pub h0: f32,  // x[n - D_int + 1]   への重み
    pub h1: f32,  // x[n - D_int]       への重み
    pub h2: f32,  // x[n - D_int - 1]   への重み
    pub h3: f32,  // x[n - D_int - 2]   への重み
}

impl LagrangeCoeffs {
    /// d ∈ [0, 1) に対する Lagrange 3 次補間係数を計算
    /// h_0(d) = -d(d-1)(d-2) / 6
    /// h_1(d) = (d+1)(d-1)(d-2) / 2
    /// h_2(d) = -(d+1)d(d-2) / 2
    /// h_3(d) = (d+1)d(d-1) / 6
    pub fn new(d: f32) -> Self {
        let d_clamped = d.clamp(0.0, 1.0);
        let dm1 = d_clamped - 1.0;
        let dm2 = d_clamped - 2.0;
        let dp1 = d_clamped + 1.0;
        Self {
            h0: -d_clamped * dm1 * dm2 / 6.0,
            h1: dp1 * dm1 * dm2 / 2.0,
            h2: -dp1 * d_clamped * dm2 / 2.0,
            h3: dp1 * d_clamped * dm1 / 6.0,
        }
    }

    /// 4 サンプル積和で補間値を返す
    /// 引数順は時間的に新しい → 古い（x_n_minus_D_plus_1 が最も新しい）
    #[inline]
    pub fn apply(&self, x_minus: f32, x_zero: f32, x_plus_1: f32, x_plus_2: f32) -> f32 {
        self.h0 * x_minus + self.h1 * x_zero + self.h2 * x_plus_1 + self.h3 * x_plus_2
    }
}
```

### 設計上の注意点

- 係数は **`note_on` 時に 1 度計算してキャッシュ**（D26）。process 内では `apply` の積和 4 回のみ実行
- `d.clamp(0.0, 1.0)` で範囲外を吸収（`note_on` で `length_frac` が確実に `[0, 1)` 内に入るが防御的に clamp）
- denormal 対策は `KarplusStrong::process_sample` 末尾の `+1e-25 -1e-25` で吸収するため、Lagrange 内部では行わない
- インライン化（`#[inline]`）で関数呼び出しオーバーヘッドを削減

### ユニットテスト方針

| テスト名 | 内容 |
|---|---|
| `test_lagrange_d_zero_gives_x_zero` | `d = 0.0` のとき係数が `(0, 1, 0, 0)` で、`apply` が `x_zero` をそのまま返す |
| `test_lagrange_d_one_gives_x_plus_1` | `d = 1.0` のとき係数が `(0, 0, 1, 0)`（実質）で、`apply` が `x_plus_1` を返す |
| `test_lagrange_coeffs_sum_to_one` | 任意の `d ∈ [0, 1]` で `h0 + h1 + h2 + h3 ≈ 1.0`（DC ゲイン保存）|
| `test_lagrange_pitch_a4` | KarplusStrong 統合後、`note_on(440Hz)` で出力周波数が 440Hz ± 0.5%（F12）|
| `test_lagrange_pitch_a1` | KarplusStrong 統合後、`note_on(55Hz)` で出力周波数が 55Hz ± 0.5%（F13、Phase 1 課題解消）|

## KarplusStrong（Phase 2 改修）

### 構造体の変更

```rust
use crate::fractional_delay::LagrangeCoeffs;
use crate::params::{BRIGHTNESS_DEFAULT, DAMPING_DEFAULT};
use crate::rng::XorShift32;
use crate::smoothing::SmoothedValue;
use crate::traits::Voice;

const NOTE_OFF_DAMPING: f32 = 0.95;
const ENERGY_RISE: f32 = 0.001;
const ENERGY_DECAY: f32 = 0.999;
const ENERGY_THRESHOLD: f32 = 1.0e-9;
const MIN_FREQ_HZ: f32 = 27.5;
const LAGRANGE_BUFFER_MARGIN: usize = 3;  // Phase 2: 補間カーネルの過去サンプル参照に必要な余裕（D27）

pub struct KarplusStrong {
    buffer: Vec<f32>,
    write_index: usize,
    length_int: usize,           // Phase 2: 整数部
    lagrange: LagrangeCoeffs,    // Phase 2: 分数部の補間係数（note_on 時にキャッシュ、D26）
    damping: SmoothedValue,
    brightness: SmoothedValue,
    last_filter_out: f32,
    energy: f32,
    active: bool,
    rng: XorShift32,
    sample_rate: f32,
    note_off_target_damping: f32,
    // Phase 2 追加（Voice trait の追加メソッド対応、D19）
    current_note: Option<u8>,    // 現在発音中の MIDI ノート番号
    age_samples: u32,            // 最後の note_on からの経過サンプル数
}
```

### `prepare` の変更

```rust
impl KarplusStrong {
    pub fn prepare(&mut self, sample_rate: f32, _max_block_size: usize) {
        self.sample_rate = sample_rate;

        // 27.5Hz + Lagrange 3 次補間分の +3 サンプル余裕（D27）
        let max_buffer_len = (sample_rate / MIN_FREQ_HZ).ceil() as usize + LAGRANGE_BUFFER_MARGIN;
        self.buffer = vec![0.0; max_buffer_len];

        self.damping.set_time_constant(sample_rate, 0.02);
        self.brightness.set_time_constant(sample_rate, 0.02);

        self.write_index = 0;
        self.length_int = 0;
        self.lagrange = LagrangeCoeffs::new(0.0);
        self.last_filter_out = 0.0;
        self.energy = 0.0;
        self.active = false;
        self.current_note = None;
        self.age_samples = 0;
    }
}
```

### `note_on` の変更

`Engine::note_on` から freq_hz だけでなく midi note も渡すように API を拡張する（Phase 1 既存の freq_hz 引数は維持し、midi を追加）。または **`Engine` 側で `note_id` を VoicePool に渡し、KarplusStrong には freq_hz のみ渡す** 設計が C ABI 互換維持に有利。Phase 2 では後者を採用し、`current_note` の設定は VoicePool が `note_on_with_id` 経由で行う。

```rust
impl KarplusStrong {
    /// freq_hz を整数部 + 分数部に分解し、Lagrange 係数を計算。
    /// buffer.len() は `(sample_rate / 27.5).ceil() + LAGRANGE_BUFFER_MARGIN` で確保済み (D27)。
    /// length_int の上限を `buffer.len() - LAGRANGE_BUFFER_MARGIN` にすることで、
    /// process_sample で参照する 4 点（write_index から最大 length_int + 2 サンプル前）が
    /// 必ず buffer 範囲内に収まるよう保証する。
    pub fn note_on(&mut self, freq_hz: f32, velocity: f32) {
        let raw_len = self.sample_rate / freq_hz.max(1.0);
        // length_int の最小は 3（Lagrange 4 点参照のため）、上限は buffer.len() - LAGRANGE_BUFFER_MARGIN
        let max_len = self.buffer.len().saturating_sub(LAGRANGE_BUFFER_MARGIN);
        let len_int = (raw_len.floor() as usize).clamp(3, max_len);
        let len_frac = (raw_len - len_int as f32).clamp(0.0, 1.0);

        self.length_int = len_int;
        self.lagrange = LagrangeCoeffs::new(len_frac);

        // バッファ全体をゼロクリアしてから length_int 分を励振。
        // 前回 note_on の残骸が Lagrange 4 点に混じるのを防ぐため全領域を初期化
        // （buffer.len() が ~1749 程度で O(N)、note_on 時のみで process 中ではないため D4 維持）。
        for v in self.buffer.iter_mut() {
            *v = 0.0;
        }
        // 励振配置: buffer[0..length_int] にノイズを書き、write_index = length_int から開始する。
        // これにより note_on 直後 1 サンプル目の process_sample で
        //   read_z = (length_int + buf_len - length_int) % buf_len = 0       → buffer[0] = 励振
        //   read_m = (length_int + buf_len - length_int + 1) % buf_len = 1   → buffer[1] = 励振
        //   read_p1 = buf_len - 1, read_p2 = buf_len - 2 → ゼロ（励振範囲外）
        // となり、Lagrange 補間値が励振ノイズの寄与を持つ非ゼロ出力になる（h0 + h1 が支配的）。
        // 一方 write_index = 0 + buffer[0..length_int] 励振にすると初回 read 位置がゼロ領域に
        // なり、励振サンプルがゼロで上書きされてしまうため発音しない（High 修正）。
        for i in 0..len_int {
            self.buffer[i] = self.rng.next_unit_bipolar() * velocity;
        }

        self.write_index = len_int;  // 励振範囲の直後から書き込み開始
        self.last_filter_out = 0.0;
        self.energy = velocity * velocity;
        self.active = true;
        self.age_samples = 0;
    }

    /// VoicePool から呼ぶ拡張版。note_id を内部に保持し、Voice trait の note_id() で公開する
    pub fn note_on_with_id(&mut self, midi_note: u8, freq_hz: f32, velocity: f32) {
        self.note_on(freq_hz, velocity);
        self.current_note = Some(midi_note);
    }

    pub fn note_off(&mut self) {
        self.damping.set_target(self.note_off_target_damping);
        self.current_note = None;
    }
}
```

### `process_sample` の変更

Phase 2 では **ピッチ精度を実効化するため、Lagrange 3 次補間値をフィードバックループ内に取り込む**（出力経路だけに補間を適用すると、KS の周期はフィードバックループ内のディレイで決まるため整数ディレイ由来の周期誤差が残ってしまう）。Lagrange 3 次補間は **FIR（有限インパルス応答）** であり、係数和は 1.0 に正規化されている（DC ゲイン保存）。これは個別フィルタとしての安定性は保証するが、フィードバックループ全体（補間 → LPF → damping → 書き戻し）の安定性は damping < 1.0 と LPF が低域通過であることに依存する。Phase 1 で damping ∈ [0.90, 0.9999] / brightness ∈ [0, 1] の範囲で安定動作が確認済みであり、Phase 2 で補間がループに加わっても **安定性リスクは低い**（Smith *Physical Audio Signal Processing* の `pluckString` モデルと同方針）。Thiran allpass のような IIR 補間は loop 内で別途安定性議論が必要だが、Phase 2 採用案 (D14) は FIR Lagrange なのでこのリスクが少ない。長時間動作での発散・NaN・異常ピーク不発生は §テスト方針の `test_long_term_stability_high_damping` で確認する。

**重要**: `% length_int` で添字を回すと Lagrange の 4 点が時系列順に並ばない。`length_int` 分の周期内で剰余を取ると `read_plus_1 / read_plus_2` がリング上の「より新しい」サンプル側に巻き込まれ、`x[n - D_int - 1] / x[n - D_int - 2]` を取れない。Phase 2 では **buffer 全体（`buffer.len()` = `(sample_rate / 27.5).ceil() + LAGRANGE_BUFFER_MARGIN`）で剰余を取り**、`write_index` から `length_int + 2` サンプル前までを参照可能にする。`LAGRANGE_BUFFER_MARGIN = 3` の確保（D27、`prepare`）はこの目的のため。

```rust
impl KarplusStrong {
    #[inline]
    pub fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        // Lagrange 3 次補間で fractional read（フィードバックループ内）。
        // write_index は「次に書き込む位置」（pre-write index）。
        // y[n - k] に対応する添字は (write_index + buf_len - k) % buf_len。
        // 期待 read 位置 = D_int + frac、frac は LagrangeCoeffs に焼き込み済み。
        // 4 点（時系列で新しい順 → 古い順）: y[n - D_int + 1], y[n - D_int], y[n - D_int - 1], y[n - D_int - 2]
        let buf_len = self.buffer.len();   // フル buffer 長で剰余を取る（length_int ではない）
        let d_int = self.length_int;
        let base = self.write_index + buf_len;  // 巻き戻し時のアンダーフロー回避用オフセット
        let read_m  = (base - d_int + 1) % buf_len;  // x[n - D_int + 1]、最新（h0 重み）
        let read_z  = (base - d_int)     % buf_len;  // x[n - D_int]、中心（h1 重み）
        let read_p1 = (base - d_int - 1) % buf_len;  // x[n - D_int - 1]（h2 重み）
        let read_p2 = (base - d_int - 2) % buf_len;  // x[n - D_int - 2]、最古（h3 重み）

        // 補間値が KS のループ内 read 値そのものになる（出力もこれを返す）
        let read_value = self.lagrange.apply(
            self.buffer[read_m],
            self.buffer[read_z],
            self.buffer[read_p1],
            self.buffer[read_p2],
        );

        // ワンポール LPF（Phase 1 同等の式、ただし入力を補間値ベースに変更）
        // Phase 1 では `0.5 * (current + next)` の 2 サンプル平均を使ったが、
        // Phase 2 では Lagrange 補間値を 1 入力として使う。LPF 自体は同じ係数。
        let b = self.brightness.next_sample();
        let filtered = b * read_value + (1.0 - b) * self.last_filter_out;
        self.last_filter_out = filtered;

        // 減衰（Phase 1 と同じ）
        let d = self.damping.next_sample();
        let mut damped = d * filtered;

        // denormal flush（D6 維持）
        damped += 1.0e-25;
        damped -= 1.0e-25;

        // ディレイラインへ書き戻し（write_index 位置に書く、buf_len で巻く）
        self.buffer[self.write_index] = damped;
        self.write_index = (self.write_index + 1) % buf_len;

        // envelope tracking（Phase 1 と同じ）
        self.energy = self.energy * ENERGY_DECAY + damped * damped * ENERGY_RISE;
        if self.energy < ENERGY_THRESHOLD {
            self.active = false;
        }

        // age 増加（Phase 2 追加）
        self.age_samples = self.age_samples.saturating_add(1);

        read_value
    }
}
```

> **設計上の注意**: Phase 2 では Lagrange 補間値を **フィードバックループ内** で使い、出力にも同じ補間値を返す。Karplus–Strong の周期はフィードバックループ内のディレイで決まるため、出力経路だけに補間を入れるとピッチ精度は改善しない（F12/F13 が達成できない）。**剰余は `buffer.len()` で取る（`length_int` ではない）**: Lagrange 4 点は時系列で `n-D_int+1, n-D_int, n-D_int-1, n-D_int-2` の順に並ぶ必要があり、`% length_int` で回すとリング上で新しい側に巻き戻ってしまう。`LAGRANGE_BUFFER_MARGIN = 3`（D27）で `buffer.len() >= length_int + 3` を保証することで、4 点が時系列順に取れる。Lagrange 3 次補間自体は FIR で係数和 1.0（DC ゲイン保存）、ループ全体の安定性は damping < 1.0 と LPF（低域通過）の組み合わせに依存する。Phase 1 で damping ∈ [0.90, 0.9999] / brightness ∈ [0, 1] の動作が確認済みであり、Phase 2 で補間がループに加わっても **安定性リスクは低い**（Thiran allpass IIR と異なり別途の極配置議論は不要）。LPF 入力は Phase 1 の「2 サンプル平均」から「Lagrange 補間値 1 入力」に変わるが、Lagrange 補間自体が周辺サンプルを重み付けして 1 サンプルに集約しているため等価以上の平滑化（pre-research §3、Smith *Physical Audio Signal Processing* `pluckString` モデル）。長時間動作での安定性は §テスト方針の `test_long_term_stability_high_damping` で 30 秒分・damping=0.9999・複数 brightness 値・低域 (A1) と高域 (C8) で発散・NaN・異常ピーク不発生を確認する。

### Voice trait の追加メソッド実装

`voice.rs` に Phase 1 既存の委譲に加えて 3 メソッドを追記:

```rust
impl Voice for KarplusStrong {
    fn note_on(&mut self, freq_hz: f32, velocity: f32) {
        KarplusStrong::note_on(self, freq_hz, velocity);
    }
    fn note_off(&mut self) {
        KarplusStrong::note_off(self);
    }
    fn process_sample(&mut self) -> f32 {
        KarplusStrong::process_sample(self)
    }
    fn is_active(&self) -> bool {
        KarplusStrong::is_active(self)
    }
    // Phase 2 追加
    fn note_id(&self) -> Option<u8> {
        self.current_note
    }
    fn age(&self) -> u32 {
        self.age_samples
    }
    fn amplitude(&self) -> f32 {
        self.energy.sqrt()  // RMS-like 推定
    }
}
```

> `amplitude` は `energy` の平方根として近似する（envelope tracking が `energy = energy * 0.999 + sample² * 0.001` で動いているため、平方根が振幅 RMS の近似値になる）。

## VoicePool

### `voice_pool.rs`

```rust
use crate::karplus_strong::KarplusStrong;
use crate::note_allocator::{select_voice_for_steal, StealResult};
use crate::params::PARAM_DESCRIPTORS;
use crate::traits::Voice;

pub const POLYPHONY: usize = 8;  // D12: N=8 固定

pub struct VoicePool<const N: usize> {
    voices: [KarplusStrong; N],
    sample_rate: f32,
}

impl<const N: usize> VoicePool<N> {
    pub fn new() -> Self {
        // 各ボイスが異なる初期シードを持つように配列を構築
        // const generic で配列初期化するため core::array::from_fn を使う
        Self {
            voices: core::array::from_fn(|i| {
                let mut ks = KarplusStrong::new();
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

    /// note_on をボイスに割り当てる。
    /// 戦略（D13）:
    /// 1. 同じ midi_note を発音中のボイスがあれば再励振（same-note-replace）
    /// 2. 非アクティブのボイスがあれば最若番に割当
    /// 3. 全ボイスがアクティブなら note_allocator::select_voice_for_steal で選定（energy 閾値以下のうち最古、なければ最古）
    ///
    /// 戻り値: 割り当てたボイスの index。Engine 側がこの index に対してのみ damping を復元する
    /// （release 中の他ボイスを誤って復活させないため、High 2 修正）
    pub fn note_on(&mut self, midi_note: u8, freq_hz: f32, velocity: f32) -> usize {
        // (1) same-note-replace
        for (i, v) in self.voices.iter_mut().enumerate() {
            if v.note_id() == Some(midi_note) {
                v.note_on_with_id(midi_note, freq_hz, velocity);
                return i;
            }
        }
        // (2) 空きボイス検索
        for (i, v) in self.voices.iter_mut().enumerate() {
            if !v.is_active() {
                v.note_on_with_id(midi_note, freq_hz, velocity);
                return i;
            }
        }
        // (3) voice stealing
        let steal_index = select_voice_for_steal(&self.voices);
        match steal_index {
            StealResult::Index(i) => {
                self.voices[i].note_on_with_id(midi_note, freq_hz, velocity);
                i
            }
        }
    }

    /// 指定 index のボイスのみに damping を設定（Engine::note_on から呼ぶ）。
    /// fan-out 版 set_damping は release 中ボイスを復活させてしまうため、
    /// 該当ボイスのみ Phase 1 互換の damping 復元を行うために用意（High 2 修正）。
    pub fn set_damping_voice(&mut self, index: usize, value: f32) {
        if let Some(v) = self.voices.get_mut(index) {
            let clamped = PARAM_DESCRIPTORS[0].clamp(value);
            v.set_damping(clamped);
        }
    }

    /// 該当 midi_note を発音中のボイスに note_off を発火（同名複数ボイスは想定外だが全件適用）
    pub fn note_off(&mut self, midi_note: u8) {
        for v in self.voices.iter_mut() {
            if v.note_id() == Some(midi_note) {
                v.note_off();
            }
        }
    }

    /// パラメータを全ボイスに fan-out
    pub fn set_damping(&mut self, value: f32) {
        let clamped = PARAM_DESCRIPTORS[0].clamp(value);  // Damping は id=0
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

    /// 全ボイスを reset（Engine::reset から呼ばれる）
    pub fn reset(&mut self) {
        for v in self.voices.iter_mut() {
            v.reset();
        }
    }

    /// process_sample を全ボイスで実行し、合算ゲインで割り戻して返す（D20: 1/sqrt(N) スケール）
    #[inline]
    pub fn process_sample(&mut self) -> f32 {
        let mut sum = 0.0;
        for v in self.voices.iter_mut() {
            sum += v.process_sample();
        }
        // 1/sqrt(N) スケール（知覚的にエネルギー保存）
        const SCALE: f32 = 1.0 / 2.828_427_2;  // 1/sqrt(8) = 0.353553...
        sum * SCALE
    }

    /// アクティブなボイス数（テスト・診断用、Phase 2 では C ABI で公開しない、D22）
    pub fn active_count(&self) -> usize {
        self.voices.iter().filter(|v| v.is_active()).count()
    }
}

impl<const N: usize> Default for VoicePool<N> {
    fn default() -> Self {
        Self::new()
    }
}
```

### `KarplusStrong::set_seed` 追加

VoicePool 構築時に各ボイスへ異なるシードを与えるため、Phase 2 で `KarplusStrong` に `set_seed(u32)` を追加:

```rust
impl KarplusStrong {
    pub fn set_seed(&mut self, seed: u32) {
        self.rng = XorShift32::new(seed);
    }
}
```

### N=8 ハードコードについて

`pub const POLYPHONY: usize = 8;` を定数として公開し、`Engine` は `VoicePool<POLYPHONY>` で型を確定させる（D12）。const generic を使うのは将来 N=4 や N=16 への切替を可能にするためで、Phase 2 実装時点では値を 1 つに固定する。

### 1/sqrt(N) スケールの根拠と限界（D20）

KS の各ボイスは励振直後にピーク振幅約 1.0（velocity に比例）まで届くため、最悪ケースで `sum = 8.0`。`1/sqrt(8) ≈ 0.354` でスケールすると `8.0 * 0.354 ≈ 2.83`。実際には 8 ボイスのピークが完全に同位相で揃うことは稀（励振ノイズはボイスごとに独立した seed で無相関）で、エンベロープの減衰位相も揃わないため、**統計的には sum のピークが 2.0 を超える場面は稀**。代表的な演奏（複数のキーが時間差で押される、すべて全力ではない）では 1/sqrt(N) スケール後のピークは 0.5-1.0 程度に収まる。

ただし「**全ボイス全力 + 同時 note_on + OutputGain 最大 (1.5)**」という最悪ケースでは:
- 1/sqrt(N) 適用後の信号ピーク `≈ 2.83`
- OutputGain 最大時の最終出力 `2.83 * 1.5 ≈ 4.24` → ハードクリップ域

Phase 2 ではこの最悪ケースに対する soft clip / limiter を **入れない**（リアルタイム制約の遵守を優先、コードシンプル維持）。代わりに以下の方針:

1. **検証 F24 を「常用範囲で音割れなし」に緩和**: OutputGain ≤ 1.0 + 通常の押下パターン（時間差あり、ベロシティ平均的）でハードクリップ歪みが知覚されないこと
2. **OutputGain 1.5 上限は維持**（Phase 1 互換、独奏時の音量稼ぎ用途）。ただし「OutputGain > 1.0 で 8 音同時全力では歪みが出る場合あり」を [`06-build-and-verify.md` リスク R22](./06-build-and-verify.md#リスクと対策表) と F24 注記で明示
3. **Phase 3 で soft clip / limiter 検討**: `tanh(x)` ベース、または look-ahead limiter。実装は Engine::process 末尾に挿入する位置として確保

`Engine::process` の OutputGain ループは Phase 1 と同じ実装（`s = raw * g; output_l[i] = s; output_r[i] = s;`）を維持する。

## Note allocator（voice stealing 戦略）

### `note_allocator.rs`

```rust
use crate::traits::Voice;

const ENERGY_THRESHOLD_FOR_STEAL: f32 = 1.0e-3;  // この値以下なら「ほぼ静か」とみなす

#[derive(Debug, Clone, Copy)]
pub enum StealResult {
    Index(usize),
}

/// voice stealing 戦略（D13、VoicePool::note_on 内の (1)/(2) 段で割当が決まらず全ボイスが
/// アクティブな状態に到達した場合のみ呼ばれる）:
/// 1. amplitude が ENERGY_THRESHOLD_FOR_STEAL 以下のボイスのうち最も古いもの（age 最大）を返す
/// 2. 該当なし（全ボイスがある程度の振幅で鳴っている）なら最も古いボイスを返す
///
/// この戦略は「閾値以下なら最古を優先」であり「最も静かなボイス（min amplitude）」とは異なる。
/// 閾値ベースにする理由は (a) クリック対策（ほぼ無音のボイスを優先的に犠牲にすると知覚されにくい、D28）、
/// (b) min を取ると「ごくわずかに静かなボイス」が連続的に犠牲になり同じ位置で stealing 偏りが起きる、
/// (c) 閾値以下を「ほぼ無音グループ」と扱い、その中で最古を選ぶことで知覚と公平性のバランスを取る。
pub fn select_voice_for_steal<V: Voice, const N: usize>(voices: &[V; N]) -> StealResult {
    // (1) energy 閾値以下のボイスを探す（D28: クリック対策で静かなボイス優先）
    let mut best_quiet: Option<(usize, u32)> = None;
    for (i, v) in voices.iter().enumerate() {
        if v.amplitude() < ENERGY_THRESHOLD_FOR_STEAL {
            if let Some((_, best_age)) = best_quiet {
                if v.age() > best_age {
                    best_quiet = Some((i, v.age()));
                }
            } else {
                best_quiet = Some((i, v.age()));
            }
        }
    }
    if let Some((i, _)) = best_quiet {
        return StealResult::Index(i);
    }

    // (2) フォールバック: 最も古いボイス
    let mut best_oldest = (0, voices[0].age());
    for (i, v) in voices.iter().enumerate().skip(1) {
        if v.age() > best_oldest.1 {
            best_oldest = (i, v.age());
        }
    }
    StealResult::Index(best_oldest.0)
}
```

### Voice stealing 時のクリック対策（D28）

KarplusStrong の `note_on` がバッファをノイズで上書きするため、stealing 時に追加の fade out 処理を入れずとも自然な切り替わりになる。energy 閾値以下を優先することで「ほぼ無音のボイス」が犠牲になるため、知覚的に検知しにくい。F23（voice stealing 連打でクリックなし）で実機検証する。

### ユニットテスト方針

| テスト名 | 内容 |
|---|---|
| `test_steal_picks_quietest_voice` | 8 ボイスのうち 1 つだけ amplitude が低い状態で `select_voice_for_steal` がそのインデックスを返す |
| `test_steal_falls_back_to_oldest` | 全ボイスが loud（閾値超）の状態で `select_voice_for_steal` が age 最大のボイスを返す |
| `test_steal_among_quiet_voices_picks_oldest` | 複数のボイスが閾値以下で、その中で age 最大が選ばれる |

## Hold note stack

### `hold_stack.rs`

```rust
pub const MAX_HELD: usize = 16;  // D16: 容量 16

/// 固定容量 N の自前 LIFO スタック（D23: heapless 等の依存追加なし）
/// 溢れ時は最古を破棄（D16）
pub struct LinearStack<T: Copy + PartialEq, const N: usize> {
    items: [Option<T>; N],
    len: usize,
}

impl<T: Copy + PartialEq, const N: usize> LinearStack<T, N> {
    pub fn new() -> Self {
        Self {
            items: [None; N],
            len: 0,
        }
    }

    /// push: 容量超なら最古（先頭）を破棄して詰める
    pub fn push(&mut self, value: T) {
        if self.len < N {
            self.items[self.len] = Some(value);
            self.len += 1;
        } else {
            // 最古を破棄、全要素を 1 つずつ前にシフト、新規を末尾に
            for i in 0..(N - 1) {
                self.items[i] = self.items[i + 1];
            }
            self.items[N - 1] = Some(value);
        }
    }

    /// 指定値を 1 件削除（最初に見つかったもの）。残りの要素を詰める
    pub fn remove(&mut self, value: T) {
        let mut found = false;
        for i in 0..self.len {
            if !found && self.items[i] == Some(value) {
                found = true;
            }
            if found && i + 1 < self.len {
                self.items[i] = self.items[i + 1];
            } else if found {
                self.items[i] = None;
            }
        }
        if found {
            self.len -= 1;
        }
    }

    /// 最後に push された値を返す（pop はしない）
    pub fn top(&self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.items[self.len - 1]
        }
    }

    pub fn clear(&mut self) {
        self.items = [None; N];
        self.len = 0;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T: Copy + PartialEq, const N: usize> Default for LinearStack<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

/// MIDI ノート用の hold stack 型エイリアス
pub type HoldStack = LinearStack<u8, MAX_HELD>;
```

### モノモードでの使用（D29）

`SynthMode::Mono` のとき、Engine が以下のロジックで hold_stack を使う:

- `note_on(midi)`: stack.push(midi)、新規ノートを発音（VoicePool は実質 1 ボイスのみアクティブ）
- `note_off(midi)`: stack.remove(midi)、stack.top() があれば top のノートに復帰、なければ note_off を発火

### ユニットテスト方針

| テスト名 | 内容 |
|---|---|
| `test_hold_stack_push_pop_basic` | C 押 → D 押 → top が D、D remove → top が C |
| `test_hold_stack_overflow_drops_oldest` | 容量 16 を超えて push したら最古が消える、現在の最新は残る |
| `test_hold_stack_remove_middle` | 中間の値を remove しても先頭・末尾の順序が壊れない |
| `test_hold_stack_clear` | clear 後 is_empty() == true |

## Engine（Phase 2 改修）

### `engine.rs` の Phase 2 版

```rust
use crate::hold_stack::HoldStack;
use crate::karplus_strong::midi_to_freq;
use crate::params::{ParamId, PARAM_DESCRIPTORS, OUTPUT_GAIN_DEFAULT};
use crate::smoothing::SmoothedValue;
use crate::traits::AudioProcessor;
use crate::voice_pool::{VoicePool, POLYPHONY};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynthMode {
    Poly,
    Mono,
}

pub struct Engine {
    sample_rate: f32,
    pool: VoicePool<POLYPHONY>,
    output_gain: SmoothedValue,
    hold_stack: HoldStack,
    mode: SynthMode,
    current_damping: f32,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            sample_rate: 44100.0,
            pool: VoicePool::new(),
            output_gain: SmoothedValue::new(OUTPUT_GAIN_DEFAULT),
            hold_stack: HoldStack::new(),
            mode: SynthMode::Poly,
            current_damping: PARAM_DESCRIPTORS[0].default,  // Damping default
        }
    }

    pub fn note_on(&mut self, midi_note: u8, velocity: f32) {
        let freq = midi_to_freq(midi_note);
        match self.mode {
            SynthMode::Poly => {
                // pool.note_on は割り当てたボイスの index を返す（High 2 修正）
                let assigned = self.pool.note_on(midi_note, freq, velocity);
                // Phase 1 互換: damping を ユーザー設定値に復元（note_off で 0.95 になっている可能性のため）
                // 該当ボイスのみに復元することで、release 中の他ボイスを誤って復活させない
                self.pool.set_damping_voice(assigned, self.current_damping);
            }
            SynthMode::Mono => {
                self.hold_stack.push(midi_note);
                let assigned = self.pool.note_on(midi_note, freq, velocity);
                self.pool.set_damping_voice(assigned, self.current_damping);
            }
        }
    }

    pub fn note_off(&mut self, midi_note: u8) {
        match self.mode {
            SynthMode::Poly => {
                self.pool.note_off(midi_note);
            }
            SynthMode::Mono => {
                self.hold_stack.remove(midi_note);
                if let Some(top) = self.hold_stack.top() {
                    // last-note 復帰: top のノートに切り替え
                    let freq = midi_to_freq(top);
                    let assigned = self.pool.note_on(top, freq, 0.8);  // velocity はデフォルト
                    self.pool.set_damping_voice(assigned, self.current_damping);
                } else {
                    self.pool.note_off(midi_note);
                }
            }
        }
    }

    pub fn set_param(&mut self, id: u32, value: f32) {
        match ParamId::from_u32(id) {
            Some(ParamId::Damping) => {
                let v = PARAM_DESCRIPTORS[ParamId::Damping as usize].clamp(value);
                self.current_damping = v;
                self.pool.set_damping(v);
            }
            Some(ParamId::Brightness) => {
                let v = PARAM_DESCRIPTORS[ParamId::Brightness as usize].clamp(value);
                self.pool.set_brightness(v);
            }
            Some(ParamId::OutputGain) => {
                let v = PARAM_DESCRIPTORS[ParamId::OutputGain as usize].clamp(value);
                self.output_gain.set_target(v);
            }
            None => {}
        }
    }

    pub fn set_mode(&mut self, mode: SynthMode) {
        self.mode = mode;
        self.hold_stack.clear();
    }

    pub fn mode(&self) -> SynthMode {
        self.mode
    }

    pub fn current_damping(&self) -> f32 {
        self.current_damping
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioProcessor for Engine {
    fn prepare(&mut self, sample_rate: f32, max_block_size: usize) {
        self.sample_rate = sample_rate;
        self.pool.prepare(sample_rate, max_block_size);
        self.output_gain.set_time_constant(sample_rate, PARAM_DESCRIPTORS[2].smoothing_tau);
        self.pool.set_damping(self.current_damping);
    }

    fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32]) {
        debug_assert_eq!(output_l.len(), output_r.len());
        for i in 0..output_l.len() {
            let raw = self.pool.process_sample();
            let g = self.output_gain.next_sample();
            let s = raw * g;
            output_l[i] = s;
            output_r[i] = s;
        }
    }

    fn reset(&mut self) {
        self.pool.reset();
        self.pool.set_damping(self.current_damping);
        self.output_gain.set_immediate(OUTPUT_GAIN_DEFAULT);
        self.hold_stack.clear();
    }
}
```

### Phase 1 との API 変更

| 項目 | Phase 1 | Phase 2 |
|---|---|---|
| `Engine::new` | 単一 voice 保持 | VoicePool / HoldStack / SynthMode を保持 |
| `Engine::note_on` | KarplusStrong 単一に発火 | mode に応じて VoicePool または hold_stack 経由 |
| `Engine::note_off` | last-note 簡易判定（current_note と一致時のみ） | mode に応じて VoicePool または hold_stack 復帰ロジック |
| `Engine::set_param` | 単一 voice の SmoothedValue 設定 | VoicePool に fan-out |
| `Engine::set_mode` | （存在しない） | **新規**: mono / poly 切替 |
| `Engine::current_note` | `Option<u8>` 返却 | **削除**（VoicePool 化で単一 note 概念がなくなる、retrospective §5 負債解消） |

`current_note()` メソッドを削除する代わりに、Phase 2 では `pool.active_count()` を診断用として提供（C ABI 非公開、D22）。

## last-note 挙動（Phase 2 仕様）

### 動作仕様（mono モード時、D29）

- `hold_stack` で押下中のキー履歴を保持（容量 16、溢れ時最古破棄、D16）
- `note_on(N)`: stack.push(N)、新規ノートを VoicePool に発火
- `note_off(X)`: stack.remove(X)、stack.top() があればそのノートに復帰、なければ damping 加速（D7 維持）

### 想定シナリオ（mono モード、Phase 1 との比較）

| 操作 | Phase 1 (簡易版) | Phase 2 (hold stack 版) |
|---|---|---|
| C(60) on | 鳴っている音 = C、`current_note=Some(60)` | 鳴っている音 = C、stack=[60] |
| D(62) on | 鳴っている音 = D（C 即破棄）、`current_note=Some(62)` | 鳴っている音 = D、stack=[60, 62] |
| D off | 鳴っている音 = D（damping 加速で減衰） | 鳴っている音 = C（top 復帰）、stack=[60] |
| C off | 鳴っている音 = D（C は復帰しない） | 鳴っている音なし（damping 加速で減衰）、stack=[] |

### 想定シナリオ（poly モード）

ポリフォニー時は hold_stack を使わない（D29）。各ノートが独立した VoicePool 上のボイスを持ち、`note_off` は該当ボイスにのみ damping 加速を発火する。

## リアルタイム制約の遵守ルール

[Phase 1 03 章 §リアルタイム制約の遵守ルール](../2026-05-06-001-mvp/03-dsp-core-spec.md#リアルタイム制約の遵守ルール) に Phase 2 固有の項目を追加。

| ルール | Phase 2 適用箇所 |
|---|---|
| `prepare` 以外でヒープ確保しない | VoicePool::note_on / note_allocator::select_voice_for_steal / hold_stack の各操作で `Vec::push` / `Box::new` 禁止 |
| `Mutex`/`RwLock` を使わない | VoicePool の voices 配列は単一スレッドからのみアクセス（Worklet 内）、ロック不要 |
| `process` 内で panic しない | clamp ですべての範囲外を吸収、`debug_assert!` のみリリースで消える |
| `println!` / `dbg!` を呼ばない | 維持 |
| 整数除算・剰余は **必ず `buffer.len()` で取る** | `process_sample` の 4 read 位置と `write_index` 進行のすべてで `% buffer.len()` を使う。`% length_int` は使わない（Lagrange 4 点が時系列順にならず High 修正で発覚した不具合の原因になる）。`buffer.len()` は `note_on` 時に確定し以降不変なので予算内 |
| denormal 対策必須 | `KarplusStrong::process_sample` 末尾の DC injection（D6 維持） |
| sample_rate 変化時は `prepare` を再呼び出し | 維持 |
| **Lagrange 係数の毎サンプル再計算禁止** | `LagrangeCoeffs::new` は `note_on` 時のみ呼ぶ（D26）|
| **VoicePool 全ボイスの process_sample 累積はインライン化** | `VoicePool::process_sample` は `#[inline]` を付けて関数呼び出しを展開 |
| **hold_stack の操作は O(N) でも N=16 固定なので許容** | push / remove は線形走査だが N=16 なので予算内 |

## envelope tracking の意図

Phase 1 と同じ式を維持（`energy = energy * 0.999 + sample² * 0.001`）。Phase 2 で voice stealing の amplitude 推定に使うため、`energy.sqrt()` を `Voice::amplitude` として公開する（D19）。

## テスト方針（cargo test）

[Phase 1 03 章 §テスト方針](../2026-05-06-001-mvp/03-dsp-core-spec.md#テスト方針cargo-test) の 6 件を維持し、Phase 2 で 9 件以上を追加。

### Phase 1 から維持する既存テスト（6 件以上、retrospective §2 で 11 件パス）

- `test_silence_when_inactive`
- `test_energy_rises_after_note_on`
- `test_decay_with_low_damping`
- `test_length_matches_freq`（Phase 2 では `length_int + length_frac` の合算で検証）
- `test_no_allocation_in_process`
- `test_paramid_roundtrip`
- `test_damping_preserved_across_note_on`（retrospective で言及）
- `test_last_note_priority_simple`（retrospective で言及）

### Phase 2 で追加するテスト（18 件）

| テスト名 | 内容 | 対応する F# |
|---|---|---|
| `test_voice_pool_allocates_distinct_voices` | 8 個の異なる midi_note を順に note_on すると 8 ボイスがアクティブ | F10 |
| `test_voice_pool_same_note_replace` | 同じ midi_note を 2 回 note_on すると同じボイスが再励振される（active_count は 1 のまま） | F10 |
| `test_voice_pool_note_on_returns_assigned_index` | pool.note_on の戻り値が割り当てたボイスの index と一致（High 2 修正検証） | F10 |
| `test_engine_note_on_does_not_revive_released_voice` | note_on(C) → note_off(C) → note_on(D) のシーケンスで、release 中ボイス（C）の damping が note_off 時の `note_off_target_damping = 0.95` のまま、新規ボイス（D）のみ current_damping に復元される | F10 / F11 |
| `test_voice_pool_steals_quietest` | 8 ボイスを鳴らし、1 つだけ damping=0.9 で早く減衰させた後 9 音目で stealing → そのボイスが選ばれる | F11 / F23 |
| `test_voice_pool_polyphonic_mix_rms_bounded` | 8 ボイス全力 (velocity=0.8) で `note_on` 後 1 秒間 process_sample を実行、出力サンプルの **RMS が 0.7 以下、ピークが 2.0 以下** であることを確認。1/sqrt(N) スケールが概ね機能し、励振直後の最悪過渡応答でも 2.0 を超えないことの統計的検証（実機での音割れ判定は F24(a) に委ねる、ランダム励振では peak <= 1.0 の決定論的保証は強すぎるため避ける）| F24（補助）|
| `test_note_on_first_block_nonzero` | `note_on` 直後の最初の 128 サンプル（1 ブロック）で出力サンプルの絶対値最大が `velocity * 1e-3` 以上であること（励振配置 + write_index 初期値の組み合わせが正しく機能し、初回 read で励振ノイズ範囲を読めることを確認、High 修正の主検証）| F1 / F12 |
| `test_pitch_a1` | A1 (midi=33, 55Hz) のピッチ精度 ± 0.5%（Phase 1 課題解消、autocorrelation 推定）| F12 / F13 |
| `test_pitch_a2` | A2 (midi=45, 110Hz) のピッチ精度 ± 0.5% | F12 |
| `test_pitch_a4` | A4 (midi=69, 440Hz) のピッチ精度 ± 0.5% | F12 |
| `test_pitch_c6` | C6 (midi=84, 1046.5Hz) のピッチ精度 ± 0.5%（中高域での Lagrange 動作検証）| F12 |
| `test_pitch_c8` | C8 (midi=108, 4186Hz) のピッチ精度 ± 0.5%（高域での Lagrange 動作検証、基本周期 11.5 サンプル）| F12 |
| `test_hold_stack_last_note_priority` | C 押→D 押→D 離→C 復帰のシーケンスで pool.active_count() / アクティブボイスの note_id 軌跡が [C, D, C] になる | F18 |
| `test_hold_stack_overflow_behavior` | 17 鍵を順に押すと最古が消えるが、現在押下中の鍵は残る | F19 |
| `test_synth_mode_switch_no_break` | Poly → Mono 切替時に hold_stack がクリアされ、進行中のボイスは消音されない | F20 |
| `test_no_allocation_in_polyphonic_process` | VoicePool::prepare 後の note_on 8 連発 → 1 秒分の process_sample で alloc 回数 0 | F17 |
| `test_paramdescriptor_default_value` | PARAM_DESCRIPTORS[0].default == 0.996（Damping）等の既知値確認 | F14 |
| `test_long_term_stability_high_damping` | damping=0.9999 + 各 brightness ∈ {0.0, 0.5, 1.0} + 各 midi ∈ {33 (A1), 69 (A4), 108 (C8)} の組み合わせで 30 秒分 (1,440,000 サンプル) を process_sample で生成し、(a) 全サンプルが finite (`is_finite()`)、(b) 絶対値ピークが 10.0 以下（KS の理論上ありえない値が出ない）、(c) 最終 1 秒のサンプル平均絶対値が 100.0 以下（IIR 発散していない）を確認 | 安定性検証（D14 補強）|

> WASM 環境でのテストは Phase 2 でも行わない（Phase 1 と同じく `cargo test` の native build）。

## 実装上の注意点

[Phase 1 03 章 §実装上の注意点](../2026-05-06-001-mvp/03-dsp-core-spec.md#実装上の注意点) を継承し、Phase 2 で追加する点:

1. **`Vec::resize` を `note_on` で呼ばないこと**: Phase 1 と同じ。VoicePool でも各ボイスの `KarplusStrong::note_on` は length_int 更新と励振のみ
2. **`VoicePool::voices` の初期化は `core::array::from_fn`**: const generic 配列で `[KarplusStrong::new(); N]` は使えない（KarplusStrong が Copy でないため）
3. **Lagrange 係数は note_on 時のみ計算**: `process_sample` 内では `LagrangeCoeffs::apply` の積和のみ。Phase 2 で pitch bend 対応するなら SmoothedValue 化を再検討（D26 / 01 章 D26）
4. **`process_sample` のリングバッファ添字は必ず `buffer.len()` で剰余**: `length_int` で剰余を取ると Lagrange 4 点が時系列順に並ばず、`x[n - D_int - 1] / x[n - D_int - 2]` がリング上で巻き戻ったサンプルになってピッチ精度が崩れる。`LAGRANGE_BUFFER_MARGIN = 3` を加味した `buffer.len()` で剰余を取り、`length_int` の上限を `buffer.len() - 3` で clamp することで 4 点が必ず時系列順に取れる
5. **`note_on` でバッファ全体をゼロクリア**: Phase 1 では length 分のみ書いていたが、Phase 2 では Lagrange が write_index 直後の (length_int + 1, length_int + 2) 位置を参照するため、前回 note_on の残骸が過渡応答に混じらないよう全領域を初期化する。`buffer.len()` は ~1749 で O(N) ループだが `note_on` 時のみで `process` 中ではないため D4 維持
6. **hold_stack は mono 専用、ランタイム mode で if 分岐**: コンパイル時 `cfg!(feature = "mono")` ではなく実行時 `match self.mode` で分岐。C ABI 互換と将来の動的切替を両立（D29）
7. **`process_sample` の `#[inline]` を維持**: KarplusStrong / VoicePool 双方に `#[inline]` を付ける。release ビルドで関数呼び出し展開され WASM 性能に直結
