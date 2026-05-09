# Phase 4b 調査資料

## ピアノ音色 (Stretching all-pass for inharmonicity B≈10⁻³ + impact model) の前提整理

本書は Phase 4b 仕様策定で参照する追加調査トピックを集約する。Phase 1〜Phase 4a で既に決着した基礎理論（Karplus–Strong、Lagrange / Thiran 補間、Modal Body M=8 並列 biquad、Loss filter ρ_base=0.05、Pick position 励振 shaping、ParamDescriptor 自動生成、SmoothedValue、SustainState、SoftClip、VoiceState 通信、グローバル LFO + Mod Wheel、Preset JSON v1、Factory Preset 7 種、`wasm-opt -O3`、`excitation_snapshot` cfg(test)、`#![allow(clippy::approx_constant)]` + `#[rustfmt::skip]` の生成器パターン）は重複させず、各 pre-research の該当節を参照する。

Phase 4b は **方式選定の重みが高い**（Stretching all-pass の cascade 段数 M、Hammer model の段数、ピアノ用 Modal Body 係数の独立性）が、**Phase 4a と異なり明確な物理理論（Rauhala–Välimäki 2006 ほか）が存在する**ため、文献調査の比重が大きく実装面の選択幅は比較的少ない。各章末に **結論ボックス（◎採用 / ○検討 / △Phase 4c 以降送り / ×不採用）** を置く（Phase 3 / 4a と同形式）。

---

## 0. Phase 1 / 2 / 3 / 4a pre-research との関係

Phase 4b は以下の節を **既存 pre-research を一次資料**として参照する。

