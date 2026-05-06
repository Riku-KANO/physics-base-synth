# 物理ベース・シンセサイザー調査レポート

## Rust + WebAssembly 実装に向けた原理・研究・設計方針

## 1. 結論

物理ベースのシンセサイザー、つまり **Physical Modeling Synthesis** は、録音済みサンプルを再生するのではなく、**弦・管・膜・板・リード・弓・ハンマー・共鳴胴などの発音メカニズムを数理モデルとして実装し、リアルタイムに音を生成する方式**です。Frontiers の2025年の総説でも、物理モデリング合成は「楽器や音響システムの挙動を支配する方程式を解くことで音をシミュレートする」と説明されています。([Frontiers][1])

Rust + WASM で最初に作るなら、実装難度・音の面白さ・計算量のバランスから、次の順番が良いです。

```text
1. Karplus–Strong 弦モデル
2. Extended Karplus–Strong / Digital Waveguide
3. Modal Synthesis
4. 簡易な管楽器モデル
5. Mass-Spring / FDTD 系の実験モデル
```

Web 実装では、**AudioWorklet + WASM** が基本構成になります。MDN は AudioWorklet を、低レイテンシな音声処理のために別スレッドでカスタム音声処理を実行する仕組みとして説明しています。([MDN Web Docs][2])

---

## 2. 物理モデリング音源とは何か

物理モデリング音源は、音色そのものではなく、**音が発生する原因**をモデル化します。

たとえば弦楽器なら、

```text
励振: ピック、指、ハンマー、弓
↓
振動体: 弦
↓
境界条件: ナット、ブリッジ
↓
損失: 摩擦、空気抵抗、内部損失
↓
共鳴: ボディ、響板、空洞
↓
放射: 空気中への音響放射
```

という流れを扱います。

この方式の強みは、**演奏パラメータと音響結果が因果的につながる**ことです。Frontiers の総説では、物理モデリングは信号ベースの方法と異なり、音の発生に関する cause-and-effect relationship を直接表現する点が特徴だと説明されています。([Frontiers][1])

---

## 3. 主要な研究・方式

## 3.1 Karplus–Strong アルゴリズム

最初に実装すべき代表的方式です。

Karplus–Strong は、短いノイズ列をディレイラインに入れ、フィードバックしながらローパスフィルタで減衰させることで、弦を弾いたような音を生成します。1983年の Karplus と Strong の論文は、デジタル合成による plucked-string / drum timbre の古典的文献です。([Baskin School of Engineering][3])

基本構造はこうです。

```text
noise burst
  → delay line
  → averaging / low-pass filter
  → feedback
  → output
```

擬似コードはかなり単純です。

```rust
for sample in output {
    let y0 = delay.pop_front();
    let y1 = delay.front();
    let next = damping * 0.5 * (y0 + y1);
    delay.push_back(next);
    *sample = y0;
}
```

音程はディレイ長で決まります。

```text
delay_length ≒ sample_rate / frequency
```

Karplus–Strong は単純ですが、Jaffe と Smith による Extended Karplus–Strong では、ピック位置、損失フィルタ、分数ディレイ、チューニング補正などを導入して、より物理的な弦モデルへ拡張されました。Karjalainen、Välimäki、Tolonen の論文も、Karplus–Strong からより物理的な waveguide model へ接続できることを示しています。([Aalto University Users][4])

---

## 3.2 Digital Waveguide Synthesis

本格的な物理モデリングの中心技術です。

Digital Waveguide は、弦や管の中を進む波を、**双方向ディレイライン**と**フィルタ**で表現します。資料では、Stanford の Julius O. Smith と Perry Cook らによって発展した方式で、弦のような一次元波動伝播媒体を waveguide として扱うと説明されています。([Sound ETI][5])

弦の場合、理想的には右向き波と左向き波があります。

```text
left-going wave  <──────────────
right-going wave ──────────────>
```

実装上は、2本のディレイラインで表現できます。

```text
bridge reflection
   ↑             ↓
[ delay → ] + [ ← delay ]
   ↓             ↑
nut reflection
```

