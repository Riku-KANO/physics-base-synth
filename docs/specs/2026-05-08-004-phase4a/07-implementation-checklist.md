# 07. Phase 4a 実装順序チェックリスト

## 目的

Phase 4a 仕様書承認後、本仕様を実装する際の作業順序を Phase α〜η の 7 フェーズ・全 17 ステップで定義する。各ステップは独立して進捗確認でき、検証チェックリスト（F38b + F39〜F47）の充足ポイントを明示する。Phase 1 / 2 / 3 の `07-implementation-checklist.md` と同じ粒度（1 ステップ ≈ 1 コミット）で構成する。

## 他文書との関係

- 上流: 全ての仕様書（pre-research、01〜06）
- 参考: Phase 1 / 2 / 3 [07 章] — **Phase 4a ステップは Phase 3 の構成パターンを踏襲**
- このドキュメントは **実装専用のチェックリスト** であり、各ステップの設計詳細は対応する仕様書を参照する

## 前提条件

Phase 4a 実装着手前に以下を確認:

1. **Phase 3 の 94 PASS + 1 IGNORED テストがすべて維持されている**
   - `cargo test -p dsp-core` で確認
2. **`pnpm dev` でブラウザでの 8 音ポリフォニック + Modal Body + MIDI CC + Voice Meter が動作確認できる状態**
3. **F1〜F25 / F34 の実機検証は持ち越し継続**（ある程度動作確認済み、ユーザー承認済み）
4. **Phase 4 を 4a / 4b に分割するユーザー承認が完了**（2026-05-08 時点で確定）

## 実装ステップ（全 17 段階、Phase α〜η の 7 フェーズ）

### フェーズ α — F38b 計測 + 既存負債解消（3 ステップ）

#### Step 1. F38b 実機計測 + retrospective §5 追記（D44）

- [ ] `pnpm build && pnpm preview` で本番ビルド + 4173 ポート起動
- [ ] Chrome 最新版で `http://localhost:4173/physics-base-synth/` を開く
- [ ] F12 → Performance タブ → ⚙ 歯車 → CPU: "No throttling"
- [ ] ⏺ Record 開始 → 8 voice 同時押下 + Pitch Bend + CC#7 + Sustain Pedal を 10 秒間
- [ ] ⏹ Record 停止、"Audio Worklet" レーンの各 task の self time を集計
- [ ] **avg / max を計測値として記録**
- [ ] `docs/retrospective/2026-05-07-003-phase3.md` §5 の `F38b 実機計測` 項目を更新（計測日時 / avg / max / 達成判定）
- [ ] **判断**:
  - avg < 1.5 ms かつ max < 2.5 ms → ✅ Phase 4a 本実装へ進む
  - avg ≥ 2.0 ms または max ≥ 3.0 ms → R30 対策を Phase 4a §1 として組み込み（`VOICE_STATE_STRIDE_FRAMES` を 1024 → 4096 等）
- [ ] git commit `chore(retrospective): F38b 実機計測結果を追記 (D44)`
- **検証**: F38b 達成。Phase 3 完成判定の最終案件が閉じる

#### Step 2. `wasm-opt -O3` を `scripts/copy-wasm.mjs` に統合（D45）

