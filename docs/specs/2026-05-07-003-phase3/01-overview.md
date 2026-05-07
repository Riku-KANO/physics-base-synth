# 01. Phase 3 概要とスコープ

## 目的

Phase 2 で確立した「ブラウザで動作する 8 音ポリフォニック Karplus–Strong シンセ」を土台に、**Modal Body Resonator によるボディ共鳴**、**Extended Karplus–Strong による弦の物理的精緻化（loss filter / pick position）**、**MIDI CC マッピング (Pitch Bend / Channel Volume / Sustain Pedal / All Notes Off) による表現力拡張**（Mod Wheel CC#1 は LFO 仕様確定が次フェーズのため Phase 4 送り）、**Voice Meter / mono–poly トグルの UI 正式化**、**区間関数型 Soft clip による振幅安全性**、**Thiran allpass 試作による C8 ピッチ精度の救済可能性検証** を行う。Phase 1 / Phase 2 の互換性制約（C ABI、リアルタイム制約、Svelte 5 runes、依存ゼロ）はすべて維持する。

## 他文書との関係

- 上流: [`pre-research.md`](./pre-research.md)（Phase 3 追加調査、§2 Modal Body / §3 Extended KS / §4 Thiran / §6 MIDI CC / §7 UI / §8 Soft clip）、[Phase 1 全 8 章](../2026-05-06-001-mvp/)、[Phase 2 全 8 章](../2026-05-07-002-phase2/)（既存資産）
- 下流: [`02-architecture.md`](./02-architecture.md)（全体構成の差分）→ `03〜05`（各レイヤ詳細）→ [`06-build-and-verify.md`](./06-build-and-verify.md) → [`07-implementation-checklist.md`](./07-implementation-checklist.md)
- 並行: `docs/retrospective/2026-05-07-002-phase2.md`（Phase 2 振り返り、§5 既存コードの負債と §7 Phase 3 候補を本フェーズで解消）
- 本書は「Phase 3 で何を作るか」を確定し、以降の文書は「どう作るか」を定義する。

## Phase 3 の完成像

> **ブラウザで動作する Rust/WASM 製の物理モデリング弦シンセ。Phase 2 の 8 音ポリフォニック Karplus–Strong を土台に、Modal Body Resonator (M=8 並列 bandpass biquad、stereo) で楽器ボディ共鳴を加え、Extended KS (one-zero loss filter は process 内、pick position は note_on 励振 shaping) で弦の物理的振る舞いを精緻化、MIDI CC (Pitch Bend / Sustain Pedal / Channel Volume / All Notes Off) で表現力を拡張（Mod Wheel は LFO 仕様確定が次フェーズのため Phase 4 送り）、Voice Meter UI と mono / poly トグルで操作性を向上、区間関数型 soft clip で音割れリスクを抑える。Step 1 で Thiran allpass を試作評価し、案 A（全置換）採用なら C8 ピッチ精度の物理限界も解消する。**

「音楽的に演奏できる土台」（Phase 2 ゴール）から「音色のリアリティと表現力を備えた弦シンセ」へ進める。Phase 2 で「弦音だけでは安っぽい」と判定された音色面の課題を Body Resonator + Extended KS で解消し、Pitch Bend / Sustain で演奏表現を可能にすることが Phase 3 の主目的。新規楽器（管楽器 / 打楽器 / ピアノ）は Phase 4 以降に送る。

## ゴール