重要な部品は以下です。

| 部品       | 実装                                     |
| -------- | -------------------------------------- |
| 波の伝播     | delay line                             |
| 周波数依存の減衰 | low-pass / one-pole filter             |
| 分数ディレイ   | Lagrange / all-pass interpolation      |
| 境界反射     | reflection coefficient                 |
| ピック位置    | 入力分布、comb 的効果                          |
| ボディ共鳴    | resonator / convolution / modal filter |

Julius O. Smith の *Physical Audio Signal Processing* は、この分野の中核的なオンライン書籍として参照されています。DAFx の資料でも、Smith の *Physical Audio Signal Processing* が CCRMA の物理音響信号処理の主要文献として挙げられています。([DAFx][6])

Rust 実装では、まず `DelayLine`、`FractionalDelay`、`OnePoleLowpass`、`Allpass`、`Biquad` を自作すると理解が深まります。

---

## 3.3 Modal Synthesis

Modal Synthesis は、物体の音を「固有振動モードの足し合わせ」として表現します。

金属板、鐘、木片、ガラス、ボディ共鳴などに向いています。Bilbao の *Sound synthesis and physical modeling* でも、Modal Synthesis は MOSAIC や Modalys などの物理モデリングシステムの基礎になった方式として説明されています。([School of Physics and Astronomy][7])

基本モデルは、

```text
output = Σ mode_i
mode_i = amplitude_i * exp(-decay_i * t) * sin(2π f_i t + phase_i)
```

です。

実装上は、各モードを **2次共振フィルタ**として持つのが一般的です。

```rust
struct Mode {
    frequency: f32,
    decay: f32,
    gain: f32,
    z1: f32,
    z2: f32,
}
```

Modal Synthesis の利点は、少ない計算量で「物体っぽい共鳴」を作りやすいことです。欠点は、弦や管のような連続的な波動伝播や非線形相互作用の表現には弱いことです。

最近では、物理モデルと機械学習を組み合わせる研究も進んでいます。2024年の Differentiable Modal Synthesis の論文では、非線形弦の時空間運動を、物理パラメータと基本周波数を入力としてシミュレートするモデルが提案されています。([arXiv][8])

---

## 3.4 Mass-Spring Model

Mass-Spring は、物体を質点とバネとダンパーのネットワークとして扱います。

```text
mass -- spring -- mass -- spring -- mass
```

Faust の公式 physical modeling library には、waveguide、mass-spring、digital wave models が含まれており、弦、膜、バー、共鳴系などに使えると説明されています。([ファウストライブラリ][9])

Mass-Spring は直感的で拡張しやすいですが、安定性と計算量に注意が必要です。特に explicit Euler のような単純積分は発散しやすいため、symplectic Euler、Verlet、または安定性条件を満たす差分法を検討する必要があります。

---

## 3.5 Finite Difference / FDTD

有限差分法は、波動方程式や弦・膜・板の偏微分方程式を離散化して解く方式です。

たとえば理想弦の波動方程式は、

```text
∂²y/∂t² = c² ∂²y/∂x²
```

のように書けます。これを時間・空間方向に格子化して計算します。

この方式は最も「物理シミュレーション」らしいですが、計算量が大きく、数値安定性が難しいです。Faust に有限差分スキームを導入する研究では、FDS physical models を Faust で形式化する方法が提案されています。([ResearchGate][10])

WASM でリアルタイムに動かすなら、最初から FDTD を主軸にするのはおすすめしません。研究・実験用途として後段に回すのが良いです。

---

# 4. 実装対象としての優先順位

## MVP におすすめ

最初の MVP は、**物理モデリング弦シンセ**が良いです。

理由は、

```text
・Karplus–Strong なら実装が小さい
・Digital Waveguide へ自然に拡張できる
・音の変化がわかりやすい
・WASM の性能検証に向いている
・UI パラメータを作りやすい
```

からです。

最初の音源仕様はこれで十分です。

