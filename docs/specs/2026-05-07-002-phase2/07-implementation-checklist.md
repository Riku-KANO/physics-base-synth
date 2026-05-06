# 07. Phase 2 実装順序チェックリスト

## 目的

Phase 2 仕様書承認後、別タスクで本仕様を実装する際の作業順序を Phase α〜η の 7 フェーズ・全 22 ステップで定義する。各ステップは独立して進捗確認でき、検証チェックリスト（F10〜F25）の充足ポイントを明示する。Phase 1 の `07-implementation-checklist.md` と同じ粒度（1 ステップ ≈ 1 コミット）で構成する。

## 他文書との関係

- 上流: 全ての仕様書（pre-research、01〜06）
- 参考: [Phase 1 07 章](../2026-05-06-001-mvp/07-implementation-checklist.md)（19 ステップ構成、達成ライン早見表、実装メモ）— **Phase 2 ステップは Phase 1 の構成パターンを踏襲**
- このドキュメントは **実装専用のチェックリスト** であり、各ステップの設計詳細は対応する仕様書を参照する

## 前提条件

Phase 2 実装着手前に以下を確認:

1. **Phase 1 F1〜F9 が retrospective §2 で達成済みと記載されている**（F25）
   - [`docs/retrospective/2026-05-06-001-mvp.md` §2](../../retrospective/2026-05-06-001-mvp.md) を確認
   - 未達ならば Phase 2 着手前に追加検証 + retrospective §2 更新を実施
2. **Phase 1 のすべてのテスト（11 件）がパス**
   - `cargo test -p dsp-core` で確認
3. **`pnpm dev` でブラウザでの音再生が確認できる状態**

## 実装ステップ（全 22 段階、Phase α〜η の 7 フェーズ）

### フェーズ α — ParamDescriptor 基盤（4 ステップ）

新パラメータ追加や Phase 3 の MIDI CC マッピングで `ParamId` / `PARAM_IDS` 二重管理が雪だるま化する前に、コード生成パイプラインを Phase 2 冒頭で整備する。

#### Step 1. `params.json` を作成し、Phase 1 の 3 パラメータを定義

