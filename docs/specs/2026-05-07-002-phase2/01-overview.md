# 01. Phase 2 概要とスコープ

## 目的

Phase 1 (MVP) で確立した「ブラウザで動作する Karplus–Strong 単音シンセ」を土台に、**ポリフォニー化**、**fractional delay によるピッチ精度向上**、**ParamDescriptor + コード生成によるパラメータ二重管理の解消**、**hold note stack による last-note priority の正規化** を行う。Phase 1 の互換性制約（C ABI、リアルタイム制約、Svelte 5 runes）はすべて維持する。

## 他文書との関係

- 上流: [`pre-research.md`](./pre-research.md)（Phase 2 追加調査）、[Phase 1 全 8 章](../2026-05-06-001-mvp/)（既存資産）
- 下流: [`02-architecture.md`](./02-architecture.md)（全体構成の差分）→ `03〜05`（各レイヤ詳細）→ [`06-build-and-verify.md`](./06-build-and-verify.md) → [`07-implementation-checklist.md`](./07-implementation-checklist.md)
- 並行: `docs/retrospective/2026-05-06-001-mvp.md`（Phase 1 振り返り、§7「次イテレーションへの引き継ぎ」と §5「既存コードの負債」を Phase 2 で解消）
- 本書は「Phase 2 で何を作るか」を確定し、以降の文書は「どう作るか」を定義する。

## Phase 2 の完成像

> **ブラウザで動作する Rust/WASM 製の物理モデリング弦シンセ。最大 8 音同時発音できる Karplus–Strong ポリフォニックシンセが、画面鍵盤・PCキーボード・MIDIキーボードで発音し、damping/brightness/output_gain の 3 パラメータを音色に反映する。Lagrange 3 次補間による fractional delay でピッチ精度を保ち、ParamDescriptor 経由で Rust/TS 二重管理を解消し、hold note stack で内部的にモノモードの last-note 復帰挙動も提供する。**

「最初の音が出ること」（Phase 1 ゴール）から「音楽的に演奏できる土台」へ進める。Phase 1 の音響的・実装的な妥協点を解消することが Phase 2 の主目的で、新規音色追加（Body Resonator、Modal Synthesis、Digital Waveguide）は Phase 3 以降に送る。

## ゴール

- `VoicePool<const N: usize>` で N=8 同時発音できる
- 9 音目押下時の voice stealing が「same-note-replace → 空きボイス最若番 → energy 閾値以下のうち最古 → 全 loud なら最古」の 4 段フォールバック（D13）で動作し、耳障りなクリックノイズを発生させない
- A1 (55 Hz) から C6 (1046 Hz) まで主要音域でピッチ誤差が ± 0.5% 以内（Phase 1 の整数ディレイで A1=55Hz の 2.3% 誤差を解消）。C7-C8 は FIR Lagrange 補間の magnitude が 1 を下回ることで loop gain が < 1 となり C8 fundamental が自己発振しない物理的制約があるため、R23 フォールバック (5) に従い `#[ignore]` 扱いとし Phase 3 で Thiran allpass / pitch tracker / FFT-based estimator の検討時に再評価する
- `params.json` を単一ソースとし、Rust 側 `ParamId` enum / 範囲定数 と TS 側 `PARAM_IDS` / 範囲定義を **コード生成で同期**、CI で drift を検知
- 内部的にモノ/ポリの両モードを持ち、モノモードでは hold note stack による last-note 復帰挙動を提供する（UI トグルは Phase 2 では出さない）
- ポリフォニー 8 音時の AudioWorklet `process` 中ヒープ確保ゼロ（Phase 1 で達成した制約を維持）
- WASM gzip サイズ < 30 KB、Worklet 本番バンドル < 10 KB
- ポリフォニー 8 音時の `process` 1 回 < 1.5 ms（128 frames @ 48kHz、CPU 予算 2.67 ms の 56%）

## 非ゴール（Phase 2 には含めない）