```text
Instrument: PluckedString
Controls:
  - pitch
  - excitation brightness
  - damping
  - decay
  - pick position
  - body resonance
  - string stiffness
  - stereo spread
```

---

# 5. Rust + WASM + Web Audio のアーキテクチャ

## 5.1 推奨構成

```text
React / Next.js UI
  ↓
AudioContext
  ↓
AudioWorkletNode
  ↓
AudioWorkletProcessor JS wrapper
  ↓
WASM module compiled from Rust
  ↓
Rust DSP engine
```

Web Audio API は、AudioNode を接続して音声処理グラフを作る高レベル API として定義されています。W3C の仕様でも、Web Audio は audio routing graph を基本パラダイムとし、AudioNode 同士を接続してレンダリングを定義すると説明されています。([W3C][11])

AudioWorklet の `process()` は、音声レンダリングスレッドから同期的に呼ばれます。MDN では、処理可能な新しい音声ブロックが来るたびに `process()` が呼ばれると説明されています。([MDN Web Docs][12])

---

## 5.2 128 sample render quantum 問題

Web Audio の AudioWorklet は通常、128サンプル単位で処理されます。Chrome Developers の Audio Worklet + WebAssembly 設計記事でも、AudioWorkletProcessor が 128 frames を扱い、WASM 側が 512 frames 単位で処理する場合は ring buffer で吸収する設計が紹介されています。([Chrome for Developers][13])

つまり Rust 側の DSP は、

```rust
fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32])
```

のように、任意のブロック長に対応できる設計にするのが良いです。

ただし、AudioWorklet ではリアルタイム制約が強いため、以下は避けるべきです。

```text
・process 中のメモリアロケーション
・Mutex ロック
・重いログ出力
・panic
・Vec の伸長
・JS との細かすぎる往復
・GC に依存する処理
```

---

# 6. Rust 側 DSP 設計

## 6.1 クレート構成案

```text
physical-synth/
  crates/
    dsp-core/
      src/
        delay.rs
        filters.rs
        interpolation.rs
        karplus_strong.rs
        waveguide.rs
        modal.rs
        voice.rs
        engine.rs
    wasm-audio/
      src/
        lib.rs
    web/
      Next.js / Vite / React
```

`dsp-core` は WASM に依存させないのが重要です。後で CLI、VST/CLAP、ネイティブアプリにも転用できます。

Rust の音声 DSP 周辺では、`dasp` は no dynamic allocations と no dependencies を掲げる基礎 DSP ライブラリです。([GitHub][14]) また、FunDSP は audio components や procedural generation tools を含む Rust の音声処理・合成ライブラリとして公開されています。([Crates][15])

ただし、物理モデリングの理解を目的にするなら、最初は delay line や filter を自作する方が良いです。

---

## 6.2 コア trait 案

```rust
pub trait AudioProcessor {
    fn prepare(&mut self, sample_rate: f32, max_block_size: usize);
    fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32]);
}

pub trait Voice {
    fn note_on(&mut self, note: u8, velocity: f32);
    fn note_off(&mut self);
    fn process_sample(&mut self) -> f32;
    fn is_active(&self) -> bool;
}
```

物理モデリング音源では、一般的な oscillator よりも内部状態が多くなります。たとえば弦モデルなら、

```rust
pub struct PluckedString {
    delay: FractionalDelay,
    loss_filter: OnePole,
    damping: f32,
    brightness: f32,
    pick_position: f32,
    body: BodyResonator,
}
```

のようになります。

---

# 7. 最初に実装するべきモデル

## 7.1 Karplus–Strong Voice

最小構成です。

