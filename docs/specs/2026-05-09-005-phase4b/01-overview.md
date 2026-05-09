# 01. Phase 4b 概要とスコープ

## 目的

Phase 4a で確立した「ブラウザで動作する 8 音ポリフォニック Karplus–Strong + Modal Body Resonator + Extended KS + MIDI CC + Voice Meter UI + Soft clip + Thiran allpass + LFO + Mod Wheel + Preset (localStorage v1) + 多楽器プリセット 7 種 (Default + 6 楽器) + `wasm-opt -O3` + `excitation_snapshot` cfg(test)」を土台に、**ピアノ音色を Stretching all-pass cascade (M=8 段、Rauhala-Välimäki closed form, B≈7.5×10⁻⁴ at A4) + Commuted impulse + velocity-dependent LPF (Hammer model) + Piano 用 Modal Body 係数 (soundboard mode 1=55Hz) で実装し、`InstrumentKind::Piano = 7` を追加 + Factory Preset に Piano 1 件追加** する。補助的に **`.gitattributes` で改行 LF 統一**、**`__synthDev.measureProcessTime` で F38b 計測自動化スクリプト整備**、**`wasm-opt --print-stats` で Phase 4a の WASM サイズベースライン記録** で Phase 4a の負債を整理する。Phase 1 / Phase 2 / Phase 3 / Phase 4a の互換性制約（C ABI、リアルタイム制約、Svelte 5 runes、依存ゼロ、Mod Wheel = 0 で Phase 3 互換）はすべて維持する。

## 他文書との関係

- 上流: [`pre-research.md`](./pre-research.md)（Phase 4b 追加調査、§2 物理モデル概観 / §3 B 係数 / §4 Stretching all-pass / §5 KS 組込み / §6 Hammer model / §7 Piano Modal Body / §8 新パラメータ判断 / §9 既存負債 / §10 性能予算）、[Phase 1 全 8 章](../2026-05-06-001-mvp/)、[Phase 2 全 8 章](../2026-05-07-002-phase2/)、[Phase 3 全 8 章](../2026-05-07-003-phase3/)、[Phase 4a 全 8 章](../2026-05-08-004-phase4a/)（既存資産）
- 下流: [`02-architecture.md`](./02-architecture.md)（全体構成の差分）→ `03〜05`（各レイヤ詳細）→ [`06-build-and-verify.md`](./06-build-and-verify.md) → [`07-implementation-checklist.md`](./07-implementation-checklist.md)
- 並行: `docs/retrospective/2026-05-08-004-phase4a.md`（Phase 4a 振り返り、§5 既存負債 / §7 推奨スコープ Phase 4b 候補 13 件 を本フェーズで一部解消）
- 本書は「Phase 4b で何を作るか」を確定し、以降の文書は「どう作るか」を定義する。
- **Phase 4c（C8 自己発振 / WASM SIMD / Sustain × sympathetic / Pick fractional 化 等）は別計画扱い**: 本書には Phase 4c の決定事項を含めない。Phase 4b 完了後の retrospective を経て、別仕様書ディレクトリ `docs/specs/<YYYY-MM-DD>-006-phase4c/` で策定する。

## Phase 4b の完成像

> **ブラウザで動作する Rust/WASM 製の物理モデリング弦シンセ。Phase 4a の 7 楽器プリセット + LFO + Mod Wheel + Preset 基盤を土台に、ピアノ音色を物理ベースで実装する。Stretching all-pass cascade (M=8 段、Rauhala-Välimäki 2006 の closed-form 係数式) で stiff string の inharmonicity (`f_n = n·f_0·√(1+B·n²)`) を再現、Commuted impulse + velocity-dependent LPF で felt hammer の打鍵を表現、Piano 専用の Modal Body 係数 (soundboard 第 1 モード 55Hz、M=8) で響板共鳴を加える。`InstrumentKind::Piano = 7` を 8 番目の楽器として追加し、Factory Preset に Piano プリセット 1 件を追加。Phase 4a 互換性は Default 含む既存 7 楽器で `dispersion_active = false` として完全保持（出力バイト一致をテストで保証）。補助的に `.gitattributes` で CRLF/LF 戦争を断ち、`__synthDev.measureProcessTime` で F38b 計測を自動化、`wasm-opt --print-stats` で WASM サイズのベースラインを記録する。**

「演奏表現を完成させた多楽器プリセット式の物理ベースシンセ」（Phase 4a ゴール）から「物理ベースのピアノ音色を獲得した多楽器シンセ」へ進める。Phase 4a retrospective §7 推奨スコープの「ピアノ音色」を主目的、補助として「`.gitattributes` LF 統一」「`__synthDev` 自動計測」「WASM サイズ調査」を採用、その他 10 候補は Phase 4c 以降に送る（C8 自己発振、Pick fractional 化、Look-ahead limiter、LFO 拡張、Cross-tab 同期、Preset import/export、Mono+Sustain 本実装、WASM SIMD 等）。新規楽器（管楽器 / 打楽器）は引き続き Phase 5 以降。

## ゴール