- Modal Body Resonator が `engine.rs` の `output_gain` 前に M=8 並列 bandpass biquad（DC ゲイン 0、ピーク at `mode.freq` で `mode.gain` 正規化）で挿入され、デフォルトでギターボディ共鳴を反映する。ステレオは左右係数 ±5% で広がりを持つ
- One-zero loss filter `(1 + ρ·z⁻¹)/(1 + ρ)` が `KarplusStrong::process_sample` の brightness LPF 直後・damping 乗算前に挿入され、`note_on` 時に周波数依存式 `ρ = ρ_base · clamp(freq/220, 0.5, 2.0)` で算出される
- Pick position が **`note_on` 時の励振 shaping** で実装される（noise burst を `noise[n] − noise[n−K]` で in-place 整形してから delay buffer にロード、K = round(β · length_int)）。`β = pick_position ∈ [0.05, 0.5]` をパラメータ化、process 内コスト 0、追加メモリ 0、専用モジュールなし。fractional β は Phase 4
- **Step 1 の Thiran allpass 試作評価**で案 A（全置換）/ B（高域のみ）/ C（Lagrange 維持）を確定。案 A 採用なら C8 ピッチ精度の物理限界（Phase 2 retrospective §4.1）が解消される
- Brightness LPF 群遅延補正がディレイ長 1 行補償（`adjusted_length = base_length − τ_g(brightness)`）で `note_on` 時に適用され、A4 で 0.89% の下方偏移を解消
- MIDI CC マッピング: CC#7 (Channel Volume) / CC#64 (Sustain Pedal) / CC#123 (All Notes Off) と Pitch Bend (±2 半音) が `synth_midi_cc` / `synth_pitch_bend` 経由で全 active voice に fan-out（CC#1 Mod Wheel は LFO 仕様確定が Phase 3 スコープ外のため Phase 4 送り）
- Voice trait に `set_pitch_bend(semitones)` を追加、SmoothedValue 5 ms tau で滑らかに遷移
- Sustain Pedal は Engine 状態 `sustain_active: bool` で管理、note_off 時に sustain 中なら damping 加速をスキップし、sustain off で蓄積分を一括 release
- Voice Meter UI が Worklet → main push（1024 サンプル毎 ≈ 21 ms 周期、事前確保 Float32Array スクラッチで `process` 内 alloc ゼロ維持）で 8 セルの active 状態と振幅を表示、Header 直下に配置
- mono / poly トグルが UI に正式化（デフォルト poly）、`__synthDev.setMode()` の dev-only 経路は QA 用として残す
- **区間関数型 soft clip**（|x| ≤ 0.95 で完全 linear・誤差ゼロ、|x| → ∞ で ±1.0 に厳密漸近）が `output_gain` 後、`output_l/r` 書き込み前に挿入され、最悪ケース (8 鍵 + body 共鳴ピーク) でも振幅を ±1.0 以内にクランプ
- Phase 2 の制約をすべて維持: AudioWorklet `process` 中ヒープ確保ゼロ（WASM 側 + JS 側）、C ABI 既存 12 関数完全互換、Svelte 5 runes、`dsp-core` / `wasm-audio` 依存ゼロ
- WASM gzip サイズ < 30 KB（Phase 2 実測 10.56 KB → Phase 3 想定 12.5 KB / target の 42%）、Worklet 本番バンドル < 10 KB
- ポリフォニー 8 音 + Body Resonator 動作時の `process` 1 回 < 1.5 ms（128 frames @ 48kHz、Phase 2 比 +1.3%、F37 release cargo timing test で必須化）

## 非ゴール（Phase 3 には含めない）