```rust
pub struct KarplusStrong {
    buffer: Vec<f32>,
    index: usize,
    damping: f32,
    feedback: f32,
    active: bool,
    last: f32,
}

impl KarplusStrong {
    pub fn note_on(&mut self, freq: f32, velocity: f32, sample_rate: f32) {
        let len = (sample_rate / freq) as usize;
        self.buffer.resize(len.max(2), 0.0);

        for x in self.buffer.iter_mut() {
            *x = (rand_unit() * 2.0 - 1.0) * velocity;
        }

        self.index = 0;
        self.active = true;
    }

    pub fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let current = self.buffer[self.index];
        let next_index = (self.index + 1) % self.buffer.len();
        let next = self.buffer[next_index];

        let filtered = self.feedback * self.damping * 0.5 * (current + next);
        self.buffer[self.index] = filtered;

        self.index = next_index;

        if current.abs() < 1e-5 {
            // 実際には RMS やエネルギーで判定する方がよい
        }

        current
    }
}
```

この時点で「弦っぽい」音が出ます。

---

## 7.2 Extended Karplus–Strong

次に追加するべき要素です。

```text
・fractional delay
・loss filter
・pick position filter
・stretching all-pass
・body resonator
```

特に fractional delay は重要です。整数ディレイだけだと正確なピッチが出しにくいためです。

```text
delay = sample_rate / frequency
integer_delay = floor(delay)
fraction = delay - integer_delay
```

分数部分は linear interpolation でも動きますが、音質を上げるなら Lagrange interpolation や all-pass fractional delay を使います。

---

## 7.3 Body Resonator

物理モデリング弦シンセで「安っぽい音」から抜ける鍵は、弦そのものよりも **ボディ共鳴**です。

最初は簡単な biquad resonator を数個並べるだけで十分です。

```text
body = resonator(110Hz)
     + resonator(220Hz)
     + resonator(440Hz)
     + high-frequency damping
```

Modal Synthesis 的に共鳴モードを足す設計にすると、木製ボディ、金属ボディ、ガラスボディのような音色変化を作りやすくなります。

---

# 8. WebAssembly 実装上の注意点

## 8.1 WASM 側に持たせるべきもの

WASM / Rust 側に置くべきものは以下です。

```text
・音声 DSP
・voice allocation
・parameter smoothing
・MIDI note handling
・polyphony management
・乱数生成
・物理モデルの内部状態
```

JS / TS 側に置くべきものは以下です。

```text
・UI
・プリセット管理
・MIDI デバイス接続
・AudioContext 管理
・パラメータ送信
・可視化
```

## 8.2 パラメータ更新

UI から AudioWorklet へパラメータを送る方法は主に2つです。

```text
1. AudioParam
2. MessagePort
```

音量、damping、brightness のように連続的に動かすものは AudioParam が向いています。プリセット変更や note_on/note_off は MessagePort でよいです。

ただし、リアルタイム音声ではパラメータの急変によるクリックノイズが起きやすいため、Rust 側で smoothing を入れるべきです。

```rust
pub struct SmoothedValue {
    current: f32,
    target: f32,
    coeff: f32,
}

impl SmoothedValue {
    pub fn next(&mut self) -> f32 {
        self.current += self.coeff * (self.target - self.current);
        self.current
    }
}
```

---

# 9. 既存実装・参考プロジェクト

## 9.1 STK / Synthesis ToolKit

STK は Perry Cook と Gary Scavone による C++ の音声合成・信号処理ツールキットです。公式 GitHub では、STK は C++ で書かれた audio signal processing and algorithmic synthesis classes であり、リアルタイム制御、クロスプラットフォーム性、教育用サンプルコードを重視して設計されたと説明されています。([GitHub][16])

STK は物理モデリング実装の参考として非常に重要です。Rust に移植するというより、クラス設計、楽器モデル、フィルタ構成を読む対象として有用です。

## 9.2 Faust physmodels.lib

Faust の `physmodels.lib` は、物理モデリング楽器のための公式ライブラリです。waveguide、mass-spring、digital wave model を含み、弦、膜、バー、共鳴システムを扱うと説明されています。([ファウストライブラリ][9])

Rust/WASM 実装の前に Faust のモデルを読むと、物理モデリング音源の部品分解がかなり理解しやすくなります。

## 9.3 Web Audio Worklet Samples