- **Stretching all-pass cascade**: `dsp-core/src/dispersion.rs` を新規実装、M=8 段の 1 次 allpass cascade、Rauhala-Välimäki 2006 の closed-form 係数式（k1=-0.00179, k2=-0.0233, k3=-2.93, m1=0.0126, m2=0.0606, m3=-0.00825, m4=1.97 のマジック定数を含む）で a1 を算出。`dispersion_active = false` の楽器（Phase 4a 既存 7 楽器）では process_sample で skip、CPU 影響ゼロ
- **KarplusStrong 拡張**: `dispersion_stages: [DispersionStage; 8]` + `dispersion_active: bool` フィールド追加（heap 確保ゼロ）、`note_on` で a1 算出、`process_sample` で `read_z` の値を 8 段に通してから既存 Thiran allpass に渡す
- **Hammer model**: `note_on_internal` の buffer 初期化を分岐、`dispersion_active = true` で **Commuted impulse + velocity-dependent LPF**（buffer[0] = velocity、cutoff = lerp(800Hz, 4000Hz, velocity) の 1pole IIR で平滑化）。pluck 経路（既存）は Phase 4a 互換のため温存
- **Piano 用 Modal Body 係数**: `BODY_MODES_PIANO_L/R` 8 値を `params.json` + `gen-params.mjs` で生成、Conklin 1996 の grand piano soundboard 第 1 モード = 49〜60 Hz に基づき第 1 mode = 55 Hz、stereo_spread = 0.05
- **`InstrumentKind::Piano = 7`**: 既存 0-6 (Default + 6 楽器) を保持、Piano を 8 番目として追加。`from_u32(7)` で `Some(InstrumentKind::Piano)`、`synth_apply_instrument(handle, 7)` で切替
- **Piano プリセット**: `factory-presets.ts` に Piano エントリ 1 件追加、合計 8 種に拡張
- **楽器切替**: Phase 4a D53（即時 `pool.all_notes_off()`）を継承。当初提案した 5 ms fade-out は SmoothedValue の同期 set_target だけでは実現不能と判明したため Phase 4c 送りに変更（指摘事項 #3 反映、D63 改訂）。Phase 4b で `apply_instrument` に追加するのは `pool.set_dispersion_active(piano)` の 1 行のみ（D67）
- **新規 ParamId / C ABI 関数追加なし**: Inharmonicity B / HammerCutoff は Piano プリセット内のフィールドで完結、UI 露出は Phase 4c 送り。required exports 19（Phase 4a 末尾）を維持
- **Phase 4a 互換性**: Default 含む既存 7 楽器で `dispersion_active = false`、`test_dispersion_disabled_matches_phase4a` で **process 出力がバイト一致 (ε=1e-6)** を機械保証
- **`.gitattributes` LF 統一**: リポジトリ root の `.gitattributes` で `* text=auto eol=lf` + 主要拡張子を明示、`git add --renormalize .` で既存 file を LF へ統一、prettier format での CRLF/LF 差分再発を防止
- **F38b 計測自動化**: `web/src/lib/audio/__synthDev.ts` に `measureProcessTime(durationMs)` を追加（dev ビルド限定）、AudioWorklet 側で **`performance.now()` の差分**で `process` の self time を実測し、リングバッファ (`Float32Array(4096)`、約 10.92 秒分) に蓄積、`port.postMessage` で main へ集約。Console から `await window.__synthDev.measureProcessTime(10000)` で avg/max を取得（指摘事項 #1 反映: `currentFrame` は callback 内で進まないため self time 計測には使えず `performance.now()` に変更）
- **WASM サイズベースライン記録**: `wasm-opt --print-stats` を Phase 4b 着手最初に実行、Phase 4a 18.42 KB の内訳を retrospective §5 へ追記
- Phase 4a の制約をすべて維持: AudioWorklet `process` 中ヒープ確保ゼロ（WASM 側 + JS 側）、C ABI 既存 18 関数 + memory export = 19 required exports 完全互換、Svelte 5 runes、`dsp-core` / `wasm-audio` 依存ゼロ
- WASM gzip サイズの **3 段階基準（Phase 4a から継承、目標は実態に合わせて再調整）**: **目標 < 22 KB**（Phase 4a 18.42 KB + Phase 4b 純増 ~0.7 KB = 19 KB 想定、余裕 +3 KB）、**警戒 < 25 KB**、**撤退 < 30 KB**（R32 楽器係数削減 / Modal M=5）。Worklet 本番バンドル < 12 KB（Phase 4a 8.17 KB + Piano 経路 +1 KB = 9 KB 想定）
- ポリフォニー 8 音 + Body + LFO + Piano dispersion 動作時の `process` 1 回 < 1.7 ms（128 frames @ 48 kHz、Phase 4a 比 +0.04 ms 余裕、**F50 で必須化** = release cargo timing test）

## 非ゴール（Phase 4b には含めない）