| 項目 | 理由 / 送り先 |
|---|---|
| IR convolution（時間域 / FFT-based）| Modal Body で代替。FFT は外部 crate 禁止 + 自前実装で +5 KB のため除外。Modal の音響評価が不十分なら Phase 4 で時間域 IR fallback 検討 |
| Stretching all-pass（弦の inharmonicity）| ギター系では効果薄、CPU +320 演算/sample が割に合わない。Phase 4 のピアノ音色追加時に再評価 |
| Pick position の fractional 化 | 整数 K で十分。可変中の滑らかさは Phase 4 |
| Mod Wheel (CC#1) | LFO の rate / 波形 / 配分 / 深さの仕様確定が Phase 3 スコープ外。Phase 4 で LFO + `set_mod_depth` を併せて確定 |
| Look-ahead limiter（5 ms 遅延型） | Soft clip で十分。Phase 4 で透明度が必要なら検討 |
| プリセット保存・ロード（localStorage / IndexedDB）| Modal 係数を保存する preset 構造の前提が Phase 3 完成後にしか定まらない。Phase 4 |
| WASM SIMD（`target-feature=+simd128`）| Phase 2 retrospective §7 同様、Phase 3 では音作り優先。Phase 4 |
| 多楽器プリセット切替（クラシックギター / ウクレレ / マンドリン）| Modal 係数テーブルは Phase 3 ではギターボディ 1 種類固定、複数化はプリセット保存と並行で Phase 4 |
| 管楽器（reed / digital waveguide tube）| Phase 5 領域 |
| 打楽器（FDTD membrane / mass-spring）| Phase 5 領域 |
| ピアノ音色（Stretching all-pass + impact model）| Phase 4 |
| 録音 / WAV 書き出し / MIDI export | スコープ外 |
| `KarplusStrong::note_on` の buffer ゼロクリア最適化 | Phase 2 retrospective §5、計測してから判断、Phase 3 では着手せず |
| アクセシビリティ機能（ARIA、キーボード操作完備、スクリーンリーダー）| Phase 1 同様、Phase 4 以降 |
| iOS Safari 以外のモバイル動作保証 | Phase 1 / 2 と同じくデスクトップ Chromium が主、iOS Safari は検証のみ（実機検証は持ち越し継続） |

## 確定事項（ユーザー承認済み）

| 決定事項 | 内容 |
|---|---|
| 機能スコープ | Body Resonator (Modal) / Extended KS (loss filter + pick position) / MIDI CC / Voice Meter UI / Soft clip / Thiran allpass 試作評価 の 6 件採用。Stretching all-pass / Look-ahead / プリセット / SIMD は Phase 4 送り |
| Body Resonator 方式 | Modal Synthesis (M=8 並列 biquad)、初期係数は §2.3 のギターボディ値、ステレオ ±5% |
| Step 1 の位置付け | **Thiran allpass 試作評価を Phase 3 Step 1** とする（pre-research §4.4）。仕様書 07 章では Step 1 の達成ラインに「ピッチテストの全候補（A1〜C8）の精度測定 + 案 A/B/C/D 確定」を明示 |
| F1〜F25 実機検証の扱い | 持ち越し継続。Phase 1 retrospective §7 / Phase 2 retrospective §7 で記載済み、ある程度動作確認できているため Phase 3 着手前提条件としない |
| ディレクトリ命名 | `docs/specs/2026-05-07-003-phase3/`、Phase 1 / 2 と同じ pre-research + 01〜07 の 8 章構成 |
| レビュー運用 | 一括レビュー方式（pre-research → 01〜07 を全部書き切ってから一度に提示） |

## Phase 1 D1〜D11 / Phase 2 D12〜D29 の Phase 3 での扱い

Phase 1 の主要設計判断 11 項目と Phase 2 の 18 項目を Phase 3 で「維持 / 変更 / 拡張」のいずれかに分類。詳細は各章で展開する。

### Phase 1 D1〜D11

| # | Phase 1 D# | 内容 | Phase 3 での扱い | 主たる記述章 |
|---|---|---|---|---|
| D1 | 整数ディレイで割り切る → Phase 2 で Lagrange に変更済 | — | **再変更可能性あり** → Step 1 で Thiran 試作。案 A 採用なら全 fractional delay が Thiran 化（D36 で詳細） | 03 章 |
| D2 | MessagePort + SmoothedValue | パラメータ送信経路 | **維持** → MIDI CC 経路でも同方式で全ボイス fan-out | 02 / 03 章 |
| D3 | WASM ロードはメインスレッド経由 | — | **維持** → 完全互換 | 04 / 05 章 |
| D4 | `process` 中ヒープ確保ゼロ | — | **維持** → Modal Body 状態・係数も `Engine::prepare` で一括確保 | 03 / 04 章 |
| D5 | iOS Safari 対策で StartButton 必須 | — | **維持** | 05 章 |
| D6 | denormal 対策で DC injection | — | **維持** → 各ボイス + Modal Body biquad の出力にも適用検討 | 03 章 |
| D7 | note_off は damping 加速で自然減衰 | Phase 2 で mono mode 連携 | **拡張** → Sustain Pedal 中はスキップ（D40） | 03 章 |
| D8 | wasm-audio は C ABI、wasm-bindgen 不使用 | — | **維持** → 既存 12 関数のシグネチャ完全互換、追加関数も同方式 | 04 章 |
| D9 | AudioWorklet の Float32Array view を init 時にキャッシュ | — | **維持** → Voice State 共有メモリ用の view を追加（D41） | 05 章 |
| D10 | secure context 必須 | — | **維持** | 05 / 06 章 |
| D11 | Svelte 5 runes ベース | — | **維持** → Voice Meter 表示は `$derived` で構成 | 05 章 |

### Phase 2 D12〜D29

| # | Phase 2 D# | 内容 | Phase 3 での扱い | 主たる記述章 |
|---|---|---|---|---|
| D12 | VoicePool N=8 固定 | — | **維持** | 03 章 |
| D13 | 4 段 voice stealing | — | **維持** | 03 章 |
| D14 | Lagrange 3 次補間 | — | **変更可能性あり** → Step 1 試作で D36 確定後、案 A なら Thiran に置換 | 03 章 |
| D15 | gen-params.mjs codegen | — | **維持** → Body 係数 / Pick position / 新規パラメータも codegen | 02 章 |
| D16 | Hold stack 容量 16 / 最古破棄 | — | **維持** | 03 章 |
| D17 | `synth_set_polyphony_mode` 追加 | — | **維持** + 追加関数（D38） | 04 章 |
| D18 | C ABI 既存 10 関数完全互換 | Phase 2 で 12 関数化 | **維持** → Phase 3 で関数追加時も既存は不変 | 04 章 |
| D19 | Voice trait の `note_id / age / amplitude` | — | **拡張** → `set_pitch_bend` 追加（D39、Mod Wheel 用 `set_mod_depth` は Phase 4 送り） | 03 章 |
| D20 | 1/sqrt(N) ポリ合成ゲイン | — | **維持** → Modal Body は post-mix（VoicePool 出力後）に配置（D31） | 03 章 |
| D21 | mono / poly トグル UI 出さない | — | **変更** → Phase 3 で UI 正式化（D42） | 05 章 |
| D22 | active voice 数 UI 出さない | — | **変更** → Voice Meter で表示（D41） | 05 章 |
| D23 | LinearStack 自前実装、依存ゼロ | — | **維持** → Modal Body / MIDI CC / Voice Meter も依存ゼロ | 03 章 |
| D24 | params.json (JSON) | — | **維持** → Body 係数を 24 値追加 | 02 章 |
| D25 | 生成物 git commit | — | **維持** | 02 章 |
| D26 | Lagrange 係数は note_on 時のみ計算 | — | **拡張** → Pitch Bend で `length_target` を SmoothedValue 化、補間係数は process 中に再計算が必要になる場合あり（D36 試作結果次第） | 03 章 |
| D27 | LAGRANGE_BUFFER_MARGIN = 3 | — | **維持**（Thiran 採用時は不要だが、Lagrange 維持なら継続） | 03 章 |
| D28 | voice stealing クリック対策 | — | **維持** | 03 章 |
| D29 | mono / poly はランタイム mode 分岐 | — | **維持** → UI トグル経由で `synth_set_polyphony_mode` 呼出（D42） | 03 / 05 章 |

## 主要な設計判断（Phase 3 新規 D30〜D43）

仕様策定の過程で確定した、Phase 3 実装時に逸脱しない 14 項目。詳細な根拠と適用箇所は各レイヤ仕様書に記載する。

| # | 判断 | 内容 / 採用案 | 主たる記述章 |
|---|---|---|---|
| **D30** | Body Resonator の方式 | **Modal Synthesis (M=8 並列 biquad)**。理由: Phase 1 §3.3 で予習済 / パラメータ可変 / 依存ゼロ・サイズ +0.7 KB gzip / M=8 で 40 MAC/sample / プリセット展開と相性良（pre-research §2.2） | 03 章 |
| **D31** | Body Resonator の配置 | **Engine の `process()` 内、`pool.process_sample()` 後・`output_gain` 前に単一段**。VoicePool の `1/sqrt(N)` スケール後の単一モノラル信号に作用、ボイス数非依存の CPU | 03 章 |
| **D32** | Body 係数のソース管理 | **`params.json` で `body_mode_<n>_{freq,q,gain}` を const テーブル化**（24 値 = M=8 × 3 係数）。ステレオは左右で ±5% を const で揺らす。`gen-params.mjs` で Rust / TS 両方に出す。初期値はギターボディ（pre-research §2.3 の表） | 02 / 03 章 |
| **D33** | Loss filter の方式 | **One-zero `(1 + ρ·z⁻¹)/(1 + ρ)`** を `KarplusStrong::process_sample` の brightness LPF 直後・damping 前に挿入。`ρ` は `note_on` 時に `ρ = ρ_base · clamp(freq/220, 0.5, 2.0)` で算出（`ρ_base = 0.05` を初期値、Smith *PASP* 標準形）。process 内コスト +3 演算 | 03 章 |
| **D34** | Pick position の方式 | **励振 shaping**: `KarplusStrong::note_on` 内で生成した noise burst を `noise[n] − noise[n − K]` で in-place comb 整形してから delay buffer にロード（K = round(β · length_int)、Smith *PASP* "Plucked String" の標準形）。`β = pick_position ∈ [0.05, 0.5]` を `params.json` でパラメータ化、Engine が `f32` で保持し全 voice に fan-out。**SmoothedValue 化せず、process 中の動的変更は次回 `note_on` で反映**（連打すれば追従）。fractional β は Phase 4。デフォルト β = 0.125（やや bridge 寄り）。process 内コスト 0、追加メモリ 0。**旧版仕様（feedback loop 内 1-tap comb）は loop gain 安定性議論を生み、PICK_DELAY_MAX が A1 β=0.5 (K=437) に届かない問題があり撤回** | 03 章 |
| **D35** | Stretching all-pass | **Phase 3 では不採用、Phase 4 ピアノ音色追加時に再評価**。ギター系では効果薄、CPU +5 演算 × 8 段 × 8 voice = 320 演算/sample が割に合わない | 03 章（記述のみ） |
| **D36** | Thiran allpass の採否 | **Step 1 で全置換（案 A）を試作評価**。`crates/dsp-core/src/fractional_delay.rs` に `ThiranCoeffs` 構造体を追加（既存 `LagrangeCoeffs` と並列）、`pitch_accuracy.rs` に Thiran 版テストを追加。低音域（A1〜C6）の精度劣化が知覚閾以下なら案 A 採用、悪化があれば案 B（高域のみ Thiran）または案 C（Lagrange 維持・C8 honest skip）。最終決定は Step 1 完了時、本書改訂で D36 を更新する | 03 章 |
| **D37** | Brightness 群遅延補正 | **`note_on` 時のディレイ長補償方式**: `τ_g(brightness) = (1 − brightness) / brightness` sample を `base_length` から減じて fractional delay に渡す。process 中の brightness 変化はピッチ偏移として許容（vibrato 効果として捉える）。Phase 4 で allpass 直列補正を再評価 | 03 章 |
| **D38** | MIDI CC C ABI | **`synth_midi_cc(handle, cc, value_normalized)` 1 関数で集約**（**Phase 3 では CC#7 / #64 / #123 の 3 種類のみ dispatch**、CC#1 Mod Wheel は Phase 4 送り）+ **`synth_pitch_bend(handle, semitones: f32)` 独立**（連続値で頻度高、CC とは独立スループット必要）。CC ごとの個別関数は drift リスクで不採用 | 04 章 |
| **D38b** | Channel Volume の状態管理 | **CC#7 を UI OutputGain とは直交した `channel_volume: SmoothedValue` で保持**（デフォルト 1.0、final gain = `output_gain * channel_volume`）。CC#7 で UI スライダー値を「上書き」しないため両者の状態が独立に保たれ、デバッグ時に「音量がどちらで決まっているか」が明瞭になる | 03 章 |
| **D39** | Voice trait 拡張 | **`set_pitch_bend(semitones: f32)` 1 メソッド追加**。Pitch Bend は `length_target = base_length × 2^(-semitones/12)` を SmoothedValue 化（5 ms tau）。**Mod Wheel (`set_mod_depth`) は Phase 4 送り**: LFO の rate / 波形 / 配分 / 深さの仕様確定が Phase 3 スコープ外と判断、Phase 4 で LFO 仕様と併せて確定 | 03 章 |
| **D40** | Sustain Pedal | **Engine 状態 `sustain_active: bool` + `pending_release: u128 (note bitmap)` で管理**。`note_off` 時に **Poly mode のみ** sustain_active なら該当 voice の damping 加速をスキップし pending bit を立てる（**Mono mode は Sustain 無視で Phase 2 既存挙動を継承**、Mono+Sustain は Phase 4 で再評価）。sustain off 時に pending 全件を一括 release。CC#64 ≥ 64 で on、< 64 で off。**`Engine::set_mode` で mode 切替時は pending を即時 release してから sustain_state を reset**（境界仕様、P2-1 対策、mode 切替で pending が宙ぶらりんにならない）。**`Engine::note_on` 冒頭で `clear_pending(midi)` を呼ぶ**（同一ノート再打鍵で古い pending bit をクリア、再打鍵後にまだ離していないのに pedal off で誤 release されるのを防ぐ）。**`handle_midi_cc(123)` (All Notes Off) は `sustain_state.reset()` も呼ぶ** | 03 章 |
| **D41** | Voice State 通信 | **Worklet → main push 方式、1024 サンプル毎（≈ 21 ms 周期、48kHz 換算）**。C ABI に `synth_voice_state_ptr(handle) -> *const u8` 1 関数追加（active mask + 8 振幅 = 33 bytes を共有メモリ経由で公開）。Worklet 側で stride カウンタ管理し view 経由で main へ message 化。`Engine::pool()` の `doc(hidden)` 露出を正式 API 化（`Engine::voice_state_ptr()`） | 04 / 05 章 |
| **D42** | mono / poly トグル UI | **Header 直下に正式トグル配置**（`<input type="radio">` 2 択、デフォルト poly）。`engine.ts.setMode('mono'|'poly')` を発火し既存 `synth_set_polyphony_mode` を呼ぶ。`__synthDev.setMode()` の dev-only 経路は QA 用に残す | 05 章 |
| **D43** | Soft clip | **区間関数型 saturator**: 安全域 (|x| ≤ 0.95) は完全 linear（誤差ゼロ、`assert_eq!` で検証可能）、超過分 `e = |x| − 0.95` を rational mapping `0.05·e/(e+0.05)` で `[0, 0.05)` に圧縮し、|x| → ∞ で出力 ±1.0 に厳密漸近。**`tanh` Padé 近似は |x| → ∞ で発散するため不採用**（旧版仕様撤回）。OutputGain 後・`output_l/r` 書き込み前に挿入、計算量 6-7 演算/sample、`f32::tanh` 不使用。Look-ahead は Phase 4 | 03 章 |

## C ABI 既存 12 関数の互換性チェックリスト

Phase 3 では以下の Phase 2 確定 C ABI 関数を **シグネチャ・export 名・動作すべて完全に維持** する（D18 継承）。

| 関数名 | シグネチャ | Phase 3 での扱い |
|---|---|---|
| `synth_new` | `(f32, u32) -> *mut SynthHandle` | 維持。内部で Modal Body 係数 / Voice State 共有メモリも一括確保するように **動作のみ拡張**、外部仕様は不変 |
| `synth_free` | `(*mut SynthHandle)` | 維持 |
| `synth_note_on` | `(*mut SynthHandle, u8, f32)` | 維持。内部で Pick position / loss filter / brightness 補正を `note_on` 時に計算（外部仕様不変） |
| `synth_note_off` | `(*mut SynthHandle, u8)` | 維持。**Poly mode のみ** Sustain 中は damping 加速をスキップし pending に積む（D40）。**Mono mode では Sustain は無視**し Phase 2 既存 release 経路（hold_stack 復帰判定 + `pool.note_off`）を完全継承。Mono+Sustain は Phase 4 で再評価（外部仕様不変、内部動作のみ拡張） |
| `synth_set_param` | `(*mut SynthHandle, u32, f32)` | 維持。Pick position / Body Wet / その他新規パラメータも fan-out |
| `synth_set_polyphony_mode` | `(*mut SynthHandle, u32)` | 維持。UI トグルから呼ばれる経路が追加されるが C ABI 自体は不変 |
| `synth_reset` | `(*mut SynthHandle)` | 維持。Modal Body 状態・Pitch Bend・Sustain 全 reset |
| `synth_out_l_ptr` | `(*const SynthHandle) -> *const f32` | 維持 |
| `synth_out_r_ptr` | `(*const SynthHandle) -> *const f32` | 維持 |
| `synth_capacity` | `(*const SynthHandle) -> u32` | 維持 |
| `synth_process_block` | `(*mut SynthHandle, u32)` | 維持。内部で Modal Body / Soft clip も走るが外部仕様は不変 |
| (memory export) | WebAssembly.Memory | 維持。byteLength 不変（Voice State 共有メモリも `synth_new` で一括確保） |

### Phase 3 で追加する C ABI 関数

| 関数名 | シグネチャ | 役割 |
|---|---|---|
| `synth_midi_cc` | `(*mut SynthHandle, cc: u8, value_normalized: f32)` | CC#7 / #64 / #123 の汎用 dispatch（D38、Mod Wheel CC#1 は Phase 4 送り、未対応 CC は無視）。`value_normalized ∈ [0, 1]` |
| `synth_pitch_bend` | `(*mut SynthHandle, semitones: f32)` | Pitch Bend ±2 半音（D38）。全 active voice に fan-out |
| `synth_voice_state_ptr` | `(*const SynthHandle) -> *const u8` | Voice State 共有メモリへのポインタ（D41）。33 bytes（active mask 1 byte + 8 振幅 × 4 bytes） |

`scripts/check-wasm-exports.mjs` の `REQUIRED` 配列に上記 3 関数を追加する（本仕様 04 / 06 章）。

## Phase 4 への申し送り（Phase 3 完成後の検討）

Phase 3 では実装しないが、Phase 4 で検討すべき設計:

- **プリセット保存・ロード**: Modal Body 係数 + Pick position + その他全パラメータの JSON 保存。`localStorage` / IndexedDB / OPFS の選択を Phase 4 冒頭で確定
- **多楽器プリセット**: クラシックギター / ウクレレ / マンドリン / ベース の Modal 係数セット 4-6 種類、UI でドロップダウン切替
- **ピアノ音色**: Stretching all-pass + impact model（hammer-string interaction）。inharmonicity 係数 B = 10⁻³ 級
- **Pick position の fractional 化**: 励振 shaping を fractional K に拡張、または出力経路の comb filter（loop 外）と組み合わせ
- **Mod Wheel (CC#1) + LFO**: 波形 (sine / triangle)、レート (1〜8 Hz)、配分 (pitch / brightness / volume への送り)、深さの仕様を確定し、`Voice::set_mod_depth(depth)` を実装
- **Look-ahead limiter**: 5 ms 遅延型、240 sample × f32 = 960 B バッファ。Soft clip より透明
- **WASM SIMD**: `target-feature=+simd128` 安定化と Safari/Firefox 対応状況を再評価
- **Brightness 群遅延の allpass 直列補正**: Phase 3 のディレイ長補償が知覚的に不十分なら追加
- **PWA 化 / オフライン対応**: Service Worker + Web App Manifest
- **Web MIDI 拡張**: ProgramChange でプリセット切替、複数チャンネル対応
- **録音 / WAV エクスポート**: AudioWorkletNode の出力を MediaRecorder で記録

## アーキテクチャ概要（詳細は 02-architecture.md）

Phase 1 / 2 の 4 レイヤ構成は維持。Modal Body Resonator が dsp-core 内 Engine の責務として追加され、MIDI CC dispatch が wasm-audio で集約される。

```
┌────────────────────────────────────────────────────┐
│ Svelte UI（メインスレッド）                          │
│  StartButton / Keyboard / Slider / MIDI            │
│  + ParamSlider が ParamDescriptor 駆動              │
│  + VoiceMeter（8 セル active + 振幅、Header 直下）   │  ← Phase 3 差分
│  + PolyphonyToggle（mono / poly ラジオ）            │  ← Phase 3 差分
│  + WebMIDI CC handler（pitch bend / volume / sustain / all-notes-off）│  ← Phase 3 差分
└──────────────┬─────────────────────────────────────┘
               │ MessagePort（既存 + midiCC / pitchBend / voiceState）
               ▼
┌────────────────────────────────────────────────────┐
│ AudioWorkletProcessor（音声スレッド）               │
│  WASM ロード、process 委譲、Voice State stride push │
│  + WasmExports に midi_cc / pitch_bend / voice_state│  ← Phase 3 差分
└──────────────┬─────────────────────────────────────┘
               │ FFI（共有メモリ + ポインタ、既存 12 関数 + 3 関数）
               ▼
┌────────────────────────────────────────────────────┐
│ wasm-audio（Rust crate, cdylib）                   │
│  SynthHandle が dsp-core を呼ぶ                      │
│  + synth_midi_cc / synth_pitch_bend                 │  ← Phase 3 差分
│  + synth_voice_state_ptr                            │  ← Phase 3 差分
└──────────────┬─────────────────────────────────────┘
               │
               ▼
┌────────────────────────────────────────────────────┐
│ dsp-core（Rust crate, rlib, 純粋）                   │
│  Engine / VoicePool<8> / KarplusStrong              │
│  + ModalBodyResonator (M=8 並列 biquad, stereo)      │  ← Phase 3 差分
│  + LossFilter (one-zero)                            │  ← Phase 3 差分
│  + Pick position 励振 shaping (KarplusStrong::note_on 内)│  ← Phase 3 差分
│  + ThiranCoeffs (Lagrange と並列、Step 1 試作)       │  ← Phase 3 差分
│  + SustainState (sustain_active + pending_release)  │  ← Phase 3 差分
│  + SoftClip (区間関数、|x|≤0.95 linear、|x|→∞ ±1.0)  │  ← Phase 3 差分
│  + VoiceStateBuffer (33 B)                          │  ← Phase 3 差分
│  Smoothing / XorShift32 / ParamDescriptor /         │
│  HoldStack / SynthMode / FractionalDelay            │
└────────────────────────────────────────────────────┘
```

ビルドパイプラインは Phase 2 の 2 スクリプト（`gen-params.mjs` / `check-params-sync.mjs`）を継続使用。`params.json` に Modal Body 係数 24 値 + Pick position β + Body Wet を追加（D32）:

```
params.json (単一ソース、Phase 3 で +N パラメータ)
       │
       │ scripts/gen-params.mjs
       ▼
crates/dsp-core/src/params.rs (生成、git commit)
web/src/lib/audio/generated/params.ts (生成、git commit)
       │
       │ scripts/check-params-sync.mjs (CI で検証)
       ▼
PR diff で drift 検知
```

## 用語集（Phase 3 追加分）

Phase 1 [01 章 用語集](../2026-05-06-001-mvp/01-overview.md#用語集) と Phase 2 [01 章 用語集](../2026-05-07-002-phase2/01-overview.md#用語集phase-2-追加分) の用語に加えて、Phase 3 で新規導入する用語を定義する。

| 用語 | 説明 |
|---|---|
| **Body Resonator** | 楽器ボディ（ギター響板 / 共鳴胴）の共鳴特性を再現するフィルタ段。Phase 3 では Modal Synthesis 方式（M=8 並列 biquad）を採用（D30） |
| **Modal Synthesis** | 振動体の卓越モード（共鳴周波数とそのQ）を独立振動子の重ね合わせで再現する手法。Phase 1 §3.3 が一次資料 |
| **Modal Body Resonator** | Modal Synthesis を Body Resonator に応用した実装。M=8 のモード（Helmholtz / トップ板主モード / 高次モード）を並列加算（D30） |
| **One-zero loss filter** | `(1 + ρ·z⁻¹)/(1 + ρ)` の 1 段 FIR フィルタ。DC ゲイン 1 を保ったまま Nyquist 付近を `(1−ρ)/(1+ρ)` 倍に減衰し、弦の周波数依存損失を再現する（D33、Smith *PASP* 標準形） |
| **Pick position（ピック位置）** | 弦を励振する位置（ナットからブリッジまでの相対位置 β ∈ (0, 0.5]）。Phase 3 では **`note_on` 時の励振 shaping**（`buffer[i] -= buffer[i−K]` を in-place 適用、K = round(β·length_int)）で実装。process 内コスト 0、追加メモリ 0（既存 KS buffer 使い回し）。β·L の節を持つ倍音が消失することで音色が変化（D34） |
| **Stretching all-pass** | 弦の剛性による高次倍音の上方偏移（inharmonicity）を再現する dispersive all-pass フィルタ列。Phase 3 では不採用、Phase 4 ピアノ音色で採用予定（D35） |
| **Thiran allpass** | IIR allpass による fractional delay 実装。`H(z) = (a₁ + z⁻¹)/(1 + a₁·z⁻¹)`、`a₁ = (1−d)/(1+d)`。`\|H(ω)\| = 1` 厳密保持で C8 自己発振条件を満たす。Phase 3 Step 1 で試作評価（D36） |
| **Pitch Bend** | MIDI で発音中のピッチを連続的に上下する操作。Phase 3 では ±2 半音、Voice trait の `set_pitch_bend(semitones)` で全ボイスに fan-out、SmoothedValue 5 ms tau で遷移（D39） |
| **Mod Wheel（Modulation Wheel, CC#1）** | LFO 強度を制御する MIDI CC。Phase 3 では非対応（LFO 仕様確定が次フェーズ）、Phase 4 で対応予定 |
| **Sustain Pedal（CC#64）** | note_off 時の damping 加速を保留する MIDI CC。値 ≥ 64 で on、< 64 で off。Engine 状態 `sustain_active: bool` で管理（D40） |
| **All Notes Off（CC#123）** | 全 active voice を即時 release する MIDI CC（D38） |
| **Voice State** | UI Voice Meter 表示用に Worklet → main へ push される 33 bytes のスナップショット（active mask 1 byte + 8 振幅 × 4 bytes、D41） |
| **Soft clip** | 振幅を非線形に飽和させる処理。Phase 3 では **区間関数型** (`|x| ≤ 0.95` で完全 linear、`|x| > 0.95` で `signum(x)·(0.95 + 0.05·e/(e+0.05))`、`e = |x| − 0.95`、`|x|→∞` で ±1.0 厳密漸近) を OutputGain 後に挿入（D43）。`tanh` は使わない |
| **Look-ahead limiter** | 信号を 5 ms 遅延させて先読みし、ピーク検出後に gain reduction を遡及適用するリミッタ。Phase 3 では不採用、Phase 4 候補 |