| 項目 | 理由 / 送り先 |
|---|---|
| Body Resonator（ボディ共鳴 IR / modal / static） | 実装方式選択が Phase 2 の主軸から外れる。Phase 3 で着手 |
| Extended Karplus–Strong: loss filter / pick position / stretching all-pass | fractional delay 完了後に余裕があれば追加。Phase 3 候補 |
| Digital Waveguide（双方向ディレイライン）| Phase 3 領域 |
| Modal Synthesis、Mass-Spring、FDTD | Phase 4-5 領域 |
| MIDI CC マッピング（pitch bend / mod / sustain）| UI 改修込みで実装ボリュームが大きい。Phase 3 候補 |
| プリセット保存・ロード | localStorage / IndexedDB / file の戦略決定が必要。Phase 3 以降 |
| オーディオ録音・WAV 書き出し | スコープ外 |
| WASM SIMD（`target-feature=+simd128`）| ブラウザ互換性の安定を待つ。Phase 3 改善余地 |
| UI でのポリフォニー / モード可視化（active voice 数表示、mono/poly トグル）| Phase 1 のミニマル UI 思想を維持。内部両対応のみ |
| iOS Safari 以外のモバイル動作保証 | Phase 1 と同じくデスクトップ Chromium が主、iOS Safari は検証のみ |

## 確定事項（ユーザー承認済み）

| 決定事項 | 内容 |
|---|---|
| 機能スコープ | ポリフォニー / fractional delay / ParamDescriptor + コード生成 / hold note stack の 4 件採用 |
| レビュー運用 | 一括レビュー方式（pre-research → 01〜07 を全部書き切ってから一度に提示） |
| ディレクトリ命名 | `docs/specs/2026-05-07-002-phase2/`、Phase 1 と同じ pre-research + 01〜07 の 8 章構成 |

## Phase 1 設計判断 D1〜D11 の維持・変更マッピング

Phase 1 の主要設計判断 11 項目を Phase 2 で「維持 / 変更 / 拡張」のいずれかに分類する。詳細は各章で展開する。

| # | Phase 1 D# | 内容 | Phase 2 での扱い | 主たる記述章 |
|---|---|---|---|---|
| D1 | 整数ディレイで割り切る | A1=55Hz で 2.3% 誤差 | **変更** → fractional delay へ（Lagrange 3 次補間、D14 で詳細） | 03 章 |
| D2 | MessagePort + Rust 側 SmoothedValue | パラメータ送信経路 | **維持** → ポリフォニー時も同じ経路で全ボイス fan-out | 02 / 03 章 |
| D3 | WASM ロードはメインスレッド経由 | Worklet の fetch 制限回避 | **維持** → 完全互換 | 04 / 05 章 |
| D4 | WASM linear memory の grow を起こさない | `prepare` 一括確保、`process` 中 Vec 操作禁止 | **維持** → VoicePool も `Engine::prepare` で N × max_buffer 一括確保 | 03 / 04 章 |
| D5 | iOS Safari 対策で StartButton 必須 | ユーザージェスチャ内 `resume()` | **維持** → Phase 1 SynthEngine 起動シーケンスをそのまま継承 | 05 章 |
| D6 | denormal 対策で DC injection | `process_sample` 末尾の `+1e-25 -1e-25` | **維持** → 各ボイスの process_sample で継続 | 03 章 |
| D7 | note_off は damping 加速で自然減衰 | `note_off_target_damping = 0.95` | **拡張** → モノモードでは hold stack 連携、ポリモードでは Phase 1 と同じ damping 加速（D29） | 03 章 |
| D8 | wasm-audio は C ABI、wasm-bindgen 不使用 | `#[unsafe(no_mangle)] extern "C"` | **維持** → 既存 10 関数のシグネチャ完全互換、追加関数も同方式 | 04 章 |
| D9 | AudioWorklet の Float32Array view を init 時にキャッシュ | GC 圧排除 | **維持** → ポリフォニー時も同じ scratch_l/r を使用、view も同じ | 05 章 |
| D10 | secure context 必須 | HTTPS / localhost のみ動作 | **維持** → SynthEngine.start / MidiSelect の既存チェック継続 | 05 / 06 章 |
| D11 | Svelte 5 runes ベース | `$state` / `$bindable` / `$effect` action / `onclick` 記法 | **維持** → ParamSlider を ParamDescriptor 駆動に改修するが runes は継続 | 05 章 |

## 主要な設計判断（Phase 2 新規 D12〜D29）

仕様策定の過程で確定した、Phase 2 実装時に逸脱しない 18 項目。詳細な根拠と適用箇所は各レイヤ仕様書に記載する。