| 項目 | 理由 / 送り先 |
|---|---|
| C8 ピッチ自己発振モード（damping=1.0 / FFT estimator） | ピアノとは別軸の物理限界、Phase 4c |
| Pick position の fractional 化 | ピアノは hammer 固定位置で pick 概念なし、Phase 4c |
| Look-ahead limiter（5 ms 遅延型） | Soft clip で十分、Phase 4c |
| WASM SIMD（`target-feature=+simd128`） | CPU 余裕大（3.3%）で必要性低、Phase 4c |
| LFO 波形 S&H / Square / Sawtooth | 楽器表現として非標準、Phase 4c |
| LFO destinations 拡張 (Pick / Damping / BodyWet) | 効果薄、Phase 4c |
| Voice State `SharedArrayBuffer + Atomics` 化 | COOP/COEP 必須、GitHub Pages 不可 |
| Cross-tab preset 同期（storage event） | UX 需要薄、Phase 4c |
| Preset JSON ファイル import / export | localStorage 内のみで十分、Phase 4c |
| Mono + Sustain の本実装 | Phase 2 D29 / Phase 3 D40 / Phase 4a D55 で no-op 確定、Phase 5 |
| Sustain × Sympathetic resonance | ピアノ damper の物理は別 Engine 構造が要、Phase 4c |
| 複数 Piano 機種プリセット (Grand / Upright / Honkytonk) | Phase 4b は Piano 1 種で実機検証、Phase 4c |
| Hammer Hardness UI 露出 | Phase 4b では Piano プリセット 1 種で固定、Phase 4c |
| Soundboard / lid の高次モード (M=16) | Modal Body M=8 で十分、Phase 4c |
| 管楽器 / 打楽器 / 録音・MIDI export | Phase 5 領域 |
| アクセシビリティ機能（ARIA、スクリーンリーダー） | Phase 4c 以降 |
| iOS Safari 以外のモバイル動作保証 | デスクトップ Chromium 主、iOS Safari は検証のみ |
| `scripts/copy-wasm.mjs:34` の DEP0190 警告対処 | Phase 4a 既存負債、`cross-spawn` 検討は Phase 4c |

## 確定事項（pre-research §11 で承認済み、2026-05-09）

| 決定事項 | 内容 |
|---|---|
| Phase 4b の主目的 | **ピアノ音色（Stretching all-pass + Hammer model + Modal Body Piano）** |
| Stretching all-pass の段数 M | **8（Faust 標準、Rauhala-Välimäki 推奨）** |
| Inharmonicity B の表現 | **Faust 方式（per-instrument 固定値 + `Ikey(f0)` で a1 自動補正）**、per-voice の B 個別保持なし |
| Hammer モデルの方式 | **Commuted impulse + velocity-dependent LPF**（CPU ホットパス影響ゼロ） |
| Piano kind の Modal 係数 | **§7.2 文献値ベース（Conklin 1996 / soundboard mode 1 = 55Hz）**、聴感調整は実装後 |
| 新規 ParamId 追加 | **なし**（Inharmonicity / HammerHardness は楽器プリセット内のフィールド） |
| 楽器切替時の挙動 | **Phase 4a D53 を継承（即時 release）**、当初の「5 ms fade-out」は SmoothedValue の同期 set_target だけでは実現不能と判明したため Phase 4c 送りに変更（指摘事項 #3 反映） |
| C8 自己発振の Phase 4b 取扱 | **Phase 4c 送り** |
| 規模感 | **18 Step（Phase 4a の 17 Step + 1 Step）** |
| F1〜F25 / F34 / F38b 実機検証の扱い | 持ち越し継続、Phase 4b 着手前提条件としない（Phase 4a の cargo timing 0.023 ms で代替確認済） |
| ディレクトリ命名 | `docs/specs/2026-05-09-005-phase4b/`、Phase 1 / 2 / 3 / 4a と同じ pre-research + 01〜07 の 8 章構成 + IMPLEMENTATION_PROMPT.md |
| レビュー運用 | 一括レビュー方式（pre-research → 01〜07 → IMPLEMENTATION_PROMPT を全部書き切ってから一度に提示） |

## Phase 1 D1〜D11 / Phase 2 D12〜D29 / Phase 3 D30〜D43 / Phase 4a D44〜D55 の Phase 4b での扱い

Phase 1 / 2 / 3 / 4a の主要設計判断 55 項目（D1〜D55、D38b 含む）を Phase 4b で「維持 / 変更 / 拡張」のいずれかに分類。詳細は各章で展開する。

### 全 D 項目の Phase 4b での扱い