| Phase 節 | 内容 | Phase 4b での参照箇所 |
|---|---|---|
| Phase 1 [§3.1 Karplus–Strong](../2026-05-06-001-mvp/pre-research.md) | 基本原理 (delay + LPF feedback) | §5 Stretching all-pass の挿入点 |
| Phase 2 [§3 Lagrange / Thiran](../2026-05-07-002-phase2/pre-research.md) | 補間 allpass の安定性 | §4 Dispersion allpass cascade の数値安定性 |
| Phase 3 [§2.3 Modal Body 係数](../2026-05-07-003-phase3/pre-research.md) | 8 モード並列 biquad | §7 Piano Modal Body の係数差し替え |
| Phase 3 [§5 Brightness 群遅延補正](../2026-05-07-003-phase3/pre-research.md) | LPF 由来の τ_g 補正 | §5 Stretching allpass の追加群遅延補正 |
| Phase 3 [D36](../../retrospective/2026-05-07-003-phase3.md) | Thiran allpass 案 D 採用 | §5 既存 Thiran との直列同居設計 |
| Phase 3 retrospective [§7.2 Phase 4 候補 #2](../../retrospective/2026-05-07-003-phase3.md) | C8 ピッチ自己発振 | §9 Phase 4b で再評価 (damping=1.0 路線か別路線か) |
| Phase 4a [§7 多楽器プリセット](../2026-05-08-004-phase4a/pre-research.md) | 6 種の Modal 係数手法 | §7 Piano kind を 7 番目として追加する手法 |
| Phase 4a retrospective [§5 既存負債](../../retrospective/2026-05-08-004-phase4a.md) | F38b / WASM 18.42 KB / CRLF / clippy approx_constant | §9 で個別対応 |
| Phase 4a retrospective [§7 推奨スコープ](../../retrospective/2026-05-08-004-phase4a.md) | Phase 4b 候補 13 件 | §1 でリプリント、本書全体で深掘り |

---

## 1. Phase 4b スコープと前提制約

### スコープ確定（仮、§11 でユーザー承認を得る前提）

retrospective §7 の候補 13 件を本書では下表で扱う。Phase 4a の文体に合わせ「主目的 + 補助的負債整理」の二段構成。

| 候補（retrospective §7） | 重要度 | Phase 4b 採否（暫定） |
|---|---|---|
| ピアノ音色 (Stretching all-pass + impact model) | 1 | **◎ 主目的** |
| C8 ピッチ自己発振 | 2 | △ Phase 4c 送り（damping 物理限界、ピアノとは別軸） |
| WASM gzip サイズ調査 (`wasm-opt --print-stats`) | 3 | **◎ §9 で対処（楽器係数増は不可避なのでサイズ計測不可欠）** |
| `__synthDev` 自動計測スクリプト (F38b) | 4 | **◎ §9.2 で組み込み、Phase 4b §0 化** |
| `.gitattributes` で改行 LF 統一 | 5 | **◎ §9.3 着手最初の Step (CRLF 戦争を断つ)** |
| Pick position fractional 化 | 6 | △ Phase 4c 送り |
| Look-ahead limiter | 7 | △ Phase 4c 送り |
| LFO 波形拡張 (S&H / Square / Saw) | 8 | △ Phase 4c 送り |
| LFO destinations 拡張 (Pick / Damping / BodyWet) | 9 | △ Phase 4c 送り |
| 楽器切替の fade-out | 10 | △ Phase 4c 送り（§7.5 で当初軽量実装を検討したが、SmoothedValue 同期 set_target だけでは fade-out が実現できないことが指摘事項 #3 で判明、Phase 4c で `PendingInstrumentChange` 状態機械として再評価） |
| Cross-tab preset 同期 (`storage` event) | 11 | △ Phase 4c 送り |
| Preset JSON import/export | 12 | △ Phase 4c 送り |
| Mono + Sustain 本実装 | 13 | × Phase 5 送り（D29 / D40 / D55 を継承） |
| WASM SIMD | 14 | △ Phase 4c 以降（§10.4 で評価） |

### 制約（Phase 1〜4a 継承、Phase 4b でも維持）

- **WASM gzip < 30 KB（撤退ライン）**: Phase 4a 実測 18.42 KB → Phase 4b 目標 < 22 KB（警戒 22 KB / 撤退 30 KB）。target 15 KB は Phase 4a で諦めた経緯から再目標化しない。
- **依存ゼロ**: `dsp-core` / `wasm-audio` で外部 crate を追加しない。Stretching all-pass cascade も自前、Hammer モデルも `f32::powf` 程度で実装。
- **`Engine::prepare` 以外でヒープ確保禁止**: Stretching all-pass の状態配列 (M 段 × 8 voice = 64 f32) も `KarplusStrong::prepare` 内で固定確保。`apply_instrument` で楽器が Piano に切り替わる際の係数再計算は heap 不要（係数も固定領域）。
- **C ABI のみ**: `wasm-bindgen` 不使用、`#[unsafe(no_mangle)] extern "C"` を継続。Phase 4b では新規関数追加は最大 2 つ（`synth_set_inharmonicity` / `synth_set_hammer_hardness` を検討）。
- **Float32Array view キャッシュ**: Worklet 側の原則維持。
- **Svelte 5 runes**: `$state` / `$derived` / `$effect`、`.svelte.ts` 拡張子。
- **Auto-generated コード**: `gen-params.mjs` 出力に `#![allow(clippy::approx_constant)]` (module) + `#[rustfmt::skip]` (item) を必ず付与（Phase 4a feedback memory 参照）。

### 本書の確定責任

Phase 4b 着手前に以下 6 件を本書で確定させる（仕様書 01〜07 へ橋渡し）:

1. §3 で **Inharmonicity 係数 B の鍵盤範囲依存式 / per-voice or per-instrument 表現**を確定
2. §4 で **Dispersion all-pass cascade の段数 M（4 / 8 / 16）と closed-form 係数式**を確定
3. §5 で **既存 Thiran allpass + Brightness LPF + LossFilter チェーンへの挿入点**を確定
4. §6 で **Hammer / Impact model の方式（commuted impulse + velocity LPF / Hertz Law spring / pre-rendered LUT）**を確定
5. §7 で **Piano 用 Modal Body 係数（soundboard 8 mode 値）**を提示
6. §8 で **新パラメータ追加 (Inharmonicity / HammerHardness) の必要性と ParamId 拡張**を判断

§11 の「実装着手前に答えを出すべき問い」7 件は仕様書策定時に順次決める。

---

## 2. ピアノ物理モデル概観

ピアノ音は「**ハンマーによる弦の打撃 (impact)** → **stiff string の分散波動 (dispersion)** → **soundboard / lid の共鳴**」の連鎖で生成される。Karplus–Strong (KS) 単一弦モデルとの主要差分は以下の 3 点:

| 物理現象 | KS 既存実装 | ピアノで必要 |
|---|---|---|
| 励振 (excitation) | 白色雑音 burst + pick position comb | Hammer felt の非線形応答 (impulse + velocity-dependent LPF or Hertz spring) |
| 弦の波動 | 一様弦 (TM mode のみ、整数倍音) | **Stiff string で f_n = n·f_0·√(1+B·n²) の分散** ← Phase 4b の核心 |
| ボディ共鳴 | M=8 並列 biquad（ギター系） | soundboard + lid の M=8（周波数帯域とゲイン分布が異なる） |

### 2.1 ピアノ弦の物理式

弦の運動方程式は **non-stiff string** (Phase 1〜4a) と **stiff string** (Phase 4b で対応) で異なる:

```
non-stiff:  ∂²y/∂t² = c²·∂²y/∂x²                          (波動方程式、整数倍音)
stiff:      ∂²y/∂t² = c²·∂²y/∂x² − S²·∂⁴y/∂x⁴             (Euler–Bernoulli 項追加)
```

ここで c = 弦上の波速、S = 剛性に由来する係数。stiff string では分散関係が周波数依存となり、第 n 倍音の周波数は:

```
f_n = n·f_0·√(1 + B·n²)
```

ここで **B = (π³·E·a⁴)/(16·L²·T)** が **inharmonicity coefficient**（無次元）。E=Young 率、a=弦半径、L=弦長、T=張力。

### 2.2 主要 DSP 文献（Phase 4b で参照）

- **Smith, J.O. (CCRMA)** *Physical Audio Signal Processing* — "Piano Synthesis" 章、`f_n = n·f_0·√(1+B·n²)` の式 (Eq. 10.32)、JND 閾値 `B_thresh = exp(2.54·log(f_0) − 24.6)` (Eq. 10.31)、commuted piano synthesis の図解
- **Rauhala, J. & Välimäki, V. (2006)** "Tunable Dispersion Filter Design for Piano Synthesis" *IEEE Signal Processing Letters* — 1 次 allpass cascade の **closed-form 係数式**、M=8 推奨
- **Rauhala, J. & Välimäki, V. (2006)** "Dispersion Modeling in Waveguide Piano Synthesis Using Tunable Allpass Filters" DAFx-2006 — 上記の DAFx 版、係数表とプロット
- **Bank, B. & Sujbert, L. (2002)** "Modeling the Longitudinal Vibration of Piano Strings" / DAFx-02 commuted piano synthesis 関連 — Hammer モデルの実時間化
- **Boutillon, X. (1988)** "Model for piano hammers" *J. Acoust. Soc. Am.* 83(2) — Hertz law / 非線形 spring
- **Conklin, H.A. Jr. (1996)** "Piano Design Factors" Acoustics of Pianos — soundboard 第 1 モード 49〜60 Hz
- **Faust standard library** `misceffects.lib` の `piano_dispersion_filter(M, B, f0)` — 上記 closed-form 式の実装、§4 で Rust 移植

---

## 3. Inharmonicity 係数 B の鍵盤範囲依存

### 3.1 測定値（Steinway B / 一般的アップライト）

文献値・実機計測・DAFx 等の既知データから、ピアノ全鍵盤の B レンジは以下:

| 鍵盤レンジ | MIDI ノート | f_0 (Hz) | B 典型値 | 由来 |
|---|---|---|---|---|
| A0 (低音) | 21 | 27.5 | ~3.1 × 10⁻⁴ | Steinway B 計測 (esjs 2024) |
| A2 | 33 | 110 | ~2 × 10⁻⁴ | A0 と A3 の中間補間 |
| A3 | 45 | 220 | ~2.1 × 10⁻⁴ | Steinway B 計測 |
| A4 (中音) | 69 | 440 | ~7.5 × 10⁻⁴ | Steinway B 計測 |
| A5 | 81 | 880 | ~2 × 10⁻³ | A4→C8 の指数増加から補間 |
| A6 | 93 | 1760 | ~5 × 10⁻³ | 同上 |
| A7 | 105 | 3520 | ~2 × 10⁻² | 同上 |
| C8 (最高音) | 108 | 4186 | ~5 × 10⁻² 〜 0.4 | 機種依存大、treble の極端短弦 |

> 参考: Wikipedia / Penn State acs / Acta Acustica 2021 — 「ピアノで B は 0.0002 (bass) から 0.4 (treble) まで」と一般化

**観察**:
- 低音は **bass strings (巻線)** で剛性を抑える設計のため B が小さい (~10⁻⁴)
- 中音 (A4) で `B ≈ 7.5 × 10⁻⁴`（Phase 4b の最初の調整目標、CLAUDE.md の `B≈10⁻³` 記述と整合）
- 高音は短弦 + 太径で B が急増、C8 で B = 0.05〜0.4（機種差大）

### 3.2 鍵盤レンジ依存式

Faust `piano_dispersion_filter` の D 式（実装は §4）が **MIDI 鍵番号 × B の対数双線形** で a1 を決めるため、Phase 4b でも **B の値そのものは MIDI ノートに依存させない**設計が可能:

- **B はピアノ楽器に固有の単一値（仮 7.5 × 10⁻⁴ = A4 値）として保持し、a1 計算式が MIDI ノートに応じて鍵盤位置補正を入れる**

これは Faust `piano_dispersion_filter(M, B, f0)` がそのまま「M=8、B=10⁻⁴ 固定、f0 のみ note ごと」のシグネチャである事実と一致。

### 3.3 per-voice or per-instrument

| 案 | 利点 | 欠点 |
|---|---|---|
| A: B を per-instrument 固定（楽器プリセットの 1 値） | 実装簡素、楽器切替で 1 値だけ更新 | bass→treble の B 違いを表現できない（実機差 >100 倍） |
| B: B を MIDI ノートに応じて per-voice 算出（B(note) = B_mid · 2^((note−69)·k)） | bass/treble の表現力 | k の値選定に試行が必要、テスト複雑化 |
| C: Faust 方式 (B 固定 + a1 式が note 補正) | 実装簡素 + ある程度の鍵盤依存 | a1 補正の精度は文献値と完全一致しない |

**Phase 4b 採用案**: **C の Faust 方式**。`B = 7.5 × 10⁻⁴` を A4 基準で楽器プリセット内に保持し、a1 計算式の `Ikey(f0) = log_2^(1/12)(f0 · 2^(1/12) / 27.5)` 項で note ごとの a1 自動補正を行う（§4.2）。bass〜treble 両端の B 差は犠牲にするが、実装簡素 + 文献根拠ありで Phase 4b の MVP として妥当。

> **§3 結論ボックス: ◎ 採用**
>
> - **B は楽器プリセット固定値 (Piano kind の 1 値、A4 基準で 7.5×10⁻⁴)**
> - **a1 計算式が `Ikey(f0)` 経由で MIDI ノートごとに自動補正**（C 案）
> - **per-voice の B 個別保持はしない**（実装簡素を優先、bass/treble の極端な B 差は Phase 4c 以降で再評価）

---

## 4. Stretching / Dispersion All-pass 設計（Phase 4b の音響面最大決断）

### 4.1 物理的位置付けと既存実装との関係

Stretching all-pass = 周波数依存の位相遅延を持つ allpass フィルタ。ループ内に挿入することで **ループ周回ごとに高次倍音が低次倍音より遅く戻る** → 倍音が「上に伸びる (stretched)」 → `f_n = n·f_0·√(1+B·n²)` の分散関係を再現。

| 既存 (Phase 1〜4a) | Phase 4b 追加 |
|---|---|
| 1 段 Thiran allpass (`fractional_delay.rs:51-90`) | M 段 1 次 dispersion allpass cascade（**新規**） |
| `KarplusStrong::process_sample:317` で `thiran.process(buffer[read_z])` | `thiran.process` の **直前 or 直後** に dispersion cascade を挿入 |
| 1 次 allpass、群遅延 = fractional 部 d (≤ 1 sample) | M 段で群遅延 = 全ての段の和（典型 0〜M sample 程度） |

### 4.2 Closed-form 係数式（Rauhala-Välimäki 2006、Faust `piano_dispersion_filter` 由来）

Faust `misceffects.lib::piano_dispersion_filter(M, B, f0)` の Rust 移植形:

```rust
// M: 1 次 allpass の段数 (8 推奨、< 20)
// B: inharmonicity coefficient (1e-6 で底打ち)
// f0: 基音周波数 (Hz)
// fs: サンプリングレート (Hz)

let trt = 2.0_f32.powf(1.0 / 12.0);
let bc = b.max(1.0e-6);
let log_bc = bc.ln();

// 鍵盤位置 (A0=27.5Hz を 0 とする半音単位インデックス)
let ikey = ((f0 * trt) / 27.5).ln() / trt.ln();

// 係数 kd / Cd (Rauhala-Välimäki 論文の 2 段 fitting 由来)
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

// 主係数
let d = (cd - ikey * kd).exp();
let a1 = (1.0 - d) / (1.0 + d);

// 群遅延 (基音における 1 段あたり、length 補正用)
let wt = 2.0 * core::f32::consts::PI * f0 / fs;
let polydel = |a: f32| -> f32 {
    (wt.sin() / (a + wt.cos())).atan() / wt
};
let group_delay_per_stage = polydel(a1) - polydel(1.0 / a1);
let length_compensation = (m as f32) * group_delay_per_stage;  // 全 M 段
```

各段は 1 次 allpass `H(z) = (a1 + z⁻¹)/(1 + a1·z⁻¹)`（Phase 1〜4a の Thiran と同型、係数式のみ異なる）。M 段カスケードで M 倍の群遅延 → KS ループ全長の中で stretched harmonics を生成。

### 4.3 段数 M の選定

| M | 利点 | 欠点 | Phase 4b 採否 |
|---|---|---|---|
| 1 | CPU 最小 (+1 段) | 高域分散表現が弱い | × 不採用 (1 段では Thiran と同型) |
| 4 | CPU 中 (+4 段)、低中音域は十分 | 高音域 (A6 以上) で分散不足 | ○ 第二候補 |
| **8** | **Faust デフォルト、医療水準のピアノに十分** | CPU +8 段 / 8 voice = 64 段/sample | **◎ Phase 4b 採用 (Faust 標準)** |
| 16 | 高域までほぼ完璧、極端に inharmonic な高音救済 | CPU +16 段、Phase 4b の +0.2 ms target を超えるリスク | △ 高域救済が必要なら Phase 4c |

**Phase 4b 採用**: **M = 8** （Faust 標準、Rauhala-Välimäki 論文の "8 が medium-sized piano に十分" の記述に従う）。

### 4.4 状態配列のヒープ確保ゼロ要件

各 1 次 allpass は z1_in / z1_out の 2 f32 状態を持つため、M=8 / 8 voice で 8 × 2 × 8 = 128 f32 = 512 byte の状態。`KarplusStrong` 構造体に固定配列 `dispersion_states: [DispersionState; 8]` として保持し、`KarplusStrong::prepare` で zero 初期化、`reset` でクリア。

```rust
#[derive(Debug, Clone, Copy)]
struct DispersionStage {
    a1: f32,    // note_on 時に確定 (note ごとに変化)
    z1_in: f32,
    z1_out: f32,
}

pub struct KarplusStrong {
    // ... 既存 fields
    dispersion_stages: [DispersionStage; 8],  // M=8 固定、コンパイル時定数
}
```

heap 確保ゼロ条件は `[DispersionStage; 8]` の inline 配列で満たされる（Vec ではない）。

### 4.5 数値安定性

- a1 ∈ (-1, 1) なら極が単位円内で安定、Phase 1〜4a Thiran と同様の clamp 戦略を流用
- B → 0 で a1 → 0 (allpass = passthrough)、B 大 → a1 大 で分散強
- 上記 closed-form 式は B ∈ [10⁻⁶, 0.4] / f0 ∈ [27.5 Hz, 5000 Hz] で安定動作（Faust 実証済）
- ピアノ最高音 C8 (4186 Hz) でも `Ikey ≈ 87` 程度の rough 計算で発散しないが、念のため Phase 4b では `a1.clamp(-0.999, 0.999)` を適用

### 4.6 既存 Thiran allpass との関係

| 件 | 結論 |
|---|---|
| Thiran allpass (Phase 3 D36 案 D) を残すか | **残す**: dispersion は分数遅延補間とは独立の役割（pitch 精度は Thiran が担う、分散は dispersion cascade が担う） |
| 直列順序 | dispersion cascade → Thiran (整数遅延 + 分数遅延 + 分散の順)、または dispersion → Thiran も等価。順序は **process_sample 内で `read_z` の値を dispersion cascade 8 段に通してから Thiran に通す**で固定 |
| Brightness LPF / LossFilter との順序 | 既存順序維持: dispersion → Thiran → Brightness LPF → LossFilter → damping。Brightness と LossFilter は出力側の音色形成、dispersion は loop 内部の分散表現 |

### 4.7 CPU コスト見積

- 1 段 dispersion allpass: 4 演算/sample (a1·x + z1_in − a1·z1_out + 状態更新 2)
- M=8 / 8 voice: 8 × 8 × 4 = **256 演算/sample**
- 128 frames/process で 32768 演算/process
- WASM 1 GHz 仮定: 32768 / 1e9 = **+0.0328 ms/process**
- Phase 4a 実測 0.023 ms/process に加算で **0.056 ms/process** (target 1.7 ms の 3.3%)

ピアノ kind 以外（Default / GuitarClassical 等）では dispersion を skip する条件付き処理にすれば既存楽器の CPU 増加はゼロ。

> **§4 結論ボックス: ◎ Phase 4b 採用**
>
> - **段数 M = 8**（Faust デフォルト、Rauhala-Välimäki 標準）
> - **closed-form 係数式は §4.2 の Faust 由来式を Rust 移植**（k1〜k3 / m1〜m4 は magic constants として `#[allow(clippy::approx_constant)]` 付きで実装）
> - **状態 `[DispersionStage; 8]` を `KarplusStrong` に inline 保持**（heap 確保ゼロ）
> - **CPU +0.0328 ms/process**（予算余裕大、Phase 4a の 0.023 ms と合計 0.056 ms）
> - **dispersion → Thiran → Brightness LPF → LossFilter → damping の順序**
> - **ピアノ kind 以外では dispersion を skip**（active flag を `KarplusStrong` に持たせて `apply_instrument` で切替）

---

## 5. KarplusStrong への組込み設計

### 5.1 構造変更

```rust
pub struct KarplusStrong {
    // 既存 fields...
    /// Phase 4b: ピアノ用 stretching all-pass cascade (M=8 段)。
    /// `dispersion_active = false` の楽器（ギター系、ベース等）では process_sample で skip。
    dispersion_stages: [DispersionStage; 8],
    /// `apply_instrument` で楽器が Piano に切り替わると true。
    dispersion_active: bool,
}
```

### 5.2 `note_on` での係数算出

`note_on_internal` の末尾（`base_length` を確定した直後）で、**`dispersion_active = true` のときだけ** §4.2 の closed-form を 8 段分計算し、各段の `a1` を保存:

```rust
if self.dispersion_active {
    let a1 = compute_dispersion_a1(M, B_GLOBAL, freq_hz, self.sample_rate);
    for stage in self.dispersion_stages.iter_mut() {
        stage.a1 = a1;     // 全段 同一 a1（Faust 標準）
        stage.z1_in = 0.0;
        stage.z1_out = 0.0;
    }
}
```

### 5.3 `process_sample` ホットパスへの挿入

```rust
// 既存
let read_value_pre_dispersion = self.thiran.process(self.buffer[read_z]);

// Phase 4b 追加: dispersion cascade を Thiran の前 or 後に挿入
let read_value = if self.dispersion_active {
    let mut x = self.buffer[read_z];
    for stage in self.dispersion_stages.iter_mut() {
        let y = stage.a1 * x + stage.z1_in - stage.a1 * stage.z1_out;
        stage.z1_in = x;
        stage.z1_out = y;
        x = y;
    }
    self.thiran.process(x)
} else {
    self.thiran.process(self.buffer[read_z])
};
```

ベンチで dispersion 有無の差分が分かるよう、テスト `test_dispersion_disabled_matches_phase4a` で **Default kind では process 出力が Phase 4a と完全一致**を保証（バイト一致または ε=1e-6 一致）。これが Phase 4a 互換性の中核。

### 5.4 length 補正（Stretching allpass の群遅延）

dispersion cascade も群遅延を持つため、`note_on` 時の `adjusted_length` から **M·polydel(a1) を追加で差し引く** 必要あり:

```rust
let brightness_tau_g = ...;        // 既存 (Phase 3 D37)
let dispersion_tau_g = if self.dispersion_active {
    M as f32 * group_delay_per_stage  // §4.2 の length_compensation
} else { 0.0 };
let total_compensation = brightness_tau_g + dispersion_tau_g;
let adjusted = (raw_len - total_compensation).max(3.0);
```

これにより stretched harmonics の f_0 自体は所望の MIDI 周波数を維持（高次倍音だけが上方ずれる、これがピアノ音色の本質）。

### 5.5 既存テストへの影響

| テスト | 想定影響 | 対処 |
|---|---|---|
| `test_pitch_accuracy_*` (Phase 1〜3) | dispersion_active = false (Default kind) で影響ゼロ | Phase 4a 出力と完全一致を維持 |
| `test_modal_body_*` | 影響なし | — |
| `test_no_allocation_in_process` | dispersion stages は inline 配列なので alloc ゼロ維持 | テスト追加: piano kind で alloc 0 を機械保証 |
| `test_pitch_internal_k_zero_branch` | dispersion_active = false で影響なし | — |

### 5.6 楽器切替経路 (`apply_instrument`)

```rust
pub fn apply_instrument(&mut self, kind: InstrumentKind) {
    // 既存 Phase 4a 処理
    self.pool.all_notes_off();
    ...

    // Phase 4b 追加: pool 内の全 voice の dispersion_active を切替
    let active = matches!(kind, InstrumentKind::Piano);
    self.pool.set_dispersion_active(active);

    // 既存
    self.modal_body.set_instrument(kind, self.sample_rate);
}
```

> **§5 結論ボックス: ◎ Phase 4b 採用**
>
> - **`KarplusStrong` に `dispersion_stages: [DispersionStage; 8]` + `dispersion_active: bool` を追加**
> - **`note_on` で `dispersion_active = true` のとき Faust 由来 closed-form で a1 算出**（§4.2 移植）
> - **`process_sample` で Thiran 前段に dispersion cascade を挿入**
> - **`adjusted_length` から M·polydel(a1) を追加で減算**（length 補正、stretched harmonics の f_0 維持）
> - **Phase 4a 互換**: `dispersion_active = false` (Default + Guitar 系) で出力バイト一致をテスト保証

---

## 6. Hammer / Impact モデル

### 6.1 ピアノの励振が「pick noise burst」と異なる点

| 観点 | Phase 1〜4a (pluck excitation) | ピアノ (hammer impact) |
|---|---|---|
| 物理 | 指 / pick が弦を引いて離す → 三角波 displacement | フェルト hammer が弦を打つ → 短い力パルス (impulse-like force) |
| 励振信号 | 白色雑音 burst で代用 + pick position comb (β) | 力パルス、velocity 依存で **brighter/dimmer** に変化（felt の非線形 spring） |
| KS buffer 初期化 | `[0..length_int]` を bipolar noise で埋める | impulse-train + lowpass filter (commuted synthesis) or Hertz spring 数値積分 |

### 6.2 Hammer モデルの方式選定

文献 (Smith Piano Synthesis 章、Bank-Sujbert 2002、Boutillon 1988、Rauhala 2007) で確立した 4 案:

| 案 | 物理忠実度 | 実装複雑度 | CPU | Phase 4b 採否 |
|---|---|---|---|---|
| A: Boutillon hammer (Hertz law `F = K·x^p`、p≈2.5) を per-sample 数値積分 | ◎ 物理的 | × 高（felt compression x の状態変数 + 弦変位との結合） | × 高（per-sample で 4〜10 演算） | × 不採用 (Phase 4b スコープ超過) |
| B: Wave Digital Hammer (WDF) | ◎ | × 高 | × | × 不採用 |
| C: **Commuted impulse + velocity-dependent LPF**（Bank-Sujbert / Smith CCRMA 推奨） | ○ | ○ 中 | ◎ note_on 時のみ計算、process は KS と同コスト | **◎ Phase 4b 採用** |
| D: Pre-rendered hammer impulse LUT (velocity 1〜127 で 128 LUT) | ○ | ◎ 低 | ◎ note_on で 1 回 lookup | ○ 第二候補 (LUT 容量で不利) |

**Phase 4b 採用**: **案 C（Commuted impulse + velocity-dependent LPF）**。理由:

- 既存 `note_on_internal` が buffer 初期化を担当するパターンと整合（pick noise を hammer impulse に置換）
- velocity 依存の brightness 変化（強打鍵で明るく）が音楽的に必須、案 D の 128 LUT より代数式 1 つの方が WASM サイズに優しい
- CPU ホットパス影響ゼロ（process_sample に追加なし、note_on 時の buffer 初期化のみ）

### 6.3 案 C の実装詳細

`note_on_internal` の buffer 初期化を以下に置換（`dispersion_active = true` のとき）:

```rust
// Phase 4a の pluck excitation 経路 (dispersion 無効時)
if !self.dispersion_active {
    // 既存: noise burst + pick comb
    for i in 0..len_int { self.buffer[i] = self.rng.next_unit_bipolar() * velocity; }
    let k = ...;
    // ... pick comb
} else {
    // Phase 4b: hammer impulse + velocity LPF
    // 1) Impulse 配置 (1 sample に集中、`buffer[0] = 0` に近い位置)
    for v in self.buffer.iter_mut() { *v = 0.0; }
    self.buffer[0] = velocity;  // 単位 impulse

    // 2) Velocity-dependent LPF を buffer 全体に適用 (1 段 1pole IIR)
    //    cutoff = lerp(brightness_low, brightness_high, velocity)
    //    velocity=0.1: cutoff ≈ 800 Hz (dim)
    //    velocity=1.0: cutoff ≈ 4000 Hz (bright)
    let cutoff_hz = HAMMER_CUTOFF_LOW + velocity * (HAMMER_CUTOFF_HIGH - HAMMER_CUTOFF_LOW);
    let alpha = compute_lpf_alpha(cutoff_hz, self.sample_rate);
    let mut z = 0.0;
    for v in self.buffer[..len_int].iter_mut() {
        z = alpha * (*v) + (1.0 - alpha) * z;
        *v = z;
    }

    // 3) Pick position は適用しない (ピアノの hammer は固定位置)
    //    あるいは "hammer position" として keep position 比率で comb 適用 (option)
}
```

定数:
- `HAMMER_CUTOFF_LOW = 800.0`
- `HAMMER_CUTOFF_HIGH = 4000.0`
- これらは `params.json` の楽器プリセット内に持たせる（Phase 4c で UI 露出を検討）

### 6.4 Hammer Hardness（オプションパラメータ）

Phase 4b では UI には露出しない（プリセット切替で代替）。仮にパラメータとして出すなら:

```
HammerHardness ∈ [0, 1]
  0 → soft (cutoff_low_low = 400 Hz, cutoff_high = 2000 Hz)
  1 → hard (cutoff_low = 1500 Hz, cutoff_high = 6000 Hz)
```

§8 で ParamId 追加判断、現状は **Phase 4b では Piano プリセット 1 種で固定**。

> **§6 結論ボックス: ◎ Phase 4b 採用**
>
> - **方式 C: Commuted impulse + velocity-dependent LPF**（CPU ホットパス影響ゼロ）
> - **`note_on_internal` の buffer 初期化を分岐**: `dispersion_active = false` で既存 pluck 経路、`true` で hammer 経路
> - **velocity → cutoff の線形補間**（HAMMER_CUTOFF_LOW=800Hz, HIGH=4000Hz、Piano プリセット内に保持）
> - **Hammer Hardness は Phase 4b では Piano プリセット 1 種で固定、UI 露出は Phase 4c**

---

## 7. Piano 用 Modal Body 係数

### 7.1 ピアノ soundboard / lid と既存 BodyMode の差異

| 観点 | ギター系（Phase 4a） | ピアノ |
|---|---|---|
| 第 1 モード | Helmholtz 60〜130 Hz | **Soundboard mode 1 ≈ 49〜60 Hz**（Conklin 1996） |
| Q | ギターは Q=20〜60 (中程度) | ピアノ soundboard は **Q=8〜30 (中低 Q、減衰早い)** |
| モード密度 | 50 Hz〜2.5 kHz に 8 モード | 50 Hz〜3 kHz に soundboard 主モード + lid 寄与（高域、Q 低） |
| 生成器 | `params.json` + `gen-params.mjs` で 16 値 | 同一経路、新 kind を追加 |

### 7.2 Piano kind の Modal 係数案（文献値ベース）

Conklin 1996 の grand piano soundboard 第 1 モード = 49〜60 Hz、Vibroacoustics of the piano soundboard (HAL 2012) の中低周波モード密度を参考:

```rust
pub const BODY_MODES_PIANO_L: [BodyMode; 8] = [
    BodyMode { freq: 55.0,   q: 10.0, gain: 1.0  },  // soundboard mode 1 (49-60 Hz、Conklin)
    BodyMode { freq: 110.0,  q: 12.0, gain: 0.85 },  // mode 2
    BodyMode { freq: 175.0,  q: 15.0, gain: 0.7  },  // mode 3
    BodyMode { freq: 280.0,  q: 18.0, gain: 0.55 },  // mode 4
    BodyMode { freq: 460.0,  q: 22.0, gain: 0.45 },  // soundboard 中域
    BodyMode { freq: 750.0,  q: 28.0, gain: 0.35 },  // 中高域
    BodyMode { freq: 1300.0, q: 35.0, gain: 0.28 },  // lid 寄与の始まり
    BodyMode { freq: 2200.0, q: 40.0, gain: 0.22 },  // lid 高域
];

pub const BODY_MODES_PIANO_R: [BodyMode; 8] = applyStereoSpread(L, 0.05);  // gen-params.mjs で生成
```

`stereo_spread = 0.05`（Default と同じ、ピアノは対称配置でステレオ広がり中庸）。

### 7.3 Sustain Pedal × Body 共鳴（Phase 4c 検討、Phase 4b スコープ外）

実機ピアノで Sustain Pedal 押下時は **damper が全弦から離れる** ため、打鍵していない弦も sympathetic resonance で振動する。これは現状の `SustainState` (release defer のみ) では再現できない。Phase 4c の独立検討項目とし、Phase 4b では既存 release defer を維持。

### 7.4 楽器プリセット拡張

`params.json` の `instruments` 配列に Piano kind を追加（既存 7 種 → 8 種）:

```json
{
  "kind": "Piano",
  "stereo_spread": 0.05,
  "body_modes": [
    { "freq": 55.0,   "q": 10.0, "gain": 1.0  },
    ...
  ],
  "inharmonicity_b": 7.5e-4,
  "hammer_cutoff_low_hz": 800.0,
  "hammer_cutoff_high_hz": 4000.0
}
```

`inharmonicity_b` / `hammer_cutoff_low_hz` / `hammer_cutoff_high_hz` は Piano kind 専用フィールド。Default 等の他楽器では `null` or 省略（gen-params.mjs で skip）。

### 7.5 楽器切替時の挙動と fade-out（指摘事項 #3 反映で改訂）

Phase 4a D53 の即時 release を継承（楽器切替で全 voice 即時音切れ）。当初本節で「`pool.all_notes_off()` の前後で 5 ms の `output_gain` ramp を入れて pop noise 軽減」を提案していたが、**`SmoothedValue::set_target` は target 代入のみで `current` は `next_sample()` でしか進まないため、同期メソッド内で `set_target(0.0)` → `set_target(prev_value)` を実行しても fade-out は発生しない**ことが指摘事項 #3 で判明。

retrospective §7 の「楽器切替の fade-out」は Phase 4b では:

- △ 軽量実装案: `pool.all_notes_off()` の前後で 5 ms の `output_gain` ramp → **撤回**（SmoothedValue 同期 set_target で実現不能）
- △ 中量実装案: `PendingInstrumentChange` 状態機械（`apply_instrument` で pending 状態を立て、`process` の per-sample loop で fade-out → Modal 差し替え → fade-in を進行） → **Phase 4c 送り**（実装複雑度が大きい）
- × 完全実装案: voice 単位の release ramp（実装複雑、Phase 4c 送り）

**Phase 4b 採用（改訂）**: **× 全部 Phase 4c 送り、Phase 4a D53 即時 release を完全継承**。Phase 4b で `apply_instrument` に追加するのは `pool.set_dispersion_active(piano)` の 1 行のみ（D67）。

> **§7 結論ボックス: ◎ Phase 4b 採用**
>
> - **`InstrumentKind::Piano = 7` を追加**（既存 0-6 を保持）
> - **`BODY_MODES_PIANO_L/R` 8 値を §7.2 の文献値で実装、聴感調整は実装後**
> - **`params.json` の Piano エントリに `inharmonicity_b` / `hammer_cutoff_low_hz` / `hammer_cutoff_high_hz` を追加**
> - **gen-params.mjs を拡張**: Piano kind の追加フィールドを Rust const + TS 定数で出力
> - **楽器切替は Phase 4a D53 の即時 release を継承**（指摘事項 #3 反映、当初の 5 ms fade-out 案は SmoothedValue 同期 set_target の実現不能性により撤回。fade-out / cross-fade は Phase 4c 送り）
> - **Sustain × sympathetic resonance は Phase 4c 送り**（既存 release defer を維持）

---

## 8. 新パラメータ追加の判断

### 8.1 Inharmonicity (B) と HammerHardness の UI 露出

Phase 4b で Piano プリセット 1 種に固定する場合、UI には不要。ただし将来の拡張性（複数 Piano プリセットや Sympathetic 強度など）を考慮すると、**ParamId 拡張**を検討する価値あり。

| パラメータ | UI 露出 | ParamId 追加 | gen-params.mjs |
|---|---|---|---|
| Inharmonicity B | × Phase 4b では非露出（Piano kind 固定） | × | 楽器プリセット内のフィールドのみ |
| HammerHardness | × Phase 4b では非露出（Piano kind 固定） | × | 同上 |

**Phase 4b 採用**: **新規 ParamId は追加しない**。Inharmonicity / HammerHardness は楽器プリセット内のフィールドとして保持し、UI からの直接編集は Phase 4c 以降。

### 8.2 C ABI の追加判断

| 関数案 | Phase 4b 採否 |
|---|---|
| `synth_set_inharmonicity(handle, b)` | × 不採用（Piano kind 固定値で操作不要） |
| `synth_set_hammer_hardness(handle, h)` | × 不採用（同上） |
| `synth_apply_instrument` の拡張 (kind=7 で Piano) | **◎ 既存関数のまま、kind の値域を 0-7 に拡張** |

C ABI 関数追加なし → required exports 19 を維持（Phase 4a と同じ）。

### 8.3 楽器プリセット (Factory Preset) の追加

`web/src/lib/state/factory-presets.ts` に Piano プリセット 1 種を追加:

```typescript
{
  version: 1,
  name: 'Piano',
  createdAt: '2026-05-09T00:00:00.000Z',
  instrument: 'piano',
  params: { damping: 0.998, brightness: 0.55, outputGain: 0.7, pickPosition: 0.13, bodyWet: 0.4 },
  lfo: { rate: 5.0, waveform: 'sine', pitchDepth: 0.0, brightnessDepth: 0.0, volumeDepth: 0.0 }
}
```

Factory Preset は 7 → 8 種に増加、User Preset 上限 32 件は据え置き。

> **§8 結論ボックス: ◎ Phase 4b 採用**
>
> - **新規 ParamId 追加なし**（Inharmonicity / HammerHardness は楽器プリセット内のフィールド）
> - **C ABI 関数追加なし**（required exports 19 維持）、`synth_apply_instrument` の kind 値域を 0-7 に拡張
> - **`factory-presets.ts` に Piano エントリ 1 件追加**（合計 8 種）
> - **将来拡張**: Phase 4c 以降で複数 Piano プリセット (Grand / Upright / Honkytonk) や Sympathetic 強度を ParamId 追加で UI 露出

---

## 9. Phase 4a 持ち越し負債の整理

### 9.1 WASM gzip サイズ調査と最適化（retrospective §5）

**現状**: Phase 4a 18.42 KB（target 15 KB 超過 / 警戒 18 KB 微超過 / 撤退 30 KB クリア）。

**Phase 4b 追加分（見積、§10 詳細）**:
- Stretching all-pass cascade コード: +0.5 KB raw / +0.25 KB gzip
- Hammer LPF コード: +0.2 KB raw / +0.1 KB gzip
- Piano BodyMode 16 値 + 楽器パラメータ: +0.5 KB raw / +0.25 KB gzip
- 合計 +0.6 KB gzip → Phase 4b 想定 19 KB

**対処**:
1. **`wasm-opt --print-stats` で重い pass / 関数を特定**（Phase 4b §0 として組み込み）
2. retrospective §5 推奨「楽器係数を 4 種に削減」は Phase 4b で **採用しない**（Sitar / Mandolin の音色は試聴で良好と判断、削減は音楽性損失）
3. 18 KB 警戒ライン超過は Phase 4b でも継続見込み（19 KB）、撤退ライン 30 KB から余裕は十分

### 9.2 F38b 計測自動化スクリプト（retrospective §5）

**Phase 4b §0 として組み込み**:

```typescript
// web/src/lib/audio/__synthDev.ts (release ビルドにも一時露出)
declare global {
  interface Window {
    __synthDev?: {
      measureProcessTime(durationMs: number): Promise<{ avg: number; max: number; samples: number[] }>;
    };
  }
}

// SynthEngine 起動時に export
window.__synthDev = {
  measureProcessTime: async (durationMs) => {
    const samples: number[] = [];
    const startTime = performance.now();
    while (performance.now() - startTime < durationMs) {
      // Worklet 側で performance.now() ベースで `process` 開始/終了時刻を記録、
      // `port.postMessage({type: 'timing', avg, max})` で main スレッドに送る
      ...
    }
    return { avg: ..., max: ..., samples };
  }
};
```

**実装の制約**:
- **計測方式（指摘事項 #1 反映）**: AudioWorkletGlobalScope は `performance.now()` を持つ（Chrome / Firefox / Safari、精度 ~5μs）ため、`process` 開始/終了で `performance.now()` 差分を取って self time を実測。当初「`currentFrame / sampleRate` を時刻代替」を案としていたが、**`currentFrame` は callback 内で進まないため self time 計測には使えない**（音声時間 128/sampleRate ≈ 2.67ms を返すだけ）と判明、`performance.now()` 方式に変更
- main スレッドに timing を `port.postMessage` で集約 → setInterval ベースで Console 出力
- production deploy では削除する（Phase 4b の最終 commit で `if (import.meta.env.DEV)` ガード）

**判定基準**: avg < 1.7 ms / max < 2.7 ms（Phase 4a 仕様書から継承）。

### 9.3 `.gitattributes` で改行 LF 統一（retrospective §5）

**Phase 4b 着手最初の Step**（CRLF/LF 戦争を断つため）:

```
# .gitattributes (リポジトリ root)
* text=auto eol=lf
*.md   text eol=lf
*.svelte text eol=lf
*.ts   text eol=lf
*.rs   text eol=lf
*.json text eol=lf
*.toml text eol=lf
*.lock text eol=lf
*.mjs  text eol=lf
*.yml  text eol=lf

*.png  binary
*.jpg  binary
*.ico  binary
```

**手順**:
1. `.gitattributes` を作成 + commit
2. `git add --renormalize .` で全 tracked file を LF へ統一
3. CRLF 由来の差分が大量に出るので、独立した commit `chore: normalize line endings to LF` で分離
4. 以降の prettier format で LF/CRLF 差分が再発しなくなることを確認

### 9.4 retrospective §5 のその他項目

| 項目 | Phase 4b 対処 |
|---|---|
| `scripts/copy-wasm.mjs:34` の DEP0190 警告 (`shell: true`) | △ Phase 4c 送り（`cross-spawn` への置換は Windows + Node 24 互換テストが必要） |
| Voice State の Float32Array 再構築 (誤解の訂正、実 alloc 0) | × 対処不要（Phase 4a で確認済） |
| `gen-params.mjs` の整形ポリシー (`#![allow]` + `#[rustfmt::skip]`) | ◎ Phase 4b でも継続、Piano kind 追加時にも適用（feedback memory 参照） |
| `preset-schema.ts` と `messages.ts` の重複参照リスク | ◎ Phase 4b でも単方向 re-export パターン継続（Piano kind の文字列キー追加時もこの方針） |

> **§9 結論ボックス**:
> - **◎ §9.1 wasm-opt --print-stats 適用 + サイズ計測**: Phase 4b 着手最初の調査
> - **◎ §9.2 F38b 計測自動化スクリプト整備**: Phase 4b §0
> - **◎ §9.3 `.gitattributes` LF 統一**: Phase 4b 最初の commit
> - **△ §9.4 DEP0190 警告**: Phase 4c 送り
> - **= §9.4 Voice State**: 対処不要（Phase 4a で確認済）

---

## 10. Phase 4b 性能予算

### 10.1 WASM サイズ予算（gzip）

Phase 4a 実測 18.42 KB、Phase 4b target < 22 KB（警戒 22 KB / 撤退 30 KB）。

| 追加コンポーネント | raw | gzip |
|---|---|---|
| Dispersion all-pass cascade (M=8、係数式 + state 構造体) | +0.5 KB | +0.25 KB |
| Hammer LPF + buffer 初期化分岐 | +0.2 KB | +0.1 KB |
| Piano BodyMode 16 値 + stereo_spread + inharmonicity_b + hammer cutoff (gen-params.mjs 出力) | +0.5 KB | +0.25 KB |
| Piano Factory Preset 1 件 (TS 側) | +0.05 KB | +0.02 KB |
| `apply_instrument` の Piano 分岐 + `set_dispersion_active` 経路 | +0.2 KB | +0.1 KB |
| **Phase 4b 純増** | **+1.45 KB** | **+0.72 KB** |
| **合計（Phase 4a 18.42 KB + 純増）** | — | **~19 KB** |

Phase 4b 後 gzip 想定: **~19 KB**（警戒 22 KB 内、撤退 30 KB から余裕 11 KB）。

### 10.2 早期検証ポイント

| Step | 期待 gzip | 閾値（超過なら撤退検討） |
|---|---|---|
| Step 1 (`__synthDev` 計測 + .gitattributes) | 18.42 KB（不変） | — |
| Step 4 (`gen-params.mjs` 拡張 + Piano BodyMode + inharmonicity_b) | 18.7 KB | > 21 KB なら BodyMode 8 → 5 削減 |
| Step 7 (Dispersion cascade 完成) | 18.95 KB | > 22 KB なら M=8 → M=4 削減 |
| Step 9 (Hammer LPF 完成) | 19.0 KB | > 23 KB なら hammer LPF を簡素化 |
| Phase 4b 全完了 | 19.0 KB | > 25 KB なら R30 等の対策 |

### 10.3 CPU 予算

Phase 4a 実測 0.023 ms/process（128 frames @ 48kHz）。Phase 4b 加算:

| 追加 | 演算数/sample | × 128 |
|---|---|---|
| Dispersion all-pass cascade (M=8 段 × 8 voice、各段 4 演算) | +256 | +32768 |
| Hammer LPF (note_on のみ、process 影響 0) | 0 | 0 |
| `apply_instrument` の Piano 分岐 (event-driven、process 影響 0) | 0 | 0 |
| **合計（Piano 演奏時のみ）** | **+256** | **+32768** |

WASM 1 GHz 仮定: +32768 / 1e9 = **+0.0328 ms/process**。

性能目標 (Phase 4b):
- Piano 演奏時 avg < 0.1 ms（Phase 4a 0.023 ms + 0.033 ms = 0.056 ms、余裕 +0.04 ms）
- Piano 演奏時 max < 0.15 ms
- 他楽器演奏時は Phase 4a と同一（0.023 ms）
- target 1.7 ms に対し利用率 < 6%、余裕 17×

### 10.4 メモリ予算

Phase 4a で `Engine::prepare` 一括確保済。Phase 4b 追加分:

| 追加バッファ | サイズ |
|---|---|
| `KarplusStrong::dispersion_stages: [DispersionStage; 8]` × 8 voice | 8 × 8 × 12 byte = 768 byte |
| `dispersion_active: bool` × 8 voice | 8 byte |
| Piano BodyMode L/R 16 値 (TS 側 const + WASM コード内 const) | TS 約 0.5 KB / WASM コード 約 0.6 KB |
| Hammer cutoff 定数 + LPF 係数 (note_on 内一時変数) | 0 byte (stack のみ) |
| **合計（WASM ヒープ）** | **+0.78 KB** |

`memory.buffer.byteLength` 不変条件は維持可能（Phase 4a の prepare 一括確保戦略を継承）。

### 10.5 WASM SIMD 評価（Phase 4c 送りの根拠）

retrospective §7 候補 #14。2026-05-09 時点の状況:

- WebAssembly SIMD: Chrome 91+ (2021)、Safari 16.4+ (2023)、Firefox 89+ (2021)、ブラウザ対応は **広域達成**（2025 末時点）
- ピアノ用 dispersion cascade (M=8 段 × 8 voice) は SIMD で **8 voice 並列化** が自然な候補
- ただし Phase 4b 主目的（音色実装）の障害にならない CPU 余裕がある（0.056 ms / 1.7 ms = 3.3%）
- `target-feature=+simd128` の Rust ビルドフラグ + `core::arch::wasm32` の `f32x4` intrinsics 活用は **Phase 4c 以降**で実装 + ベンチで効果確認

**Phase 4b では SIMD は採用しない**。Phase 4c 検討候補。

---

## 11. 実装着手前に答えを出すべき問い

Phase 4b 仕様書（01〜07）を策定する前に、以下を確定する。本書 §2〜§10 は方針確定の根拠を提供しているが、**最終判断はユーザー承認**が必要:

1. **Stretching all-pass の段数 M**: 8 (Faust 標準) で確定するか、4 に削減して CPU 余裕を取るか → **8 推奨**
2. **Inharmonicity B の表現**: Faust 方式 (per-instrument 固定値 + 鍵盤位置補正) で確定するか、per-voice で動的調整するか → **Faust 方式推奨**
3. **Hammer モデル**: Commuted impulse + velocity LPF で確定するか、Hertz law spring（重い）か pre-rendered LUT（容量大）か → **Commuted 方式推奨**
4. **Piano kind の Modal 係数**: §7.2 の文献値ベースで確定するか、別案（Steinway / Yamaha 計測値の併記）か → **文献値ベース推奨、聴感調整は実装後**
5. **新規 ParamId 追加**: なし（Inharmonicity / HammerHardness は楽器プリセット内）で確定するか → **なしで確定**
6. **楽器切替時の fade-out（指摘事項 #3 反映で改訂）**: 5 ms ramp 軽量実装 / 即時 release 継続 / `PendingInstrumentChange` 状態機械のいずれか → **即時 release 継続（Phase 4a D53 継承）で確定**（5 ms ramp は SmoothedValue 同期 set_target で実現不能、`PendingInstrumentChange` は Phase 4c 送り）
7. **C8 ピッチ自己発振の Phase 4b 取扱**: △ Phase 4c 送りで確定するか、Phase 4b で同時着手か → **Phase 4c 送りで確定（ピアノとは別軸の damping 物理限界）**

---

## 12. 文献 + 参考実装

### 12.1 Phase 4b で新規参照（必読）

- **Smith, J.O.** *Physical Audio Signal Processing*（CCRMA、Web 公開） — "Piano Synthesis"、"Piano Hammer Modeling"、"Stiff String Synthesis"、"Commuted Piano Synthesis" 各章。Phase 4b の理論基盤
- **Rauhala, J. & Välimäki, V. (2006)** "Tunable Dispersion Filter Design for Piano Synthesis" *IEEE Signal Processing Letters* 13(5), pp. 253–256 — closed-form 式の出典
- **Rauhala, J. & Välimäki, V. (2006)** "Dispersion Modeling in Waveguide Piano Synthesis Using Tunable Allpass Filters" *Proc. DAFx-2006*, pp. 71–76 — 上記の DAFx 版、a1 値の B / f0 マップ図あり
- **Bank, B. & Sujbert, L. (2002)** "Modeling the Longitudinal Vibration of Piano Strings" / *Proc. DAFx-2002* — Hammer モデルの実時間化、commuted synthesis 拡張
- **Boutillon, X. (1988)** "Model for piano hammers: Experimental determination and digital simulation" *J. Acoust. Soc. Am.* 83(2), pp. 746–754 — Hertz law `F = K·x^p`、p ≈ 2.5
- **Conklin, H.A. Jr. (1996)** "Design and tone in the mechanoacoustic piano: III. Piano strings and scale design" *J. Acoust. Soc. Am.* 100(3) — soundboard 第 1 モード 49〜60 Hz
- **Fletcher, N.H. (1964)** "Normal Vibration Frequencies of a Stiff Piano String" *J. Acoust. Soc. Am.* 36(1) — `f_n = n·f_0·√(1+B·n²)` の原典
- **Faust standard library** `misceffects.lib::piano_dispersion_filter` ([GitHub](https://github.com/grame-cncm/faustlibraries/blob/master/misceffects.lib)) — closed-form 式の実装。本書 §4.2 の Rust 移植元

### 12.2 Phase 1〜4a の継続参照

- Phase 4a [§7 多楽器プリセット](../2026-05-08-004-phase4a/pre-research.md) — Piano kind 追加時の既存パターン
- Phase 3 [§2.3 Modal Body](../2026-05-07-003-phase3/pre-research.md) — 8 mode 並列 biquad の Piano 用係数
- Phase 3 [D36 案 D](../../retrospective/2026-05-07-003-phase3.md) — Thiran allpass の安定性、dispersion cascade と同居設計の参考
- Phase 2 [§3 Lagrange / Thiran](../2026-05-07-002-phase2/pre-research.md) — fractional delay の挿入順序

### 12.3 参考実装

- **JUCE `PianoForte` voice / `MPE Synth`** — ピアノモデル実装の構造参考
- **Pianoteq (Modartt)** — 物理ベースピアノ商用実装、内部アルゴリズムは非公開だが Bank-Sujbert 系
- **MoForte / MOPHO** — DAFx 系の物理モデルピアノオープンソース実装
- **Pianobook STK Synth** — Stanford STK ライブラリのピアノクラス（C++）

---

## 13. Phase 4b で参照しない領域（Phase 4c 以降送り）

| 領域 | 理由 |
|---|---|
| **C8 ピッチ自己発振** | damping=1.0 経路 or FFT estimator が要、ピアノとは別軸の物理限界、Phase 4c |
| **Pick position fractional 化** | ピアノは hammer 固定位置で pick 概念なし、Phase 4c |
| **Look-ahead limiter** | 5 ms 遅延型、Soft clip より透明だが Phase 4b 主目的と無関係、Phase 4c |
| **WASM SIMD** | CPU 余裕大（3.3%）、Phase 4b 後でベンチ取得後に判断、Phase 4c |
| **LFO 波形 S&H / Square / Sawtooth** | Phase 4a §3.3 と同じ理由、Phase 4c |
| **LFO destinations 拡張 (Pick / Damping / BodyWet)** | 同上、Phase 4c |
| **Cross-tab preset 同期 (storage event)** | UX 需要が Phase 4c 以降で出れば検討 |
| **Preset import / export (JSON file)** | localStorage 内のみで当面十分、Phase 4c |
| **Mono+Sustain 本実装** | Phase 2 D29 / Phase 3 D40 / Phase 4a D55 を継承、相反性の本質的問題、Phase 5 |
| **Sustain × Sympathetic resonance** | ピアノ damper の物理は別 Engine 構造が要、Phase 4c |
| **複数 Piano 機種プリセット (Grand / Upright / Honkytonk)** | Phase 4b は Piano 1 種で実機検証、複数化は Phase 4c |
| **Hammer Hardness UI 露出** | Phase 4b では Piano kind 内固定、UI 露出は Phase 4c |
| **管楽器 / 打楽器** | Phase 5 領域 |
| **録音・MIDI export** | Phase 5 領域 |
| **Soundboard / lid の高次モード (M=16)** | Modal Body M=8 で Phase 4b は妥当、M 拡張は Phase 4c |
| **Voice State SAB 化** | COOP/COEP 必要で GitHub Pages 不可（Phase 4a 継承） |

---

## 14. Phase 4b 実装順序の試案（07 章への種）

本書の結論を統合した実装順:

1. **Step 1**: `.gitattributes` で改行 LF 統一 + `git add --renormalize .` で既存 file 統一（§9.3）
2. **Step 2**: `__synthDev.measureProcessTime` で F38b 計測自動化スクリプト整備（§9.2）
3. **Step 3**: `wasm-opt --print-stats` で Phase 4a の WASM 重い pass を確認 + ベースライン記録（§9.1）
4. **Step 4**: `params.json` 拡張: Piano kind の `body_modes` 8 値 + `stereo_spread` + `inharmonicity_b` + `hammer_cutoff_low_hz` + `hammer_cutoff_high_hz`（§7.4）
5. **Step 5**: `gen-params.mjs` 拡張: Piano 専用フィールドを Rust const + TS 定数で出力（§7.4）
6. **Step 6**: `dsp-core/src/dispersion.rs` 新規実装 (M=8 段 cascade + closed-form 係数式)（§4 / §5）
7. **Step 7**: `KarplusStrong` 構造体に `dispersion_stages` / `dispersion_active` 追加 + `note_on` で a1 算出 + `process_sample` で cascade 適用（§5.1〜§5.3）
8. **Step 8**: `note_on_internal` の buffer 初期化を分岐: pluck (既存) / hammer impulse + velocity LPF（§6）
9. **Step 9**: `Engine::apply_instrument` 末尾に `pool.set_dispersion_active(matches!(kind, Piano))` の 1 行を追加（§5.6、§7.5、Phase 4a D53 即時 release を継承。当初の 5 ms fade-out 提案は指摘事項 #3 反映で撤回）
10. **Step 10**: `InstrumentKind::Piano = 7` を `gen-params.mjs` 出力に追加 + `synth_apply_instrument` の値域拡張（§7.4）
11. **Step 11**: `web/src/lib/state/factory-presets.ts` に Piano プリセット 1 件追加（§8.3）
12. **Step 12**: `dsp-core/tests/dispersion_tests.rs` 新規 (a1 値の closed-form 検証 + cascade 安定性 + alloc 0)（§4.5）
13. **Step 13**: `dsp-core/tests/instrument_tests.rs` 拡張 (Piano kind 追加検証 + Phase 4a 互換性 = `dispersion_active = false` で出力一致)（§5.5）
14. **Step 14**: 統合 cargo test + alloc ゼロ検証 + WASM サイズ計測 + Piano 演奏 cargo timing（§10）
15. **Step 15**: 実機確認: `pnpm dev` で Piano プリセット選択 → 音響的に「ピアノっぽい」聴感確認 + Phase 1-4a の他楽器が regression なし（§11 #4）
16. **Step 16**: `__synthDev.measureProcessTime` で Phase 4b 実機 process 時間計測（§9.2）
17. **Step 17**: ドキュメント整備（README / CLAUDE.md / Phase 4b 仕様書 retrospective 準備）
18. **Step 18**: PR 作成 + main マージ

各 Step は仕様書 07 章で `cargo test` / 実機検証の達成ラインを明示する（Phase 1〜4a と同じ流儀）。

---

## まとめ（1 行）

> Phase 4b は「**Stretching all-pass cascade (M=8, Rauhala-Välimäki closed form, B≈7.5×10⁻⁴ at A4)** で stiff string の inharmonicity を表現 + **Commuted impulse + velocity-dependent LPF** で felt hammer を表現 + **Piano 用 Modal Body 係数 (soundboard mode 1=55Hz, M=8)** + **InstrumentKind::Piano = 7** + **Factory Preset Piano 1 件**」を主目的、補助的に **`.gitattributes` LF 統一 / `__synthDev` 計測自動化 / `wasm-opt --print-stats` 調査** で Phase 4a の負債を整理。WASM gzip target ~19 KB（警戒 22 KB / 撤退 30 KB から余裕大）、CPU +0.033 ms/process（合計 0.056 ms = target 1.7 ms の 3.3%）で予算余裕大、新規 ParamId / C ABI 追加なしで Phase 4a 互換を維持。Phase 4c は WASM SIMD / C8 自己発振 / Sustain×sympathetic で別計画。