| # | 判断 | 内容 / 採用案 | 主たる記述章 |
|---|---|---|---|
| **D12** | VoicePool のサイズ N | **N=8 固定**（const generic で `VoicePool<const N: usize>` 形式は維持し、Phase 2 では値を 1 つに固定） | 03 章 |
| **D13** | Voice stealing 戦略 | **same-note-replace → 空きボイス最若番 → energy 閾値以下のうち最古 → 全ボイス loud なら最古** の 4 段フォールバック | 03 章 |
| **D14** | Fractional delay 実装 | **Lagrange 3 次補間**（係数は `note_on` 時に 1 度計算してキャッシュ、process 内は 4 サンプル積和のみ） | 03 章 |
| **D15** | ParamDescriptor 生成方式 | **外部 Node スクリプト** `scripts/gen-params.mjs`（Phase 1 の `copy-wasm.mjs` / `check-wasm-exports.mjs` と同パターン） | 02 章 |
| **D16** | Hold note stack 容量と溢れ時挙動 | **容量 16、溢れ時は最古破棄**（PC キーボード 15 鍵 + 余裕） | 03 章 |
| **D17** | wasm-audio に追加する新 export | **`synth_set_polyphony_mode(handle, mode: u32)` のみ追加**（mode: 0=poly, 1=mono）。voice count 表示はしないため active_voice_count は追加しない | 04 章 |
| **D18** | C ABI メッセージ ID（既存 10 関数） | **完全互換維持**（シグネチャ・export 名・動作すべて Phase 1 と一字一句同じ） | 04 章 |
| **D19** | `Voice` trait に追加するメソッド | **`note_id() -> Option<u8>` / `age() -> u32` / `amplitude() -> f32`** の 3 メソッドを追加。`set_*` は inherent のままで VoicePool が KarplusStrong 固有 API として呼ぶ | 03 章 |
| **D20** | ポリフォニー時の合成ゲイン | **1/sqrt(N) スケール**（知覚的にエネルギー保存、ユーザー OutputGain で最終調整可能）。常用範囲（OutputGain ≤ 1.0、通常の押下パターン）でクリップ回避。最悪ケース（8 鍵同時全力 + OutputGain=1.5）の歪みは Phase 2 では許容し soft clip/limiter は Phase 3 候補（[`03-dsp-core-spec.md` §1/sqrt(N) スケールの根拠と限界](./03-dsp-core-spec.md#1sqrtn-スケールの根拠と限界d20)）| 03 章 |
| **D21** | mono / poly モード切替 UI | **内部両対応 + UI は固定（Phase 2 では mono/poly トグルを UI に出さない）**。デフォルトは poly モード。`synth_set_polyphony_mode` C ABI は提供するが UI からは呼ばない | 05 章 |
| **D22** | UI で active voice 数表示 | **表示しない**（Phase 1 ミニマル UI 思想を維持。Phase 3 でビジュアライザー検討時に再評価） | 05 章 |
| **D23** | Hold note stack のデータ構造 | **自前 `LinearStack<u8, MAX_HELD>`**（固定配列 + len、ヒープ確保なし、`heapless` 等の外部依存追加せず Phase 1 の依存ゼロ方針継続） | 03 章 |
| **D24** | params.json のフォーマット | **JSON**（Node スクリプトでの読み取りが標準ライブラリのみで完結。TOML/YAML は依存追加のため不採用） | 02 章 |
| **D25** | 生成物の git 管理 | **コミットする**（`crates/dsp-core/src/params.rs`、`web/src/lib/audio/generated/params.ts` の両方）。drift を PR diff で可視化、CI で `pnpm gen:params` 実行後の git diff チェック | 02 章 |
| **D26** | Fractional delay 係数の更新タイミング | **`note_on` 時のみ計算してキャッシュ**（pitch bend は Phase 2 非対応のため `length_frac` は note の生存中変化しない） | 03 章 |
| **D27** | KarplusStrong バッファサイズの余裕 | **27.5Hz + Lagrange 3 次補間分の +3 サンプル余裕**（最小周波数 27.5Hz を維持しつつ、補間カーネルの過去サンプル参照に必要な余裕を確保） | 03 章 |
| **D28** | Voice stealing 時のクリック対策 | **energy 閾値以下優先選択 + ステアリング後の即時再励振**（fade out を別途実装せず、KarplusStrong の `note_on` がバッファをノイズで上書きすることで自然に切り替わる） | 03 章 |
| **D29** | Hold note stack の適用範囲 | **mono モード専用、ランタイム mode で if 分岐**（poly モードでは hold stack を参照しない。コンパイル時 cfg 分岐ではなく実行時切替で C ABI 互換を維持） | 03 章 |

## C ABI 既存 10 関数の互換性チェックリスト

Phase 2 では以下の Phase 1 確定 C ABI 関数を **シグネチャ・export 名・動作すべて完全に維持** する（D18）。

| 関数名 | シグネチャ | Phase 2 での扱い |
|---|---|---|
| `synth_new` | `(f32, u32) -> *mut SynthHandle` | 維持。内部で N=8 ボイスとスクラッチを一括確保するように **動作のみ拡張**、外部仕様は不変 |
| `synth_free` | `(*mut SynthHandle)` | 維持 |
| `synth_note_on` | `(*mut SynthHandle, u8, f32)` | 維持。内部で VoicePool に allocation / stealing が走るが、外部仕様は不変 |
| `synth_note_off` | `(*mut SynthHandle, u8)` | 維持。mono モードでは hold stack 連携、poly モードでは該当ボイスに note_off 発火 |
| `synth_set_param` | `(*mut SynthHandle, u32, f32)` | 維持。内部で全ボイスへ fan-out |
| `synth_reset` | `(*mut SynthHandle)` | 維持。全ボイスを reset、hold stack をクリア |
| `synth_out_l_ptr` | `(*const SynthHandle) -> *const f32` | 維持。scratch_l のポインタは Phase 1 と同じ |
| `synth_out_r_ptr` | `(*const SynthHandle) -> *const f32` | 維持 |
| `synth_capacity` | `(*const SynthHandle) -> u32` | 維持。返値は scratch_l.len() = 128 |
| `synth_process_block` | `(*mut SynthHandle, u32)` | 維持。内部で全アクティブボイスをミックスして scratch に書き込む |

### Phase 2 で追加する C ABI 関数

| 関数名 | シグネチャ | 役割 |
|---|---|---|
| `synth_set_polyphony_mode` | `(*mut SynthHandle, mode: u32)` | mode=0: poly、mode=1: mono。Phase 2 UI からは呼ばないが C ABI として提供（D17） |

`scripts/check-wasm-exports.mjs` の `REQUIRED` 配列に `synth_set_polyphony_mode` を追加する（本仕様 04 / 06 章）。

## Phase 3 への申し送り（Phase 2 完成後の検討）

Phase 2 では実装しないが、Phase 3 で検討すべき設計:

- **Body Resonator**: 弦音だけでは「安っぽい音」から抜けない。IR convolution / modal filter / static IR の選択を Phase 3 冒頭で確定し、`dsp-core/src/body.rs` 系として導入する
- **Extended Karplus–Strong 残機能**: loss filter（弦の周波数依存減衰）、pick position（comb 効果）、stretching all-pass（弦の硬さ）を Phase 3 で順次追加
- **MIDI CC マッピング**: pitch bend / mod wheel / sustain pedal を Web MIDI から受信し、`SmoothedValue` 経由で全ボイスにマッピング。`messages.ts` の `ToWorkletMessage` 拡張が必要
- **プリセット保存・ロード**: `localStorage` / IndexedDB の選択、ParamDescriptor からプリセット JSON スキーマを生成
- **WASM SIMD**: `target-feature=+simd128` 安定化と Safari/Firefox 対応状況を再評価
- **UI でのポリフォニー / モード可視化**: active voice 数のメーター、mono/poly トグル、ボイスごとのエンベロープ表示
- **Digital Waveguide / Modal Synthesis / Mass-Spring**: Phase 4 以降の楽器拡張領域

## アーキテクチャ概要（詳細は 02-architecture.md）

Phase 1 の 4 レイヤ構成は維持。VoicePool が dsp-core 内 Engine の責務として追加され、ParamDescriptor codegen が新規ビルドパイプラインとして追加される。

```
┌─────────────────────────────────────────┐
│ Svelte UI（メインスレッド）              │
│  StartButton / Keyboard / Slider / MIDI │
│  + ParamSlider が ParamDescriptor 駆動  │  ← Phase 2 差分
└──────────────┬──────────────────────────┘
               │ MessagePort（noteOn/noteOff/setParam/setMode/init）
               │  ※ setMode は Phase 2 で内部 API として用意（UI からは呼ばない）
               ▼
┌─────────────────────────────────────────┐
│ AudioWorkletProcessor（音声スレッド）   │
│  WASM ロード、process 委譲              │
│  + WasmExports に synth_set_polyphony_mode 追加  ← Phase 2 差分
└──────────────┬──────────────────────────┘
               │ FFI（共有メモリ + ポインタ、既存 10 関数互換）
               ▼
┌─────────────────────────────────────────┐
│ wasm-audio（Rust crate, cdylib）        │
│  SynthHandle が dsp-core を呼ぶ          │
│  + synth_set_polyphony_mode 新規追加     │  ← Phase 2 差分
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│ dsp-core（Rust crate, rlib, 純粋）       │
│  Engine / VoicePool<8> / KarplusStrong  │  ← Phase 2 差分
│  + FractionalDelay / NoteAllocator       │
│  + HoldStack / SynthMode                 │
│  Smoothing / XorShift32 / ParamDescriptor│
└─────────────────────────────────────────┘
```

ビルドパイプラインは新規スクリプト 2 つを追加:

```
params.json (単一ソース)
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

## 用語集（Phase 2 追加分）

Phase 1 [01 章 用語集](../2026-05-06-001-mvp/01-overview.md#用語集) の用語に加えて、Phase 2 で新規導入する用語を定義する。

| 用語 | 説明 |
|---|---|
| **VoicePool** | 固定数 N のボイスを配列として保持し、note_on のたびに空きボイス検索 / voice stealing を行うコンテナ。dyn dispatch せず `[KarplusStrong; N]` の const generic 配列で実装する |
| **Voice stealing** | ボイス上限 N に達した状態で新しい note_on が来たとき、既存ボイスを 1 つ犠牲にして新音を発音する処理。Phase 2 では D13 の 4 段フォールバック「same-note-replace → 空きボイス最若番 → energy 閾値以下のうち最古 → 全 loud なら最古」を採用。閾値ベース（min amplitude ではない）にする理由は (a) クリック対策で「ほぼ無音グループ」を優先的に犠牲にすると知覚されにくい、(b) min を取ると stealing 偏りが起きる、(c) 閾値以下を 1 グループとして扱い age で順序付けることで知覚と公平性のバランスを取るため |
| **Note allocator** | VoicePool 内で voice stealing 戦略を判定する責務を持つロジック。`note_on` のとき空きボイス検索とフォールバック判定を行う |
| **Lagrange interpolation（Lagrange 補間）** | 複数の標本点を通る多項式で標本間の値を推定する補間手法。Phase 2 では 3 次（4 サンプル参照）を採用し fractional delay を実装する（D14） |
| **Thiran allpass** | IIR allpass フィルタを使った fractional delay 実装。位相応答が良いが係数計算が複雑。Phase 2 では採用せず Phase 3 候補（pre-research §3） |
| **Fractional delay** | ディレイ長を整数サンプル単位に丸めず、`length_int + length_frac` のような小数部を含む形で扱う設計。Phase 2 でピッチ精度向上のため導入（D14 / D26 / D27） |
| **ParamDescriptor** | パラメータ 1 件のメタデータ（id, name, min, max, default, smoothing_tau）を保持する構造体。`params.json` を単一ソースとし、Rust と TS 双方をコード生成する（D15 / D24 / D25） |
| **Hold note stack（hold stack）** | モノモード時に押下中のキー履歴を保持するスタック。最後に押されたキーを発音し、リリース時に次に新しいキーへ復帰する（last-note priority）。Phase 2 では mono 専用、容量 16、溢れ時最古破棄（D16 / D23 / D29） |
| **last-note priority** | モノフォニーでの複数キー同時押し時の挙動方針で「最後に押されたキーを優先する」もの。Phase 1 は簡易版（押し直さない）、Phase 2 は hold stack による完全版（リリース時に前のキーへ復帰）|
| **same-note-replace** | voice stealing の最優先戦略。新しい note_on のノート番号と同じノートを既に発音中のボイスがあれば、そのボイスを再励振する（音楽的にトリル / 連打が自然になる、D13） |
| **SynthMode** | mono / poly のいずれかを表す enum。Engine が保持し、note_on / note_off の経路で hold_stack を使うかを切り替える（D29） |