| # | 範囲 | Phase 4b での扱い | 主な記述章 |
|---|---|---|---|
| **D1〜D11**（Phase 1 基本制約） | process ヒープ確保ゼロ / C ABI / Svelte 5 runes / secure context / denormal flush 等 | **全件維持** | 02 / 03 / 04 / 05 章 |
| **D12〜D29**（Phase 2 polyphony / fractional delay / hold stack） | VoicePool N=8 / voice stealing / Lagrange→Thiran / hold stack / params codegen | **全件維持** | 02 / 03 章 |
| **D30〜D32**（Phase 3 Modal Body） | Modal Synthesis (M=8) / 配置 / 係数管理 | **拡張**: D32 の楽器係数を 7 種 → 8 種化（Piano kind を追加、Phase 4a 既存値はすべて温存） | 03 / 04 章 |
| **D33〜D34**（Loss filter / Pick position） | One-zero loss / 励振 shaping | **維持** （Pick position は Piano 以外の既存 7 楽器でのみ作用、Piano は hammer 経路で comb 適用しない） | 03 章 |
| **D35**（Stretching all-pass 不採用） | Phase 3 で不採用 | **撤回**（Phase 4b で主目的として採用、§4 で D59 として詳細） | 03 章 |
| **D36**（Thiran allpass 案 D 採用） | 全 Thiran + C8 ignore | **維持**（dispersion cascade と直列同居、§5 で D60 として順序確定） | 03 章 |
| **D37**（Brightness 群遅延補正） | ディレイ長補償 | **拡張**: dispersion cascade も群遅延を持つため、`note_on` の `adjusted_length` 計算で `M·polydel(a1)` を追加で減算（§5 で D60） | 03 章 |
| **D38**（MIDI CC dispatch） | `synth_midi_cc` 集約、CC#1 (Mod Wheel) | **維持** | 03 / 04 章 |
| **D38b**（Channel Volume 直交配置） | `channel_volume` SmoothedValue | **維持** | 03 章 |
| **D39**（Voice trait `set_pitch_bend`） | ±2 半音 fan-out | **維持** | 03 章 |
| **D40**（Sustain Pedal、Poly のみ defer / Mono no-op） | | **維持**（Mono+Sustain は Phase 4b でも no-op 継続、§1 確定事項より） | 03 章 |
| **D41**（Voice State 33 byte 共有メモリ） | | **維持** | 04 / 05 章 |
| **D42**（mono / poly トグル UI 正式化） | | **維持** | 05 章 |
| **D43**（区間関数型 soft clip） | | **維持** | 03 章 |
| **D44**（F38b 実機計測） | Phase 4a §0 で計測準備のみ、実測未取得 | **拡張**: D66 で `__synthDev.measureProcessTime` 自動計測スクリプト整備 | 05 / 06 章 |
| **D45**（`wasm-opt -O3` 適用） | Phase 4a §1 / `copy-wasm.mjs` 統合 | **維持**（Phase 4b 着手最初に `wasm-opt --print-stats` で内訳調査） | 06 章 |
| **D46**（LFO グローバル 1 個） | Engine 内 1 個、Sine/Triangle | **維持** | 03 章 |
| **D47**（LFO 波形 Sine + Triangle） | `f32::sin()` 直接呼出 | **維持** | 03 章 |
| **D48**（LFO destinations 3 つ：Pitch / Brightness / Volume） | 各独立 depth、Engine 側 exp2 fan-out | **維持** | 03 章 |
| **D49**（Mod Wheel CC#1 を LFO master） | `mod_wheel: SmoothedValue tau=0.05s` | **維持** | 03 章 |
| **D50**（Preset JSON v1） | `physbase.preset.v1.` prefix | **維持**（Piano プリセット 1 件追加、schema 不変） | 05 章 |
| **D51**（User Preset 上限 32 件） | | **維持** | 05 章 |
| **D52**（楽器プリセット 6 種） | Default + Guitar Classical / Ukulele / Mandolin / Bass / Guitar Steel / Sitar | **拡張**: 7 種 → 8 種、Piano を `InstrumentKind::Piano = 7` で追加（D62 で詳細） | 03 章 |
| **D53**（楽器切替時の即時 release） | `pool.all_notes_off()` + Modal 差し替え + reset | **維持**（D63 で fade-out 拡張を当初提案したが、SmoothedValue 同期 set_target の実現不能性により撤回。Phase 4a D53 を完全継承） | 03 章 |
| **D54**（stereo_spread 楽器別保持） | 楽器プリセット内のフィールド | **維持**（Piano `stereo_spread = 0.05`） | 03 章 |
| **D55**（Mono+Sustain 現状維持） | no-op 継続 | **維持**（Phase 5 以降で需要があれば再評価） | 03 章 |

## 主要な設計判断（Phase 4b 新規 D56〜D67）

仕様策定の過程で確定した、Phase 4b 実装時に逸脱しない 12 項目。詳細な根拠と適用箇所は各レイヤ仕様書に記載する。

