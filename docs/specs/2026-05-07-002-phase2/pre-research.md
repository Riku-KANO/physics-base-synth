# Phase 2 調査資料

## ポリフォニー・分数ディレイ・ParamDescriptor・hold note stack の前提整理

本書は Phase 2 仕様策定で参照する追加調査トピックを集約する。Phase 1 で既に調査済みの基礎理論（Karplus–Strong、Digital Waveguide、Modal Synthesis、リアルタイム制約一般）は重複させず、Phase 1 `pre-research.md` の該当節を参照する。

## 0. Phase 1 pre-research との関係

Phase 2 でも以下の節は **Phase 1 pre-research を一次資料**として参照する。

| Phase 1 節 | 内容 | Phase 2 での参照箇所 |
|---|---|---|
| [§3.1 Karplus–Strong アルゴリズム](../2026-05-06-001-mvp/pre-research.md) | 基本原理、擬似コード、`delay_length ≒ sample_rate / frequency` | 03 章 KarplusStrong に分数ディレイを統合する際の起点 |
| [§3.2 Digital Waveguide Synthesis](../2026-05-06-001-mvp/pre-research.md) | 双方向ディレイライン、fractional delay の概論（Lagrange / all-pass）| 本書 §3 で深掘り |
| [§7.2 Extended Karplus–Strong](../2026-05-06-001-mvp/pre-research.md) | fractional delay / loss filter / pick position / stretching all-pass / body resonator | Phase 2 で fractional delay のみ着手、残りは Phase 3 |
| [§8.1 WASM 側に持たせるべきもの](../2026-05-06-001-mvp/pre-research.md) | voice allocation、polyphony management が WASM 側責務 | 本書 §1 で具体化 |
| [§10 Phase 2: Polyphonic Plucked String](../2026-05-06-001-mvp/pre-research.md) | voice allocator、最大同時発音数 8〜32、parameter smoothing、stereo output、MIDI keyboard | 本書全体で Phase 2 ロードマップとして引用 |
| [§13 実装時のアンチパターン](../2026-05-06-001-mvp/pre-research.md) | `process` 中 Vec::push 禁止、整数 delay のみは避ける、parameter smoothing なしで値を変えない | Phase 2 でも継承、本書 §6 で再強調 |

## 1. ポリフォニー設計の原則

ポリフォニーシンセは「同時発音できる音の数 N」と「ボイスの確保戦略」と「ボイスの割当戦略」の 3 軸で設計される。Phase 1 はモノフォニー（N=1）固定だったため、Phase 2 では N≥2 を前提に再設計する。

### ボイスの確保戦略

| 戦略 | 概要 | リアルタイム適性 |
|---|---|---|
| **固定配列 + busy/free フラグ**（Phase 2 採用想定） | `[Voice; N]` を起動時一括確保し、各ボイスに「使用中か否か」を表すフラグを持たせる | ◎ ヒープ確保ゼロ、キャッシュ局所性も良い |
| フリーリスト方式 | 空いているボイスのインデックスを別配列で管理（`free_indices: Vec<usize>`）し、`pop` / `push` で取得・返却 | △ 動的容量変動が起きやすい |
| ヒープアロケーション方式（`Vec<Box<dyn Voice>>`） | 必要時に `Box::new` で確保 | × `process` 中の確保はリアルタイム禁止 |

Phase 1 [§13 実装時のアンチパターン](../2026-05-06-001-mvp/pre-research.md)「`note_on` のたびに巨大 Vec を確保する」を回避するため、固定配列方式を採用する想定。

### ボイス割当戦略

新しい `note_on` が来たときに「どのボイスに割り当てるか」を決めるロジック。複数候補がある:

| 戦略 | 動作 | 想定用途 |
|---|---|---|
| **same-note-replace** | 同じノートが既に鳴っていればそのボイスを再励振 | キー連打時に音が二重化しない |
| **first-free** | 空きボイスがあれば最若番に割当 | シンプル、決定的動作 |
| oldest-first | 最も古いボイスを上書き | ボイスが詰まったときの自然な置換 |
| round-robin | 単に順番に割り当てる | テスト時の追跡が簡単 |

実用ポリシーは「同名ノート優先 + 空きボイス優先 + 全埋まりなら stealing」の 3 段階。本書 §2 の voice stealing と組み合わせる。

## 2. Voice stealing 文献整理

ボイス上限 N に達した状態で新しい `note_on` が来たとき、既存ボイスを 1 つ犠牲にする処理を **voice stealing** と呼ぶ。

### 主要アルゴリズム

