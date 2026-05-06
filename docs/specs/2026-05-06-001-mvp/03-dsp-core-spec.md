# 03. dsp-core クレート仕様

## 目的

Rust 製の DSP コアクレート `dsp-core` の構造、公開API、内部実装方針、リアルタイム制約への適合方法を定義する。WASM 非依存・**std依存最小**（MVPでは `Vec` を使うため完全な no_std ではない。将来 `alloc` のみへ移行可能な設計に保つ）に保ち、将来の他環境（CLI、VST/CLAP、ネイティブ）への転用余地を残す。

## 他文書との関係

- 上流: [`02-architecture.md`](./02-architecture.md)（クレート位置とビルド連鎖）
- 並列: [`04-wasm-audio-spec.md`](./04-wasm-audio-spec.md)（dsp-core を呼び出す側）
- 参考: pre-research 3.1（Karplus–Strong）、6.2（trait 案）、7.1（最小実装）、8.2（smoothing）、13章（アンチパターン）

## クレート設定

### `crates/dsp-core/Cargo.toml`

```toml
[package]
name = "dsp-core"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["rlib"]

[features]
default = ["std"]
std = []
```

依存はゼロ。`dasp`、`fundsp` などは導入しない。学習目的でDelay LineやFilterを自作する。

> **no_std についての注意**: MVPは `Vec` を使うため厳密には no_std 対応していない。`std` feature flag は将来 `alloc` のみへ切り替える際の構造を確保するためのもの。完全な no_std 化は Phase 2 以降の課題。

### モジュール一覧

| ファイル | 内容 |
|---|---|
| `lib.rs` | 公開モジュール宣言、`pub use` での re-export |
| `traits.rs` | `AudioProcessor`、`Voice` trait |
| `params.rs` | `ParamId` enum と既定値 |
| `smoothing.rs` | `SmoothedValue` 構造体 |
| `rng.rs` | `XorShift32` 構造体 |
| `karplus_strong.rs` | `KarplusStrong` 構造体（中核モデル） |
| `voice.rs` | `Voice` trait の `KarplusStrong` 向け実装 |
| `engine.rs` | `Engine` 構造体（最上位の発音管理） |

## 公開trait

### `traits.rs`

```rust
pub trait AudioProcessor {
    /// サンプルレートと最大ブロックサイズを通知し、必要なバッファを確保する。
    /// このメソッドが唯一のヒープ確保許可ポイント。
    fn prepare(&mut self, sample_rate: f32, max_block_size: usize);

    /// 出力スライスを埋める。スライス長は呼び出し側が決定する。
    /// 内部状態の更新と出力書き込みのみ行い、確保・ロックは禁止。
    fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32]);

    /// 内部状態をゼロに戻す。バッファ容量は保持する。
    fn reset(&mut self);
}

pub trait Voice {
    fn note_on(&mut self, freq_hz: f32, velocity: f32);
    fn note_off(&mut self);
    fn process_sample(&mut self) -> f32;
    fn is_active(&self) -> bool;
}
```

> 注: `Voice` trait は dyn dispatch せず、ポリフォニー化時は `[KarplusStrong; N]` 配列で扱う。MVPは1ボイスのため trait は将来拡張用の型契約として用意するのみ。

## パラメータ定義

### `params.rs`

```rust
#[repr(u32)]
#[non_exhaustive]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ParamId {
    Damping     = 0,
    Brightness  = 1,
    OutputGain  = 2,
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
}
```

### 各パラメータの値域と既定値

| ParamId | 範囲 | 既定値 | 意味 |
|---|---|---|---|
| `Damping` | `[0.90, 0.9999]` | `0.996` | ディレイラインの減衰係数。値が大きいほど長く鳴る |
| `Brightness` | `[0.0, 1.0]` | `0.5` | ワンポール LPF のミックス比。1.0で生のまま、0.0で完全にスムージング |
| `OutputGain` | `[0.0, 1.5]` | `0.8` | 最終出力ゲイン |