Google Chrome Labs の Web Audio Samples には、AudioWorklet、WebAssembly、ring buffer、SharedArrayBuffer + Worker などの実例がまとまっています。特に「Audio Worklet and WebAssembly」「Ring Buffer in AudioWorkletProcessor」は、WASM 音声処理の設計に直結します。([Chrome Labs][17])

---

# 10. 研究・実装ロードマップ

## Phase 1: 最小 Karplus–Strong

目的は「ブラウザで物理モデル音源が鳴る」ことです。

```text
・Rust で KarplusStrong voice 実装
・WASM に compile
・AudioWorklet から process 呼び出し
・note_on / note_off
・monophonic
・damping / brightness UI
```

## Phase 2: Polyphonic Plucked String

```text
・voice allocator
・最大同時発音数 8〜32
・parameter smoothing
・body resonator
・stereo output
・MIDI keyboard 対応
```

## Phase 3: Digital Waveguide

```text
・双方向 delay line
・reflection coefficient
・fractional delay
・loss filter
・pick position
・string stiffness
・bridge model
```

## Phase 4: Modal Resonator

```text
・複数 resonant modes
・material preset
・wood / metal / glass / ceramic
・弦 + ボディ共鳴の分離
```

## Phase 5: 実験的物理モデル

```text
・mass-spring string
・膜モデル
・板モデル
・衝突音
・弓擦弦モデル
```

---

# 11. 技術的な設計判断

## 11.1 Rust は適しているか

適しています。

理由は、

```text
・低レベル DSP を書きやすい
・WASM へコンパイルできる
・メモリ管理を明示しやすい
・リアルタイム処理で避けたい allocation を制御しやすい
・dsp-core を将来 VST/CLAP/ネイティブにも転用できる
```

からです。

NIH-plug のような Rust 製プラグインフレームワークもあり、GitHub では VST3 / CLAP 向けの API-agnostic audio plugin framework として説明されています。([GitHub][18]) つまり、`dsp-core` を独立させておけば、Web 版から DAW プラグイン版へ展開する道もあります。

## 11.2 WASM は適しているか

適していますが、AudioWorklet の制約を理解する必要があります。

AudioWorklet は低レイテンシ音声処理向けですが、Chrome Developers の記事では WASM の処理ブロックサイズと AudioWorklet の 128 frame 処理の差を ring buffer で吸収する設計が示されています。([Chrome for Developers][13])

最初は 128 sample 直接処理で十分です。FFT や大きな FDTD を入れ始めたら ring buffer や worker 分離を検討します。

---

# 12. 最初の設計案

## 音源名

```text
Rust PhysString Synth
```

## 最小仕様

```text
Engine:
  - sample_rate: f32
  - voices: [PluckedString; N]
  - global_gain
  - body_resonator

Voice:
  - Karplus–Strong delay line
  - damping
  - brightness
  - feedback
  - excitation_noise
  - envelope energy tracking

Web:
  - AudioContext
  - AudioWorkletNode
  - WASM module
  - keyboard UI
  - parameter sliders
```

## パラメータ

```text
Pitch
Velocity
Damping
Brightness
Decay
Pick Position
Body Size
Material
String Stiffness
Output Gain
```

---

# 13. 実装時のアンチパターン

避けるべきものです。

```text
・AudioWorklet process 内で Vec::push する
・note_on のたびに巨大 Vec を確保する
・JS から sample 単位で関数呼び出しする
・console.log を audio thread で呼ぶ
・Arc<Mutex<...>> を audio thread に置く
・denormal 対策をしない
・parameter smoothing なしで値を変える
・ピッチを整数 delay のみで実装する
・body resonance なしでリアルさを求める
```

---

# 14. おすすめの学習・参照順

実装前に読むなら、この順番が良いです。

```text
1. Karplus & Strong 1983
2. Jaffe & Smith Extended Karplus–Strong
3. Julius O. Smith - Physical Audio Signal Processing
4. Karjalainen / Välimäki / Tolonen - Plucked String Models
5. STK の Plucked / Mandolin / Clarinet 系クラス
6. Faust physmodels.lib
7. Web Audio AudioWorklet + WASM examples
```

---

# 15. 最終提案