| 戦略 | 判定基準 | コスト | 音楽的自然さ |
|---|---|---|---|
| **oldest-first** | ボイスごとに `age: u32` を持ち、最大値を選ぶ | O(N) 比較のみ | 自然（最も古い音は耳に残らない傾向） |
| **quietest-first** | ボイスごとの `energy / amplitude` 推定値で最小を選ぶ | O(N) 比較 + envelope tracking が必要 | より自然（鳴っていない音を優先的に犠牲）|
| **same-note-replace** | 既存ボイスの note_id と一致するものを優先選択 | O(N) 比較 | 連打・トリル時に最自然 |
| round-robin | 単純にインデックスを循環 | O(1) | 不自然な上書きが起こる |
| released-first | `note_off` 済みで減衰中のボイスを優先 | O(N) フラグ確認 | 持続音の保護に有効 |

### 参考実装

| プロジェクト | 戦略 | 補足 |
|---|---|---|
| **STK** (`Synthesiser`) | oldest-first を基本に、note_off 済みボイス優先のフォールバック | C++、教育用 |
| **JUCE** (`Synthesiser`) | 仮想 `findVoiceToSteal` をユーザー実装可能、デフォルトは released > oldest | C++、商用 DAW プラグイン基盤 |
| **SuperCollider** (`Sustain.kr`) | release time との連携、quietest-first 寄り | DSP 言語、ライブコーディング |
| **Pure Data** (`poly`) | oldest-first | dataflow 環境 |

### Phase 2 での想定

Phase 2 では「同名ノート最優先 + 全埋まりは energy が閾値以下のボイス優先 + さらに同点なら oldest」の 2 段階フォールバックを採用する想定。Phase 1 `KarplusStrong::energy` フィールド（`crates/dsp-core/src/karplus_strong.rs:18`）を流用できるため追加コストが小さい。

## 3. Fractional delay 文献整理

整数ディレイのピッチ精度限界を超えるための補間手法。Phase 1 [`docs/specs/2026-05-06-001-mvp/01-overview.md` D1](../2026-05-06-001-mvp/01-overview.md) は「整数ディレイで割り切る」を採用、Phase 1 retrospective §3 D1 で「A1=55Hz の 2.3% 誤差は実音で要確認、Phase 2 で fractional delay へ」と申し送られている。

### 主要手法の比較

| 手法 | 補間次数 | 群遅延の周波数依存 | 実装コスト | 安定性 | Phase 2 採用候補 |
|---|---|---|---|---|---|
| **Linear interpolation** | 1 次 | 中（高域で位相歪み） | 最小（2 サンプル線形補間） | ○ | × Phase 1 妥協の延長線で改善幅小 |
| **Lagrange interpolation 3 次** | 3 次 | 小（fractional delay として一般的） | 4 サンプル積和 + 3 つの係数計算 | ◎ | ◎ Phase 2 第一候補 |
| **Cubic Hermite (Catmull-Rom 等)** | 3 次 | 中 | 4 サンプル + 係数計算 | ○ | △ Lagrange とほぼ同等性能だが採用例少 |
| **Thiran allpass 1 次** | 1 次 IIR | 極小（位相応答ほぼ理想） | 1 サンプル + 1 係数 | △ 係数が delay に依存し再計算が必要 | ○ Phase 3 での再検討候補 |

### Lagrange 3 次の式（参考）

ディレイ長 `D = D_int + d`（`d ∈ [0, 1)`）に対し、4 サンプル `x[n-D_int+1], x[n-D_int], x[n-D_int-1], x[n-D_int-2]` を取り出して

```text
y = Σ_{k=0..3} h_k(d) * x[n-D_int+1-k]
h_0(d) = -d(d-1)(d-2) / 6
h_1(d) = (d+1)(d-1)(d-2) / 2
h_2(d) = -(d+1)d(d-2) / 2
h_3(d) = (d+1)d(d-1) / 6
```

係数は `d` のみに依存するため、Phase 2 では **`note_on` 時に 1 度だけ計算**してキャッシュすれば `process_sample` 内のコストは積和 4 回のみ。

### 参考文献

- Välimäki & Laakso (2000) "Principles of Fractional Delay Filters" IEEE ICASSP — Lagrange / Thiran の原理整理
- Smith J.O. *Physical Audio Signal Processing* CCRMA — fractional delay の章で各種手法を網羅
- STK `DelayA` クラス — Thiran allpass 1 次の C++ 実装
- Faust `de.fdelay` — Lagrange / Thiran の Faust 実装

### Phase 2 での想定

