# 07. Phase 3 実装順序チェックリスト

## 目的

Phase 3 仕様書承認後、本仕様を実装する際の作業順序を Phase α〜η の 7 フェーズ・全 14 ステップで定義する。各ステップは独立して進捗確認でき、検証チェックリスト（F26〜F38）の充足ポイントを明示する。Phase 1 / 2 の `07-implementation-checklist.md` と同じ粒度（1 ステップ ≈ 1 コミット）で構成する。

## 他文書との関係

- 上流: 全ての仕様書（pre-research、01〜06）
- 参考: Phase 1 / 2 [07 章] — **Phase 3 ステップは Phase 1 / 2 の構成パターンを踏襲**
- このドキュメントは **実装専用のチェックリスト** であり、各ステップの設計詳細は対応する仕様書を参照する

## 前提条件

Phase 3 実装着手前に以下を確認:

1. **Phase 2 の 41 既存テストがすべてパス**
   - `cargo test -p dsp-core` で確認
2. **`pnpm dev` でブラウザでの 8 音ポリフォニック再生が確認できる状態**
3. **F1〜F25 の実機検証は持ち越し継続**（ある程度動作確認済み、ユーザー承認済み）

## 実装ステップ（全 14 段階、Phase α〜η の 7 フェーズ）

### フェーズ α — Thiran allpass 試作評価（1 ステップ）

D36 を Phase 3 着手の最初に確定させる。Lagrange / Thiran の選択は KarplusStrong の処理系の根幹に関わるため、後続 Step の実装に影響する。

#### Step 1. Thiran allpass 試作評価（F29）

