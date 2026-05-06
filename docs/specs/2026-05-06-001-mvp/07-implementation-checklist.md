# 07. 実装順序チェックリスト

## 目的

仕様書承認後、別タスクで本MVPを実装する際の作業順序をステップバイステップで定義する。各ステップは独立して進捗確認でき、検証チェックリスト（F1〜F9）の充足ポイントを明示する。

## 他文書との関係

- 上流: 全ての仕様書（01〜06）
- このドキュメントは **実装専用のチェックリスト** であり、各ステップの設計詳細は対応する仕様書を参照する

## 実装ステップ（19段階）

### フェーズ I — モノレポ基盤

#### Step 1. ワークスペース骨格を作成
- [ ] `Cargo.toml`（ワークスペース定義）を [`02-architecture.md`](./02-architecture.md#cargotomlワークスペースルート) に従い作成
- [ ] `rust-toolchain.toml` を作成
- [ ] `pnpm-workspace.yaml` を作成
- [ ] ルート `package.json` を作成（[02章のスクリプト一覧](./02-architecture.md#ルート-packagejson) 参照）
- [ ] `scripts/copy-wasm.mjs` を [04章のビルド方法](./04-wasm-audio-spec.md#ビルド方法) に従い作成
- [ ] `scripts/check-wasm-exports.mjs` を [06章](./06-build-and-verify.md#export-名の自動検証スクリプト) に従い作成
- [ ] `.gitignore` を作成
- [ ] `README.md` を作成（プロジェクト概要、セットアップ手順、`pnpm dev` の起動方法）
- [ ] `git init` & 初回コミット
- **検証**: `pnpm install` がエラーなく完了する

### フェーズ II — Rust dsp-core

#### Step 2. `dsp-core` クレート雛形を作成
- [ ] `crates/dsp-core/Cargo.toml` を [`03-dsp-core-spec.md`](./03-dsp-core-spec.md#crates-dsp-core-cargotoml) に従い作成
- [ ] `crates/dsp-core/src/lib.rs` を作成（モジュール宣言と pub use）
- [ ] 各モジュールファイル（`traits.rs`, `params.rs`, `smoothing.rs`, `rng.rs`, `karplus_strong.rs`, `voice.rs`, `engine.rs`）を空で作成
- **検証**: `cargo check -p dsp-core` が通る

#### Step 3. `traits.rs`、`params.rs`、`smoothing.rs`、`rng.rs` を実装
- [ ] `AudioProcessor` / `Voice` trait を定義
- [ ] `ParamId` enum と `from_u32` を実装
- [ ] `SmoothedValue` を実装（係数計算: `1 - exp(-1/(sr*tau))`）
- [ ] `XorShift32` を実装
- **検証**: `cargo build -p dsp-core` が通る

#### Step 4. `KarplusStrong` を実装
- [ ] 構造体定義（[03章のフィールド一覧](./03-dsp-core-spec.md#構造体フィールドの役割)）
- [ ] `new()`、`prepare()`、`note_on()`、`note_off()`、`process_sample()` を実装
- [ ] `set_damping(value)` / `set_brightness(value)` ヘルパを実装
- [ ] denormal 対策（`+1e-25 - 1e-25`）を `process_sample` 末尾に挿入
- [ ] envelope tracking（`energy` 蓄積で `is_active` 判定）
- **検証**: `cargo build -p dsp-core` が通る

#### Step 5. `Voice` trait を `KarplusStrong` に実装
- [ ] `Voice` trait の各メソッドを `KarplusStrong` で実装
- **検証**: `cargo build -p dsp-core` が通る

#### Step 6. `Engine` を実装
- [ ] 構造体定義（`current_damping` フィールドを含む）
- [ ] `new()`、`prepare()`、`note_on()`（damping を `current_damping` に復元）、`note_off()`（last-note 簡易版）、`set_param()`（Damping 時に `current_damping` を更新）、`process()`、`reset()` を実装
- [ ] MIDI to Hz 変換（`440 * 2^((midi-69)/12)`）
- **検証**: `cargo build -p dsp-core` が通る

#### Step 7. ユニットテストを追加・実行
- [ ] `karplus_strong.rs` または `tests/` 配下に [03章のテスト一覧](./03-dsp-core-spec.md#テスト方針cargo-test) を実装
- [ ] `test_silence_when_inactive` / `test_energy_rises_after_note_on` / `test_decay_with_low_damping` / `test_length_matches_freq` / `test_no_allocation_in_process` / `test_paramid_roundtrip`
- [ ] `test_damping_preserved_across_note_on`（追加）: `set_param(Damping, 0.999)` → `note_on` → `note_off` → `note_on` で damping が `0.999` に戻ることを確認
- **検証**: `cargo test -p dsp-core` が全て通る

### フェーズ III — Rust wasm-audio

#### Step 8. `wasm-audio` クレート雛形を作成（C ABI）
- [ ] `crates/wasm-audio/Cargo.toml` を [`04-wasm-audio-spec.md`](./04-wasm-audio-spec.md) に従い作成（`crate-type = ["cdylib"]`、wasm-bindgen 依存なし）
- [ ] `crates/wasm-audio/src/lib.rs` を作成し、`#[no_mangle] extern "C"` の関数群を実装
- [ ] 公開関数: `synth_new`, `synth_free`, `synth_note_on`, `synth_note_off`, `synth_set_param`, `synth_reset`, `synth_out_l_ptr`, `synth_out_r_ptr`, `synth_capacity`, `synth_process_block`
- **検証**: `cargo build -p wasm-audio --target wasm32-unknown-unknown` が通る

#### Step 9. WASM ビルドと export 名の自動検証
- [ ] `scripts/check-wasm-exports.mjs` を [06章](./06-build-and-verify.md#export-名の自動検証スクリプト) に従い作成
- [ ] ルート `package.json` の `build:wasm` / `build:wasm:dev` に `node scripts/check-wasm-exports.mjs` をチェーン
- [ ] `pnpm build:wasm:dev` を実行
- [ ] `web/src/lib/wasm/wasm_audio.wasm` が生成され、`All required WASM exports present.` が出力される
- **検証**: 期待する export 名（`synth_new`, `synth_process_block` など）が `#[no_mangle]` のとおりに含まれる。スクリプトが exit 0 で終了

### フェーズ IV — Web フロントエンド基盤

#### Step 10. SvelteKit プロジェクトを作成
- [ ] プロジェクトルートで `npx sv create web` 実行（[05章のセットアップ](./05-web-frontend-spec.md#プロジェクト作成) 参照）
- [ ] `@sveltejs/adapter-static` に差し替え
- [ ] `svelte.config.js` を [05章の設定](./05-web-frontend-spec.md#websveltesconfigjs) に更新
- [ ] `vite.config.ts` を [05章の設定](./05-web-frontend-spec.md#webviteconfigts) に更新
- [ ] `web/package.json` に `build:worklet` スクリプトと `esbuild` を追加
- [ ] `pnpm install` で依存解決
- **検証**: `pnpm --filter web dev` で初期ページが http://localhost:5173 に表示される

#### Step 11. Worklet スケルトンと SynthEngine を実装
- [ ] `web/src/lib/audio/messages.ts`（メッセージ型定義、`reset` を含む）を作成
- [ ] `web/src/lib/audio/synth-processor.ts` を [05章](./05-web-frontend-spec.md#audioworkletprocessorsynth-processorts) に従い作成
  - C ABI export を `WasmExports` interface 経由で呼び出す
  - **Float32Array view を init 時にキャッシュ** し、`memory.buffer` 変化時のみ `refreshViews()`
  - import object は `{ env: {} }` のみ
- [ ] `web/src/lib/audio/engine.ts`（`SynthEngine` クラス）を作成
  - `import wasmUrl from '$lib/wasm/wasm_audio.wasm?url'` で dev/build 統一
  - Worklet スクリプトは `import { base } from '$app/paths'` で `${base}/worklet/synth-processor.js` を参照
  - `window.isSecureContext` と `ctx.audioWorklet` の存在チェックを `start()` 冒頭に追加
  - **`AudioContext.resume()` を `start()` 内のユーザージェスチャ連鎖で必ず await する**（iOS Safari 対策）
  - **`start()` を `ready` メッセージ受信まで resolve しない Promise** にし、5 秒の `setTimeout` タイムアウトを設ける
  - **失敗時は内部で `dispose()` を呼び、`node.disconnect()` / `ctx.close()` / `ready=false` で再試行可能な状態へ戻す**
  - `currentParams: Map<number, number>` で起動前の値を保持し、`ready` 受信後に再送
  - `dispose()` メソッドで `cancelAnimationFrame` / Worklet への `dispose` メッセージ送信 / `node.disconnect()` / `ctx.close()` を実装
- [ ] `synth-processor.ts` の `onMessage` に `dispose` ケースを追加し、`synth_free` を呼んで view をクリア
- [ ] `synth-processor.ts` の `process()` 冒頭で `outputs[0][0].length !== FRAMES` をチェックし、無音返却 + 警告 1 度送信
- [ ] rAF スロットルを `setParam` 内に実装（`currentParams` は常時更新、`pendingParams` は ready 後のみ）
- [ ] `pnpm --filter web build:worklet:dev` で `web/static/worklet/synth-processor.js` が sourcemap 付きで生成されることを確認
- **検証**: ブラウザで Console エラーなく `await engine.start()` が完了し、その時点で `engine.isReady() === true`。意図的に WASM を壊して再ビルドすると 5 秒で reject、再度ボタンを押せば再試行可能

#### Step 12. 共有ステートと最小UI（StartButton）を作成
- [ ] `web/src/lib/state/synth.svelte.ts` を Svelte 5 の `$state` ベースで作成（`writable` ストアは使わない）
- [ ] `web/src/lib/components/StartButton.svelte` を作成（`onclick` 記法、`$state` ベース、エラー表示、再試行可能）
- [ ] `web/src/routes/+page.svelte` で StartButton と動作テスト用の固定ボタン（C4 を1音だけ鳴らす）を配置
- [ ] `+page.svelte` の `onDestroy` で `synth.engine.dispose()` を呼び、HMR / 画面遷移時にリソースを解放
- **検証**: 「Start Audio」を押した後、テストボタンを押すと弦らしい音が鳴る → **F1 達成**。HMR でページが再描画されても AudioContext が複数生成されない

### フェーズ V — 演奏入力

#### Step 13. 画面鍵盤コンポーネント
- [ ] `web/src/lib/components/Keyboard.svelte` を [05章](./05-web-frontend-spec.md#keyboardsvelte) に従い作成（Svelte 5 記法）
- [ ] C3〜C5 の2オクターブを表示
- [ ] `onpointerdown` / `onpointerup` / `onpointerleave` ハンドラ内で `e.preventDefault()`（修飾子は廃止）
- [ ] `+page.svelte` に組み込み
- **検証**: 画面の任意のキーをクリックで発音、リリースで自然減衰 → **F2 達成**

#### Step 14. PCキーボード入力（Svelte 5 $effect ベース action）
- [ ] `web/src/lib/actions/pc-keyboard.svelte.ts` を [05章](./05-web-frontend-spec.md#pcキーボード-svelte-actionactionspc-keyboardsveltets) に従い作成
  - `.svelte.ts` 拡張子（`$effect` runes 使用のため）
  - `Action<HTMLElement, PcKeyboardParams>` 型、内部で `$effect(() => { ... return cleanup })` を呼ぶ
  - `event.repeat` を弾き、`heldKeys` Set で同一キー重複押下を抑制
- [ ] `+page.svelte` の `<main>` に `use:pcKeyboard={{ onNote: ... }}` で適用
- **検証**: A〜L 行 + W〜O 行が C4〜D5 の半音階に対応 → **F3 達成**

#### Step 15. Web MIDI 入力
- [ ] `web/src/lib/input/midi.ts` を [05章](./05-web-frontend-spec.md#web-midimidits) に従い作成（`initMidi` / `disposeMidi` / `setActiveInput` / `listInputs`）
- [ ] `web/src/lib/components/MidiSelect.svelte` を作成
  - `$state` で `supported` / `inputs` / `selectedId` を宣言
  - **非同期初期化は `onMount` で行い、cleanup return で `disposeMidi()` を呼ぶ**（`$effect` 内での状態更新は再実行ループのリスクがあるため避ける）
  - `selectedId` の変化追跡だけ `$effect` で行い、`setActiveInput(selectedId)` を呼ぶ
  - `window.isSecureContext` チェックを `supported` 判定に含める
- [ ] selectedId に応じた絞り込みを `setActiveInput` で反映、null は全入力購読
- [ ] `+page.svelte` に組み込み
- [ ] `requestMIDIAccess` 非対応時のフォールバック表示
- **検証**: MIDI キーボード接続 → デバイス選択 → 鍵盤押下で発音、`(all inputs)` 選択時は全デバイス、特定デバイス選択時はそのデバイスのみ → **F4 達成**

### フェーズ VI — パラメータ制御

#### Step 16. ParamSlider と3つのパラメータ
- [ ] `web/src/lib/components/ParamSlider.svelte` を [05章](./05-web-frontend-spec.md#paramslidersvelte) に従い作成
  - `$props()` で props 宣言
  - `value` は `$bindable()` で双方向バインド可能に
  - イベントハンドラは `oninput` 記法
- [ ] `+page.svelte` で Damping / Brightness / OutputGain の3スライダーを配置
- [ ] `bind:value={synth.damping}` のように `$state` プロパティと双方向バインド
- **検証**:
  - damping=0.99 → 0.999 で減衰時間が伸びる → **F5 達成**
  - brightness=0.1 → 0.9 で高域含有量が変わる → **F6 達成**

#### Step 17. クリックノイズ対策の確認
- [ ] スライダーを左右に高速ドラッグして音を聞く
- [ ] dsp-core の `SmoothedValue` の時定数を必要に応じて調整（既定tau=0.02s）
- **検証**: プチノイズが聞こえない → **F7 達成**

### フェーズ VII — 性能とブラウザ互換

#### Step 18. メモリ確保プロファイル
- [ ] Worklet の `process` で `memory.buffer.byteLength` の不変チェックを仕込む（[06章 F8(a)](./06-build-and-verify.md)）
- [ ] note_on を 100 回連打して `[F8] WASM memory grew` の警告が一度も出ないことを確認
- [ ] `cargo expand -p wasm-audio` で `synth_process_block` 経路に `Vec::push`/`Vec::with_capacity` が無いことを確認（または `cargo build --release` 後の wasm を `wasm-objdump` で確認）
- [ ] Chrome DevTools Performance で JS Heap が線形増加しないことを補助確認
- [ ] 検証完了後、開発時のチェックコードを除去
- **検証**: WASM memory.buffer.byteLength が不変 → **F8 達成**

#### Step 19. 本番ビルド & iOS Safari 動作確認 & README 整備
- [ ] `pnpm build` で本番ビルドを生成
- [ ] `pnpm --filter web preview` で http://localhost:4173 を開き、F1〜F8 が再現することを確認
- [ ] HTTPS で外部公開（ngrok / Cloudflare Tunnel / mkcert いずれか、[06章 F9](./06-build-and-verify.md)）
- [ ] iPhone Safari で HTTPS URL にアクセス → `window.isSecureContext === true` を確認 → Start Audio タップで音が鳴る
- [ ] `README.md` に動作環境（Chrome/Edge推奨、Web MIDI/AudioWorklet は HTTPS 必須）、セットアップ手順、`pnpm dev` の起動方法、F1〜F9 の自己検証手順を記載
- **検証**: iOS Safari（HTTPS）で発音 → **F9 達成**

## ステップごとの依存関係

```
Step 1 (基盤)
  └─ Step 2 ─ Step 3 ─ Step 4 ─ Step 5 ─ Step 6 ─ Step 7 (dsp-core 完成)
                                                    │
                                                    ▼
                                            Step 8 ─ Step 9 (wasm-audio 完成)
                                                    │
                                                    ▼
                                            Step 10 ─ Step 11 ─ Step 12 (F1)
                                                                  │
                                          ┌──────────────────────┼──────────────────────┐
                                          ▼                       ▼                       ▼
                                       Step 13 (F2)          Step 14 (F3)            Step 15 (F4)
                                          │                       │                       │
                                          └───────────────────────┼───────────────────────┘
                                                                  ▼
                                                              Step 16 (F5/F6)
                                                                  │
                                                                  ▼
                                                              Step 17 (F7)
                                                                  │
                                                                  ▼
                                                              Step 18 (F8)
                                                                  │
                                                                  ▼
                                                              Step 19 (F9)
```

並列実装可能なポイント:
- Step 13 / 14 / 15 は独立しており、UI実装に余裕があれば並行進行可能
- Step 7（ユニットテスト）は Step 4〜6 の実装と並行して書ける（TDD）

## 達成ライン早見表

| ステップ完了 | 達成する検証項目 |
|---|---|
| Step 12 | F1 |
| Step 13 | F2 |
| Step 14 | F3 |
| Step 15 | F4 |
| Step 16 | F5, F6 |
| Step 17 | F7 |
| Step 18 | F8 |
| Step 19 | F9 |

すべての F1〜F9 が達成された時点で MVP 完成。

## 実装着手者へのメモ

- **Step 4 が最大の山場**。pre-research 7.1 のサンプルコードをそのまま写経すると `Vec::resize` 問題に陥るため、必ず [03章](./03-dsp-core-spec.md) の `note_on` 設計に従うこと
- **Step 8/9 で wasm-bindgen を使わない C ABI 方針を徹底**。`#[no_mangle]` 忘れで export 名が mangling されると Worklet 側の `WasmExports` interface と不一致になる。`wasm-objdump` で必ず確認
- **Step 11 で Float32Array view のキャッシュを忘れない**。`process()` 内で毎回 `new Float32Array(...)` すると音切れの原因
- **Svelte 5 の落とし穴**:
  - 共有ステートのファイルは必ず `.svelte.ts` 拡張子（`$state` runes が普通の `.ts` ではコンパイルエラー）
  - イベントハンドラは小文字記法（`onclick` / `oninput` / `onpointerdown`）。Svelte 4 の `on:click` や `|preventDefault` 修飾子は使えない
  - `bind:value` する prop は子側で `$bindable()` を付けないと双方向バインドにならない
  - 副作用は可能な限り Svelte action（`use:action`）でカプセル化し、`src/lib/actions/` に置く
- **Step 19 の iOS Safari 確認** は HTTPS 必須かつ実機推奨。シミュレータでは AudioContext の挙動が異なる場合がある
- 各ステップで **コミットを分ける** ことを推奨。問題発生時に二分探索しやすい
- 詰まったら [`06-build-and-verify.md` のトラブルシューティング](./06-build-and-verify.md#トラブルシューティング-tips) を参照