Phase 2 では **Lagrange 3 次補間** を採用候補とする。理由は (a) 係数が `d` のみに依存し再計算コストが軽い、(b) 群遅延の周波数依存が小さく弦シミュとして十分、(c) FIR で係数和 1.0（DC ゲイン保存）のため Thiran allpass IIR と異なり別途の極配置議論が不要で **フィードバックループ全体の安定性リスクが低い**（damping < 1.0 と LPF が低域通過なら発散しない、03 章 §process_sample の変更を参照）。Thiran allpass は Phase 3 で pitch bend / vibrato が必要になった段階で再評価する。

## 4. ParamDescriptor / コード生成パターン

Phase 1 では `crates/dsp-core/src/params.rs` の `ParamId` enum と `web/src/lib/audio/messages.ts` の `PARAM_IDS` 定数を **手動で同期**しており、Phase 1 retrospective §5「既存コードの負債」で「`params.json` + コード生成で解消」と申し送られている。

### 既存プラグイン規格の ParamDescriptor 構造比較

| 規格 / ライブラリ | 構造体名 | フィールド | 備考 |
|---|---|---|---|
| **VST3 SDK** | `Vst::Parameter` / `ParameterInfo` | id, title, units, defaultNormalizedValue, stepCount, flags | C++、normalize 0..1 前提 |
| **CLAP** | `clap_param_info_t` | id, name, module, min_value, max_value, default_value, flags | C ABI、min/max を直接保持 |
| **JUCE** | `RangedAudioParameter` | name, range (min, max, step), defaultValue, label | C++、範囲オブジェクト分離 |
| **Faust** | `metadata` 注釈 | min, max, init, step, unit, scale | DSL、ソース内に注釈 |

### Phase 2 想定の ParamDescriptor 構造

Phase 1 retrospective §5 で挙げられた負債（clamp パターン重複 / Rust↔TS drift）を解消するため、最低限以下を含む:

```text
ParamDescriptor {
  id: u32,
  name: &'static str,
  min: f32,
  max: f32,
  default: f32,
  smoothing_tau: f32,  // SmoothedValue 設定用
}
```

JUCE の `range` 構造を意識して min/max/step/default を構造体内に閉じ込める形が、Rust enum + 定数群（`DAMPING_MIN/MAX/DEFAULT`、`crates/dsp-core/src/params.rs:21-32`）を一本化しやすい。

### コード生成パイプラインの選択肢

| 方式 | 生成タイミング | Phase 2 採用候補 | 理由 |
|---|---|---|---|
| `build.rs`（Rust 公式の build script） | `cargo build` 時 | △ | dsp-core に build dependency が増える、Node 側との連携が組みづらい |
| **外部 Node スクリプト** (`scripts/gen-params.mjs`) | `pnpm gen:params` 手動 + `pnpm build:wasm` 前段で自動 | ◎ | Phase 1 の `scripts/copy-wasm.mjs` / `scripts/check-wasm-exports.mjs` と同パターンで統一 |
| `xtask` パターン（cargo subcommand） | `cargo xtask gen-params` | × | Phase 1 が Node に統一しているため逸脱 |

### 同期チェック方針

Rust/TS 双方の生成物が `params.json` から正しく派生していることを CI で保証するため、`scripts/check-params-sync.mjs` を `pnpm check` の一部として常時実行する想定（Phase 2 検証項目 F14/F15、本仕様 06 章）。

## 5. Hold note stack の実装パターン

モノフォニーシンセで複数キー押下時の挙動を「**last-note priority + hold stack**」で扱う伝統的設計。Phase 1 retrospective §5（`crates/dsp-core/src/engine.rs:80-94`）で「last-note priority 簡易版（hold note stack なし）」が負債として記録されている。

### 想定シナリオの差分（再掲）

| 操作 | Phase 1 の挙動（hold なし） | Phase 2 の挙動（hold あり） |
|---|---|---|
| C(60) on | C 発音、`current_note = Some(60)` | C 発音、stack = [60] |
| D(62) on | D 発音、`current_note = Some(62)`、C は破棄 | D 発音、stack = [60, 62] |
| D off | `current_note = None`、damping 加速で減衰 | stack = [60]、top の C に再トリガー |
| C off | C は復帰しない（`current_note != Some(60)` のため何もしない） | stack = []、damping 加速で減衰 |

Minimoog や Roland SH-101 など 1970-80 年代モノフォニックシンセが採用した方式。**最後に押されたキーを最優先し、離されたら次に古いキーへ復帰**する。

### データ構造の選択肢

| 構造 | メモリ | 操作コスト | Phase 2 採用候補 |
|---|---|---|---|
| **固定配列 + len（自前 LinearStack）** | `[u8; MAX_HELD]` + `len: usize` | push / remove は O(N) 線形走査 | ◎ heapless 等の依存なし、Phase 1 の依存ゼロ方針を維持 |
| `heapless::Vec<u8, MAX>` | 同等 | 同等 | × 外部依存追加 |
| `[u8; MAX_HELD]` 直接管理 | 同等 | 同等だが API 露出が煩雑 | × |
| `Vec<u8>` | ヒープ確保 | O(N) | × `process` 中の push でヒープ確保 |

