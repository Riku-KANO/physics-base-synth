# 01. Phase 4a 概要とスコープ

## 目的

Phase 3 で確立した「ブラウザで動作する 8 音ポリフォニック Karplus–Strong + Modal Body Resonator + Extended KS + MIDI CC + Voice Meter UI + Soft clip + Thiran allpass」を土台に、**F38b 実機計測で Phase 3 完成判定を閉じ**、**LFO + Mod Wheel (CC#1) で表現力を獲得**、**localStorage プリセット保存・ロード**、**多楽器プリセット 6 種（Guitar Classical / Ukulele / Mandolin / Bass / Guitar Steel / Sitar）**、**`wasm-opt -O3` 適用と `excitation_snapshot` cfg(test) 化で既存負債を解消** する。Phase 1 / Phase 2 / Phase 3 の互換性制約（C ABI、リアルタイム制約、Svelte 5 runes、依存ゼロ）はすべて維持する。

## 他文書との関係

- 上流: [`pre-research.md`](./pre-research.md)（Phase 4a 追加調査、§2 F38b / §3 LFO / §4 Mod Wheel / §5 プリセット / §6 localStorage / §7 多楽器 / §8 既存負債）、[Phase 1 全 8 章](../2026-05-06-001-mvp/)、[Phase 2 全 8 章](../2026-05-07-002-phase2/)、[Phase 3 全 8 章](../2026-05-07-003-phase3/)（既存資産）
- 下流: [`02-architecture.md`](./02-architecture.md)（全体構成の差分）→ `03〜05`（各レイヤ詳細）→ [`06-build-and-verify.md`](./06-build-and-verify.md) → [`07-implementation-checklist.md`](./07-implementation-checklist.md)
- 並行: `docs/retrospective/2026-05-07-003-phase3.md`（Phase 3 振り返り、§5 既存負債 / §7.2 Phase 4 候補 / §7.3 設計改善 を本フェーズで一部解消）
- 本書は「Phase 4a で何を作るか」を確定し、以降の文書は「どう作るか」を定義する。
- **Phase 4b（ピアノ音色 / Stretching all-pass）は別計画扱い**: 本書には Phase 4b の決定事項を含めない。Phase 4a 完了後の retrospective を経て、別仕様書ディレクトリ `docs/specs/<YYYY-MM-DD>-005-phase4b/` で策定する。

## Phase 4a の完成像

> **ブラウザで動作する Rust/WASM 製の物理モデリング弦シンセ。Phase 3 の 8 音ポリフォニック Karplus–Strong + Modal Body + Extended KS + MIDI CC + Voice Meter を土台に、Phase 3 検証の最終案件 F38b（Worklet `process` self time の Chrome DevTools 実機計測）を §0 として閉じ、グローバル LFO（Engine 内 1 個、Sine/Triangle、Pitch/Brightness/Volume の 3 destinations、Mod Wheel CC#1 で master 制御）で vibrato / tremolo / wah を獲得、localStorage プリセット保存・ロード（v1 JSON、最大 32 User Preset + Factory 7 種）と多楽器プリセット 6 種（Guitar Classical / Ukulele / Mandolin / Bass / Guitar Steel / Sitar）で音色バリエーションを拡充、`wasm-opt -O3` で WASM gzip を 27.78 KB → ~13 KB に削減、`excitation_snapshot` を `#[cfg(test)]` ガードで production cleanliness を向上させる。**

「音色のリアリティと表現力を備えた弦シンセ」（Phase 3 ゴール）から「演奏表現を完成させた多楽器プリセット式の物理ベースシンセ」へ進める。Phase 3 で送りになった LFO + Mod Wheel + プリセット系を一括実装し、Phase 3 retrospective §5 の主要負債を Phase 4a 内で解消することが目的。ピアノ音色（Stretching all-pass）は Phase 4b で別計画扱い、新規楽器（管楽器 / 打楽器）は Phase 5 以降に送る。

## ゴール

- **F38b 実機計測**: `pnpm preview` + Chrome DevTools Performance タブで Worklet `process` self time avg<1.5ms / max<2.5ms を計測し、結果を `docs/retrospective/2026-05-07-003-phase3.md` §5 に追記。target 超過なら R30 対策（Voice Meter stride 4096 化等）を Phase 4a 内で適用
- **`wasm-opt -O3` 適用**: `scripts/copy-wasm.mjs` で wasm-opt を呼出、`devDependency` に binaryen を追加、WASM gzip 27.78 KB → ~13 KB に削減
- **`excitation_snapshot` cfg(test) 化**: `#[cfg(test)] pub fn excitation_snapshot(...)` で production binary から完全除外
- **LFO（グローバル 1 個）**: `dsp-core/src/lfo.rs` に `Lfo` 型定義、Engine 内 1 段、`f32::sin()` 直接呼出（LUT 不要）、レンジ 0.1-8 Hz、SmoothedValue tau=0.05s で rate 平滑、波形 Sine / Triangle 切替
- **Mod Wheel (CC#1)**: `Engine::handle_midi_cc` の `CC_MOD_WHEEL` 分岐を実装、`mod_wheel: SmoothedValue (tau=0.05s)` を保持、LFO destination depth の master 乗数として作用
- **LFO destinations 3 つ**: Pitch (±0.5 半音 max) / Brightness (±0.5 max) / Volume (±0.5 multiplier) の 3 つを独立 depth スライダーで制御、process 内で voice / engine の各 SmoothedValue target に offset として加算
- **プリセット保存・ロード**: `web/src/lib/state/preset-store.svelte.ts` に `PresetStore` を実装、JSON `version: 1` スキーマ、localStorage キー prefix `physbase.preset.v1.`、Factory Preset 7 種（Default + 楽器 6 種）+ User Preset 最大 32 件
- **PresetSelector UI**: `web/src/lib/components/PresetSelector.svelte` を新規、ドロップダウン上段に Factory Preset、下段に User Preset、保存・削除ボタン
- **多楽器プリセット 6 種**: `BODY_MODES_GUITAR_CLASSICAL` / `BODY_MODES_UKULELE` / `BODY_MODES_MANDOLIN` / `BODY_MODES_BASS` / `BODY_MODES_GUITAR_STEEL` / `BODY_MODES_SITAR` を dsp-core に const テーブル化（Phase 3 D32 と同形式）、それぞれ stereo_spread 個別値
- **`Engine::apply_instrument(kind)`**: 楽器切替で `pool.all_notes_off()` → 新楽器の Modal 係数差し替え → `modal_body.prepare(sample_rate)` → `modal_body.reset()` を実行
- **C ABI 4 関数追加**: `synth_apply_instrument` / `synth_lfo_set_rate` / `synth_lfo_set_waveform` / `synth_lfo_set_depth`、Phase 3 既存 14 C ABI 関数 + memory export = 15 required exports（D18 + D38 / D38b / D39 / D41 で 11 + 3）に追加。Phase 4a 後は **18 C ABI 関数 + memory export = 19 required exports**
- **ModWheel UI**: `<input type="range">` でモジュレーション wheel の UI スライダー、WebMIDI 物理 wheel と同経路（CC#1 dispatch）
- **LFO controls UI**: rate / waveform / 3 destinations の depth × 3 を含む LfoSection コンポーネント
- Phase 3 の制約をすべて維持: AudioWorklet `process` 中ヒープ確保ゼロ（WASM 側 + JS 側）、C ABI 既存 14 関数 + memory export = 15 required exports 完全互換、Svelte 5 runes、`dsp-core` / `wasm-audio` 依存ゼロ
- WASM gzip サイズの **3 段階基準**: **目標 < 15 KB**（wasm-opt -O3 適用後の想定 ~13 KB）、**警戒 < 18 KB**（要調査ライン、F39 で計測）、**撤退 < 30 KB**（Phase 3 から継承の最終 target、超過で R32 楽器係数削減）。Worklet 本番バンドル < 10 KB
- ポリフォニー 8 音 + Body + LFO 動作時の `process` 1 回 < 1.7 ms（128 frames @ 48 kHz、Phase 3 比 +0.2 ms 余裕、**F46 で必須化** = release cargo timing test）

## 非ゴール（Phase 4a には含めない）

| 項目 | 理由 / 送り先 |
|---|---|
| ピアノ音色（Stretching all-pass + impact model）| **Phase 4b で別計画**、本書には含めない |
| C8 ピッチ自己発振モード（damping=1.0 / FFT estimator）| Phase 3 D36 で物理限界として確定済、Phase 4b 以降で再評価 |
| Pick position の fractional 化 | 効果薄、Phase 4b 以降 |
| Look-ahead limiter（5 ms 遅延型）| Soft clip で十分、Phase 4b 以降 |
| WASM SIMD（`target-feature=+simd128`）| Safari/Firefox 対応再評価、Phase 4b 以降 |
| Brightness allpass 直列補正 | Phase 3 ディレイ長補償で十分なら不要、Phase 4b 以降 |
| LFO 波形 S&H / Square / Sawtooth | 楽器表現として非標準、Phase 4b 以降 |
| LFO destinations 拡張 (Pick / Damping / BodyWet) | 効果薄、Phase 4b 以降 |
| Voice State `SharedArrayBuffer + Atomics` 化 | COOP/COEP 必須、GitHub Pages 不可 |
| Modal Body M=8 → M=5 削減 | 多楽器プリセットで表現力担保、削減不要 |
| Cross-tab preset 同期（storage event）| UX 需要薄、Phase 4b 以降 |
| Preset JSON ファイル import / export | localStorage 内のみで十分、Phase 4b 以降 |
| Mono + Sustain の本実装 | Phase 3 D40 P1-2 で no-op 確定、Phase 4a でも継続 |
| 管楽器 / 打楽器 / 録音・MIDI export | Phase 5 領域 |
| アクセシビリティ機能（ARIA、スクリーンリーダー）| Phase 4b 以降 |
| iOS Safari 以外のモバイル動作保証 | デスクトップ Chromium 主、iOS Safari は検証のみ |

## 確定事項（ユーザー承認済み、2026-05-08）

| 決定事項 | 内容 |
|---|---|
| Phase 4 の分割 | **Phase 4a / Phase 4b に明示分割**。Phase 4a = F38b + LFO/Mod Wheel + プリセット + 多楽器、Phase 4b = ピアノ |
| F38b の扱い | **Phase 4a §0（Step 1）として最初に組み込み**。実装着手前に計測 |
| LFO destinations | **3 つ確定**: Pitch / Brightness / Volume |
| LFO 波形 | **2 つ確定**: Sine / Triangle |
| プリセット ストレージ | **localStorage で確定**（同期 API、5 MB 上限で十分） |
| 楽器選定 | **6 種で確定**: Guitar Classical / Ukulele / Mandolin / Bass / Guitar Steel / Sitar |
| User Preset 上限 | **32 件** |
| 楽器切替挙動 | **即時 release（fade-out なし）**。fade-out は Phase 4b 以降 |
| 規模感 | **大きめ（17 Step、Phase 3 の 14 Step + 3 Step）** |
| F1〜F25 / F34 実機検証の扱い | 持ち越し継続。Phase 4a 着手前提条件としない |
| ディレクトリ命名 | `docs/specs/2026-05-08-004-phase4a/`、Phase 1 / 2 / 3 と同じ pre-research + 01〜07 の 8 章構成 + IMPLEMENTATION_PROMPT.md |
| レビュー運用 | 一括レビュー方式（pre-research → 01〜07 → IMPLEMENTATION_PROMPT を全部書き切ってから一度に提示） |

## Phase 1 D1〜D11 / Phase 2 D12〜D29 / Phase 3 D30〜D43 の Phase 4a での扱い

Phase 1 / 2 / 3 の主要設計判断 43 項目（D1-D43、D38b 含む）を Phase 4a で「維持 / 変更 / 拡張」のいずれかに分類。詳細は各章で展開する。

### 全 D 項目の Phase 4a での扱い

| # | 範囲 | Phase 4a での扱い | 主な記述章 |
|---|---|---|---|
| **D1〜D11**（Phase 1 基本制約）| process ヒープ確保ゼロ / C ABI / Svelte 5 runes / secure context / denormal flush 等 | **全件維持** | 02 / 03 / 04 / 05 章 |
| **D12〜D29**（Phase 2 polyphony / fractional delay / hold stack）| VoicePool N=8 / voice stealing / Lagrange→Thiran (D14 は D36 で更新済) / hold stack / params codegen | **全件維持** | 02 / 03 章 |
| **D30〜D32**（Phase 3 Modal Body）| Modal Synthesis (M=8) / 配置 / 係数管理 | **拡張**: D32 の `BODY_MODES_*` を楽器ごとに 6 種化（Phase 3 既存値は kind=0 Default として温存）。`stereo_spread` は楽器ごとに個別値 | 03 / 04 章 |
| **D33〜D34**（Loss filter / Pick position）| One-zero loss / 励振 shaping | **維持** | 03 章 |
| **D35**（Stretching all-pass 不採用）| Phase 3 で不採用 | **Phase 4b で再評価** | 03 章（記述のみ） |
| **D36**（Thiran allpass 案 D 採用）| 全 Thiran + C8 ignore | **維持** | 03 章 |
| **D37**（Brightness 群遅延補正）| ディレイ長補償 | **維持**（LFO Brightness destination との相互作用は §LFO で検討） | 03 章 |
| **D38**（MIDI CC dispatch）| `synth_midi_cc` 集約 | **拡張**: CC#1 (Mod Wheel) 分岐を有効化、Phase 3 で no-op だった経路を実装 | 03 / 04 章 |
| **D38b**（Channel Volume 直交配置）| `channel_volume` SmoothedValue | **維持** | 03 章 |
| **D39**（Voice trait `set_pitch_bend`）| ±2 半音 fan-out | **拡張**: LFO Pitch destination が `set_pitch_bend` の SmoothedValue に offset として加算される。`set_mod_depth` は Phase 4a で実装する形に変更（D49） | 03 章 |
| **D40**（Sustain Pedal、Poly のみ defer / Mono no-op）| | **維持**（Mono+Sustain は Phase 4a でも no-op 継続、§1 確定事項より） | 03 章 |
| **D41**（Voice State 33 byte 共有メモリ）| | **維持** | 04 / 05 章 |
| **D42**（mono / poly トグル UI 正式化）| | **維持** | 05 章 |
| **D43**（区間関数型 soft clip）| | **維持**（LFO Volume destination が soft clip 前の output_gain × channel_volume × volume_multiplier に作用） | 03 章 |

## 主要な設計判断（Phase 4a 新規 D44〜D55）

仕様策定の過程で確定した、Phase 4a 実装時に逸脱しない 12 項目。詳細な根拠と適用箇所は各レイヤ仕様書に記載する。

| # | 判断 | 内容 / 採用案 | 主たる記述章 |
|---|---|---|---|
| **D44** | F38b 実機計測の組み込み | **Phase 4a §0（Step 1）として最初に実施**。`pnpm preview` + Chrome DevTools Performance タブで Worklet `process` self time avg/max を 10 秒記録、判定基準 avg<1.5ms / max<2.5ms。結果を `docs/retrospective/2026-05-07-003-phase3.md` §5 に追記して負債を閉じる。target 超過時は R30 対策（stride 4096 化 / Voice Meter 削除）を Phase 4a 内で適用 | 06 章 |
| **D45** | `wasm-opt -O3` 適用 | **`scripts/copy-wasm.mjs` に wasm-opt 実行を組み込み**、`package.json` の `devDependencies` に `binaryen` を追加（npm の build-time tooling、依存ゼロ制約に抵触しない）。WASM gzip 27.78 KB → ~13 KB を target、超過時は調査要 | 02 / 06 章 |
| **D46** | LFO の配置 | **グローバル 1 個（Engine 内）**。Voice 単位は不採用（演奏者の慣習は MIDI シンセの LFO 1 個、CPU +24 演算 vs +3 演算で 8x 効率）。`Lfo` 型定義（pre-research §3.2 結論ボックス） | 03 章 |
| **D47** | LFO 波形 | **Sine + Triangle の 2 種類**。`f32::sin()` 直接呼出（LUT 不要、CPU +5 演算/sample × 1 LFO で軽微）、Triangle は `4·\|phase − 0.5\| − 1` の linear ramp。S&H / Square / Sawtooth は Phase 4b 以降 | 03 章 |
| **D48** | LFO destinations | **Pitch / Brightness / Volume の 3 つ確定**。各独立 depth スライダー、深さ 0-1 を ±0.5 (半音 / brightness 値 / volume multiplier) に scale。Pick / Damping / BodyWet destinations は Phase 4b 以降 | 03 章 |
| **D49** | Mod Wheel (CC#1) を LFO master として実装 | **`Engine::handle_midi_cc` の CC#1 分岐を有効化**、`mod_wheel: SmoothedValue (tau=0.05s)` を保持、`effective_depth = lfo_depth × mod_wheel`。Mod Wheel = 0 で LFO 効果ゼロ（Phase 3 互換挙動）、Mod Wheel = 1 で LFO depth スライダー値そのまま反映。Voice trait の `set_mod_depth` 追加は不要（depth は Engine 状態として持つ、Phase 3 D39 の前置きを更新） | 03 / 04 章 |
| **D50** | プリセット形式 | **JSON `version: 1`**、スキーマは pre-research §5.3 の `PresetV1` で確定（`name` / `createdAt` (ISO 8601) / `instrument` / `params` (5 件) / `lfo` (5 件)）。**localStorage key prefix `physbase.preset.v1.`**、不明 version は console.warn してデフォルトを返す（throw しない）、migration 関数 `migrateV1ToV2` は将来 v2 追加時に書く | 05 章 |
| **D51** | User Preset 上限 | **32 件**。32 件目を超える保存試行は `throw new Error('Preset slot full')` で UI に通知、QuotaExceededError は `try/catch` で防御 | 05 章 |
| **D52** | 楽器プリセット 6 種 | **Guitar Classical / Ukulele / Mandolin / Bass / Guitar Steel / Sitar**。各 8 モード × 3 係数 = 24 値 × 6 = 144 値の Modal 係数定数を dsp-core に追加（pre-research §7.2）、`enum InstrumentKind { Default, GuitarClassical, Ukulele, Mandolin, Bass, GuitarSteel, Sitar }` を定義（kind=0 が Phase 3 既存値を温存する Default） | 03 章 |
| **D53** | 楽器切替時の挙動 | **即時 release**: `pool.all_notes_off()` → Modal 係数差し替え → `modal_body.prepare(sample_rate)` → `modal_body.reset()`。fade-out で段階リリースは Phase 4b 以降の UX 改善で再評価（演奏中の音切れは UI 側で「楽器切替時は音が切れます」注意書きで補完） | 03 章 |
| **D54** | stereo_spread の楽器別化 | **楽器プリセットの一部として保持**。Phase 3 のグローバル const `STEREO_SPREAD = 0.05` を撤回し、`InstrumentPreset` 構造体内の `stereo_spread: f32` フィールドに変更。各楽器の値は pre-research §7.3 の表 | 03 章 |
| **D55** | Mono+Sustain 現状維持 | **Phase 3 D40 P1-2 と同じく no-op 継続**。Mono mode の Sustain は Phase 2 既存挙動（Phase 2 D29）を完全継承。Mono の last-note priority と release defer は本質的に相反するため Phase 4a でも実装しない。Phase 5 以降で需要があれば再評価 | 03 章 |

## C ABI 既存関数の互換性チェックリスト（14 C ABI 関数 + memory export = 15 required exports）

Phase 4a では以下の Phase 3 確定 C ABI 関数 14 件 + `memory` export を **シグネチャ・export 名・動作すべて完全に維持** する（D18 / D38 / D39 / D41 継承）。`scripts/check-wasm-exports.mjs` の `REQUIRED` 配列は memory を含む 15 entry。

| 関数名 | シグネチャ | Phase 4a での扱い |
|---|---|---|
| `synth_new` | `(f32, u32) -> *mut SynthHandle` | 維持。内部で LFO 状態 / 楽器プリセット係数も一括確保するように **動作のみ拡張**、外部仕様は不変 |
| `synth_free` | `(*mut SynthHandle)` | 維持 |
| `synth_note_on` | `(*mut SynthHandle, u8, f32)` | 維持 |
| `synth_note_off` | `(*mut SynthHandle, u8)` | 維持 |
| `synth_set_param` | `(*mut SynthHandle, u32, f32)` | 維持 |
| `synth_set_polyphony_mode` | `(*mut SynthHandle, u32)` | 維持 |
| `synth_reset` | `(*mut SynthHandle)` | 維持。LFO / Mod Wheel / 楽器選択も reset（**楽器選択は kind=0 Default に戻す、Phase 3 既存値**） |
| `synth_out_l_ptr` | `(*const SynthHandle) -> *const f32` | 維持 |
| `synth_out_r_ptr` | `(*const SynthHandle) -> *const f32` | 維持 |
| `synth_capacity` | `(*const SynthHandle) -> u32` | 維持 |
| `synth_process_block` | `(*mut SynthHandle, u32)` | 維持。内部で LFO process も走るが外部仕様は不変 |
| `synth_midi_cc` | `(*mut SynthHandle, u8, f32)` | 維持。**CC#1 (Mod Wheel) 分岐を有効化**（D49）、外部 ABI は不変、内部動作のみ拡張 |
| `synth_pitch_bend` | `(*mut SynthHandle, f32)` | 維持 |
| `synth_voice_state_ptr` | `(*const SynthHandle) -> *const u8` | 維持 |
| (memory export) | WebAssembly.Memory | 維持。byteLength 不変 |

### Phase 4a で追加する C ABI 関数

| 関数名 | シグネチャ | 役割 |
|---|---|---|
| `synth_apply_instrument` | `(*mut SynthHandle, kind: u32)` | 楽器選択（D52、kind: 0=Default, 1=GuitarClassical, 2=Ukulele, 3=Mandolin, 4=Bass, 5=GuitarSteel, 6=Sitar）。内部で `pool.all_notes_off()` + `modal_body.prepare/reset` を実行 |
| `synth_lfo_set_rate` | `(*mut SynthHandle, hz: f32)` | LFO レート (D46、0.1-8.0 Hz、SmoothedValue tau=0.05s) |
| `synth_lfo_set_waveform` | `(*mut SynthHandle, kind: u32)` | LFO 波形 (D47、kind: 0=Sine, 1=Triangle) |
| `synth_lfo_set_depth` | `(*mut SynthHandle, dest: u32, depth: f32)` | LFO destination depth (D48、dest: 0=Pitch, 1=Brightness, 2=Volume、depth ∈ [0, 1]) |

`scripts/check-wasm-exports.mjs` の `REQUIRED` 配列に上記 4 関数を追加する（本仕様 04 / 06 章）。

## Phase 4b への申し送り（Phase 4a 完成後の検討）

Phase 4a では実装しないが、Phase 4b 以降で検討すべき設計:

- **ピアノ音色（Stretching all-pass + impact model）**: inharmonicity 係数 B = 10⁻³ 級、hammer-string interaction。Phase 4b で別計画策定
- **C8 ピッチ自己発振モード**: damping=1.0 経路 or FFT-based estimator
- **Pick position の fractional 化**: 励振 shaping を fractional K に拡張、または出力経路の comb filter
- **Look-ahead limiter**: 5 ms 遅延型、240 sample × f32 = 960 B バッファ
- **WASM SIMD**: `target-feature=+simd128` 安定化と Safari/Firefox 対応再評価
- **Brightness allpass 直列補正**: Phase 3 ディレイ長補償が知覚的に不十分なら追加
- **LFO 波形拡張**: S&H / Square / Sawtooth、`Random` 系
- **LFO destinations 拡張**: Pick position / Damping / BodyWet
- **Voice State `SharedArrayBuffer + Atomics`**: COOP/COEP 必要、GitHub Pages 不可
- **楽器切替の fade-out**: 即時 release ではなく短時間 fade で滑らかな切替
- **Cross-tab preset 同期**: `window.addEventListener('storage', ...)`
- **Preset JSON ファイル import / export**: ファイルダウンロード / アップロード
- **Mono + Sustain の本実装**: 現状の no-op を撤回するなら仕様確定が必要
- **PWA 化 / オフライン対応**: Service Worker + Web App Manifest
- **Web MIDI 拡張**: ProgramChange でプリセット切替、複数チャンネル対応
- **録音 / WAV エクスポート**: AudioWorkletNode の出力を MediaRecorder で記録

## アーキテクチャ概要（詳細は 02-architecture.md）

Phase 1 / 2 / 3 の 4 レイヤ構成は維持。LFO が dsp-core 内 Engine の責務として追加され、楽器プリセット切替が wasm-audio で集約される。プリセット保存・ロードは UI 層 (`preset-store.svelte.ts`) でカプセル化、Worklet 経由で個別パラメータとして dsp-core に伝播。

```
┌────────────────────────────────────────────────────┐
│ Svelte UI（メインスレッド）                          │
│  StartButton / Keyboard / Slider / MIDI            │
│  ParamSlider が ParamDescriptor 駆動                 │
│  VoiceMeter / PolyphonyToggle / WebMIDI handler     │
│  + ModWheel.svelte（CC#1 スライダー）                │  ← Phase 4a 差分
│  + LfoSection.svelte（rate/waveform/3 destinations）│  ← Phase 4a 差分
│  + PresetSelector.svelte（Factory + User）           │  ← Phase 4a 差分
│  + InstrumentPicker.svelte（6 種ドロップダウン）      │  ← Phase 4a 差分
│  + preset-store.svelte.ts（localStorage 操作）       │  ← Phase 4a 差分
└──────────────┬─────────────────────────────────────┘
               │ MessagePort（既存 + lfo* / applyInstrument）
               ▼
┌────────────────────────────────────────────────────┐
│ AudioWorkletProcessor（音声スレッド）               │
│  WASM ロード、process 委譲、Voice State stride push │
│  WasmExports に midi_cc / pitch_bend / voice_state  │
│  + lfo_set_rate / set_waveform / set_depth          │  ← Phase 4a 差分
│  + apply_instrument                                  │  ← Phase 4a 差分
└──────────────┬─────────────────────────────────────┘
               │ FFI（共有メモリ + ポインタ、既存 14 関数 + Phase 4a 4 関数 = 18 C ABI 関数 + memory export）
               ▼
┌────────────────────────────────────────────────────┐
│ wasm-audio（Rust crate, cdylib）                   │
│  SynthHandle が dsp-core を呼ぶ                      │
│  既存: synth_midi_cc / pitch_bend / voice_state_ptr │
│  + synth_apply_instrument                            │  ← Phase 4a 差分
│  + synth_lfo_set_rate / waveform / depth             │  ← Phase 4a 差分
└──────────────┬─────────────────────────────────────┘
               │
               ▼
┌────────────────────────────────────────────────────┐
│ dsp-core（Rust crate, rlib, 純粋）                   │
│  Engine / VoicePool<8> / KarplusStrong              │
│  ModalBodyResonator / LossFilter / SoftClip /       │
│  SustainState / VoiceStateBuffer                    │
│  + Lfo (phase + rate + waveform + 3 depths)          │  ← Phase 4a 差分
│  + InstrumentKind enum + BODY_MODES_<INSTRUMENT> 6 種│  ← Phase 4a 差分
│  + Engine::apply_instrument(kind)                    │  ← Phase 4a 差分
│  + mod_wheel: SmoothedValue                          │  ← Phase 4a 差分
└────────────────────────────────────────────────────┘
```

ビルドパイプラインは Phase 1〜3 の 2 スクリプト（`gen-params.mjs` / `check-params-sync.mjs`）を継続使用 + `wasm-opt -O3` を `copy-wasm.mjs` に組み込み。`params.json` に LFO 関連パラメータと楽器選択 enum を追加（D45 / D46-49 / D52）:

```
params.json (単一ソース、Phase 4a で +N パラメータ + instrument 6 種定義 + LFO 5 値)
       │
       │ scripts/gen-params.mjs
       ▼
crates/dsp-core/src/params.rs (生成、git commit、楽器ごとの BODY_MODES 6 種を出力)
web/src/lib/audio/generated/params.ts (生成、git commit、InstrumentKind enum を出力)
       │
       │ scripts/check-params-sync.mjs (CI で検証)
       ▼
PR diff で drift 検知

build pipeline:
  cargo build --target wasm32-unknown-unknown --release
       │
       │ scripts/copy-wasm.mjs (+ wasm-opt -O3)        ← Phase 4a 差分
       ▼
  web/static/wasm-audio.wasm (~13 KB target)
       │
       │ scripts/check-wasm-exports.mjs (REQUIRED 配列 + 4 関数)
       ▼
  PR で export 名 drift 検知
```

## 用語集（Phase 4a 追加分）

Phase 1 [01 章 用語集](../2026-05-06-001-mvp/01-overview.md#用語集) / Phase 2 [01 章 用語集](../2026-05-07-002-phase2/01-overview.md#用語集phase-2-追加分) / Phase 3 [01 章 用語集](../2026-05-07-003-phase3/01-overview.md#用語集phase-3-追加分) の用語に加えて、Phase 4a で新規導入する用語を定義する。

| 用語 | 説明 |
|---|---|
| **LFO (Low Frequency Oscillator)** | 0.1〜数十 Hz の低周波振動子で、変調信号として他のパラメータに乗算 / 加算される。Phase 4a ではグローバル 1 個（Engine 内）、波形 Sine / Triangle、レンジ 0.1-8 Hz、destinations Pitch / Brightness / Volume の 3 つ（D46-D48） |
| **Mod Wheel（Modulation Wheel, CC#1）** | LFO 強度を制御する MIDI CC。Phase 4a で `Engine::handle_midi_cc` の CC#1 分岐を有効化、`mod_wheel: SmoothedValue` を保持し全 LFO destination depth の master 乗数として作用（D49） |
| **Vibrato** | LFO Pitch destination による周期的なピッチ変動（典型 4-7 Hz、深さ ±0.05〜0.5 半音） |
| **Tremolo** | LFO Volume destination による周期的な音量変動（典型 4-8 Hz、深さ ±0.2〜0.6） |
| **Wah / Filter Sweep** | LFO Brightness destination による周期的な明るさ変動（典型 0.5-3 Hz） |
| **LFO Destination** | LFO 値が変調する対象パラメータ。Phase 4a では Pitch / Brightness / Volume の 3 つ（D48） |
| **LFO Depth** | LFO destination ごとの変調深さ ∈ [0, 1]。Mod Wheel value × LFO depth が effective depth になる（D49） |
| **Preset** | シンセの全ユーザー操作可能パラメータ + 楽器選択を JSON で永続化したもの。Factory Preset (7 種、読み取り専用) + User Preset (最大 32 件、編集可) |
| **Factory Preset** | 出荷時定義のプリセット 7 種（Default + 楽器 6 種）。`web/src/lib/state/factory-presets.ts` の const テーブルで定義、編集不可 |
| **User Preset** | ユーザーが localStorage に保存するプリセット。`web/src/lib/state/preset-store.svelte.ts` で管理、最大 32 件（D51） |
| **Instrument Kind** | 楽器選択の enum。Phase 4a で `InstrumentKind { Default, GuitarClassical, Ukulele, Mandolin, Bass, GuitarSteel, Sitar }` の 7 値（kind=0 Default は Phase 3 既存ギターボディ係数を温存）（D52） |
| **`synth_apply_instrument`** | 楽器切替の C ABI 関数。`pool.all_notes_off()` + Modal 係数差し替え + `modal_body.prepare/reset` を実行（D53） |
| **`wasm-opt`** | Binaryen の WASM 最適化ツール。`-O3 --strip-debug` で WASM gzip を 27.78 KB → ~13 KB に削減（D45） |