| # | 判断 | 内容 / 採用案 | 主たる記述章 |
|---|---|---|---|
| **D56** | ピアノ音色を Phase 4b 主目的として採用 | **Phase 4a retrospective §7 候補 #1** を主目的に確定。Stretching all-pass + Hammer model + Modal Body Piano + InstrumentKind::Piano = 7 + Factory Preset Piano 1 件の一括実装。物理理論が確立 (Rauhala-Välimäki / Bank-Sujbert / Conklin) のため文献ベースで実装 | 03 / 05 章 |
| **D57** | Stretching all-pass cascade の段数 M = 8 | **Faust `piano_dispersion_filter` のデフォルト**、Rauhala-Välimäki 2006 の "8 が medium-sized piano に十分" の記述に従う。M=4 では高音域分散不足、M=16 は CPU リスク。状態 `[DispersionStage; 8]` を `KarplusStrong` に inline 配列で保持（heap 確保ゼロ） | 03 章 |
| **D58** | Inharmonicity B の表現 = Faust 方式 | **per-instrument 固定値 (Piano kind の `inharmonicity_b = 7.5e-4`、A4 基準) + `Ikey(f0)` で a1 自動補正**。per-voice の B 個別保持はしない（実装簡素を優先、bass/treble の極端な B 差は Phase 4c 以降で再評価）。a1 計算式の `Ikey(f0)` 項が note ごとの自動補正を担う | 03 章 |
| **D59** | Stretching all-pass の closed-form 係数式 | **Rauhala-Välimäki 2006 の Faust 由来式を Rust 移植**。マジック定数 `k1=-0.00179`, `k2=-0.0233`, `k3=-2.93`, `m1=0.0126`, `m2=0.0606`, `m3=-0.00825`, `m4=1.97` を `dispersion.rs` に const 定義（`#[allow(clippy::approx_constant)]` 付き）、`compute_dispersion_a1(M, B, f0, fs) -> (a1, group_delay_per_stage)` で a1 + 群遅延を返す | 03 章 |
| **D60** | dispersion → Thiran → Brightness LPF → LossFilter → damping の順序 | **既存 Phase 1〜4a 順序を維持**し、`process_sample` の `read_z` 値を **dispersion cascade 8 段に通してから** `thiran.process(...)` に渡す。`note_on` の `adjusted_length` 計算で **`M·polydel(a1)` を追加で減算**（dispersion cascade の群遅延補正、stretched harmonics の f_0 を保つ） | 03 章 |
| **D61** | Hammer モデル = Commuted impulse + velocity-dependent LPF | **`note_on_internal` の buffer 初期化を分岐**: `dispersion_active = false`（Phase 4a 既存 7 楽器）で既存 pluck 経路、`true` (Piano) で hammer 経路。hammer 経路は `buffer[0] = velocity` の単位 impulse に **velocity 依存 1pole IIR LPF（cutoff_low=800Hz, cutoff_high=4000Hz の線形補間）を適用**して buffer[0..len_int] を初期化。**process_sample への影響ゼロ**（note_on 時のみ計算）。Hertz law spring (Boutillon) や WDF は実装複雑度のため Phase 4c 送り | 03 章 |
| **D62** | Piano 用 Modal Body 係数 | **Conklin 1996 の grand piano soundboard 第 1 モード = 49〜60 Hz** をベースに 8 モード設計。第 1 mode = 55 Hz, Q = 10、`stereo_spread = 0.05`。文献値で初期実装、聴感調整は実装後（§7.2）。`InstrumentKind::Piano = 7` を `params.json` に追加、`gen-params.mjs` で `BODY_MODES_PIANO_L/R` + `STEREO_SPREAD_PIANO` + `INHARMONICITY_B_PIANO` + `HAMMER_CUTOFF_LOW_PIANO` + `HAMMER_CUTOFF_HIGH_PIANO` を出力 | 03 章 |
| **D63 (改訂)** | 楽器切替は Phase 4a D53 を継承（即時 release） | **当初「5 ms fade-out」を提案したが実現性に問題ありとして撤回**: `SmoothedValue::set_target` は target 代入のみで current は `next_sample()` でしか進まず、同期メソッド内で `set_target(0.0)` → `set_target(prev_value)` しても fade-out は発生しない。`PendingInstrumentChange` 状態機械を導入する案もあるが Phase 4b 主目的（ピアノ音色）に対する実装複雑度が大きいため、**Phase 4a D53「即時 `pool.all_notes_off()`」を完全継承**。Phase 4b 新規追加は `pool.set_dispersion_active(piano)` の 1 行のみ（D67）。fade-out / cross-fade は Phase 4c 以降の UX 改善で再評価 | 03 章 |
| **D64** | 新規 ParamId / C ABI 関数追加なし | **Inharmonicity B / HammerCutoff は Piano プリセット内のフィールド**として保持、UI 露出は Phase 4c。`ParamId` enum (5 値) は不変、required exports 19（Phase 4a 末尾）を維持。`synth_apply_instrument` の `kind` 値域を 0-6 から 0-7 に拡張するのみ（C ABI シグネチャ不変） | 03 / 04 章 |
| **D65** | `.gitattributes` で改行 LF 統一 | **Phase 4b 着手最初の commit**（Step 1）。リポジトリ root に `.gitattributes` を作成（`* text=auto eol=lf` + 主要拡張子）、`git add --renormalize .` で既存 file を LF へ統一。Phase 4a で頻発した CRLF/LF 戦争を断つ。独立した `chore: normalize line endings to LF` commit で分離 | 06 章 |
| **D66 (改訂)** | F38b 計測自動化スクリプト | **`web/src/lib/audio/__synthDev.ts` に `measureProcessTime(durationMs)` を追加**（dev ビルド限定、`import.meta.env.DEV` ガード + worklet build script に `--define:DEV_MODE=true` を渡す）。AudioWorklet 側で `process` 開始/終了を **`performance.now()`** で記録（指摘事項 #1 反映: `currentFrame` は callback 内で進まないため self time 計測には使えない）、リングバッファ `Float32Array(4096)` (約 10.92 秒分) に蓄積、stop メッセージで時系列順に main へ集約 postMessage。`DEV_MODE` は **`declare const DEV_MODE: boolean;`** 宣言 + esbuild の `--define:DEV_MODE=...` で置換（指摘事項 #2 反映: ローカル `const DEV_MODE = ...` は define 対象外）。Console から `await window.__synthDev.measureProcessTime(10000)` で avg/max を取得 | 05 章 |
| **D67** | Phase 4a 互換性のバイト一致保証 | **Default 含む既存 7 楽器 (`InstrumentKind::Default` 〜 `Sitar`) で `dispersion_active = false`**、`note_on` の buffer 初期化も pluck 経路、`process_sample` で dispersion cascade を skip。**`test_dispersion_disabled_matches_phase4a`**: 同条件 (Damping / Brightness / OutputGain / PickPosition / BodyWet 全パラメータ + LFO depth=0 + Mod Wheel=0) で Phase 4a と Phase 4b の `process` 出力が **ε=1e-6 でバイト一致**を機械保証。これが Phase 4b 互換性の中核 | 03 章 |