- [ ] リポジトリルートに `params.json` を作成（[`02-architecture.md` §単一ソース](./02-architecture.md#単一ソース-paramsjson) 参照）
- [ ] Phase 1 の 3 パラメータ（Damping / Brightness / OutputGain）を [`03-dsp-core-spec.md` §params.json](./03-dsp-core-spec.md#params-rs-生成出力例) のスキーマで記述
- [ ] `git add params.json && git commit -m "feat(params): add params.json single source"`
- **検証**: JSON が valid（`node -e "JSON.parse(require('fs').readFileSync('params.json'))"`）

#### Step 2. `scripts/gen-params.mjs` を実装（Rust + TS 両方を生成）

- [ ] `scripts/gen-params.mjs` を [`02-architecture.md` §scripts/gen-params.mjs の責務](./02-architecture.md#scriptsgen-paramsmjs-の責務) に従い作成
- [ ] [`03-dsp-core-spec.md` §params.rs 生成出力例](./03-dsp-core-spec.md#params-rs-生成出力例) と同じ Rust ソースを文字列組み立てで出力
- [ ] [`05-web-frontend-spec.md` §generated/params.ts](./05-web-frontend-spec.md#websrclibaudiogeneratedparamsts-の出力例) と同じ TypeScript ソースを出力
- [ ] `pnpm gen:params` を `package.json` に追加（[`02-architecture.md` §ルート package.json](./02-architecture.md#ルート-packagejson)）
- [ ] `pnpm gen:params` を実行し、`crates/dsp-core/src/params.rs` と `web/src/lib/audio/generated/params.ts` が生成されることを確認
- **検証**: 生成された `params.rs` が `cargo check -p dsp-core` を通る、生成された `params.ts` が `pnpm --filter web check`（svelte-check）を通る

#### Step 3. `scripts/check-params-sync.mjs` を実装（F14 / F15）

- [ ] `scripts/check-params-sync.mjs` を [`06-build-and-verify.md` §scripts/check-params-sync.mjs](./06-build-and-verify.md#paramdescriptor-同期チェック) に従い実装
- [ ] `pnpm check:params-sync` を `package.json` に追加
- [ ] `pnpm check` の最後に `pnpm check:params-sync` をチェーン
- [ ] `params.json` を意図的に編集せず check-params-sync を実行 → exit 0
- [ ] `params.json` の Damping default を 0.997 に書き換えて check-params-sync 実行 → exit 1（F15 達成）
- [ ] 検証後、`params.json` を元に戻す
- **検証**: F14 / F15 達成

#### Step 4. 既存 `params.rs` / `messages.ts` を生成物に置換

- [ ] 既存の手書き `crates/dsp-core/src/params.rs` を `pnpm gen:params` で生成された内容に置換（git diff を確認、Phase 1 と互換性のある定数 / enum / from_u32 が出力されていること）
- [ ] `web/src/lib/audio/messages.ts` の `PARAM_IDS` 手書き定義を削除し、`generated/params.ts` から re-export する形に変更（[`05-web-frontend-spec.md` §messages.ts 変更点](./05-web-frontend-spec.md#messagesets-変更点)）
- [ ] `pnpm build:wasm` の前段に `pnpm gen:params` をチェーン（[`02-architecture.md` §ルート package.json](./02-architecture.md#ルート-packagejson)）
- [ ] `pnpm dev` を再起動し、Phase 1 と同じ動作（damping/brightness/output_gain スライダーが効く）が確認できる
- [ ] `cargo test -p dsp-core` の Phase 1 既存テスト（特に `test_paramid_roundtrip`）がパス
- **検証**: F14 達成、Phase 1 全機能が破綻なく動作する

### フェーズ β — Fractional delay（3 ステップ）

#### Step 5. `fractional_delay.rs` を実装

- [ ] `crates/dsp-core/src/fractional_delay.rs` を [`03-dsp-core-spec.md` §Fractional delay](./03-dsp-core-spec.md#fractional-delay) に従い作成
- [ ] `LagrangeCoeffs` 構造体、`new(d: f32)`、`apply(...)` メソッドを実装
- [ ] `crates/dsp-core/src/lib.rs` に `pub mod fractional_delay;` を追加
- [ ] ユニットテスト（`test_lagrange_d_zero_gives_x_zero`、`test_lagrange_d_one_gives_x_plus_1`、`test_lagrange_coeffs_sum_to_one`）を追加し全パス
- **検証**: `cargo test -p dsp-core fractional_delay` がパス

#### Step 6. `KarplusStrong` に fractional delay 統合

- [ ] [`03-dsp-core-spec.md` §KarplusStrong（Phase 2 改修）](./03-dsp-core-spec.md#karplusstrongphase-2-改修) に従い構造体フィールドを更新（`length` → `length_int`、`lagrange: LagrangeCoeffs` 追加、`current_note: Option<u8>`、`age_samples: u32`）
- [ ] `prepare` のバッファサイズを `(sample_rate / 27.5).ceil() + LAGRANGE_BUFFER_MARGIN`（D27、`LAGRANGE_BUFFER_MARGIN = 3`）で確保
- [ ] `note_on` で `length_int` と `length_frac` を計算し、`length_int` の上限を `buffer.len() - LAGRANGE_BUFFER_MARGIN` で clamp（Lagrange 4 点が時系列順に取れる範囲）、`LagrangeCoeffs::new(len_frac)` でキャッシュ（D26）
- [ ] `note_on` 冒頭で `buffer.iter_mut()` ループでバッファ全体をゼロクリアしてから `buffer[0..length_int]` に励振、**`write_index = length_int` から開始**（write_index = 0 + buffer[0..length_int] 励振だと初回 read 位置がゼロ領域になり励振サンプルがゼロ上書きされて無音になる、High 修正）
- [ ] `cargo test test_note_on_first_block_nonzero` を追加し、`note_on` 直後 1 ブロックの出力絶対値最大が `velocity * 1e-3` 以上であることを assert（励振配置 + write_index 初期値の組み合わせが機能していることを確認）
- [ ] `process_sample` 内で `lagrange.apply(...)` を呼び、**補間値をフィードバックループ内（LPF 入力 → damping → buffer 書き戻し経路）と出力の両方に使う**（補間値を出力経路だけに入れると KS の周期はループ内整数ディレイで決まるためピッチ精度が改善しない）
- [ ] `process_sample` の 4 点 read 添字は **`buffer.len()` で剰余を取る**（`length_int` で剰余を取ってはいけない）。実装は `let base = self.write_index + buf_len; let read_m = (base - d_int + 1) % buf_len; ...` の形（[`03-dsp-core-spec.md` §process_sample の変更](./03-dsp-core-spec.md#process_sample-の変更)）
- [ ] `write_index` の進行も `(self.write_index + 1) % buf_len`（`length_int` ではなく `buf_len` で巻く）
- [ ] `note_on_with_id`、`set_seed` を追加実装
- [ ] `voice.rs` に Voice trait の追加メソッド (`note_id` / `age` / `amplitude`) 委譲を追記（[`03-dsp-core-spec.md` §Voice trait の追加メソッド実装](./03-dsp-core-spec.md#voice-trait-の追加メソッド実装)）
- **検証**: `cargo test -p dsp-core` の Phase 1 既存テストすべてパス（`test_no_allocation_in_process` 含む、KarplusStrong 単体動作で alloc が増えていないこと）

#### Step 7. ピッチ精度ユニットテスト追加（F12 / F13、A1〜C8 全音域網羅）

- [ ] `crates/dsp-core/tests/pitch_accuracy.rs` を [`06-build-and-verify.md` §F12/F13 詳細手順](./06-build-and-verify.md#f12--f13ピッチ精度の詳細手順) の autocorrelation 法で実装
- [ ] 共通ヘルパ `measure_f0(midi, sample_rate)` を実装（励振直後 0.1 秒スキップ → autocorrelation で τ_peak を見つける → **parabolic interpolation で sub-sample 精度の τ_refined を計算** → f0 = sample_rate / τ_refined）。τ 探索範囲は midi 値から期待周期 ± 5% に絞る（C8 の周期 11.47 サンプルでも parabolic で sub-sample 精度を取れるようにする）
- [ ] 5 件のテストすべてが ± 0.5% 以内でパス:
  - [ ] `test_pitch_a1` (midi=33, 55Hz、Phase 1 課題解消の主検証)
  - [ ] `test_pitch_a2` (midi=45, 110Hz)
  - [ ] `test_pitch_a4` (midi=69, 440Hz)
  - [ ] `test_pitch_c6` (midi=84, 1046.5Hz、中高域)
  - [ ] `test_pitch_c8` (midi=108, 4186Hz、高域、autocorrelation の τ 探索範囲を絞る)
- [ ] 長時間安定性テスト `test_long_term_stability_high_damping` を追加（damping=0.9999 + brightness × 3 + midi × 3 の 9 組合せで 30 秒分を生成、finite / peak ≤ 10.0 / 末尾 1 秒平均 ≤ 100.0 を確認）
- **検証**: F12 / F13 達成 + 長時間安定性確認

### フェーズ γ — VoicePool（4 ステップ）

#### Step 8. `voice_pool.rs` を実装

- [ ] `crates/dsp-core/src/voice_pool.rs` を [`03-dsp-core-spec.md` §VoicePool](./03-dsp-core-spec.md#voicepool) に従い作成
- [ ] `pub const POLYPHONY: usize = 8;`、`VoicePool<const N: usize>` 構造体定義
- [ ] `new` / `prepare` / `note_on` / `note_off` / `set_damping` / `set_brightness` / `reset` / `process_sample` / `active_count` を実装
- [ ] `note_allocator.rs` の `select_voice_for_steal` を仮 stub（Step 9 で本実装）
- [ ] `crates/dsp-core/src/lib.rs` に `pub mod voice_pool;` を追加
- **検証**: `cargo check -p dsp-core` がパス、`cargo build -p dsp-core` がパス

#### Step 9. `note_allocator.rs` で voice stealing 戦略を実装

- [ ] `crates/dsp-core/src/note_allocator.rs` を [`03-dsp-core-spec.md` §Note allocator](./03-dsp-core-spec.md#note-allocatorvoice-stealing-戦略) に従い作成
- [ ] `ENERGY_THRESHOLD_FOR_STEAL: f32 = 1.0e-3` 定数、`StealResult` enum、`select_voice_for_steal` 関数を実装
- [ ] energy 閾値以下のボイス優先 + フォールバックで oldest（D13 / D28）
- [ ] ユニットテスト（`test_steal_picks_quietest_voice`、`test_steal_falls_back_to_oldest`、`test_steal_among_quiet_voices_picks_oldest`）を追加し全パス
- [ ] `crates/dsp-core/src/lib.rs` に `pub mod note_allocator;` を追加
- **検証**: `cargo test -p dsp-core note_allocator` がパス

#### Step 10. `Engine` を VoicePool ベースに書き換え

- [ ] [`03-dsp-core-spec.md` §Engine（Phase 2 改修）](./03-dsp-core-spec.md#engineenginers-の-phase-2-版) に従い `engine.rs` を書き換え
- [ ] `pool: VoicePool<POLYPHONY>` フィールド、`hold_stack: HoldStack`（Step 12 で実装、ここではコメントアウトまたは仮実装）、`mode: SynthMode`、`current_damping: f32` を保持
- [ ] `note_on` / `note_off` / `set_param` / `set_mode` / `mode` / `current_damping` メソッドを実装
- [ ] `AudioProcessor` impl の `prepare` / `process` / `reset` を VoicePool 経由で実装
- [ ] Phase 1 既存テスト（`test_damping_preserved_across_note_on`、`test_last_note_priority_simple`）をポリモードで再パス
- **検証**: `cargo test -p dsp-core engine` がパス、`cargo test -p dsp-core` 全体が Phase 1 + Phase 2 既存テストすべてパス

#### Step 11. VoicePool ユニットテスト追加（F10 / F11 / F17 / F24）

- [ ] [`03-dsp-core-spec.md` §テスト方針](./03-dsp-core-spec.md#phase-2-で追加するテスト13-件) の以下を実装:
  - [ ] `test_voice_pool_allocates_distinct_voices`（F10）
  - [ ] `test_voice_pool_same_note_replace`（F10）
  - [ ] `test_voice_pool_note_on_returns_assigned_index`（F10、High 2 修正検証）
  - [ ] `test_engine_note_on_does_not_revive_released_voice`（F10/F11、High 2 修正の主検証: release 中ボイスの damping が新規 note_on で復活しないこと）
  - [ ] `test_voice_pool_steals_quietest`（F11 / F23）
  - [ ] `test_voice_pool_polyphonic_mix_rms_bounded`（F24 補助、8 ボイス全力時に RMS <= 0.7 / peak <= 2.0 の統計的境界確認、実機判定は F24(a) に委ねる）
  - [ ] `test_no_allocation_in_polyphonic_process`（F17、Phase 1 `test_no_allocation_in_process` のポリフォニー版）
- **検証**: F10 / F11 / F17 / F24 達成（cargo test レベル）

### フェーズ δ — Hold note stack（3 ステップ）

#### Step 12. `hold_stack.rs` を実装

- [ ] `crates/dsp-core/src/hold_stack.rs` を [`03-dsp-core-spec.md` §Hold note stack](./03-dsp-core-spec.md#hold-note-stack) に従い作成
- [ ] `pub const MAX_HELD: usize = 16;`、`LinearStack<T, const N>` 構造体
- [ ] `new` / `push` / `remove` / `top` / `clear` / `len` / `is_empty` を実装
- [ ] `pub type HoldStack = LinearStack<u8, MAX_HELD>;` を定義
- [ ] `crates/dsp-core/src/lib.rs` に `pub mod hold_stack;` を追加
- [ ] ユニットテスト（`test_hold_stack_push_pop_basic`、`test_hold_stack_overflow_drops_oldest`、`test_hold_stack_remove_middle`、`test_hold_stack_clear`）を追加し全パス
- **検証**: F19 達成（cargo test レベル）

#### Step 13. `Engine` に mode 切替と hold stack 連携を追加

- [ ] [`03-dsp-core-spec.md` §Engine（Phase 2 改修）](./03-dsp-core-spec.md#engineenginers-の-phase-2-版) の `note_on` / `note_off` で `match self.mode` 分岐を完全実装
- [ ] mono モード時: hold_stack.push / remove、top で復帰、空なら note_off 発火
- [ ] poly モード時: 直接 VoicePool に発火
- [ ] `set_mode` で hold_stack.clear()
- **検証**: cargo test の `test_synth_mode_switch_no_break` がパス（F20、cargo test レベル）

#### Step 14. hold stack ユニットテスト追加（F18 / F20）

- [ ] [`03-dsp-core-spec.md` §テスト方針](./03-dsp-core-spec.md#phase-2-で追加するテスト11-件) の以下を実装:
  - [ ] `test_hold_stack_last_note_priority`（F18、C 押→D 押→D 離→C 復帰→C 離→無音 のシーケンス）
  - [ ] `test_hold_stack_overflow_behavior`（F19、cargo test レベルで Step 12 既存だが Engine 統合版でも確認）
  - [ ] `test_synth_mode_switch_no_break`（F20、Step 13 で追加済み）
- **検証**: F18 / F19 / F20 達成（cargo test レベル）

### フェーズ ε — WASM C ABI 拡張（2 ステップ）

#### Step 15. `synth_set_polyphony_mode` を追加

- [ ] [`04-wasm-audio-spec.md` §Phase 2 で追加する C ABI 関数](./04-wasm-audio-spec.md#phase-2-で追加する-c-abi-関数) に従い `crates/wasm-audio/src/lib.rs` に `synth_set_polyphony_mode` 関数を追加
- [ ] `cargo build -p wasm-audio --target wasm32-unknown-unknown` がパス
- [ ] `wasm-objdump -x web/src/lib/wasm/wasm_audio.wasm | findstr Export`（または `wasm-tools dump`）で `synth_set_polyphony_mode` が export されていることを確認
- **検証**: WASM ビルド成功 + export 確認

#### Step 16. `check-wasm-exports.mjs` の REQUIRED 配列更新（F14）

- [ ] `scripts/check-wasm-exports.mjs` の `REQUIRED` 配列に `'synth_set_polyphony_mode'` を追加（[`06-build-and-verify.md` §export 名の自動検証](./06-build-and-verify.md#export-名の自動検証スクリプトphase-2-版)）
- [ ] `pnpm build:wasm` が `All required WASM exports present.` を出力
- [ ] 試しに `synth_set_polyphony_mode` の `#[unsafe(no_mangle)]` を一時的に外して `pnpm build:wasm` 実行 → exit 1 で `Missing WASM exports: ['synth_set_polyphony_mode']` が表示されることを確認 → 戻す
- **検証**: F14 達成

### フェーズ ζ — Web フロントエンド（3 ステップ）

#### Step 17. ParamSlider を ParamDescriptor 駆動に改修

- [ ] [`05-web-frontend-spec.md` §ParamSlider の descriptor 駆動化](./05-web-frontend-spec.md#paramslider-の-descriptor-駆動化) に従い `web/src/lib/components/ParamSlider.svelte` を改修
- [ ] `min` / `max` props を削除、`getDescriptor(paramId)` から取得
- [ ] `web/src/routes/+page.svelte` の `<ParamSlider>` 呼び出しから `min` / `max` 属性を削除（step は残す）
- [ ] `web/src/lib/state/synth.svelte.ts` の初期値を `PARAM_DESCRIPTORS[i].default` から取得
- [ ] `pnpm dev` で 3 スライダーが Phase 1 と同じ範囲で動作することを確認
- **検証**: スライダー UI が Phase 1 と同じ挙動を保つ

#### Step 18. Worklet messages 型 / WasmExports interface 更新

- [ ] [`05-web-frontend-spec.md` §messages.ts の変更点](./05-web-frontend-spec.md#messagesets-変更点) に従い `web/src/lib/audio/messages.ts` を更新
  - [ ] `PARAM_IDS` 手書き削除、`generated/params.ts` から re-export
  - [ ] `ToWorkletMessage` に `setMode` variant を追加
- [ ] [`05-web-frontend-spec.md` §WasmExports interface の拡張](./05-web-frontend-spec.md#wasmexports-interface-の拡張) に従い `web/src/lib/audio/synth-processor.ts` の `WasmExports` interface に `synth_set_polyphony_mode` を追加
- [ ] `synth-processor.ts` の `onMessage` に `setMode` ケースを追加
- [ ] `pnpm --filter web check`（svelte-check）がパス
- **検証**: TypeScript 型エラーなし、`pnpm dev` で Phase 2 全機能（poly モード）が動作

#### Step 19. SynthEngine に setMode メソッド + dev 用診断 API 追加

- [ ] [`05-web-frontend-spec.md` §Phase 2 で追加するメソッド](./05-web-frontend-spec.md#phase-2-で追加するメソッドd21--d22--検証用-dev-only-api) に従い `web/src/lib/audio/engine.ts` に `setMode(mode: 'poly' | 'mono')` メソッドを追加（本番含むビルド対象）
- [ ] `web/src/lib/state/synth.svelte.ts` の末尾に **`if (import.meta.env.DEV) { (window as ...).__synthDev = { setMode: ... }; }`** を追加（dev ビルドのみ、本番ビルドでは tree-shake で除去）。Phase 2 では `setMode` のみ。`getActiveVoiceCount` 系は提供せず、active voice 数の確認は cargo test と聴感に委ねる
- [ ] `pnpm dev` 起動後、DevTools Console で `__synthDev.setMode('mono')` が動作することを確認
- [ ] `pnpm build` 後、`web/build/_app/immutable/chunks/*.js` を grep して `__synthDev` 文字列が含まれないことを確認（tree-shake 検証）
- **検証**: dev で console から setMode 呼出可、本番では `__synthDev` が完全除去される

### フェーズ η — 検証（3 ステップ）

#### Step 20. F10〜F18 の網羅検証

- [ ] [`06-build-and-verify.md` §検証チェックリスト](./06-build-and-verify.md#検証チェックリストphase-2-追加分-f10f25) に従い以下を実機検証:
  - [ ] F10: 8 音同時発音でクリップなし（PC キーボード A,S,D,F,G,H,J,K 同時押下）
  - [ ] F11: 9 音目で voice stealing 発生、耳障りなクリックなし
  - [ ] F12: A4 ピッチ精度 ± 0.5%（cargo test + 実機 FFT）
  - [ ] F13: A1 ピッチ精度 ± 0.5%（cargo test + 実機 FFT）
  - [ ] F14: ParamDescriptor 同期（`pnpm check:params-sync`）
  - [ ] F15: gen 忘れ検知（`params.json` 改変 → check-params-sync exit 1）
  - [ ] F17: ポリフォニー時メモリ確保ゼロ（`synth-processor.ts` に F8 と同じ一時挿入コード、synth_new 直後の baseline と比較）
  - [ ] F18: hold stack last-note 復帰（dev console で `__synthDev.setMode('mono')` → C 押→D 押→D 離→C 復帰）
- [ ] F17 検証完了後、`synth-processor.ts` の一時挿入コードを削除（`__synthDev` は dev-only ガード内なので削除不要）
- **検証**: F10〜F15、F17、F18 すべて達成

#### Step 21. 性能計測（F16 / F21 / F22 / F23 / F24）

- [ ] F16: ポリフォニー 8 音時の process 時間 < 1.5ms（Chrome DevTools Performance タブ）
- [ ] F21: WASM gzip サイズ < 30 KB（git bash で `gzip -kc` 計測）
- [ ] F22: Worklet 本番バンドル < 10 KB（`Get-ChildItem` で計測）
- [ ] F23: voice stealing 連打でクリックなし（PC キーボード 9 鍵以降を高速連打）
- [ ] F24 (a): 常用範囲（OutputGain ≤ 1.0、通常の押下パターン）で 30 秒継続して音割れ歪みが知覚されない
- [ ] F24 (b) 補助確認: OutputGain=1.5 + 8 鍵全力強打の最悪ケースで歪みの程度を聴感記録（許容範囲、Phase 3 で limiter 検討）
- [ ] サイズが超過した場合 `wasm-opt -O3` 適用（リスク R19）→ 再計測
- **検証**: F16 / F21〜F24 すべて達成（F24 は (a) を主、(b) は記録のみ）

#### Step 22. 本番ビルド + iOS Safari 動作確認 + retrospective §2 確認（F25）

- [ ] `pnpm build` で本番ビルドを生成
- [ ] `pnpm --filter web preview` で http://localhost:4173 を開き、F10〜F24 の主要項目（特に F10、F18）が再現することを確認
- [ ] GitHub Pages デプロイ（`main` push）後、iPhone Safari で HTTPS URL にアクセス → F1〜F4 と F10 が再現（Phase 2 機能の iOS 動作確認）
- [ ] `docs/retrospective/2026-05-06-001-mvp.md` §2 で Phase 1 F1〜F9 のすべてが「✅ 達成」になっているか最終確認（F25）
- [ ] `README.md` を Phase 2 用に更新（ポリフォニー対応を明記、F10〜F25 の自己検証手順を追記）
- **検証**: 本番ビルド + iOS で動作、F25 達成、Phase 2 完成

## ステップごとの依存関係

```
Step 1 (params.json)
  └─ Step 2 (gen-params.mjs) ─ Step 3 (check-params-sync) ─ Step 4 (既存置換)
                                                                    │
       ┌────────────────────────────────────────────────────────────┤
       ▼                                                             │
   Step 5 (fractional_delay.rs) ─ Step 6 (KS統合) ─ Step 7 (pitch test, F12/F13)
                                                                     │
       ┌────────────────────────────────────────────────────────────┤
       ▼                                                             │
   Step 8 (voice_pool.rs) ─ Step 9 (note_allocator) ─ Step 10 (Engine 書換) ─ Step 11 (F10/F11/F17/F24 test)
                                                                     │
       ┌────────────────────────────────────────────────────────────┤
       ▼                                                             │
   Step 12 (hold_stack.rs, F19 test) ─ Step 13 (Engine mode 連携) ─ Step 14 (F18/F20 test)
                                                                     │
       ┌────────────────────────────────────────────────────────────┤
       ▼                                                             │
   Step 15 (synth_set_polyphony_mode) ─ Step 16 (REQUIRED 配列, F14)
                                                                     │
       ┌────────────────────────────────────────────────────────────┤
       ▼                                                             │
   Step 17 (ParamSlider 改修) ─ Step 18 (messages.ts/WasmExports) ─ Step 19 (SynthEngine.setMode)
                                                                     │
       ┌────────────────────────────────────────────────────────────┤
       ▼                                                             │
   Step 20 (F10〜F18 検証) ─ Step 21 (F16/F21〜F24 性能) ─ Step 22 (本番 + iOS + F25)
```

並列実装可能なポイント:

- Step 5〜7（β）と Step 8〜11（γ）は `KarplusStrong` の改修で衝突するため **直列が望ましい**
- Step 12〜14（δ）は γ 完了後に着手（Engine の修正範囲が重なるため）
- Step 17〜19（ζ）は Step 16 完了後に着手（WasmExports 拡張に依存）

## 達成ライン早見表

| ステップ完了 | 達成する検証項目 |
|---|---|
| Step 3 | F14（cargo test レベル）、F15 |
| Step 4 | F14（実装レベル） |
| Step 7 | F12 / F13（cargo test レベル） |
| Step 11 | F10 / F11 / F17（cargo test レベル）、F24（cargo test レベル） |
| Step 14 | F18 / F19 / F20（cargo test レベル） |
| Step 16 | F14（WASM export レベル） |
| Step 20 | F10〜F15、F17、F18（実機検証含む） |
| Step 21 | F16 / F21〜F24（性能・実機） |
| Step 22 | F25 + iOS Safari 動作確認 |

すべての F10〜F25 が達成された時点で Phase 2 完成。

## 実装着手者へのメモ

- **Step 1〜4（α）が最重要**。ここで `params.json` を単一ソース化することで、Phase 2 で増えるパラメータの drift リスクを Phase 2 全体で防げる。Step 4 で Phase 1 の手書き `params.rs` / `messages.ts` を生成物に置換した時点で、Phase 1 既存テスト 11 件がパスすることを必ず確認
- **Step 6 が β の山場**。`KarplusStrong::process_sample` で Lagrange 補間値を **フィードバックループ内（LPF 入力 → damping → buffer 書き戻し）と出力の両方** に使う設計（[`03-dsp-core-spec.md` §process_sample の変更](./03-dsp-core-spec.md#process_sample-の変更)）。出力経路だけに入れるとループ内が整数ディレイのままになり F12/F13 が達成できない。Lagrange 3 次補間自体は FIR で係数和 1.0（DC ゲイン保存）、ループ全体の安定性は damping < 1.0 と LPF（低域通過）の組み合わせに依存し、Phase 1 の動作実績から **安定性リスクは低い**（Thiran allpass IIR の場合は別途の極配置議論が必要だが Phase 2 採用案 D14 では非該当）。長時間動作は `test_long_term_stability_high_damping` で確認
- **Step 8〜11（γ）が Phase 2 全体の中心**。`VoicePool<const N>` の const generic 配列初期化に `core::array::from_fn` を使う（`[KarplusStrong::new(); N]` は Copy トレイトがないため不可）
- **Step 9 の voice stealing**: `select_voice_for_steal` の戻り値が必ず有効な範囲内（0 ≤ i < N）であること。N=8 で固定なので panic リスクはないが、`debug_assert!(i < N)` を仕込んでおく
- **Step 13 の hold_stack**: mono モードでの note_off → top 復帰時の velocity をどうするかが微妙。Phase 2 では「デフォルト velocity=0.8」で実装し、実機検証で違和感あれば「note_off されたキーの velocity を保持する」に拡張（Phase 3 候補）
- **Step 17 の ParamSlider 改修**: `descriptor` を `const` で取得しているため `paramId` が変わると追従しない。Phase 2 では各 ParamSlider インスタンスで `paramId` 固定なので問題なし。動的に変えたいなら `$derived(getDescriptor(paramId))` 化
- **Step 19 の dev 用診断 API**: `if (import.meta.env.DEV)` ガード内で公開するため本番に漏れない。検証完了後の削除は不要（ガード内に残しておけば次回の検証時にも使える）。本番バンドルから tree-shake で完全除去されることは Step 19 末尾の grep 検証で確認
- **Step 21 の wasm-opt 適用**: WASM サイズ目標（gzip < 30 KB）を超えた場合のフォールバック。`winget install WebAssembly.Binaryen` 後、`wasm-opt -O3 -o web/src/lib/wasm/wasm_audio.wasm web/src/lib/wasm/wasm_audio.wasm` を `scripts/copy-wasm.mjs` の最後に追加（オプション化）
- **Step 22 の iOS Safari 確認**: Phase 1 と同じく HTTPS 必須かつ実機推奨。GitHub Pages デプロイ後、iPhone Safari で <https://riku-kano.github.io/physics-base-synth/> を開いて「Start Audio → 8 鍵同時押下で 8 音発音」を確認
- **各ステップで コミットを分ける** ことを推奨（Phase 1 と同じ）。問題発生時に二分探索しやすい
- **Phase 1 既存テストが各ステップで壊れないこと**を最優先。Step 6 / Step 10 で KarplusStrong / Engine を大きく書き換えるため、各 Step 完了時に `cargo test -p dsp-core` を回す
- 詰まったら [`06-build-and-verify.md` §トラブルシューティング](./06-build-and-verify.md#トラブルシューティング-tips) を参照