### 容量

PC キーボード割当（Phase 1 `web/src/lib/actions/pc-keyboard.svelte.ts` の `MAPPING`）が 15 鍵、画面鍵盤が 25 鍵 (C3-C5)、MIDI フル鍵盤が 88 鍵だが、モノモードで現実的に同時押下されるのは多くて 10 鍵程度。Phase 2 では **MAX_HELD = 16** を想定（実用上十分、配列サイズで .text セクションへの負担小）。溢れ時は **最古のノートを破棄** が音楽的に自然。

## 6. リアルタイム制約とポリフォニー時の CPU 予算

Phase 1 の性能予算は 128 frames @ 48kHz = **2.67 ms/process** で、実測 < 0.5 ms（[Phase 1 06 章 性能目標](../2026-05-06-001-mvp/06-build-and-verify.md#性能目標mvp)）。Phase 2 で N ボイス化すると単純線形スケールで N 倍。

### N=8 ボイス時の見積もり

| 項目 | Phase 1 実測 | Phase 2 N=8 見積 | 余裕 |
|---|---|---|---|
| process 1 回（128 frames） | < 0.5 ms | < 4.0 ms（線形スケール） | ✗ 予算 2.67 ms を超える可能性 |
| process 1 回（128 frames、N=8 + Lagrange 3 次） | — | 約 5-6 ms 想定 | ✗ |
| 妥当な目標値 | < 1.5 ms | — | 予算の 56%、N=8 で達成可能か要計測 |

### Phase 2 性能目標の根拠

Phase 1 の 0.5 ms から N=8 の 8 倍 = 4.0 ms に Lagrange 3 次の積和コスト（3 倍程度）を上乗せすると最悪 12 ms と試算されるが、以下の最適化で 1.5 ms 以内を目指す:

1. **早期 return**: ボイスが `is_active() == false` ならループ内で 1 行 return（Phase 1 既存）
2. **inactive ボイスのスキップ**: 全ボイスが空のときは process 全体を 0 fill
3. **Lagrange 係数の `note_on` 時計算**: process 内では積和 4 回のみ
4. **`#[inline]` 維持**: `process_sample` の関数呼び出し展開（Phase 1 既存、`crates/dsp-core/src/karplus_strong.rs:103`）
5. **wasm-opt -O3** でリリースサイズ・速度両方最適化（Phase 2 では必須化候補、本仕様 02 章）

WASM SIMD（`target-feature=+simd128`）は Phase 1 retrospective §7 で調査項目として残されているが、Phase 2 では使わず Phase 3 の改善余地として残す（ブラウザ互換性が安定したら採用）。

### WASM サイズ予算

Phase 1 実測 16.49 KB / gzip 7.98 KB（[Phase 1 §8 メトリクス](../2026-05-06-001-mvp/06-build-and-verify.md)）。Phase 2 で追加されるコード:

- VoicePool, NoteAllocator, HoldStack: 合計 +5-8 KB 想定
- Lagrange 3 次補間: +1-2 KB
- ParamDescriptor 構造体（生成コードは const テーブルとして埋め込み）: +0.5-1 KB

合計 +7-11 KB → Phase 2 release WASM 約 25-28 KB / gzip 約 12-14 KB の見積。Phase 2 性能目標 **gzip < 30 KB** は十分達成可能、ただし `wasm-opt -O3` を必須化することで余裕を確保する。

## 7. 引き続き有効な Phase 1 文献

Phase 2 でも以下の Phase 1 §9 文献を継続参照する:

- **STK（Synthesis ToolKit）** — `Synthesiser` の voice management、`DelayA` の Thiran allpass、`Plucked` の KS 拡張実装
- **Faust `physmodels.lib`** — `pluckString` の Lagrange fractional delay、`smooth` のパラメータスムージング
- **Web Audio Worklet Samples** — AudioWorklet + WebAssembly のレンダリング比較サンプル

## 8. Phase 2 で参照しない Phase 1 文献

Phase 1 で言及されたが Phase 2 スコープ外（Phase 3 以降の領域）:

- §3.3 Modal Synthesis — Body Resonator が Phase 3 で本格着手するまで再参照不要
- §3.4 Mass-Spring Model — Phase 4-5 領域
- §3.5 Finite Difference / FDTD — Phase 5 領域
- §11.1-11.2 Rust / WASM 適性議論 — Phase 1 で結論済み