## C ABI 既存 18 関数 + memory export = 19 required exports（Phase 4b で完全維持）

Phase 4b では新規 C ABI 関数追加なし。以下の Phase 4a 確定 18 C ABI 関数 + `memory` export を **シグネチャ・export 名・動作すべて完全に維持** する。`scripts/check-wasm-exports.mjs` の `REQUIRED` 配列は Phase 4a と同じ 19 entry。

| 関数名 | シグネチャ | Phase 4b での扱い |
|---|---|---|
| `synth_new` | `(f32, u32) -> *mut SynthHandle` | 維持。内部で dispersion stages も一括確保（heap 確保ゼロ維持） |
| `synth_free` | `(*mut SynthHandle)` | 維持 |
| `synth_note_on` | `(*mut SynthHandle, u8, f32)` | 維持。内部で `dispersion_active` に応じて pluck or hammer 経路で buffer 初期化 |
| `synth_note_off` | `(*mut SynthHandle, u8)` | 維持 |
| `synth_set_param` | `(*mut SynthHandle, u32, f32)` | 維持 |
| `synth_set_polyphony_mode` | `(*mut SynthHandle, u32)` | 維持 |
| `synth_reset` | `(*mut SynthHandle)` | 維持。LFO / Mod Wheel / 楽器選択 (Default に reset) / dispersion stages も reset |
| `synth_out_l_ptr` | `(*const SynthHandle) -> *const f32` | 維持 |
| `synth_out_r_ptr` | `(*const SynthHandle) -> *const f32` | 維持 |
| `synth_capacity` | `(*const SynthHandle) -> u32` | 維持 |
| `synth_process_block` | `(*mut SynthHandle, u32)` | 維持。内部で dispersion cascade も走るが外部仕様は不変 |
| `synth_set_polyphony_mode` | （重複、削除） | — |
| `synth_midi_cc` | `(*mut SynthHandle, u8, f32)` | 維持 |
| `synth_pitch_bend` | `(*mut SynthHandle, f32)` | 維持 |
| `synth_voice_state_ptr` | `(*const SynthHandle) -> *const u8` | 維持 |
| `synth_apply_instrument` | `(*mut SynthHandle, kind: u32)` | **拡張**: kind の値域を 0-6 から 0-7 へ。`InstrumentKind::from_u32(7) = Some(Piano)` で対応、不正値 (8 以上) は黙って無視（Phase 4a 既存防御的設計を継承） |
| `synth_lfo_set_rate` | `(*mut SynthHandle, hz: f32)` | 維持 |
| `synth_lfo_set_waveform` | `(*mut SynthHandle, kind: u32)` | 維持 |
| `synth_lfo_set_depth` | `(*mut SynthHandle, dest: u32, depth: f32)` | 維持 |
| (memory export) | WebAssembly.Memory | 維持。byteLength 不変 |

### Phase 4b で追加する C ABI 関数

**なし**。Inharmonicity / Hammer Hardness は Piano プリセット内に閉じ込めるため C ABI 拡張は不要。

## Phase 4c への申し送り（Phase 4b 完成後の検討）

Phase 4b では実装しないが、Phase 4c 以降で検討すべき設計:

- **C8 ピッチ自己発振モード**: damping=1.0 経路 or FFT-based estimator（Phase 3 D36 の物理限界、Phase 4c で再評価）
- **Pick position の fractional 化**: Piano 以外の楽器で励振 shaping を fractional K に拡張、または出力経路の comb filter
- **Look-ahead limiter**: 5 ms 遅延型、240 sample × f32 = 960 B バッファ（Soft clip より透明、Phase 4b 主目的との関連薄）
- **WASM SIMD**: `target-feature=+simd128`（CPU 余裕大、優先度低）。dispersion cascade の 8 段を `f32x4` × 2 段で並列化候補
- **Brightness allpass 直列補正**: Phase 3 ディレイ長補償が知覚的に不十分なら追加
- **LFO 波形拡張**: S&H / Square / Sawtooth、`Random` 系
- **LFO destinations 拡張**: Pick position / Damping / BodyWet
- **Voice State `SharedArrayBuffer + Atomics`**: COOP/COEP 必要、GitHub Pages 不可
- **楽器切替の fade-out / cross-fade**: Phase 4b D63 で当初提案した 5 ms output_gain fade-out は SmoothedValue 同期 set_target の実現不能性により撤回（指摘事項 #3）。Phase 4c で `PendingInstrumentChange` 状態機械（fade-out → 切替 → fade-in を per-sample loop 内で進行）として再実装、または voice 単位 release ramp の本実装
- **Cross-tab preset 同期**: `window.addEventListener('storage', ...)`
- **Preset JSON ファイル import / export**: ファイルダウンロード / アップロード
- **Mono + Sustain の本実装**: 現状の no-op を撤回するなら仕様確定が必要（Phase 5）
- **複数 Piano 機種プリセット (Grand / Upright / Honkytonk)**: Phase 4b の Piano 1 種で実機検証後の拡張
- **Hammer Hardness UI 露出**: Phase 4b ではプリセット内固定、UI 露出は Phase 4c
- **Sustain × Sympathetic resonance**: 実機ピアノの damper 物理（Sustain 押下時に全弦振動）、別 Engine 構造が要
- **Piano 高次モード (M=16)**: Modal Body の段数拡張、Phase 4b の M=8 で十分か実機評価後
- **`scripts/copy-wasm.mjs` の DEP0190 警告対処**: `cross-spawn` への置換 or 別 Node API
- **Hertz law hammer (Boutillon)**: Phase 4b の Commuted 方式から物理的な spring モデルへ
- **PWA 化 / オフライン対応**: Service Worker + Web App Manifest
- **Web MIDI 拡張**: ProgramChange でプリセット切替、複数チャンネル対応
- **録音 / WAV エクスポート**: AudioWorkletNode の出力を MediaRecorder で記録

## アーキテクチャ概要（詳細は 02-architecture.md）

Phase 1 / 2 / 3 / 4a の 4 レイヤ構成は維持。Stretching all-pass cascade と Hammer model が dsp-core 内 KarplusStrong の責務として追加され、Piano 楽器プリセット切替は Phase 4a の `apply_instrument` 経路で集約される。新規 UI 追加なし、`PresetSelector` のドロップダウンに Piano が 8 番目として追加されるのみ。

```
┌────────────────────────────────────────────────────┐
│ Svelte UI（メインスレッド）                          │
│  StartButton / Keyboard / Slider / MIDI            │
│  ParamSlider が ParamDescriptor 駆動                 │
│  VoiceMeter / PolyphonyToggle / WebMIDI handler     │
│  ModWheel / LfoSection / PresetSelector              │
│  + Piano エントリが PresetSelector に追加             │  ← Phase 4b 差分
│  + __synthDev.measureProcessTime() (dev only)        │  ← Phase 4b 差分
└──────────────┬─────────────────────────────────────┘
               │ MessagePort（Phase 4a 既存、変更なし）
               ▼
┌────────────────────────────────────────────────────┐
│ AudioWorkletProcessor（音声スレッド）               │
│  WASM ロード、process 委譲、Voice State stride push │
│  WasmExports 18 関数（Phase 4a と同じ）              │
│  + process 開始/終了の performance.now() 記録 (dev)   │  ← Phase 4b 差分
│  + リングバッファ Float32Array(4096) 蓄積 (dev)        │  ← Phase 4b 差分
│  + port.postMessage({type:'timing'}) 集約 (dev only) │  ← Phase 4b 差分
└──────────────┬─────────────────────────────────────┘
               │ FFI（既存 18 C ABI 関数 + memory export = 19 required exports、変更なし）
               ▼
┌────────────────────────────────────────────────────┐
│ wasm-audio（Rust crate, cdylib）                   │
│  SynthHandle が dsp-core を呼ぶ                      │
│  既存 18 関数のまま、関数追加なし                     │  ← Phase 4b 差分（kind 値域拡張のみ）
└──────────────┬─────────────────────────────────────┘
               │
               ▼
┌────────────────────────────────────────────────────┐
│ dsp-core（Rust crate, rlib, 純粋）                   │
│  Engine / VoicePool<8> / KarplusStrong              │
│  ModalBodyResonator / LossFilter / SoftClip /       │
│  SustainState / VoiceStateBuffer / Lfo              │
│  + dispersion.rs (M=8 cascade + closed-form)         │  ← Phase 4b 差分
│  + KarplusStrong に dispersion_stages /              │  ← Phase 4b 差分
│    dispersion_active 追加                             │
│  + InstrumentKind::Piano = 7                         │  ← Phase 4b 差分
│  + BODY_MODES_PIANO_L/R 8 値                         │  ← Phase 4b 差分
│  + Piano の inharmonicity_b / hammer_cutoff_*        │  ← Phase 4b 差分
│  + Engine::apply_instrument 末尾に                    │  ← Phase 4b 差分
│    set_dispersion_active 呼出 (Phase 4a D53 即時継承) │
│  + KarplusStrong::note_on で hammer 経路 (Piano 時)   │  ← Phase 4b 差分
└────────────────────────────────────────────────────┘
```