- [ ] `package.json` の `devDependencies` に `binaryen` を追加（`pnpm add -D binaryen`）
- [ ] `scripts/copy-wasm.mjs` を [`04-wasm-audio-spec.md` §wasm-opt 統合の詳細](./04-wasm-audio-spec.md#scriptscopy-wasmmjs-の-wasm-opt-統合d45) に従い拡張:
  - [ ] `resolveWasmOpt()` ヘルパで `node_modules/.bin/wasm-opt` (Linux/Mac) / `wasm-opt.cmd` (Windows) を探索
  - [ ] **既存の `profile = process.argv[2] === 'release' ? 'release' : 'debug'` 規約を維持**し、`profile === 'release'` の条件で wasm-opt を実行（`package.json` の script 引数渡しは不変）
  - [ ] `execFileSync(wasmOptBin, ['-O3', '--strip-debug', srcPath, '-o', dstPath])` で適用
  - [ ] beforeSize / afterSize をログ出力
  - [ ] wasm-opt 不在時は warn + 素コピーで続行
- [ ] `pnpm build:wasm` を実行、wasm-opt 適用ログが出力されることを確認（profile=release）
- [ ] `pnpm build:wasm:dev` を実行、wasm-opt スキップで素コピーされることを確認（profile=debug）
- [ ] gzip サイズ計測:
  ```bash
  pnpm build
  gzip -kc web/build/_app/immutable/assets/wasm_audio.*.wasm | wc -c
  ```
  - [ ] **目標値**: < 15 KB（想定 ~13 KB、Phase 3 27.78 KB から大幅削減）
  - [ ] **警戒**: < 18 KB（超えたら `wasm-opt --print-stats` で調査）
  - [ ] **撤退**: < 30 KB（超えたら R32 適用 = 楽器 4 種に削減 / Modal M=5）
- [ ] gzip サイズ超過時は R32 対策（pre-research §9.2 早期撤退ライン）
- [ ] git commit `build: wasm-opt -O3 を copy-wasm.mjs に統合 (D45, F39)`
- **検証**: F39 達成

#### Step 3. `excitation_snapshot` を `#[cfg(test)]` でガード + 該当 integration test を unit test へ移動（D45 既存負債解消）

`excitation_snapshot` は現状 7 箇所の integration test (`crates/dsp-core/tests/karplus_strong_pick_tests.rs`) から呼ばれているため、`#[cfg(test)]` ガードだけだと integration test ビルド時に未定義となる。本 Step では (a) `excitation_snapshot` を `#[cfg(test)]` ガードに変更し、(b) 該当 integration test を `karplus_strong.rs` 内の `#[cfg(test)] mod tests` ブロックに移動する。

- [ ] `crates/dsp-core/src/karplus_strong.rs` の `excitation_snapshot` 定義を確認:
  ```rust
  #[doc(hidden)]
  pub fn excitation_snapshot(&self) -> Vec<f32> { ... }
  ```
- [ ] `crates/dsp-core/tests/karplus_strong_pick_tests.rs` の `excitation_snapshot` を呼ぶ test 関数 7 箇所を抽出（Phase 3 既存）:
  - `test_pick_min_beta_minimal_shape`
  - `test_pick_position_node_at_beta_half`
  - `test_pick_position_attenuates_kth_harmonic` の 2 ループ
  - `test_pick_internal_k_zero_branch`
  - その他 `excitation_snapshot` 参照箇所
- [ ] 上記 test 関数を `crates/dsp-core/src/karplus_strong.rs` 末尾の `#[cfg(test)] mod excitation_tests { ... }` ブロックに移動。`use super::*;` を冒頭に置き、private state にアクセスできる利点を活かして `excitation_snapshot` 経由を unit 直接 access に置換しても良い
- [ ] `crates/dsp-core/tests/karplus_strong_pick_tests.rs` の対応 test 関数を削除（unit test へ移動済み）
- [ ] `excitation_snapshot` を `#[cfg(test)]` ガードに変更:
  ```rust
  #[cfg(test)]
  pub(crate) fn excitation_snapshot(&self) -> Vec<f32> { ... }
  ```
- [ ] `cargo test -p dsp-core` で全テスト通過を確認（移動した test も同じ件数）
- [ ] `cargo build --target wasm32-unknown-unknown --release` でビルド成功
- [ ] WASM gzip サイズが Step 2 から微減することを確認（~50 byte の関数除外）
- [ ] git commit `refactor(karplus-strong): excitation_snapshot を cfg(test) でガード + tests を unit test へ移動 (D45, F44)`
- **検証**: F44 達成、Phase 3 既存テスト件数の保持

### フェーズ β — params.json 拡張と多楽器係数（2 ステップ）

#### Step 4. `params.json` 拡張 + `gen-params.mjs` 拡張（D52 / D54）

- [ ] `params.json` に `instruments` セクションを追加:
  ```json
  {
    "instruments": [
      {
        "kind": "default",
        "stereo_spread": 0.05,
        "body_modes": [ /* Phase 3 既存 8 mode = BODY_MODES_L 値 */ ]
      },
      {
        "kind": "guitar_classical",
        "stereo_spread": 0.05,
        "body_modes": [ /* pre-research §7.2 の Guitar Classical 8 mode */ ]
      },
      // ... ukulele / mandolin / bass / guitar_steel / sitar も同形式（pre-research §7.2 の値）
    ]
  }
  ```
- [ ] `scripts/gen-params.mjs` を拡張:
  - [ ] `instruments` 配列を読んで、各楽器の L / R 係数（`applyStereoSpread(modes, spread)` で生成）を const として出力
  - [ ] `BODY_MODES_<INSTRUMENT>_L` / `BODY_MODES_<INSTRUMENT>_R` 12 配列を Rust / TS 両方に出力
  - [ ] `STEREO_SPREAD_<INSTRUMENT>` 6 値を出力
  - [ ] `InstrumentKind` enum を Rust / TS 両方に出力
  - [ ] `body_modes_for_instrument(kind)` / `stereo_spread_for_instrument(kind)` ヘルパ関数を Rust 側に出力
  - [ ] **Phase 3 互換のため `BODY_MODES_L` / `BODY_MODES_R` / `STEREO_SPREAD` (グローバル const) も維持**（Default の alias として）
- [ ] `pnpm gen:params` を実行、生成ファイルに 12 配列 + 6 stereo_spread + InstrumentKind enum が含まれることを確認
- [ ] `pnpm check:params-sync` がパス
- [ ] `cargo check -p dsp-core` がパス、`pnpm --filter ./web check` がパス
- [ ] git commit `feat(params): 多楽器プリセット 6 種の Modal 係数を生成パイプラインに追加 (D52, D54)`
- **検証**: 生成物 drift なし、PARAM_DESCRIPTORS は不変（5 件）、`InstrumentKind` enum 出力

#### Step 5. `lfo.rs` 実装 + Engine 統合（D46 / D47）

- [ ] `crates/dsp-core/src/lfo.rs` を [`03-dsp-core-spec.md` §Lfo](./03-dsp-core-spec.md#lfo-lfors--phase-4a-新規d46--d47) に従い作成
- [ ] `Lfo` 構造体、`LfoWaveform` enum (Sine=0, Triangle=1)、`LfoDestination` enum (Pitch=0, Brightness=1, Volume=2) を実装
- [ ] `Lfo::new()` / `prepare(sr)` / `reset()` / `set_rate(hz)` / `set_waveform(kind)` / `process_sample() -> f32` を実装
- [ ] `crates/dsp-core/src/lib.rs` に `pub mod lfo;` + `pub use lfo::{Lfo, LfoWaveform, LfoDestination};` を追加
- [ ] `crates/dsp-core/src/engine.rs` に Phase 4a フィールド追加:
  ```rust
  // Engine struct に追加
  lfo: Lfo,
  mod_wheel: SmoothedValue,
  lfo_pitch_depth: SmoothedValue,
  lfo_brightness_depth: SmoothedValue,
  lfo_volume_depth: SmoothedValue,
  current_instrument: InstrumentKind,
  stereo_spread: f32,
  ```
- [ ] `Engine::new` で各フィールドをデフォルト値で初期化
- [ ] `Engine::prepare(sr, mb)` で `lfo.prepare(sr)` + 各 SmoothedValue の `set_time_constant` を呼ぶ
- [ ] `Engine::reset` で `lfo.reset()` + 全 LFO depth / mod_wheel を 0 / Default 楽器に戻す
- [ ] `Engine::lfo_set_rate` / `lfo_set_waveform` / `lfo_set_depth(dest, depth)` の inherent methods を追加
- [ ] **`Engine::process` の per-sample loop は Step 7 で更新**（Step 5 では setter のみ実装、process 内で LFO 値はまだ使わない、Phase 3 既存挙動維持）
- [ ] テスト追加 (`crates/dsp-core/tests/lfo_tests.rs` 新規):
  - [ ] `test_lfo_sine_range` / `test_lfo_triangle_range` / `test_lfo_zero_at_init`
  - [ ] `test_lfo_period_matches_rate` / `test_lfo_rate_smoothing`
  - [ ] `test_lfo_waveform_switch_no_click` / `test_lfo_no_alloc_in_process` / `test_lfo_phase_wraps`
- [ ] `cargo test -p dsp-core` がすべてパス
- [ ] git commit `feat(dsp-core): Lfo 型実装 + Engine フィールド追加 (D46, D47, F40-a/b/g)`
- **検証**: F40-a / F40-b / F40-g (LFO 単体テスト) 達成、Engine 統合は Step 7 で完成

### フェーズ γ — Mod Wheel + LFO destinations（2 ステップ）

#### Step 6. Mod Wheel CC#1 分岐有効化 + WebMIDI / UI 同期（D49 / F41）

- [ ] `crates/dsp-core/src/engine.rs` の `Engine::handle_midi_cc` の `CC_MOD_WHEEL` 分岐を実装:
  ```rust
  CC_MOD_WHEEL => {
      self.mod_wheel.set_target(v);
  }
  ```
  Phase 3 では no-op だった経路を有効化
- [ ] **`web/src/lib/input/midi-cc.ts` を拡張**: CC#1 受信時に `synth.modWheel = data[2] / 127` を更新（[`05-web-frontend-spec.md` §midi-cc.ts の Phase 4a 変更点](./05-web-frontend-spec.md#midi-ccts-の-phase-4a-変更点)）
- [ ] テスト追加 (`crates/dsp-core/tests/midi_cc_tests.rs` 拡張):
  - [ ] `test_midi_cc_mod_wheel_sets_target`
  - [ ] `test_midi_cc_mod_wheel_clamps_to_range`
- [ ] `cargo test -p dsp-core` がすべてパス
- [ ] `pnpm --filter ./web check` がパス
- [ ] git commit `feat(engine,web): Mod Wheel (CC#1) 分岐を有効化 + WebMIDI/UI 同期 (D49, F41)`
- **検証**: F41 達成（cargo test + Web 経路）

#### Step 7. LFO destinations を Engine::process に統合（D48）

- [ ] `crates/dsp-core/src/voice_pool.rs` に `set_lfo_pitch_factor(factor)` / `set_lfo_brightness_offset(offset)` を追加（全 voice fan-out）
- [ ] `crates/dsp-core/src/traits.rs` の `Voice` trait に `set_lfo_pitch_factor` / `set_lfo_brightness_offset` を追加
- [ ] `crates/dsp-core/src/voice.rs` で `KarplusStrong` 向け委譲を追記
- [ ] `crates/dsp-core/src/karplus_strong.rs` に `lfo_pitch_factor: f32` (初期値 1.0) / `lfo_brightness_offset: f32` フィールドを追加（`#[inline(always)]` setter、Engine 側で exp2 計算済の factor を受け取る設計）
- [ ] `KarplusStrong::process_sample` で:
  - [ ] LFO pitch offset を `length_target.next_sample()` に係数化して適用（[`03-dsp-core-spec.md` §process_sample 内での適用](./03-dsp-core-spec.md#process_sample-内での適用)）
  - [ ] brightness LPF 計算で `(brightness.next_sample() + lfo_brightness_offset).clamp(0.0, 1.0)` を使用
  - [ ] 既存の length 再計算 skip ロジック（cached_length 差分 < 1e-5）は維持、ただし effective_length 計算後の差分で判定
- [ ] `Engine::process` の per-sample loop を Phase 4a 拡張（[`03-dsp-core-spec.md` §Engine::process の per-sample loop 拡張](./03-dsp-core-spec.md#engineprocess-の-per-sample-loop-拡張d46-d49)）:
  - [ ] LFO process_sample → mod_wheel next_sample → 3 つの depth を計算
  - [ ] **Engine 側で `pitch_factor = exp2(-pitch_offset_semitones / 12)` を 1 回だけ計算**、`pool.set_lfo_pitch_factor(pitch_factor)` / `pool.set_lfo_brightness_offset(brightness_offset)` を per sample 呼出（per voice exp2 を回避）
  - [ ] `volume_multiplier` を `combined` gain に乗算
- [ ] テスト追加 (`tests/lfo_destinations_tests.rs` 新規):
  - [ ] `test_lfo_pitch_destination_modulates_voice_length`
  - [ ] `test_lfo_brightness_destination_modulates_filter`
  - [ ] `test_lfo_volume_destination_modulates_output`
  - [ ] `test_mod_wheel_zero_disables_lfo` (Phase 3 互換確認、最重要)
  - [ ] `test_mod_wheel_one_full_lfo`
  - [ ] `test_lfo_no_alloc_in_engine_process`
- [ ] `cargo test -p dsp-core` がすべてパス
- [ ] git commit `feat(engine): LFO destinations (Pitch/Brightness/Volume) を process に統合 (D48, F40-c/d/e/f)`
- **検証**: F40-c / F40-d / F40-e / F40-f / F40-g 達成。**Phase 3 互換性が `test_mod_wheel_zero_disables_lfo` で保証される（重要）**

### フェーズ δ — 多楽器プリセット係数 + Engine::apply_instrument（2 ステップ）

#### Step 8. ModalBodyResonator::set_instrument 実装（D52）

- [ ] `crates/dsp-core/src/modal_body.rs` に `set_instrument(kind, sample_rate)` メソッドを追加（[`03-dsp-core-spec.md` §ModalBodyResonator の拡張](./03-dsp-core-spec.md#modalbodyresonator-の拡張d52--d53--d54)）
- [ ] `body_modes_for_instrument(kind)` ヘルパ（params.rs 生成済み）を呼んで coeffs_l / coeffs_r を差し替え、reset
- [ ] テスト追加 (`tests/modal_body_tests.rs` 拡張):
  - [ ] `test_modal_body_set_instrument_changes_coeffs`（Default → Ukulele で coeffs_l[0] が異なる）
  - [ ] `test_modal_body_set_instrument_clears_state`（active state → set_instrument → process_sample(0.0) が 0.0）
  - [ ] `test_modal_body_default_matches_phase3`（Default kind で coeffs_l[0] が Phase 3 既存値と一致）
- [ ] `cargo test -p dsp-core` がすべてパス
- [ ] git commit `feat(modal-body): set_instrument(kind) メソッド追加 (D52, F43-a/d)`
- **検証**: F43-a / F43-d (ModalBody 単体) 達成

#### Step 9. Engine::apply_instrument 実装（D52 / D53 / D54）

- [ ] `crates/dsp-core/src/engine.rs` に `Engine::apply_instrument(kind)` を追加（[`03-dsp-core-spec.md` §Engine::apply_instrument](./03-dsp-core-spec.md#engineapply_instrumentkindd52--d53)）:
  ```rust
  pub fn apply_instrument(&mut self, kind: InstrumentKind) {
      self.pool.all_notes_off();
      self.hold_stack.clear();
      self.sustain_state.reset();
      self.current_instrument = kind;
      self.stereo_spread = stereo_spread_for_instrument(kind);
      self.modal_body.set_instrument(kind, self.sample_rate);
  }
  ```
- [ ] `Engine::current_instrument()` / `Engine::stereo_spread()` の `#[doc(hidden)]` getter を追加（テスト用）
- [ ] テスト追加 (`tests/instrument_tests.rs` 新規):
  - [ ] `test_apply_instrument_changes_modal_coeffs`
  - [ ] `test_apply_instrument_releases_all_voices`
  - [ ] `test_apply_instrument_clears_sustain_state`
  - [ ] `test_apply_instrument_resets_modal_state`
  - [ ] `test_apply_instrument_no_alloc`
  - [ ] `test_stereo_spread_per_instrument`
- [ ] `cargo test -p dsp-core` がすべてパス
- [ ] git commit `feat(engine): apply_instrument(kind) で楽器切替 (D52, D53, D54, F43-b/c/e)`
- **検証**: F43-b / F43-c / F43-e 達成

### フェーズ ε — C ABI + Worklet 拡張（2 ステップ）

#### Step 10. C ABI 4 関数追加 + REQUIRED 配列更新（D45-D52）

- [ ] `crates/wasm-audio/src/lib.rs` に [`04-wasm-audio-spec.md` §Phase 4a で追加する C ABI 関数](./04-wasm-audio-spec.md#phase-4a-で追加する-c-abi-関数4-件) に従い 4 関数を追加:
  - [ ] `synth_apply_instrument`
  - [ ] `synth_lfo_set_rate`
  - [ ] `synth_lfo_set_waveform`
  - [ ] `synth_lfo_set_depth`
- [ ] `scripts/check-wasm-exports.mjs` の `REQUIRED` 配列に上記 4 関数を追加
- [ ] `pnpm build:wasm` で 4 関数すべて export されることを確認（exit 0）
- [ ] `cargo clippy --workspace -- -D warnings` がパス
- [ ] git commit `feat(wasm-audio): synth_apply_instrument / synth_lfo_set_* 4 関数追加 (D45-D52)`
- **検証**: F39（部分、build pipeline 通過）

#### Step 11. messages.ts + WasmExports + SynthEngine 拡張

- [ ] `web/src/lib/audio/messages.ts` の `ToWorkletMessage` に 4 variant 追加（[`05-web-frontend-spec.md` §messages.ts の Phase 4a 変更点](./05-web-frontend-spec.md#messagets-の-phase-4a-変更点)）
- [ ] `messages.ts` に `LfoWaveformKey` / `LfoDestinationKey` / `InstrumentKindKey` 型を export
- [ ] `web/src/lib/audio/synth-processor.ts` の `WasmExports` interface に 4 関数を追加
- [ ] `synth-processor.ts` 冒頭に `LFO_WAVEFORM_MAP` / `LFO_DESTINATION_MAP` / `INSTRUMENT_KIND_MAP` const マップ追加
- [ ] `synth-processor.ts` の `onMessage` switch に `lfoSetRate` / `lfoSetWaveform` / `lfoSetDepth` / `applyInstrument` 4 ケース追加
- [ ] `web/src/lib/audio/engine.ts` に `lfoSetRate` / `lfoSetWaveform` / `lfoSetDepth` / `applyInstrument` / `applyPreset(preset: PresetV1)` メソッド追加
- [ ] `pnpm --filter ./web check` がパス
- [ ] `pnpm dev` でブラウザ起動、DevTools Console で:
  ```javascript
  __synthDev は Phase 3 同等で setMode のみ。
  // 一時的に F40 経路確認用に手動操作（Step 13 で UI 完成、暫定）
  ```
- [ ] git commit `feat(web): messages / WasmExports / SynthEngine に LFO + applyInstrument 経路追加`
- **検証**: F40〜F43 経路確認、UI は Step 12 / 13 で完成

### フェーズ ζ — UI 実装（2 ステップ）

#### Step 12. preset-store + factory-presets + preset-schema 実装（D50 / D51）

- [ ] `web/src/lib/state/preset-schema.ts` を [`05-web-frontend-spec.md` §preset-schema.ts](./05-web-frontend-spec.md#preset-schemats-phase-4a-新規) に従い新規作成
- [ ] `PresetV1` interface、`isValidPresetV1` validator、`getDefaultPreset` を実装
- [ ] `web/src/lib/state/factory-presets.ts` を新規作成（[`05-web-frontend-spec.md` §factory-presets.ts](./05-web-frontend-spec.md#factory-presetsts-phase-4a-新規)）
- [ ] Factory Preset 7 種を const 配列で定義
- [ ] `web/src/lib/state/preset-store.svelte.ts` を新規作成（[`05-web-frontend-spec.md` §preset-store.svelte.ts](./05-web-frontend-spec.md#preset-storesveltets-phase-4a-新規)）
- [ ] `PresetStore` クラス: `load` / `save` / `delete` / `apply` / `findByName` / `capturePreset` メソッド + `userPresets` / `currentPresetName` / `errorMessage` の `$state`
- [ ] `MAX_USER_PRESETS = 32` 定数 export
- [ ] `pnpm --filter ./web check` がパス
- [ ] git commit `feat(web): preset-store + factory-presets + preset-schema 実装 (D50, D51)`
- **検証**: F42-a / F42-b / F42-c (TypeScript レベル)

#### Step 13. PresetSelector + ModWheel + LfoSection UI 実装（D46-D49 / D50 / D51 / D52）

- [ ] `web/src/lib/state/synth.svelte.ts` に Phase 4a `$state` 追加（modWheel / lfoRate / lfoWaveform / lfoPitchDepth / lfoBrightnessDepth / lfoVolumeDepth / instrument）
- [ ] `web/src/lib/components/ModWheel.svelte` を [`05-web-frontend-spec.md` §ModWheel.svelte](./05-web-frontend-spec.md#modwheelsvelte-phase-4a-新規) に従い作成
- [ ] `web/src/lib/components/LfoSection.svelte` を [`05-web-frontend-spec.md` §LfoSection.svelte](./05-web-frontend-spec.md#lfosectionsvelte-phase-4a-新規) に従い作成
- [ ] `web/src/lib/components/PresetSelector.svelte` を [`05-web-frontend-spec.md` §PresetSelector.svelte](./05-web-frontend-spec.md#presetselectorsvelte-phase-4a-新規) に従い作成（onMount で `presetStore.load()` を呼ぶ）
- [ ] `web/src/routes/+page.svelte` に `<PresetSelector />` / `<ModWheel />` / `<LfoSection />` を配置（[`05-web-frontend-spec.md` §+page.svelte の Phase 4a 変更点](./05-web-frontend-spec.md#pagesvelte-の-phase-4a-変更点)）
- [ ] `pnpm --filter ./web check` / `pnpm --filter ./web lint` がパス
- [ ] git commit `feat(web): ModWheel / LfoSection / PresetSelector UI 実装 (D46-D52)`
- **検証**: F40 / F41 / F42 / F43 のうち UI 経路が実装される（実機確認は Step 15）

### フェーズ η — 統合検証（2 ステップ）

#### Step 14. 統合 cargo test + alloc ゼロ + release timing（F45 / F46 / F47）

- [ ] `tests/no_alloc_tests.rs` に `test_no_allocation_with_lfo_and_instrument` を追加（8 voice + LFO active + Mod Wheel + 楽器切替で voice buffer / LFO 状態 capacity 不変）
- [ ] **`tests/dsp_core_tests.rs` に `test_engine_process_block_timing_phase4a` を追加**（[06 章 §F46](./06-build-and-verify.md#f46--リアルタイム性能-release-cargo-timingf37-拡張)）:
  - [ ] `#[cfg(not(debug_assertions))]` で release ビルド限定
  - [ ] 8 voice 全 active + Pitch Bend + CC#7 + Mod Wheel = 1.0 + LFO depths 全 1.0 で最悪ケース
  - [ ] `Instant::now()` で 1000 回 process の平均時間を計測
  - [ ] `assert!(per_block_ms < 1.7)` で完成条件を強制
- [ ] **`cargo test --release -p dsp-core` で全テスト**（Phase 3 既存 94 + Phase 4a 新規 ~30 = 124 件目標）がパス、特に F46 timing test が release で通ること
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` がパス
- [ ] `pnpm fmt` で全コードフォーマット
- [ ] `pnpm build` で本番ビルド成功
- [ ] WASM gzip サイズ計測:
  ```bash
  gzip -kc web/build/_app/immutable/assets/wasm_audio.*.wasm | wc -c
  ```
  - [ ] **目標**: gzip < 15 KB（想定 ~13 KB）
  - [ ] **警戒**: gzip < 18 KB（超えたら `wasm-opt --print-stats` で調査）
  - [ ] **撤退**: gzip < 30 KB（超えたら R32 適用 = 楽器 4 種に削減 / Modal M=5）
- [ ] Worklet 本番バンドルサイズ計測（< 10 KB target）:
  ```powershell
  Get-ChildItem web\build\_app\immutable\assets\synth-processor*.js | Select-Object Name, Length
  ```
- [ ] `__synthDev` が production bundle に 0 hits（grep で検証、F22 拡張）:
  ```bash
  grep -r "__synthDev" web/build/_app/immutable/ | wc -l
  # 期待値: 0
  ```
- [ ] git commit `test: Phase 4a 統合テスト (alloc / release timing / regression) (F45/F46/F47)`
- **検証**: F45 / F46 / F47 達成、Phase 3 既存テストの regression なし

#### Step 15. 実機確認（pnpm dev + F38b 再計測）

- [ ] `pnpm dev` でブラウザ起動、F40〜F43 の実機確認:
  - [ ] **F40 LFO**: vibrato（Pitch Depth=1.0 + Mod Wheel=1.0、rate 5Hz）/ tremolo（Volume Depth=1.0 + Mod Wheel=1.0）/ wah（Brightness Depth=1.0 + Mod Wheel=1.0、rate 1Hz）が音に反映
  - [ ] **F41 Mod Wheel**: WebMIDI 物理 wheel と UI スライダーが同期、Mod Wheel = 0 で LFO 効果ゼロ
  - [ ] **F42 Preset**: Save / Load / Delete が動作、リロードで User Preset が残る、32 件超過で errorMessage 表示
  - [ ] **F43 Instrument**: 6 楽器（Default 含み 7 種）プリセット選択で音色が切り替わる、楽器切替時に音切れ
- [ ] **F38b 再計測**: Phase 4a 後の実機 Worklet `process` 時間を Chrome DevTools Performance タブで計測
  - [ ] avg < 1.7 ms / max < 2.7 ms (Phase 3 1.5 ms / 2.5 ms + 0.2 ms 余裕)
  - [ ] 計測値を後の retrospective で記録
- [ ] Phase 3 機能の regression 確認:
  - [ ] Pitch Bend / Sustain Pedal / Channel Volume / All Notes Off が Phase 3 と同じく動作
  - [ ] mono / poly トグルが Phase 3 と同じ
  - [ ] VoiceMeter が 8 voice の active 状態を反映
  - [ ] Default プリセット + Mod Wheel = 0 で Phase 3 と同じ音
- [ ] iOS Safari (HTTPS URL or Pages preview) での動作確認（持ち越し可、可能なら実施）
- [ ] git commit `chore: Phase 4a 実機確認完了`
- **検証**: F38b (再計測) / F40 / F41 / F42 / F43 すべて実機達成、Phase 3 互換性確認

### フェーズ ζ — ドキュメント整備（1 ステップ）

#### Step 16. ドキュメント整備 + retrospective 準備

- [ ] `README.md` を Phase 4a 用に更新:
  - [ ] LFO / Mod Wheel / Preset / 多楽器プリセット 6 種の項目追加
  - [ ] F38b 計測手順 + F39〜F47 の自己検証手順
  - [ ] Phase 3 の F34（Voice Meter UI 実機）の空欄を埋める（実機確認結果を追記）
- [ ] `CLAUDE.md` の「現在のイテレーション」を Phase 4a 完了に更新、「次は Phase 4b（ピアノ音色）」を追記
- [ ] 仕様書群 `docs/specs/2026-05-08-004-phase4a/` の各章でリンク切れがないか確認
- [ ] retrospective テンプレートを準備（`/retrospective` カスタムコマンドを Phase 4a 完了後に発火）
- [ ] git commit `docs: Phase 4a 完了反映 (README / CLAUDE.md / retrospective 準備)`
- **検証**: Phase 4a 完成、Phase 4b への申し送りが文書化される

#### Step 17. PR 作成 + main マージ

- [ ] `cargo test --release -p dsp-core` 最終確認、全件パス
- [ ] `pnpm check` / `pnpm lint` / `pnpm fmt` 最終確認
- [ ] `pnpm build` 最終ビルド成功確認、gzip < 15 KB 目標 / < 18 KB 警戒 / < 30 KB 撤退ラインを確認
- [ ] PR 作成（`gh pr create`）:
  - [ ] PR タイトル: `Phase 4a: F38b + LFO + Mod Wheel + Preset + 多楽器 6 種`
  - [ ] PR ボディに Phase 4a スコープサマリ + 検証結果（F38b avg/max、gzip サイズ、テスト件数）
- [ ] CI 緑を確認（build / test / lint / params-sync / wasm-exports）
- [ ] main ブランチへマージ
- [ ] retrospective 着手（`/retrospective 2026-05-08-004-phase4a`）
- **検証**: Phase 4a が main にマージされる、Phase 4b 着手準備完了

## ステップごとの依存関係

```
Step 1 (F38b 計測)
  └─ Phase 3 完成判定 → Phase 4a 実装の前提条件
      ▼
Step 2 (wasm-opt -O3) ─ Step 3 (excitation_snapshot cfg(test))
  └─ 既存負債解消、独立、並列可
      ▼
Step 4 (params.json + gen-params.mjs)
  └─ InstrumentKind / 楽器係数 12 配列の生成
      ▼
Step 5 (lfo.rs + Engine フィールド = mod_wheel: SmoothedValue を含む)
      ▼
Step 6 (Mod Wheel CC#1 + WebMIDI/UI 同期) ← Step 5 で追加された mod_wheel フィールドに依存
      ▼
Step 7 (LFO destinations 統合) ← Step 5 + Step 6 完了が前提
  └─ Engine::process の per-sample loop が完成
      ▼
Step 8 (ModalBodyResonator::set_instrument) ← Step 4 完了が前提
      ▼
Step 9 (Engine::apply_instrument) ← Step 8 完了が前提
      ▼
Step 10 (C ABI 4 関数 + REQUIRED) ← Step 7 + Step 9 完了が前提
      ▼
Step 11 (messages.ts + WasmExports + SynthEngine) ← Step 10 完了が前提
      ▼
Step 12 (preset-store + factory-presets + preset-schema) ← Step 11 完了で型定義可
      ▼
Step 13 (PresetSelector + ModWheel + LfoSection UI) ← Step 12 完了が前提
      ▼
Step 14 (統合 cargo test + alloc + release timing) ← Step 13 まで完了が前提
      ▼
Step 15 (実機確認 + F38b 再計測)
      ▼
Step 16 (ドキュメント整備)
      ▼
Step 17 (PR 作成 + main マージ)
```

並列実装可能なポイント:

- Step 2（wasm-opt）と Step 3（excitation_snapshot）は独立、並列可
- **Step 6（Mod Wheel CC#1 + WebMIDI/UI 同期）は Step 5（lfo.rs + Engine フィールド追加、`mod_wheel: SmoothedValue` を含む）に依存**: `Engine::handle_midi_cc` の CC_MOD_WHEEL 分岐で `self.mod_wheel.set_target(v)` を呼ぶため、Step 5 のフィールド追加が前提。並列実装は不可
- Step 8（ModalBody set_instrument）は Step 5/6/7 と独立、Step 4 完了後ならいつでも可

## 達成ライン早見表

| ステップ完了 | 達成する F-tag |
|---|---|
| Step 1 | F38b（Phase 3 持ち越し計測） |
| Step 2 | F39（wasm-opt -O3 サイズ削減） |
| Step 3 | F44（excitation_snapshot cfg(test)） |
| Step 4 | F39（生成パイプライン） |
| Step 5 | F40-a / F40-b / F40-g（LFO 単体） |
| Step 6 | F41（Mod Wheel CC#1） |
| Step 7 | F40-c / F40-d / F40-e / F40-f / F40-g（LFO destinations 統合） |
| Step 8 | F43-a / F43-d（ModalBody 単体） |
| Step 9 | F43-b / F43-c / F43-e（Engine::apply_instrument） |
| Step 10 | C ABI export 検証 |
| Step 11 | F40〜F43 の経路確認 |
| Step 12 | F42-a / F42-b / F42-c（TypeScript レベル） |
| Step 13 | F40 / F41 / F42 / F43 の UI 実装 |
| Step 14 | F45 / F46 / F47（alloc / timing / regression） |
| Step 15 | F38b 再計測 + F40〜F43 実機確認 |
| Step 16 | ドキュメント完成 |
| Step 17 | Phase 4a 完成 |

すべての F38b + F39〜F47 が達成された時点で Phase 4a 完成。F46 は **Step 14 で release cargo timing test を必須化**、F38b は **Step 1 + Step 15 の 2 回計測**（Phase 3 検証 + Phase 4a 後の regression 確認）。

## 実装着手者へのメモ

- **Step 1（F38b 計測）が最重要**。ここで Phase 3 完成判定を閉じないと Phase 4a の CPU 余裕が不明、target 超過時は R30 を Phase 4a 内で対処する必要があり scope が大きく変わる
- **Step 4（params.json 拡張）の生成物は git commit する**（Phase 1〜3 の D25 継承）。生成物のレビュー観点は「Default kind の係数が Phase 3 既存値と完全一致しているか」が最重要
- **Step 5（lfo.rs）の `Lfo::process_sample` で `f32::sin()` を直接呼ぶ**。LUT 不要（CPU 影響軽微、03 章 §Lfo の根拠）
- **Step 6（Mod Wheel CC#1 + WebMIDI/UI 同期）**: `Engine::handle_midi_cc` の CC#1 分岐有効化（Step 5 で追加済の `self.mod_wheel.set_target(v)`）に加え、`web/src/lib/input/midi-cc.ts` の CC#1 経路で `synth.modWheel = data[2] / 127` を更新する Web 側変更を含む。Rust + Web の両側に変更が走るためテストは cargo test (`midi_cc_tests.rs`) と svelte-check の両方で確認
- **Step 7（LFO destinations 統合）が最大のロジック**。`KarplusStrong::process_sample` で `length_target.next_sample()` 後の効果値計算 + brightness 加算を per voice / per sample で実装。**`test_mod_wheel_zero_disables_lfo` が Phase 3 互換性を保証する**（Mod Wheel = 0 で LFO depth 任意でも音は変調されない）
- **Step 9（Engine::apply_instrument）の `pool.all_notes_off()` 順序**: `current_instrument` 更新の前に呼ぶ（音切れの方向性として「楽器が変わる前に古い voice を止める」が自然）
- **Step 10（C ABI）の関数追加順**: 1 commit で 4 関数まとめて追加 + REQUIRED 配列更新。**Phase 3 の D38 / D41 と同じパターン**
- **Step 12（preset-store）の bug-prone な部分**: `localStorage.setItem` での QuotaExceededError の `try/catch`、`isValidPresetV1` での null check、`STORAGE_KEY_LIST` の name 配列の同期更新（save / delete で必ず更新）
- **Step 13（UI 実装）の Svelte 5 注意点**: `$derived` を effect 内で参照する場合は `$derived.by(() => ...)` を検討、`$state` の配列は spread で更新（`this.userPresets = [...this.userPresets, preset]` パターン）
- **Step 14（release timing test）の閾値**: Phase 3 の 1.5 ms から 1.7 ms に緩める（LFO + 楽器切替の +0.2 ms 余裕）。CI で flaky なら 2.0 ms に緩めるが目標は 1.7 ms
- **Step 15（実機確認）の Phase 3 互換性**: Default プリセット + Mod Wheel = 0 で Phase 3 と同じ音が出ることが最重要 regression check
- **各ステップで コミットを分ける** ことを推奨（Phase 1 / 2 / 3 と同じ）。問題発生時に二分探索しやすい。コミットメッセージは `feat(lfo): step 5 - Lfo 型実装 + Engine フィールド追加 (D46, D47, F40-a/b/g)` のように Step 番号 + D-tag + F-tag を含める
- **Phase 3 既存テスト 94 PASS が各ステップで壊れないこと**を最優先。Step 5 / 7 / 8 / 9 で Engine / KarplusStrong / VoicePool / ModalBody を書き換えるため、各 Step 完了時に `cargo test -p dsp-core` を回す
- 詰まったら [`06-build-and-verify.md` §トラブルシューティング](./06-build-and-verify.md#トラブルシューティング-tips) を参照