> ピッチと velocity は `note_on` 経由で渡し、AudioParam 化しない（突発変化が期待値）。
>
> **drift リスク**: `ParamId`（Rust）と `PARAM_IDS`（TS、[05章 messages.ts](./05-web-frontend-spec.md#メッセージ仕様messagests)）は手動の二重管理になる。MVP では3つのみで影響は小さいが、最低限の対策として **contract test を追加**する: `cargo test` 内で `assert_eq!(ParamId::Damping as u32, 0)` のような id 値固定化テストと、TS 側で同じ数値を `expect` するテスト（vitest 等）。Phase 2 で増えてきたら `params.json` を単一ソースにしたコード生成への移行を検討する。

## SmoothedValue

### `smoothing.rs`

```rust
pub struct SmoothedValue {
    current: f32,
    target: f32,
    coeff: f32,    // 1ステップあたりの追従係数
}

impl SmoothedValue {
    pub fn new(initial: f32) -> Self {
        Self { current: initial, target: initial, coeff: 0.0 }
    }

    /// sample_rate と時定数 tau（秒）から係数を計算する。
    /// coeff = 1 - exp(-1 / (sample_rate * tau))
    pub fn set_time_constant(&mut self, sample_rate: f32, tau_seconds: f32) {
        self.coeff = 1.0 - (-1.0 / (sample_rate * tau_seconds.max(1e-6))).exp();
    }

    pub fn set_target(&mut self, target: f32) { self.target = target; }

    /// 補間なしで即座に値を設定（初期化や reset で使用）
    pub fn set_immediate(&mut self, value: f32) {
        self.current = value;
        self.target = value;
    }

    pub fn next_sample(&mut self) -> f32 {
        self.current += self.coeff * (self.target - self.current);
        self.current
    }

    pub fn current(&self) -> f32 { self.current }
}
```

### 既定の時定数

| 用途 | tau（秒） |
|---|---|
| Damping | 0.02 |
| Brightness | 0.02 |
| OutputGain | 0.01 |

> サンプルレート48kHzのとき tau=0.02s で coeff ≈ 0.00104。1/coeff ≈ 960 サンプル ≈ 20ms で目標値に約63%到達。クリック回避には十分。

## XorShift32

### `rng.rs`

```rust
pub struct XorShift32 { state: u32 }

impl XorShift32 {
    pub fn new(seed: u32) -> Self {
        // seed == 0 は xorshift で全0を生むため、固定値で置換
        Self { state: if seed == 0 { 0xDEAD_BEEF } else { seed } }
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// 戻り値は [-1.0, 1.0)
    pub fn next_unit_bipolar(&mut self) -> f32 {
        let r = self.next_u32() as f32 / u32::MAX as f32;  // [0.0, 1.0]
        r * 2.0 - 1.0
    }
}
```

外部crate（`rand`、`getrandom`等）は導入しない。`std::rand` も存在しないため、自前実装が最も摩擦が少ない。

## KarplusStrong

### `karplus_strong.rs`

```rust
use crate::rng::XorShift32;
use crate::smoothing::SmoothedValue;
use crate::traits::Voice;

pub struct KarplusStrong {
    buffer: Vec<f32>,           // 容量は prepare で確保。再allocateしない
    write_index: usize,
    length: usize,              // 実効ディレイ長（length <= buffer.len()）
    damping: SmoothedValue,
    brightness: SmoothedValue,
    last_filter_out: f32,       // ワンポールLPF状態
    energy: f32,                // RMS的エネルギー追跡
    active: bool,
    rng: XorShift32,
    sample_rate: f32,
    note_off_target_damping: f32,  // note_off 時の damping target
}
```

### 構造体フィールドの役割

| フィールド | 役割 | 値域・備考 |
|---|---|---|
| `buffer` | ディレイラインの実体。`prepare` で `max_buffer_len` 分を確保 | `Vec<f32>`。length以降の要素は使われない。インデックスアクセスは常に `buffer.len()` 未満で行う（`buffer.capacity()` は実装が len より大きい値を返しうるため使わない） |
| `write_index` | 次に書き込む位置 | `[0, length)` |
| `length` | 実効ディレイ長（音程を決定） | `note_on` で更新。process中は変更禁止 |
| `damping` | 減衰率（smoothed） | デフォルト 0.996、note_off 時に 0.95 へ |
| `brightness` | LPFミックス比（smoothed） | 0.0〜1.0 |
| `last_filter_out` | ワンポールLPFの直前出力 | 状態変数 |
| `energy` | エネルギー追跡値（envelope tracking） | RMS的な平滑値 |
| `active` | 発音中フラグ | `energy < 1e-9` で false に |
| `rng` | 励振ノイズ生成 | XorShift32 |
| `sample_rate` | プリペア時に保持 | length計算と smoothing 設定で使用 |
| `note_off_target_damping` | note_off 時の damping target | デフォルト 0.95 |

### `process_sample` 擬似コード

```rust
impl KarplusStrong {
    pub fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        // 1. read 位置の決定
        let read_index = (self.write_index + 1) % self.length;
        let current = self.buffer[self.write_index];
        let next = self.buffer[read_index];

        // 2. ワンポールLPF（brightness=1.0で生のまま、0.0で前回値に固執）
        let b = self.brightness.next_sample();
        let avg = 0.5 * (current + next);
        let filtered = b * avg + (1.0 - b) * self.last_filter_out;
        self.last_filter_out = filtered;

        // 3. 減衰
        let d = self.damping.next_sample();
        let mut damped = d * filtered;

        // 4. denormal 対策（DC injection: +1e-25 - 1e-25）
        damped += 1.0e-25;
        damped -= 1.0e-25;

        // 5. ディレイラインへ書き戻し
        self.buffer[self.write_index] = damped;
        self.write_index = read_index;

        // 6. envelope tracking（係数は1サンプルあたりのIIR平滑）
        self.energy = self.energy * 0.999 + damped * damped * 0.001;
        if self.energy < 1.0e-9 {
            self.active = false;
        }

        current
    }
}
```

### `note_on` の実装方針

```rust
impl KarplusStrong {
    pub fn note_on(&mut self, freq_hz: f32, velocity: f32) {
        // 1. ディレイ長を計算（整数）
        // capacity ではなく len を上限に使う（len 以降は初期化されておらず添字アクセスできない）
        let raw_len = (self.sample_rate / freq_hz).round() as usize;
        let len = raw_len.clamp(2, self.buffer.len());
        self.length = len;

        // 2. ノイズで励振（length 分のみ。capacity 全体には触らない）
        for i in 0..len {
            self.buffer[i] = self.rng.next_unit_bipolar() * velocity;
        }
        // length より後の領域は次の note_on で長さが伸びたとき以外触らない

        // 3. 状態をリセット
        self.write_index = 0;
        self.last_filter_out = 0.0;
        self.energy = velocity * velocity;
        self.active = true;

        // 4. damping target には触れない
        //    note_off で 0.95 等に変えられている可能性があるが、
        //    Engine 側がユーザー設定値（current_damping）を保持しており、
        //    note_on 直後に Engine が `set_damping(current_damping)` で復元する
    }

    pub fn note_off(&mut self) {
        self.damping.set_target(self.note_off_target_damping);
    }
}
```

### `prepare` の実装方針

```rust
impl KarplusStrong {
    pub fn prepare(&mut self, sample_rate: f32, _max_block_size: usize) {
        self.sample_rate = sample_rate;

        // 27.5Hz（A0）相当をカバーするバッファを確保
        let min_freq = 27.5;
        let max_buffer_len = (sample_rate / min_freq).ceil() as usize;
        self.buffer = vec![0.0; max_buffer_len];

        // smoothing の係数を設定
        self.damping.set_time_constant(sample_rate, 0.02);
        self.brightness.set_time_constant(sample_rate, 0.02);

        self.write_index = 0;
        self.length = 0;
        self.last_filter_out = 0.0;
        self.energy = 0.0;
        self.active = false;
    }
}
```

> **重要**: `prepare` 以外でのヒープ確保は禁止。`note_on` での `Vec::resize` も禁止（容量が足りない周波数は `clamp` で吸収）。

## Engine

### `engine.rs`

```rust
use crate::karplus_strong::KarplusStrong;
use crate::params::ParamId;
use crate::smoothing::SmoothedValue;

pub struct Engine {
    sample_rate: f32,
    voice: KarplusStrong,
    output_gain: SmoothedValue,
    current_note: Option<u8>,    // 最後に note_on したノート
    current_damping: f32,        // ユーザー設定値を保持（note_off で voice 側が変えても復元できる）
}

impl Engine {
    pub fn new() -> Self {
        Self {
            sample_rate: 44100.0,
            voice: KarplusStrong::new(),
            output_gain: SmoothedValue::new(0.8),
            current_note: None,
            current_damping: 0.996,
        }
    }

    pub fn prepare(&mut self, sample_rate: f32, max_block_size: usize) {
        self.sample_rate = sample_rate;
        self.voice.prepare(sample_rate, max_block_size);
        self.output_gain.set_time_constant(sample_rate, 0.01);
        self.voice.set_damping(self.current_damping);
    }

    pub fn note_on(&mut self, midi_note: u8, velocity: f32) {
        let freq = 440.0 * 2f32.powf((midi_note as f32 - 69.0) / 12.0);
        self.voice.note_on(freq, velocity);
        // note_off で 0.95 になっている可能性があるためユーザー設定値に戻す
        self.voice.set_damping(self.current_damping);
        self.current_note = Some(midi_note);
    }

    pub fn note_off(&mut self, midi_note: u8) {
        // MVPの last-note 挙動: 「最後の note_on のみを追跡し、再トリガしない」
        // 現在発音中のノートと一致したときのみ damping を加速
        // 前のキーが押下中でも復帰しない（C押し→D押し→D離す は無音方向）
        if self.current_note == Some(midi_note) {
            self.voice.note_off();
            self.current_note = None;
        }
    }

    pub fn set_param(&mut self, id: u32, value: f32) {
        match ParamId::from_u32(id) {
            Some(ParamId::Damping) => {
                let v = value.clamp(0.90, 0.9999);
                self.current_damping = v;
                self.voice.set_damping(v);
            }
            Some(ParamId::Brightness)  => self.voice.set_brightness(value.clamp(0.0, 1.0)),
            Some(ParamId::OutputGain)  => self.output_gain.set_target(value.clamp(0.0, 1.5)),
            None => {}  // 未知IDは黙って無視
        }
    }

    pub fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32]) {
        debug_assert_eq!(output_l.len(), output_r.len());
        for i in 0..output_l.len() {
            let raw = self.voice.process_sample();
            let g = self.output_gain.next_sample();
            let s = raw * g;
            // モノラルを左右両チャンネルに出力（MVPはステレオ拡散なし）
            output_l[i] = s;
            output_r[i] = s;
        }
    }

    pub fn reset(&mut self) {
        self.voice.reset();
        self.voice.set_damping(self.current_damping);
        self.output_gain.set_immediate(0.8);
        self.current_note = None;
    }
}
```

> `set_damping` と `set_brightness` は `KarplusStrong` 側で `SmoothedValue::set_target` を呼ぶ薄いメソッドとして定義する。

## last-note 挙動（MVP仕様）

MVPはモノフォニーであり、複数キー同時押し時の挙動を「**last-note priority の簡易版**」として定義する。

### 動作仕様

- `current_note: Option<u8>` で **最後に note_on したノート1つだけ** を追跡する
- `note_on(N)` 後に `note_on(M)` が来たら、いつでも M を新規発音（前のノートは即座に破棄、新しいバッファをノイズで励振）
- `note_off(X)` は `current_note == Some(X)` のときのみ発火（`note_off_target_damping` に切り替え）
- **`note_off(X)` 後に「以前押下していた別のキー」が復帰することはない**（hold note stack を持たないため）

### 想定シナリオ

| 操作 | 内部状態 | 鳴っている音 |
|---|---|---|
| C(60) on | `current_note = Some(60)`、damping = ユーザー値 | C |
| D(62) on | `current_note = Some(62)`、damping = ユーザー値（再励振） | D |
| D off | `current_note = None`、damping = 0.95 で減衰 | D（減衰中） |
| C off | `Some(62) != Some(60)` のため何もしない | D（減衰中、Cは復帰しない） |

> 真の hold note stack 実装（D離した時にCが復帰する）は **Phase 2** で導入する。MVPでは混乱を避けるためシンプルに保つ。

## リアルタイム制約の遵守ルール

| ルール | 適用箇所 |
|---|---|
| `prepare` 以外でヒープ確保しない | `note_on` 内で `Vec::resize` / `Vec::push` 禁止。`length` 変更のみ |
| `Mutex`/`RwLock` を使わない | スレッド境界はWorklet単独で完結（D4） |
| `process` 内で panic しない | `clamp` で値を有効範囲に強制。`debug_assert!` のみ使用 |
| `println!` / `dbg!` を呼ばない | `std::fmt` 経由のフォーマットも process 内では避ける |
| 整数除算・剰余を最小化 | `(write_index + 1) % length` は許容（分岐より速い場合あり） |
| denormal 対策必須 | `process_sample` 末尾の DC injection |
| sample_rate 変化時は `prepare` を再呼び出し | バッファ再確保が必要なため。MVPでは固定 |

## envelope tracking の意図

pre-research 7.1 のサンプルコードでは `if current.abs() < 1e-5` というコメントだけ残されていた箇所を、`energy` ベースのIIR平滑で置き換える。

- **平滑係数**: `energy = energy * 0.999 + sample² * 0.001`
  - サンプルレート48kHzで時定数約20ms（`-1/ln(0.999) ≒ 999サンプル`）
- **しきい値**: `1e-9`（音響的にほぼ無音）
- **active が false になると `process_sample` は早期 return で 0.0 を返す**ため、CPU負荷も下がる

## テスト方針（cargo test）

`crates/dsp-core/src/karplus_strong.rs` 末尾もしくは `tests/` 配下に以下のユニットテストを配置:

| テスト名 | 内容 |
|---|---|
| `test_silence_when_inactive` | `prepare` 直後に `process_sample` を呼ぶと 0.0 を返す |
| `test_energy_rises_after_note_on` | `note_on` 後 100 サンプルで `energy > 0` |
| `test_decay_with_low_damping` | `damping = 0.90` で 1 秒後に `is_active() == false` |
| `test_length_matches_freq` | `note_on(440Hz)` 後、内部 `length == round(sample_rate / 440)`。ピッチ検証は length の整数値で行う（ノイズ励振ベースのゼロクロス推定は不安定なため避ける） |
| `test_no_allocation_in_process` | `note_on` → 1 秒分の `process_sample` で `Vec::len()` が変わらない（添字アクセスの安全性に直結） |
| `test_paramid_roundtrip` | `ParamId::from_u32(0)` が `Some(Damping)` |

> WASM環境でのテストは MVP では行わない。`cargo test` の native build で十分。

## 実装上の注意点

1. **`Vec::resize` を `note_on` で呼ばないこと**。これは pre-research 7.1 のサンプルコードの最大の誤り。`prepare` で max_buffer を確保し、`note_on` では `length` フィールドのみ更新する。
2. **`#[derive(Default)]` を `KarplusStrong` に付けない**。`Vec::new()` から始まり、`prepare` 呼び出し前は使えない状態であることを型で示す（コンストラクタ `new()` を経由）。
3. **`SmoothedValue` の `coeff` は `prepare` 後でないと正しく計算されない**。`new()` 直後の `next_sample` は target を返すだけ（coeff=0 のため）。
4. **`process_sample` は `#[inline]` を付ける**。`Engine::process` のループから多数呼ばれるため、インライン化で関数呼び出しオーバーヘッドを削減。