- [ ] `crates/dsp-core/src/fractional_delay.rs` に `ThiranCoeffs` 構造体を追加（既存 `LagrangeCoeffs` と並列、[`03-dsp-core-spec.md` §Fractional delay の Phase 3 拡張](./03-dsp-core-spec.md#fractional-delay-の-phase-3-拡張thirancoeffs-の追加と-fractionaldelay-enum-での統一)）
- [ ] `ThiranCoeffs::new()` / `set_fractional(d)` / `process(x)` / `reset()` を実装（d ∈ [1e-4, 0.999] clamp、R25 対策）
- [ ] **`LagrangeCoeffs::set_fractional(&mut self, d: f32)` を新規追加**（中身は `*self = Self::new(d)`、enum 経由で再計算するために必要）
- [ ] **`FractionalDelay` enum で Lagrange/Thiran を統合**（`set_fractional(d)` / `apply(x_m, x_z, x_p1, x_p2)` / `reset()` / `new_lagrange()`（= `LagrangeCoeffs::default()` を内包）/ `new_thiran()` を提供）
- [ ] **`KarplusStrong` の field は `fractional_delay: FractionalDelay` の単一名のみ**。`self.lagrange` / `self.thiran` の二系統 field、`use_thiran: bool` フラグ、`if use_thiran { thiran.process(...) } else { lagrange.apply(...) }` の分岐パターンは **すべて不採用**。process_sample からは `self.fractional_delay.apply(buf_m, buf_z, buf_p1, buf_p2)` のみで呼ぶ
- [ ] `Engine::new` は `FractionalDelay::new_lagrange()` を選択。**`Engine::new_with_thiran()` を test-only constructor として追加**（`#[cfg(test)]` または `#[doc(hidden)]`）し、`FractionalDelay::new_thiran()` を選択する経路を提供。Step 1 試作の cargo test 6 件はこの経路で実行
- [ ] `crates/dsp-core/tests/pitch_accuracy.rs` に Thiran 版テストを追加:
  - [ ] `test_pitch_a1_thiran` (midi=33, 55Hz、誤差 < 0.5%)
  - [ ] `test_pitch_a2_thiran` (midi=45, 110Hz)
  - [ ] `test_pitch_a4_thiran` (midi=69, 440Hz)
  - [ ] `test_pitch_c6_thiran` (midi=84, 1046.5Hz)
  - [ ] `test_pitch_c8_thiran` (midi=108, 4186Hz、Phase 2 ignore 対象)
  - [ ] `test_pitch_c8_thiran_self_oscillates` (10 秒走らせて RMS > 0.01 に収束)
- [ ] cargo test を実行、5 件のピッチテスト + C8 自己発振テストの結果を出力
- [ ] **判断**:
  - すべて誤差 < 0.5% + C8 自己発振成立 → **案 A 採用**: `Engine::new` の選択を `FractionalDelay::new_thiran()` に切替え、`FractionalDelay` enum を解消して `fractional_delay: ThiranCoeffs` 単一型 field に置換（enum dispatch を除去）。Lagrange 経路は Phase 4 検討用に残置 or 削除
  - A1〜C6 で誤差悪化 (+0.1% 超) → **案 B**: `if midi >= 96 { thiran } else { lagrange }` で switch
  - C8 すら改善されない → **案 C**: Lagrange 維持、`test_pitch_c8` ignore 継続
- [ ] 採用案を `01-overview.md` の D36 を更新（仕様書改訂版コミット）
- [ ] `cargo test -p dsp-core` の Phase 2 既存 41 件 + Thiran 6 件がすべてパス
- **検証**: F29 達成、D36 確定。Step 2 以降の実装で確定した補間方式を使う

### フェーズ β — Modal Body Resonator（2 ステップ）

#### Step 2. `params.json` 拡張 + 生成パイプライン更新

- [ ] `params.json` に `PickPosition` (id=3) と `BodyWet` (id=4) を `params` 配列に追加（[`03-dsp-core-spec.md` §params.json の Phase 3 拡張](./03-dsp-core-spec.md#paramsjson-の-phase-3-拡張)）
- [ ] `params.json` に `body_modes` セクション（8 モード × {freq, q, gain}）と `stereo_spread: 0.05` を追加
- [ ] `scripts/gen-params.mjs` を更新:
  - [ ] `generateRustSource` が `BodyMode` struct + `BODY_MODES_L/R` const + `STEREO_SPREAD` const を出力
  - [ ] `generateTsSource` が `BodyMode` interface + 同 export を出力
  - [ ] `applyStereoSpread(modes, spread)` 純粋関数を追加（左 ch と右 ch で freq/q を ±spread% 揺らす）
- [ ] `pnpm gen:params` を実行、生成物に Body Mode 関連が含まれることを確認
- [ ] `pnpm check:params-sync` がパス
- [ ] `cargo check -p dsp-core` がパス、`pnpm --filter web check` がパス
- **検証**: 生成物 drift なし、PARAM_DESCRIPTORS が 5 件、BODY_MODES_L/R が各 8 件

#### Step 3. `modal_body.rs` 実装 + Engine 統合（F26）

- [ ] `crates/dsp-core/src/modal_body.rs` を [`03-dsp-core-spec.md` §ModalBodyResonator](./03-dsp-core-spec.md#modalbodyresonator-modal_bodyrs) に従い作成
- [ ] `ModalBodyResonator` 構造体、`new()` / `prepare(sr)` / `reset()` / `calc_coeffs(mode, sr)` / `process_sample(x) -> (f32, f32)` を実装
- [ ] denormal 対策（`+1e-25 -1e-25`）を `process_sample` に挿入（D6 拡張、R24 対策）
- [ ] `crates/dsp-core/src/lib.rs` に `pub mod modal_body;` を追加
- [ ] `crates/dsp-core/src/engine.rs` に `modal_body: ModalBodyResonator` / `body_wet: SmoothedValue` フィールドを追加
- [ ] `Engine::prepare(sr, mb)` で `modal_body.prepare(sr)` / `body_wet.set_time_constant(sr, 0.02)` を呼ぶ
- [ ] `Engine::process` の per-sample loop に Modal Body 段を挿入（`pool.process_sample()` 後・`output_gain` 前）
- [ ] `Engine::set_param` の match arm で `BodyWet` (id=4) → `body_wet.set_target(value)` を実装
- [ ] `Engine::reset` で `modal_body.reset()` を呼ぶ
- [ ] テスト追加:
  - [ ] `tests/modal_body_biquad_tests.rs` 新規: `test_single_biquad_dc_blocking` / `test_single_biquad_peak_at_freq` / `test_single_biquad_bandwidth`（係数仕様を単体で厳密検証、隣接モード干渉なし）
  - [ ] `tests/modal_body_tests.rs` 新規: `test_modal_body_dc_blocking` / `test_modal_body_peak_at_modes`（aggregate で隣接干渉を許容した広い範囲）/ `test_modal_body_inter_mode_attenuation`（定性的検証）/ `test_modal_body_stereo_spread` / `test_modal_body_no_alloc_in_process` / `test_modal_body_reset_clears_state`
  - [ ] `tests/dsp_core_tests.rs` に `test_engine_modal_body_in_signal_chain`（dry/wet ミックスが正しく動作）
- [ ] `cargo test -p dsp-core` がすべてパス
- **検証**: F26 達成

### フェーズ γ — Extended KS（2 ステップ）

#### Step 4. `loss_filter.rs` 実装 + KarplusStrong 統合（F27）

- [ ] `crates/dsp-core/src/loss_filter.rs` を [`03-dsp-core-spec.md` §LossFilter](./03-dsp-core-spec.md#lossfilter-loss_filterrs) に従い作成
- [ ] `LossFilter` 構造体、`new()` / `set_for_frequency(freq_hz)` / `reset()` / `process_sample(x)` を実装
- [ ] `crates/dsp-core/src/lib.rs` に `pub mod loss_filter;` を追加
- [ ] `crates/dsp-core/src/karplus_strong.rs` に `loss_filter: LossFilter` フィールドを追加
- [ ] `KarplusStrong::note_on` で `loss_filter.set_for_frequency(freq_hz)` を呼ぶ
- [ ] `KarplusStrong::process_sample` で brightness LPF 後・damping 前に `loss_filter.process_sample(filtered)` を挿入
- [ ] `KarplusStrong::reset` で `loss_filter.reset()` を呼ぶ
- [ ] テスト追加:
  - [ ] `tests/loss_filter_tests.rs` に `test_loss_filter_dc_gain` / `test_loss_filter_nyquist_attenuation` / `test_loss_filter_high_freq_more_loss`
- [ ] `cargo test -p dsp-core` がすべてパス（Phase 2 既存 41 + Thiran 6 + Body 4 + LossFilter 3 = 54 件）
- **検証**: F27 達成

#### Step 5. Pick position 励振 shaping を `KarplusStrong::note_on` に実装（F28）

- [ ] **専用モジュール `pick_position.rs` は作らない**（D34 設計変更版、励振 shaping で実装）
- [ ] `crates/dsp-core/src/karplus_strong.rs` に `pick_position: f32` フィールドを追加（SmoothedValue ではなく単純 f32）
- [ ] `KarplusStrong::set_pick_position(beta: f32)` を inherent method として実装（β を `clamp(0.05, 0.5)` で保持、次回 `note_on` で反映）
- [ ] **共通ヘルパ `KarplusStrong::note_on_internal(note_id: Option<u8>, freq_hz, velocity)`** を private で実装し、励振 shaping / Loss filter `set_for_frequency` / fractional delay `set_fractional` / `note_id = note_id` を一括処理。**公開 API は `note_on(freq_hz, velocity)`（`Voice` trait 互換、内部で `note_on_internal(None, ...)` を呼ぶ）と `note_on_with_id(midi_note, freq_hz, velocity)`（VoicePool 経由の主要呼び出し、`note_on_internal(Some(midi_note), ...)`）の 2 本立て**。**`note_on_with_id(0, ...)` を呼んだ後に `note_id = None` で上書きする実装は不可**（`Some(0)` と `None` を取り違えるバグの温床、P1 対策）。詳細は [`03-dsp-core-spec.md` §Pick position（励振 shaping）](./03-dsp-core-spec.md#pick-position励振-shaping専用モジュールなし):
  - [ ] `K = round(self.pick_position * length_int)` を計算、`min(length_int - 1)` で clamp
  - [ ] buffer 全体ゼロクリア → 先頭 length_int に noise burst をロード
  - [ ] K > 0 なら降順ループで `buffer[i] -= buffer[i - k]` を in-place 適用
  - [ ] `write_index = length_int` から開始（既存 Phase 2 パターン継承）
- [ ] `KarplusStrong::process_sample` には Pick position 関連のコードを **入れない**（feedback loop 内 comb は不採用）
- [ ] `crates/dsp-core/src/voice_pool.rs` に `set_pick_position(beta)` を追加（全 voice fan-out）
- [ ] `crates/dsp-core/src/engine.rs` に `pick_position: f32` フィールドを追加（SmoothedValue 不要、D34）、`Engine::set_param` の match arm で `PickPosition` (id=3) → `self.pick_position = value; self.pool.set_pick_position(value)`
- [ ] テスト追加:
  - [ ] `tests/karplus_strong_pick_tests.rs` に `test_pick_min_beta_minimal_shape`（β=0.05、外部 API の最小値で comb 効果最小）/ `test_pick_position_node_at_beta_half`（β=0.5 で偶数倍音消失、FFT 検証）/ `test_pick_position_attenuates_kth_harmonic`（β=1/k で k 番目倍音減衰）/ `test_pick_position_no_extra_alloc`（β 変えて note_on 連打、buffer.len 不変）/ `test_pick_internal_k_zero_branch`（**`length_int = 9` + `β = 0.05`（積 0.45 → round = 0）** で K=0 分岐パスを `#[cfg(test)]` 内部テスト or test-only constructor から検証、panic なく素通し動作確認。`length_int = 10` だと f32::round(0.5) = 1.0 で K=0 にならない点に注意）
- [ ] `cargo test -p dsp-core` がすべてパス
- **検証**: F28 達成、追加メモリ 0 / process 内コスト 0 確認

### フェーズ δ — ピッチ補正 + Soft clip（2 ステップ）

#### Step 6. Brightness 群遅延補正（F30）

- [ ] `crates/dsp-core/src/karplus_strong.rs` の `note_on_internal` で `tau_g(brightness) = (1 - brightness) / brightness` を計算し、`adjusted_length = (raw_length - tau_g).clamp(3.0, max_len)` を `length_target` の初期値に使う（`max_len = (buffer.len() - LAGRANGE_BUFFER_MARGIN) as f32`、ring buffer + Lagrange 4 点参照の安全条件、本文と統一、[`03-dsp-core-spec.md` §KarplusStrong の Phase 3 拡張](./03-dsp-core-spec.md#karplusstrong-の-phase-3-拡張)）
- [ ] `set_pitch_bend` でも同じ補正を適用
- [ ] テスト追加:
  - [ ] `tests/pitch_accuracy.rs` に `test_engine_brightness_pitch_correction`（A4 で brightness=0.5 設定、measure_f0 で誤差 < 0.5%）
- [ ] `cargo test -p dsp-core` がすべてパス
- **検証**: F30 達成、Phase 2 の 0.89% 偏移が < 0.5% に解消

#### Step 7. `soft_clip.rs` 実装 + Engine 統合（F35）

- [ ] `crates/dsp-core/src/soft_clip.rs` を [`03-dsp-core-spec.md` §SoftClip](./03-dsp-core-spec.md#softclip-soft_cliprs) に従い作成
- [ ] **`soft_clip(x: f32) -> f32` を区間関数型で実装**: `|x| ≤ 0.95` で `x` を完全パススルー（誤差ゼロ）、`|x| > 0.95` で `signum(x)·(0.95 + 0.05·e/(e+0.05))`（`e = |x| − 0.95`、`|x|→∞` で ±1.0 厳密漸近）。**`tanh` の Padé 近似は使わない**（旧版仕様で発散問題があり撤回済）
- [ ] `crates/dsp-core/src/lib.rs` に `pub mod soft_clip;` を追加
- [ ] `Engine::process` の per-sample loop で `output_gain` 後・`output_l/r` 書き込み前に `soft_clip(scaled_l)` / `soft_clip(scaled_r)` を挿入
- [ ] テスト追加:
  - [ ] `tests/soft_clip_tests.rs` に `test_soft_clip_linear_in_safe_range`（|x|≤0.95 で `assert_eq!(soft_clip(x), x)`）/ `test_soft_clip_bounded`（任意 x で `\|y\| < 1.0`） / `test_soft_clip_continuous_at_threshold`（x=0.95 ± 1e-6 で連続） / `test_soft_clip_extreme`（|x|=1e6 で `0.99 < \|y\| < 1.0`）
- [ ] `cargo test -p dsp-core` がすべてパス
- **検証**: F35 達成

### フェーズ ε — MIDI CC（2 ステップ）

#### Step 8. Voice trait 拡張 + Pitch Bend（F32）

- [ ] `crates/dsp-core/src/traits.rs` の `Voice` trait に `set_pitch_bend(semitones: f32)` を追加（D39、Mod Wheel `set_mod_depth` は Phase 4 送り）
- [ ] `crates/dsp-core/src/voice.rs` で `KarplusStrong` 向け委譲を追記
- [ ] `crates/dsp-core/src/smoothing.rs` は **変更なし**（既存 `set_immediate(value)` を Pitch Bend SmoothedValue の note_on 時初期化に流用、`crates/dsp-core/src/smoothing.rs:20`、API 増加を避ける、P3 対策）
- [ ] `crates/dsp-core/src/karplus_strong.rs` に `fractional_delay: FractionalDelay`（Phase 2 の `lagrange: LagrangeCoeffs` を置換）/ `pitch_bend_semitones: f32` / `length_target: SmoothedValue` / `cached_length: f32` / `base_length: f32` / `base_freq: f32` を追加
- [ ] `KarplusStrong::set_pitch_bend(semitones)` で `length_target.set_target(adjusted_length_with_bend)` を更新（係数自体はここで触らない）
- [ ] **`KarplusStrong::note_on_internal(note_id, freq_hz, velocity)` 共通ヘルパ内で `self.fractional_delay.set_fractional(self.length_frac)` を必須手順として呼ぶ**（公開 `note_on` / `note_on_with_id` は両方ともこのヘルパ経由、Step 5 の `note_on_internal` 集約と整合）: process_sample の手順 0 は cached_length 差分が < 1e-5 なら係数再計算を skip するため、note_on 直後の初回 sample で古い係数が使われないようにする（D26 拡張、Lagrange/Thiran 共通）
- [ ] `KarplusStrong::process_sample` で `length_target.next_sample()` を呼んで cached_length と差分が `> 1e-5` なら `length_int` / `length_frac` を再分解、補間係数を再計算（R26 対策、定常時は skip）
- [ ] **ring buffer 不変条件の維持**: `write_index = (write_index + 1) % buf_len` であり `% length_int` ではないこと、read 位置も `% buf_len` で計算することを実装時に厳守（[`03-dsp-core-spec.md` §統合フロー](./03-dsp-core-spec.md#統合フローprocess_sample) ring buffer 不変条件、Phase 2 既存パターン継承）
- [ ] `crates/dsp-core/src/voice_pool.rs` に `set_pitch_bend(semitones)` を追加（全 voice fan-out、`set_mod_depth` は Phase 4 送り）
- [ ] `Engine::handle_pitch_bend(semitones)` を実装、`pool.set_pitch_bend(clamped)` を呼ぶ
- [ ] テスト追加:
  - [ ] `tests/pitch_bend_tests.rs` に `test_pitch_bend_smooth_transition` / `test_pitch_bend_clamps_to_range` / `test_pitch_bend_ring_buffer_invariant`（buf_len 剰余維持確認）
- [ ] `cargo test -p dsp-core` がすべてパス
- **検証**: F32 達成、ring buffer P1-5 不変条件維持

#### Step 9. MIDI CC dispatch + Sustain Pedal（F31 / F33）

- [ ] `crates/dsp-core/src/sustain_state.rs` を [`03-dsp-core-spec.md` §SustainState](./03-dsp-core-spec.md#sustainstate-sustain_statersr) に従い作成
- [ ] `SustainState` 構造体、`new()` / `set_active(active)` / `try_defer_note_off(midi_note)` / **`clear_pending(midi_note)`**（同一ノート再打鍵対策、P1-3）/ **`pending_release_bitmap() -> u128`**（mode 切替時に pending を取り出すための read-only API、P2-1）/ `reset()` を実装
- [ ] `crates/dsp-core/src/lib.rs` に `pub mod sustain_state;` を追加
- [ ] `Engine` に `sustain_state: SustainState` および **`channel_volume: SmoothedValue`**（CC#7 用、デフォルト 1.0、D38b）フィールドを追加
- [ ] `Engine::handle_midi_cc(cc, value)` を実装:
  - [ ] CC#7 → `self.channel_volume.set_target(value)`（OutputGain は触らない、D38b 直交配置）
  - [ ] CC#64 → `sustain_state.set_active`、active=false で返った pending bitmap を全 release
  - [ ] CC#123 → `pool.all_notes_off()` + `hold_stack.clear()` + **`sustain_state.reset()`**（P1-1 対策、忘れると古い pending bit が CC#64 操作で再処理される）
  - [ ] CC#1 (Mod Wheel) や未対応 CC は no-op（panic / alloc なし）
- [ ] **`Engine::note_on(midi, vel)` 冒頭で `sustain_state.clear_pending(midi)` を呼ぶ**（P1-3 対策、Sustain 中の同一ノート再打鍵で古い pending bit を消す）。Phase 2 既存実装の Mono 分岐（`prev != midi_note` ガード付き `pool.note_off(prev)` + `hold_stack.push_unique`）と `trigger_voice` は完全継承
- [ ] **`Engine::note_off(midi)` は Phase 2 既存挙動を完全継承し、Poly 経路のみ Sustain defer を適用**（P1-2 解決方針）:
  - [ ] Poly: `if sustain_state.try_defer_note_off(midi) { return }` で defer、false なら `pool.note_off(midi)`
  - [ ] Mono: Sustain は無視、Phase 2 D29 既存ロジック（`prev_top` 取得 / `hold_stack.remove` / `pool.note_off(midi)` / `prev_top != new_top` ガードで `trigger_voice(top, MONO_REVIVE_VELOCITY)`）を **完全継承**。Mono+Sustain の挙動は Phase 4 で再評価
- [ ] `Engine::process` で `output_gain * channel_volume` を per-sample 積算（D38b）
- [ ] **`Engine::set_mode(mode)` を拡張**: Phase 2 既存の `hold_stack.clear()` に加え、**切替前の pending を `pending_release_bitmap()` で取り出して `sustain_state.reset()` してから各 note を `pool.note_off` で release**（P2-1 対策、mode 切替で Sustain pending が宙ぶらりんにならない）
- [ ] `Engine::reset` で `sustain_state.reset()` を呼ぶ
- [ ] テスト追加:
  - [ ] `tests/sustain_tests.rs` に `test_sustain_defers_note_off` / `test_sustain_release_on_off` / `test_sustain_passthrough_when_inactive` / `test_sustain_clear_pending_on_retrigger`（`clear_pending` 動作）/ `test_sustain_reset_clears_active_and_pending`（CC#123 シナリオ用）/ `test_sustain_pending_release_bitmap_readonly`（mode 切替用 read-only API）
  - [ ] `tests/dsp_core_tests.rs` に `test_engine_midi_cc_volume`（CC#7 で `channel_volume` target が変わり `output_gain` は変わらない、D38b 直交確認）/ `test_engine_midi_cc_volume_multiplied_in_output`（積算確認）/ `test_engine_midi_cc_sustain_defers` / `test_engine_midi_cc_sustain_clears_pending_on_retrigger`（再打鍵後にまだ離していないので CC#64 off で release されない、P1-3 対策）/ `test_engine_midi_cc_all_notes_off_clears_sustain`（P1-1 対策）/ `test_engine_mono_sustain_no_op`（Mono mode では Sustain 無視、Phase 2 既存挙動継承、P1-2 解決）/ `test_engine_mode_switch_clears_sustain`（Poly + pending → set_mode(Mono) で pending 全 release、P2-1 対策）/ `test_engine_mode_switch_no_pending_passes_through`（pending なしの set_mode は Phase 2 既存挙動と等価、regression なし）/ `test_engine_midi_cc_unknown_ignored`（CC#1 や未対応 CC で panic なし）
- [ ] `cargo test -p dsp-core` がすべてパス
- **検証**: F31 / F33 達成

### フェーズ ζ — Voice State / UI（3 ステップ）

#### Step 10. C ABI 3 関数追加 + REQUIRED 配列更新

- [ ] `crates/wasm-audio/src/lib.rs` に `synth_midi_cc` / `synth_pitch_bend` / `synth_voice_state_ptr` の 3 関数を追加（[`04-wasm-audio-spec.md` §Phase 3 で追加する C ABI 関数](./04-wasm-audio-spec.md#phase-3-で追加する-c-abi-関数)）
- [ ] `crates/dsp-core/src/engine.rs` に `voice_state_buffer: [u8; 33]` を追加、`Engine::process` 終端で書き込み（active mask + 8 振幅 little-endian）
- [ ] `Engine::voice_state_ptr() -> *const u8` を追加
- [ ] `crates/dsp-core/src/voice_pool.rs` に `voice_state(&self) -> VoiceStateSnapshot` を追加（D41）
- [ ] `scripts/check-wasm-exports.mjs` の REQUIRED 配列に `synth_midi_cc` / `synth_pitch_bend` / `synth_voice_state_ptr` を追加
- [ ] `pnpm build:wasm` で 3 関数がすべて export されることを確認（exit 0）
- [ ] テスト追加:
  - [ ] `tests/dsp_core_tests.rs` に `test_engine_voice_state_buffer_format`（33 byte レイアウト検証）
- [ ] `cargo test -p dsp-core` がすべてパス
- **検証**: F31 / F38 の準備完了

#### Step 11. messages.ts / WasmExports / SynthEngine 拡張

- [ ] `web/src/lib/audio/messages.ts` の `ToWorkletMessage` に `midiCC` / `pitchBend` variant を追加（[`05-web-frontend-spec.md` §messages.ts の Phase 3 変更点](./05-web-frontend-spec.md#messagets-の-phase-3-変更点)）
- [ ] `messages.ts` の `FromWorkletMessage` に `voiceState` variant を追加
- [ ] `web/src/lib/audio/synth-processor.ts` の `WasmExports` interface に 3 関数を追加
- [ ] `synth-processor.ts` の `init` で `voice_state_ptr` を取得 + `Uint8Array` view をキャッシュ
- [ ] `synth-processor.ts` の `process` で 1024 サンプル毎に Voice State を main へ push（[`05-web-frontend-spec.md` §Voice State stride push](./05-web-frontend-spec.md#voice-state-stride-pushd41)）
- [ ] `synth-processor.ts` の message dispatch に `midiCC` / `pitchBend` case を追加
- [ ] `web/src/lib/audio/voice-state.svelte.ts` を新規作成（`VoiceState` クラス + `voiceState` インスタンス export）
- [ ] `web/src/lib/audio/engine.ts` に `sendMidiCc(cc, value)` / `sendPitchBend(semitones)` / `onVoiceState(msg)` を追加
- [ ] `engine.ts` の port message handler で `voiceState` 受信時に `voiceState.activeMask = msg.activeMask; voiceState.amplitudes = msg.amplitudes` を反映
- [ ] `pnpm --filter web check` がパス
- [ ] `pnpm dev` でブラウザ起動、F34 を満たす（VoiceMeter なしでも console で `voiceState.activeMask` が更新されること）
- **検証**: F32 (経路) 達成、F34 の準備完了

#### Step 12. VoiceMeter / PolyphonyToggle / midi-cc.ts UI 実装（F34 / D42）

- [ ] `web/src/lib/components/VoiceMeter.svelte` を [`05-web-frontend-spec.md` §VoiceMeter](./05-web-frontend-spec.md#voicemeter-コンポーネント-voicemetersvelte) に従い作成
- [ ] `web/src/lib/components/PolyphonyToggle.svelte` を [`05-web-frontend-spec.md` §PolyphonyToggle](./05-web-frontend-spec.md#polyphonytoggle-コンポーネント-polyphonytogglesvelte) に従い作成
- [ ] `web/src/lib/state/ui.svelte.ts` に `polyphonyMode = $state<'poly' | 'mono'>('poly')` を追加
- [ ] `web/src/lib/input/midi-cc.ts` を [`05-web-frontend-spec.md` §WebMIDI CC handler](./05-web-frontend-spec.md#webmidi-cc-handler-midi-ccts) に従い作成
- [ ] `web/src/lib/components/MidiSelect.svelte` の MIDI message handler を `handleMidiMessage(e, engine)` に置換
- [ ] `web/src/routes/+page.svelte` に `<VoiceMeter />` と `<PolyphonyToggle {engine} />` を Header 直下に配置
- [ ] `+page.svelte` の `<section class="params">` に `<ParamSlider id={PARAM_IDS.PickPosition} />` と `<ParamSlider id={PARAM_IDS.BodyWet} />` を追加
- [ ] `pnpm --filter web check` / `pnpm --filter web lint` がパス
- [ ] `pnpm dev` でブラウザ起動、以下を確認:
  - [ ] VoiceMeter が表示され、PC キーボード 8 鍵同時押下で全 8 セルが active 表示
  - [ ] PolyphonyToggle で mono に切り替えると、複数キー押下で last-note priority が動作
  - [ ] PickPosition / BodyWet スライダーで音色が変化
  - [ ] MIDI キーボード接続時、Pitch Bend / Sustain Pedal / Channel Volume が動作（Mod Wheel は Phase 4 送りのため対象外）
- **検証**: F34 達成、D42 達成

### フェーズ η — 統合検証 + ビルド確認（2 ステップ）

#### Step 13. 統合 cargo test + alloc ゼロ検証 + サイズ計測 + **release timing test 必須**（F36 / F37 / F38）

- [ ] `tests/voice_pool_tests.rs` に `test_no_allocation_with_modal_body_and_midi_cc` を追加（[06 章 §F38](./06-build-and-verify.md#f38メモリ確保ゼロの詳細手順)）
- [ ] **`tests/dsp_core_tests.rs` に `test_engine_process_block_timing` を追加**（[06 章 §F37 詳細手順](./06-build-and-verify.md#f37process-時間の詳細手順phase-3-で必須化)）
  - [ ] `#[cfg(not(debug_assertions))]` で release ビルド限定
  - [ ] 8 voice 全 active + Pitch Bend + CC#7 を設定して最悪ケース近似
  - [ ] `Instant::now()` で 1000 回 process の平均時間を計測
  - [ ] `assert!(per_block_ms < 1.5)` で完成条件を強制（CI で flaky なら `< 2.0` に緩めるが、目標は 1.5）
- [ ] **`cargo test --release -p dsp-core` で全テスト**（Phase 2 既存 41 + Phase 3 新規 30 = 71 件目標）がパス、特に F37 timing test が release で通ること
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` がパス
- [ ] `pnpm build` で本番ビルド成功
- [ ] WASM gzip サイズ計測（[06 章 §性能目標](./06-build-and-verify.md#性能目標phase-3)）:
  ```bash
  # Git Bash で
  gzip -kc web/build/_app/immutable/assets/wasm_audio.*.wasm | wc -c
  ```
  - [ ] gzip < 30 KB（target、想定 12.9 KB）
  - [ ] サイズが超過した場合 `wasm-opt -O3` 適用（リスク R29）→ 再計測、または Modal Body M=8→5 に削減
- [ ] Worklet 本番バンドルサイズ計測（< 10 KB target）:
  ```powershell
  Get-ChildItem web\build\_app\immutable\assets\synth-processor*.js | Select-Object Name, Length
  ```
- [ ] `__synthDev` が production bundle に 0 hits（grep で検証、F22 拡張）:
  ```bash
  grep -r "__synthDev" web/build/_app/immutable/ | wc -l
  # 期待値: 0
  ```
- **検証**: F36 / F38 達成、Phase 2 の F22 相当も維持

#### Step 14. ドキュメント整備 + retrospective 準備（F37 含む）

- [ ] `README.md` を Phase 3 用に更新（Body Resonator / Extended KS / MIDI CC / Voice Meter を明記、F26〜F38 の自己検証手順を追記）
- [ ] `CLAUDE.md` の「現在のイテレーション」を Phase 3 完了に更新、「次は Phase 4 候補」を追記
- [ ] Step 1 の試作結果を反映した `01-overview.md` の D36 が最新化されていることを再確認
- [ ] 仕様書群 `docs/specs/2026-05-07-003-phase3/` の各章でリンク切れがないか確認
- [ ] retrospective テンプレートを準備（[`/retrospective` カスタムコマンド](../../.claude/commands/retrospective.md) を Phase 3 完了後に発火）
- [ ] F37 の release cargo timing test が Step 13 で達成済み（Rust DSP 内部の timing）
- [ ] **F38b（Chrome DevTools Performance タブで Worklet process 時間の実機計測）を実施**: `pnpm build && pnpm preview` → Chrome で 8 voice + Body + Pitch Bend 動作中に 10 秒 Record → "Audio Worklet" レーンで process Self time avg/max を確認、avg < 1.5 ms かつ max < 2.5 ms を達成（[06 章 §F38b](./06-build-and-verify.md#f38bworklet-process-時間の実機計測phase-3-完成後必須)）。超過時は R30 対策（stride 4096 化 / Voice Meter 削除 / SAB 化検討）を適用
- [ ] PR 作成（`gh pr create`）、CI 緑を確認、main ブランチへマージ
- **検証**: Phase 3 完成、Phase 4 への申し送りが文書化される

## ステップごとの依存関係

```
Step 1 (Thiran 試作)
  └─ D36 確定
      ▼
Step 2 (params.json + gen-params.mjs 拡張)
  └─ Step 3 (modal_body.rs + Engine 統合, F26)
       ▼
   Step 4 (loss_filter.rs + KS 統合, F27)
     ─ Step 5 (Pick position 励振 shaping in note_on, F28)
       ▼
   Step 6 (Brightness 群遅延補正, F30)
     ─ Step 7 (soft_clip.rs + Engine 統合, F35)
       ▼
   Step 8 (Voice trait 拡張 + Pitch Bend, F32)
     ─ Step 9 (MIDI CC + Sustain, F31/F33)
       ▼
   Step 10 (C ABI 3 関数 + REQUIRED 更新)
     ─ Step 11 (messages.ts + WasmExports + SynthEngine)
       ─ Step 12 (VoiceMeter / PolyphonyToggle / midi-cc.ts UI, F34)
         ▼
   Step 13 (統合 test + サイズ計測, F36/F38)
     ─ Step 14 (ドキュメント + retrospective 準備)
```

並列実装可能なポイント:

- Step 4（Loss filter）と Step 5（Pick position）は KarplusStrong の修正範囲が異なる（filter は signal chain、pick は note_on 時の K 設定）が、`process_sample` 内の挿入位置が近いため **直列が望ましい**
- Step 6（Brightness 補正）と Step 7（Soft clip）は独立、並列可能
- Step 8（Pitch Bend）と Step 9（MIDI CC dispatch）は依存（Pitch Bend は CC dispatch とは別経路だが Voice trait 拡張の commit 順は Pitch Bend が先）
- Step 10〜12 は経路上 直列（C ABI → Worklet → UI）

## 達成ライン早見表

| ステップ完了 | 達成する検証項目 |
|---|---|
| Step 1 | F29（Thiran 試作評価、D36 確定） |
| Step 3 | F26（Modal Body Resonator） |
| Step 4 | F27（Loss filter） |
| Step 5 | F28（Pick position） |
| Step 6 | F30（Brightness 補正） |
| Step 7 | F35（Soft clip） |
| Step 8 | F32（Pitch Bend） |
| Step 9 | F31 / F33（MIDI CC / Sustain） |
| Step 12 | F34（Voice Meter UI） |
| Step 13 | F36 / F37 / F38（サイズ + release cargo timing + alloc ゼロ） |
| Step 14 | F38b（Chrome DevTools Performance タブで Worklet process 実測）+ Phase 3 完成、Phase 4 へ |

すべての F26〜F38 + F38b が達成された時点で Phase 3 完成。F37 は **Step 13 で release cargo timing test を必須化**（Rust DSP 内部）、F38b は **Step 14 で実機計測を必須化**（Worklet 全体の process 時間、postMessage 込み）。両方とも持ち越し不可。

## 実装着手者へのメモ

- **Step 1（Thiran 試作）が最重要**。ここで D36 を確定させないと Step 6 / Step 8 の実装方針が定まらない。試作結果を 01 章 D36 に反映する仕様書改訂を Step 1 完了時にコミット
- **Step 3 (Modal Body) が音色面で最大のインパクト**。係数（pre-research §2.3 のギターボディ）が想定通り共鳴するか、F26 の cargo test で確認後に **dev ビルドで A4 を弾いて聴感確認**を推奨（数学的に正しくても聴感が「ボディが鳴っている」と感じるかは別軸）。違和感があれば `params.json` の `body_modes` の Q や gain を調整
- **Step 4（Loss filter）** は KarplusStrong の `process_sample` 内、brightness LPF 後・damping 前に 1 段挿入。**Step 5（Pick position）** は process_sample には入れず、`note_on` 内で励振 shaping を実装（D34 設計変更版、feedback loop 内 comb は不採用）
- **Step 6（Brightness 補正）**は 1 行追加だが、Phase 2 の `test_pitch_a4` の許容範囲が狭まる可能性。テスト調整が必要かもしれないが、本来の方向性として「ピッチ精度向上」なので閾値はむしろ厳しくする
- **Step 7（Soft clip）の区間関数型実装**: `|x| ≤ 0.95` で `assert_eq!(soft_clip(x), x)` が成立する（誤差ゼロ・linear）、`|x| → ∞` で出力 ±1.0 に厳密漸近。`tanh` Padé 近似は使わない（旧版仕様で発散問題があった、撤回済）
- **Step 8 の Pitch Bend `length_target` SmoothedValue 化**: process_sample 内で毎サンプル `next_sample()` を呼ぶが、定常時（target = current）は係数再計算をスキップする実装が必須（R26 対策）。`length_frac` の差分が `< 1e-5` なら skip
- **Step 9 の Sustain pending bitmap**: u128 で 128 MIDI ノート全件を管理。`set_active(false)` で全 pending を release する際、`for note in 0..128` のループ内で `Engine::note_off(note)` を呼ぶが、これは **`sustain_state.active = false` の後に呼ぶ**こと（active=true のままだと defer が再度発火する）
- **Step 10 の C ABI 3 関数追加**: `synth_voice_state_ptr` だけ `*const SynthHandle` を取り `*const u8` を返す（mut 不要）。他 2 関数は `*mut SynthHandle`。`#[unsafe(no_mangle)] pub extern "C" fn` を全関数に付ける
- **Step 11 の Voice State view detach**: `memory.buffer` が grow すると view が detach される（D9）。`process` 内で `byteLength === 0` をチェックし、必要なら `refreshVoiceStateView()` を呼ぶ。Phase 3 では `synth_new` 完了後 grow しない設計だが防御的に
- **Step 12 の VoiceMeter スタイル**: 8 セル × 12px グリッドが UI 横幅 96px を取る。Header の他要素との競合に注意。CSS `gap: 2px` 含めると 110px
- **Step 13 のサイズ予算超過時のフォールバック**: gzip > 30 KB なら `wasm-opt -O3` を `scripts/copy-wasm.mjs` に追加、それでも超えるなら Modal Body M=5 に削減（pre-research §9.2 早期撤退ライン）
- **Step 14 の retrospective**: `/retrospective 2026-05-XX-003-phase3` カスタムコマンドで自動生成。retrospective §3 で D30〜D43 の評価、§4 で躓きと教訓、§7 で Phase 4 候補を整理する流れ
- **各ステップで コミットを分ける** ことを推奨（Phase 1 / 2 と同じ）。問題発生時に二分探索しやすい。コミットメッセージは `feat(modal-body): step 3 - implement modal body resonator (F26)` のように Step 番号と F-tag を含める
- **Phase 2 既存テスト 41 件が各ステップで壊れないこと**を最優先。Step 1 / 6 / 8 / 9 で KarplusStrong / Engine を大きく書き換えるため、各 Step 完了時に `cargo test -p dsp-core` を回す
- 詰まったら [`06-build-and-verify.md` §トラブルシューティング](./06-build-and-verify.md#トラブルシューティング-tips) を参照