ビルドパイプラインは Phase 4a の 3 スクリプト (`gen-params.mjs` / `check-params-sync.mjs` / `copy-wasm.mjs (wasm-opt -O3 込み)`) を継続使用。`params.json` に Piano 楽器エントリを追加（D62）:

```
params.json (Phase 4a 7 楽器 → Phase 4b 8 楽器、Piano エントリは inharmonicity_b /
             hammer_cutoff_low_hz / hammer_cutoff_high_hz の専用フィールドを保持)
       │
       │ scripts/gen-params.mjs (Piano 専用フィールドを Rust const + TS 定数で出力)
       ▼
crates/dsp-core/src/params.rs (生成、Phase 4b で BODY_MODES_PIANO_L/R + STEREO_SPREAD_PIANO +
                                INHARMONICITY_B_PIANO + HAMMER_CUTOFF_LOW_PIANO +
                                HAMMER_CUTOFF_HIGH_PIANO + InstrumentKind::Piano=7 を追加)
web/src/lib/audio/generated/params.ts (Phase 4b で InstrumentKind 'piano' を追加)
       │
       │ scripts/check-params-sync.mjs (Phase 4a 既存、Piano 楽器 1 件追加で diff 検知)
       ▼
PR diff で drift 検知

build pipeline (Phase 4a 既存):
  cargo build --target wasm32-unknown-unknown --release
       │
       │ scripts/copy-wasm.mjs (wasm-opt -O3 適用、Phase 4a 既存)
       ▼
  web/static/wasm-audio.wasm (~19 KB target、Phase 4a 18.42 KB + 純増 0.6 KB)
       │
       │ scripts/check-wasm-exports.mjs (REQUIRED 配列 18 関数 + memory = 19 entry、Phase 4a と同じ)
       ▼
  PR で export 名 drift 検知
```

## 用語集（Phase 4b 追加分）

Phase 1〜4a [01 章 用語集] の用語に加えて、Phase 4b で新規導入する用語を定義する。

| 用語 | 説明 |
|---|---|
| **Inharmonicity（非調和性）** | ピアノなど stiff string で第 n 倍音が `f_n = n·f_0·√(1+B·n²)` のように整数倍からずれる現象。係数 B で定量化される |
| **Inharmonicity coefficient B** | `B = (π³·E·a⁴)/(16·L²·T)`（無次元）、ピアノで 10⁻⁴〜10⁻¹ レンジ。Phase 4b では Piano プリセット固定値 7.5×10⁻⁴ (A4 基準)（D58） |
| **Stretching all-pass / Dispersion all-pass** | 周波数依存の位相遅延を持つ allpass フィルタ。KS ループ内に挿入することで stretched harmonics を生成（D57 / D59） |
| **Dispersion cascade** | 複数段の 1 次 dispersion allpass の直列接続。Phase 4b では M=8 段固定（D57） |
| **Rauhala-Välimäki 2006 closed-form** | Inharmonicity coefficient B + 基音 f0 + 段数 M から allpass 係数 a1 を closed form で算出する式。Faust `piano_dispersion_filter` で実装、Phase 4b で Rust 移植（D59） |
| **Group delay per stage / `polydel(a1)`** | 1 段の dispersion allpass の基音における群遅延。`adjusted_length` 補正に使用（D60） |
| **Hammer model / Commuted impulse + velocity LPF** | ピアノの felt hammer 打鍵を impulse + velocity 依存 LPF で近似する手法（Bank-Sujbert / Smith CCRMA）。Phase 4b 採用方式（D61） |
| **`dispersion_active` flag** | `KarplusStrong` のフィールド。`true` で Piano kind、`false` で Phase 4a 既存 7 楽器（D67） |
| **`InstrumentKind::Piano = 7`** | Phase 4b で追加される 8 番目の楽器 enum 値。Phase 4a 既存 0-6 を保持（D62） |
| **Hammer cutoff low/high** | Velocity-dependent LPF のカットオフ周波数下限/上限（800Hz / 4000Hz、Piano プリセット内）。velocity 線形補間で実効 cutoff を算出（D61） |
| **Soundboard mode 1** | ピアノ響板の第 1 共鳴モード（49〜60 Hz、Conklin 1996）。Phase 4b の Piano Modal Body の最低周波数 mode（D62） |
| **F38b 計測自動化スクリプト** | `__synthDev.measureProcessTime(durationMs)` で AudioWorklet `process` の self time を自動集計する dev-only API（D66） |
| **PendingInstrumentChange (Phase 4c 候補)** | 楽器切替時に fade-out → Modal 差し替え → fade-in を per-sample loop 内で進行する状態機械。Phase 4b の D63 で当初 5 ms fade-out を提案したが SmoothedValue 同期 set_target で実現不能と判明し撤回、Phase 4c で本実装する候補（指摘事項 #3 反映） |