あなたが作るなら、いきなり「汎用物理シンセ」を目指すより、まずは次のような構成が現実的です。

```text
物理ベース弦シンセ
  ↓
Karplus–Strong
  ↓
Extended Karplus–Strong
  ↓
Digital Waveguide
  ↓
Modal Body Resonator
  ↓
WASM AudioWorklet 対応
```

最初の完成形は、以下のようなものが良いです。

```text
ブラウザ上で動く Rust/WASM 製の物理モデリング弦シンセ。
ノイズ励振、分数ディレイ、損失フィルタ、ピック位置、ボディ共鳴を持ち、
サンプルを使わずリアルタイムに弦楽器風・架空楽器風の音を生成する。
```

これは研究的にも実装的にも筋が良いです。
Karplus–Strong から始めれば小さく作れ、Digital Waveguide と Modal Synthesis へ自然に発展できます。Web Audio / WASM の制約とも相性がよく、Rust の強みも活かしやすいです。

[1]: https://www.frontiersin.org/journals/signal-processing/articles/10.3389/frsip.2025.1715792/full?utm_source=chatgpt.com "Editorial: Sound synthesis through physical modeling"
[2]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorklet?utm_source=chatgpt.com "AudioWorklet - Web APIs | MDN"
[3]: https://users.soe.ucsc.edu/~karplus/papers/digitar.pdf?utm_source=chatgpt.com "Digital Synthesis of Plucked-String and Drum Timbres ..."
[4]: https://users.spa.aalto.fi/vpv/publications/cmj98.pdf?utm_source=chatgpt.com "Plucked String Models: From the Karplus Strong Algorithm ..."
[5]: https://sound.eti.pg.gda.pl/student/eim/en/07-Physical.pdf?utm_source=chatgpt.com "PHYSICAL MODELLING SYNTHESIS"
[6]: https://www.dafx.de/paper-archive/2009/tutorials/DAFx09-knp-jos.pdf?utm_source=chatgpt.com "Recent CCRMA Research in Digital ..."
[7]: https://www2.ph.ed.ac.uk/~sbilbao/0470510463.pdf?utm_source=chatgpt.com "Sound synthesis and physical modeling"
[8]: https://arxiv.org/html/2407.05516v1?utm_source=chatgpt.com "Differentiable Modal Synthesis for Physical Modeling of ..."
[9]: https://faustlibraries.grame.fr/libs/physmodels/?utm_source=chatgpt.com "physmodels - Faust Libraries"
[10]: https://www.researchgate.net/publication/353522252_Introducing_Finite_Difference_Schemes_Synthesis_in_FAUST_A_Cellular_Automata_Approach?utm_source=chatgpt.com "Introducing Finite Difference Schemes Synthesis in FAUST"
[11]: https://www.w3.org/TR/webaudio-1.1/?utm_source=chatgpt.com "Web Audio API 1.1"
[12]: https://developer.mozilla.org/ja/docs/Web/API/AudioWorkletProcessor/process?utm_source=chatgpt.com "AudioWorkletProcessor: process() メソッド - Web API | MDN"
[13]: https://developer.chrome.com/blog/audio-worklet-design-pattern?utm_source=chatgpt.com "Audio worklet design pattern | Blog - Chrome for Developers"
[14]: https://github.com/rustaudio/dasp?utm_source=chatgpt.com "RustAudio/dasp: The fundamentals for Digital Audio Signal ..."
[15]: https://crates.io/crates/fundsp?utm_source=chatgpt.com "fundsp - crates.io: Rust Package Registry"
[16]: https://github.com/thestk/stk?utm_source=chatgpt.com "The Synthesis ToolKit in C++ (STK) is a set of open source ..."
[17]: https://googlechromelabs.github.io/web-audio-samples/audio-worklet/?utm_source=chatgpt.com "AudioWorklet | Web Audio Samples"
[18]: https://github.com/robbert-vdh/nih-plug?utm_source=chatgpt.com "robbert-vdh/nih-plug: Rust VST3 and CLAP plugin ..."
