# 07. Phase 4c 実装順序チェックリスト

## 目的

Phase 4c 仕様書承認後、本仕様を実装する際の作業順序を Phase α〜κ の 10 フェーズ・全 22 ステップで定義する。各ステップは独立して進捗確認でき、検証チェックリスト（F59〜F70 + Phase 4a / 4b 既存）の充足ポイントを明示する。Phase 1 / 2 / 3 / 4a / 4b の `07-implementation-checklist.md` と同じ粒度（1 ステップ ≈ 1 コミット）で構成する。

## 他文書との関係

- 上流: 全ての仕様書（pre-research、01〜06）
- 参考: Phase 1 / 2 / 3 / 4a / 4b [07 章] — **Phase 4c ステップは Phase 4b の構成パターンを踏襲**
- このドキュメントは **実装専用のチェックリスト** であり、各ステップの設計詳細は対応する仕様書を参照する

## 前提条件

Phase 4c 実装着手前に以下を確認:

1. **Phase 4b の 148 PASS + 1 IGNORED テストがすべて維持されている**
   - `cargo test -p dsp-core` で確認
2. **`pnpm dev` でブラウザでの 8 音ポリフォニック + Modal Body + LFO + Mod Wheel + Preset + 8 楽器プリセット (Piano 含む) が動作確認できる状態**
3. **Phase 4b の retrospective が完了している**（`docs/retrospective/2026-05-09-005-phase4b.md`）
4. **Phase 4c の主目的を「本格ピアノ音色」に確定するユーザー承認が完了**（pre-research §12 #1）
5. **`__synthDev.measureProcessTime` API が動作する状態（Phase 4b 完成済）**

## 実装ステップ（全 22 段階、Phase α〜κ の 10 フェーズ）

---

### フェーズ α — 前処理とベースライン取得（1 ステップ）

#### Step 1. `.gitattributes` 再確認 + F38b 実機ベースライン取得（D85、F70-a）

- [ ] `.gitattributes` が Phase 4b で確定した内容で残っていることを確認（変更なし、Phase 4b で確立）
- [ ] `pnpm dev` を起動、ブラウザで `http://localhost:5173/` を開く
- [ ] Start ボタン → Piano プリセット選択
- [ ] Console で `await window.__synthDev.measureProcessTime(5000)` を実行
- [ ] 出力された avg / max / min を Phase 4b 想定値 (avg ≈ 0.047 ms / max ≈ 0.063 ms) と照合
- [ ] Default プリセットでも同様に測定、avg ≈ 0.029 ms を確認
- [ ] 結果を `docs/specs/2026-05-13-006-phase4c/baseline-phase4b.md`（新規）に記録（Phase 4c retrospective §8 で参照）
- [ ] **検証**: F70-a 達成

**コミット例**: `chore: record Phase 4b F38b baseline (Piano 0.047ms / Default 0.029ms) before Phase 4c`

---

### フェーズ β — params.json と gen-params.mjs 拡張（2 ステップ）

#### Step 2. `params.json` 拡張: Piano に Phase 4c フィールド追加

- [ ] `params.json` の Piano エントリ (`instruments.piano`) に以下を追加:
  - `unison_detune_cents: 1.5` (D72)
  - `sympathetic_amount: 1.0` (D77、ペダル ON 時 full 効果)
  - `hammer_cutoff_low_hz: 800` (D75、Phase 4b 同値)
  - `hammer_cutoff_high_hz: 5500` (D75、Phase 4b 4000 → 5500 拡張)
  - `inharmonicity_b_curve: [88 値]` (D78 / D79、概数で OK、Step 18 で精密化)
- [ ] Phase 4b 既存の `inharmonicity_b: 7.5e-4` は維持（Phase 4b 互換性のため、新規呼出は curve を使用）
- [ ] Default / Guitar / Ukulele / Mandolin / Bass / GuitarSteel / Sitar の 7 楽器は変更なし
- [ ] JSON schema 検証 (`pnpm check`) で構文 OK

**コミット例**: `feat(params.json): add Piano Phase 4c fields (unison_detune / sympathetic / b_curve / hammer cutoff)`

#### Step 3. `gen-params.mjs` 拡張: Phase 4c 定数を出力

- [ ] `scripts/gen-params.mjs` を拡張:
  - `UNISON_DETUNE_CENTS_PIANO: f32` を Rust const 出力 + TS export 出力
  - `SYMPATHETIC_AMOUNT_PIANO: f32` 同上
  - `HAMMER_CUTOFF_LOW_PIANO: f32` / `HAMMER_CUTOFF_HIGH_PIANO: f32` 同上
  - `INHARMONICITY_B_CURVE_PIANO: [f32; 88]` を `#[rustfmt::skip]` 付きで Rust const 出力 + TS export (ReadonlyArray<number>) 出力
- [ ] `crates/dsp-core/src/params.rs` (生成) に上記が反映されている
- [ ] `web/src/lib/audio/generated/params.ts` に上記が反映されている
- [ ] `#![allow(clippy::approx_constant)]` (module 先頭) を維持（Phase 4a feedback memory）
- [ ] `crates/dsp-core/src/dispersion.rs` に `b_curve_piano(midi)` / `b_curve_zero(midi)` ヘルパを追加（`midi.clamp(21, 108)` で範囲外端値 fallback）
- [ ] `cargo build -p dsp-core` 緑、`pnpm check` 緑
- [ ] **検証**: F67-a (`test_b_curve_length_88`) / F67-f (`test_b_curve_clamps_out_of_range`) 暫定 pass

**コミット例**: `feat(gen-params): emit Phase 4c constants (UNISON_DETUNE / SYMPATHETIC / B_CURVE / HAMMER_CUTOFF) + b_curve helpers`

---

### フェーズ γ — Multi-string 基盤実装（2 ステップ）

#### Step 4. `karplus_strong.rs`: 構造体 + StringState 追加 + set_instrument_params

- [ ] `crates/dsp-core/src/karplus_strong.rs` を拡張:
  - `const MAX_STRINGS_PER_VOICE: usize = 3;` を追加
  - `StringState` 構造体を追加（`write_idx` / `length` / `length_int` / `fractional` / `thiran` / `dispersion_stages: [DispersionStage; 8]`）
  - `KarplusStrong` に以下フィールド追加:
    - `string_buffers: [Vec<f32>; MAX_STRINGS_PER_VOICE]`
    - `string_states: [StringState; MAX_STRINGS_PER_VOICE]`
    - `n_strings_active: usize`
    - `unison_detune_cents: f32` / `inharmonicity_b: f32` / `hammer_cutoff_low_hz: f32` / `hammer_cutoff_high_hz: f32`（楽器パラメータ保持用）
    - `bus_feedback_pending: f32`（Sympathetic 注入用、process_sample 内で消費）
- [ ] Phase 4b 既存の `buffer: Vec<f32>` / `dispersion_stages: [DispersionStage; 8]` / `thiran: ThiranCoeffs` を **`string_buffers[0]` / `string_states[0].dispersion_stages` / `string_states[0].thiran` に統合**（中央弦が既存経路に対応）
- [ ] `KarplusStrong::new()` / `prepare()` / `reset()` で `string_buffers` の 3 本を一括確保
- [ ] `set_instrument_params(unison_detune_cents, inharmonicity_b, hammer_cutoff_low_hz, hammer_cutoff_high_hz)` メソッドを追加（Engine から VoicePool 経由で呼ばれる、§1.4 参照）
- [ ] `inject_feedback(value: f32)` メソッドを追加（`bus_feedback_pending = value` を設定）
- [ ] **Voice trait の note_on(freq_hz, velocity) / note_off / process_sample / is_active は完全に維持**。`note_on_with_id(midi_note, freq_hz, velocity)` も Phase 4b 同等のシグネチャを維持
- [ ] **チェック**: heap 確保は `prepare()` のみ、`process_sample` の alloc は依然ゼロ
- [ ] `cargo test -p dsp-core` で Phase 4b 既存 148 PASS が依然全て通ること

**コミット例**: `feat(karplus_strong): introduce StringState + string_buffers + set_instrument_params for multi-string per voice`

#### Step 5. `karplus_strong.rs`: `n_strings(midi)` / detune 関数 + note_on_internal 拡張（公開 API は維持）

- [ ] `n_strings(midi: u8) -> usize` 関数を追加（21..=33 → 1, 34..=47 → 2, 48..=108 → 3、範囲外は端 region で fallback、D69）
- [ ] `string_detune_cents(string_idx, n_strings, base_cents) -> f32` を追加（D72）
- [ ] `note_on_internal(note_id, freq_hz, velocity)` を内部実装拡張（公開 API は Voice trait 互換維持）:
  - `n_strings_active = if self.dispersion_active && note_id.is_some() { n_strings(note_id.unwrap()) } else { 1 };`
  - 各弦のループで `detune` を適用 → `f_0_string = f_0_base × 2^(detune/1200)`
  - 弦個別の dispersion 係数: `let (a1, gd_per_stage) = compute_dispersion_a1(M, self.inharmonicity_b, f_0_string, fs)`（**tuple 戻り値、現行実装 `karplus_strong.rs:201` と同型**）
  - 弦個別 `adjusted_length = raw_len - brightness_tau_g - M·gd_per_stage`
  - 各弦の `state.thiran.set_fractional(fractional)` / `state.write_idx = 0` 設定
  - 各弦の buffer に Hertz hammer or pluck excitation を初期化（Step 6 で実装）
- [ ] **検証**:
  - F59-a〜d (`test_n_strings_for_midi` / `test_string_detune_cents_*`) 全て pass
  - F60-a〜d (`test_piano_n_strings_*_at_*` / `test_default_kind_always_1_string`) 全て pass

**コミット例**: `feat(karplus_strong): add n_strings/detune helpers + multi-string note_on_internal`

---

### フェーズ δ — Hertz law raised cosine hammer（1 ステップ）

#### Step 6. `karplus_strong.rs`: Hertz hammer 実装 (D74 / D75)

- [ ] `KarplusStrong::init_hammer_impulse_for_string(&mut self, string_idx, velocity)` を実装（cutoff は `self.hammer_cutoff_low_hz` / `self.hammer_cutoff_high_hz` から参照、Step 4 で導入したフィールド）:
  - `t_c_ms = 4.0 - 2.8 * velocity` (D75)
  - `t_c_samples = (t_c_ms * 0.001 * sample_rate) as usize`
  - `f_c_hz = self.hammer_cutoff_low_hz + velocity * (self.hammer_cutoff_high_hz - self.hammer_cutoff_low_hz)`
  - `amplitude = velocity.sqrt()` (perceptual loudness)
  - buffer zero clear → raised cosine sin² で接触時間表現 → velocity LPF 適用
- [ ] `init_pluck_excitation_for_string(string_idx, velocity)` を分離（Phase 4a / 4b の noise burst + pick comb 経路、非 Piano 用）
- [ ] `init_excitation_for_string(string_idx, velocity)` を導入し `if self.dispersion_active { init_hammer_impulse_for_string } else { init_pluck_excitation_for_string }` 分岐
- [ ] `note_on_internal` の各弦ループ末尾で `self.init_excitation_for_string(string_idx, velocity)` を呼ぶ
- [ ] Phase 4b の `init_excitation_hammer` 既存関数は削除 or 内部実装更新
- [ ] **検証**:
  - F66-a〜f (`test_hammer_t_c_decreases_with_velocity` 〜 `test_hammer_pluck_path_for_default`) 全て pass

**コミット例**: `feat(karplus_strong): replace Phase 4b hammer with Hertz law raised cosine impulse`

---

### フェーズ ε — Multi-string process_sample（1 ステップ）

#### Step 7. `karplus_strong.rs`: `process_sample` の N 弦並列化（bus_feedback_pending 経由）

- [ ] `KarplusStrong::process_sample` を拡張:
  - `for string_idx in 0..self.n_strings_active` ループで各弦を処理
  - 弦個別の `string_buffers[string_idx]` / `string_states[string_idx]` を参照
  - 各弦で dispersion cascade → Thiran → `damping * y + self.bus_feedback_pending` を write back
  - 全弦の出力を `sum_strings` に加算、`brightness_lpf + loss_filter` を共有
  - ループ末尾で `self.bus_feedback_pending = 0.0` にクリア（次 sample で Engine から再注入）
- [ ] `inject_feedback(value)` メソッドは Step 4 で追加済を確認（`bus_feedback_pending = value`）
- [ ] **検証**:
  - F61-a (`test_default_n_strings_1_matches_phase4a`) pass: **Phase 4a HEAD と byte 一致継承**（D83）
  - F61-c (`test_guitar_classical_phase4b_byte_match`) pass: Phase 4b との byte 一致
  - F61-d (`test_all_non_piano_kinds_n_strings_1`) pass
  - F62-a〜c (`test_string_detune_produces_beating` 〜 `test_two_stage_decay_observation`) 暫定 pass
  - F63 (`test_no_allocation_in_process_multi_string`) pass

**コミット例**: `feat(karplus_strong): parallelize process_sample across N strings using bus_feedback_pending`

---

### フェーズ ζ — Dispersion B 引数化と B(note) LUT 連携（1 ステップ）

#### Step 8. `dispersion.rs` の B 引数化 + Engine note_on で LUT 連携

- [ ] `crates/dsp-core/src/dispersion.rs` の `compute_dispersion_a1` シグネチャを Phase 4b 同型 **`(m: u32, b: f32, f_0: f32, fs: f32) -> (f32, f32)`** で維持確認（戻り値は `(a1, gd_per_stage)` tuple、現行 `karplus_strong.rs:201` の使い方そのまま）
- [ ] `dispersion.rs` に `b_curve_piano(midi: u8) -> f32` / `b_curve_zero(midi: u8) -> f32` を追加（Step 3 で導入済を確認）。`b_curve_piano` は `midi.clamp(21, 108) - 21` を index に LUT を引く
- [ ] `crates/dsp-core/src/engine.rs` の `Engine` 構造体に以下フィールド追加:
  - `unison_detune_cents: f32`
  - `sympathetic_amount: f32`
  - `inharmonicity_b_for_note: fn(u8) -> f32`
  - `hammer_cutoff_low_hz: f32` / `hammer_cutoff_high_hz: f32`
  - `bus_out_prev: f32`（Sympathetic 用、Step 10 で利用）
- [ ] `apply_instrument(kind)` を Phase 4b 既存 (`engine.rs:314-326`) を完全継承した上で Phase 4c 追加処理を入れる:
  - 既存: `pool.all_notes_off()` / `hold_stack.clear()` / **`sustain_state.reset()`** / `current_instrument = kind` / `stereo_spread` / `modal_body.set_instrument` / `pool.set_dispersion_active(matches!(kind, Piano))`
  - Phase 4c 追加: `unison_detune_cents` / `sympathetic_amount` / `inharmonicity_b_for_note` (関数ポインタ) / `hammer_cutoff_low_hz` / `hammer_cutoff_high_hz` を Piano / 非 Piano で切替
  - Phase 4c 追加: `self.pool.set_piano_params(detune, 0.0, cutoff_low, cutoff_high)`（`inharmonicity_b` はプレースホルダ 0、note_on 時に再 lookup）
  - Phase 4c 追加: `self.resonance_bus.set_feedback_gain_target(0.0)` （`sustain_state.reset()` 済みなので無条件 0、03 章 §4.2 整合）
- [ ] `Engine::note_on(midi, velocity)` の **公開 API 構造は Phase 4b 同型** (`engine.rs:129-147`) を完全継承し、内部の `trigger_voice` のみ Phase 4c で差し替え:
  - 既存維持: `sustain_state.clear_pending(midi)` → Mono 分岐 (`hold_stack.push_unique` + 前 top release) → `trigger_voice(midi, velocity)`
  - `trigger_voice` の中身を:
    ```rust
    let b = (self.inharmonicity_b_for_note)(midi_note);
    let freq = midi_to_freq(midi_note);
    let assigned = self.pool.note_on_with_piano_params(
        midi_note, freq, velocity,
        self.unison_detune_cents, b,
        self.hammer_cutoff_low_hz, self.hammer_cutoff_high_hz,
    );
    self.pool.set_damping_voice(assigned, self.current_damping);  // Phase 4b 同型
    ```
    に差し替え。`note_off` 経路 (Mono の revive `trigger_voice(top, MONO_REVIVE_VELOCITY)`) も同じ差し替えで透過動作（03 章 §4.3）
- [ ] `crates/dsp-core/src/voice_pool.rs` に `set_piano_params(detune, b, cutoff_low, cutoff_high)` / `note_on_with_piano_params(midi, freq, vel, detune, b, cutoff_low, cutoff_high) -> usize` を追加（[`03-dsp-core-spec.md` §5.1 / §5.2](./03-dsp-core-spec.md#51-楽器パラメータ-fan-out)）。`voices` は private のまま、既存 3 段フォールバック (same-note replace / free voice / steal、`voice_pool.rs:47-60`) を `allocate_voice` ヘルパに切り出して共通化
- [ ] **検証**:
  - F67-a〜h (`test_b_curve_*`) 全て pass、特に F67-f (`test_b_curve_clamps_out_of_range`) で 0 / 127 の範囲外動作確認

**コミット例**: `feat(engine): wire B(note) LUT via instrument-specific fn ptr + replace trigger_voice with note_on_with_piano_params`

---

### フェーズ η — Sympathetic resonance bus（2 ステップ）

#### Step 9. `resonance_bus.rs` 新規実装（D76）

- [ ] `crates/dsp-core/src/resonance_bus.rs` を新規作成
- [ ] `ResonanceBus` 構造体定義（buffer / lpf / feedback_gain / write_idx / sample_rate）
- [ ] `new()` / `prepare(sample_rate)` / `reset()` / `set_feedback_gain_target(target)` / `process(bus_in) -> f32` / `next_feedback_gain() -> f32` を実装
- [ ] `BUS_DELAY_MS = 2.0`、`BUS_INTERNAL_DECAY = 0.95`、`FEEDBACK_GAIN_MAX = 0.05` を const 定義
- [ ] `crates/dsp-core/src/lib.rs` に `pub mod resonance_bus;` 追加
- [ ] **検証**:
  - F64-a〜d (`test_resonance_bus_*`) 全て pass

**コミット例**: `feat(dsp-core): add resonance_bus module for sympathetic resonance`

#### Step 10. `engine.rs` への `ResonanceBus` 統合 + `handle_midi_cc(CC#64)` 経路拡張（D77）+ VoicePool::process_sample_with_feedback

- [ ] `Engine` 構造体に `resonance_bus: ResonanceBus` を追加（`bus_out_prev: f32` は Step 8 で追加済を確認）
- [ ] `Engine::prepare(sample_rate)` で `resonance_bus.prepare(sample_rate)` 呼出 + `bus_out_prev = 0.0`
- [ ] `Engine::reset()` で `resonance_bus.reset()` 呼出 + `bus_out_prev = 0.0`
- [ ] `crates/dsp-core/src/voice_pool.rs` に `process_sample_with_feedback(bus_out_prev, feedback_gain) -> f32` を追加（[`03-dsp-core-spec.md` §5.3](./03-dsp-core-spec.md#53-sympathetic-bus-と連動した-process_sample_with_feedback)）:
  - 内部で `inject = bus_out_prev * feedback_gain` を計算し、各 voice に `inject_feedback(inject)` を呼んでから `process_sample()` で合算
  - 戻り値は **`sum * poly_scale`**（Phase 2 D20 / `voice_pool.rs:149` の 1/√N スケール維持）
  - 既存の `process_sample()` メソッドは削除せず維持（テスト互換性のため）
- [ ] `Engine::process(&mut self, output_l, output_r)` の **既存 per-sample loop 内で 3 行のみ書き換える**（既存ブロック関数 `engine.rs:445-492`、03 章 §4.4 と完全一致）:
  - **(a)** `let dry = self.pool.process_sample();` を:
    ```rust
    let feedback_gain = self.resonance_bus.next_feedback_gain();
    let dry = self.pool.process_sample_with_feedback(self.bus_out_prev, feedback_gain);
    ```
    に置換（poly_scale は内部適用済、戻り値の意味は Phase 4b と同型）
  - **(b)** 上記の直後に:
    ```rust
    let bus_out = self.resonance_bus.process(dry);
    self.bus_out_prev = bus_out;
    ```
    を追加
  - **(c)** `let (body_l, body_r) = self.modal_body.process_sample(dry);` を:
    ```rust
    let bus_mix = feedback_gain * (1.0 / FEEDBACK_GAIN_MAX);  // 0..=1、SmoothedValue 同期で滑らかに gate
    let (body_l, body_r) = self.modal_body.process_sample(dry + bus_out * BUS_DIRECT_MIX_GAIN * bus_mix);
    ```
    に変更（**Default kind や Piano + Sustain OFF では `feedback_gain = 0` で `bus_mix = 0` → modal_body 入力は `dry + 0.0` で Phase 4b と byte 一致**、F61-e / F65-g の検証根拠）
- [ ] **既存責務を完全維持**（リグレッション防止のため明示）:
  - LFO 値 / Mod Wheel / pitch_factor / brightness_offset / volume_multiplier の per-sample 計算
  - `body_wet.next_sample()` での dry/wet ミックス
  - `output_gain.next_sample() * channel_volume.next_sample() * volume_multiplier` の combined gain 計算
  - `soft_clip(...)` **関数呼出**（`SoftClip` 型ではない）
  - `voice_state_sample_counter` の stride 制御 + `write_voice_state()` 呼出
- [ ] **`Engine::set_sustain` 独立メソッドは新設しない**。現行 `handle_midi_cc(CC_SUSTAIN_PEDAL, v)` (`engine.rs:222-226`) を拡張する（03 章 §4.5 と完全一致）:
  - Phase 3 D40 既存（**絶対に維持**）:
    ```rust
    let released = self.sustain_state.set_active(v >= 0.5);
    self.release_pending(released);
    ```
    戻り値 bitmap を `release_pending` に渡す経路を落とすと Sustain OFF 時の保留 note_off が解放されない
  - Phase 4c 追加:
    ```rust
    let on = v >= 0.5;
    let target_gain = if matches!(self.current_instrument, InstrumentKind::Piano) && on {
        self.sympathetic_amount * FEEDBACK_GAIN_MAX
    } else {
        0.0
    };
    self.resonance_bus.set_feedback_gain_target(target_gain);
    ```
  - 既存 `CC_ALL_NOTES_OFF` ブランチにも追加:
    ```rust
    self.resonance_bus.set_feedback_gain_target(0.0);
    self.resonance_bus.reset();                       // 残留 bus を切る
    self.bus_out_prev = 0.0;
    ```
- [ ] `Engine::apply_instrument(kind)` は **`sustain_state.reset()` 既存挙動を継承するため、楽器切替時点で sustain は inactive 確定**。Step 8 で追加した `resonance_bus.set_feedback_gain_target(0.0)` で bus feedback_gain も無条件 0 ターゲット。CC#64 ON が再送されれば `handle_midi_cc(CC_SUSTAIN_PEDAL, ≥0.5)` 経由で動的に target が立ち上がる
- [ ] **`Engine::apply_instrument(kind)` に bus 完全リセットも追加** (Step 8 で骨格、ここで仕上げ):
  ```rust
  self.resonance_bus.reset();   // bus 内部 delay line 完全クリア
  self.bus_out_prev = 0.0;
  ```
  Phase 4a D53 の「楽器切替で全 voice 即時 release」と整合し、Default kind 経路で bus 残留が出ない（F65-h で検証）
- [ ] **`Engine::reset()` にも bus 完全リセットを追加**（`synth_reset` C ABI 経由で呼ばれる、03 章 §4.6）:
  ```rust
  self.resonance_bus.reset();
  self.bus_out_prev = 0.0;
  ```
- [ ] **検証**:
  - F65-a〜f (`test_engine_inject_zero_when_feedback_gain_zero` / `test_engine_sustain_on_*` / `test_no_allocation_in_resonance_bus_process`) 全て pass
  - F68-a〜d (`test_apply_instrument_piano_activates_all_features` / `test_apply_instrument_default_deactivates_all_features` / `test_apply_instrument_piano_resets_sustain_and_bus_gain` / `test_apply_instrument_piano_preset_byte_diverges_from_phase4b`) 全て pass（dsp-core 内部の `tests/instrument_tests.rs` 拡張で検証、wasm-audio 側にはテストを追加しない）

**コミット例**: `feat(engine): integrate ResonanceBus via VoicePool::process_sample_with_feedback (poly_scale preserved)`

---

### フェーズ θ — テスト整備（3 ステップ）

#### Step 11. Multi-string テスト追加 (`tests/multi_string_tests.rs`) + test-only accessor 追加

- [ ] **test-only accessor の追加** (03 章 §7.5 の表に基づく、`#[doc(hidden)] pub fn ..._for_test`):
  - `KarplusStrong::n_strings_active(&self) -> usize` / `inharmonicity_b(&self) -> f32` / `unison_detune_cents(&self) -> f32` / `is_dispersion_active(&self) -> bool`（`karplus_strong.rs`）
  - `VoicePool::voice_n_strings_active_for_test(&self, idx) -> Option<usize>` / `voice_inharmonicity_b_for_test` / `voice_unison_detune_cents_for_test` / `voice_dispersion_active_for_test`（`voice_pool.rs`）
  - `Engine::voice_n_strings_active_for_test(&self, midi) -> Option<usize>`（`engine.rs`、内部で `voice_index_for_note(midi)` 経由）
- [ ] `crates/dsp-core/tests/multi_string_tests.rs` を新規作成
- [ ] F59-a〜d / F60-a〜d / **F61-a〜e** / F62-a〜c / F63 テストを実装（F61-e: Default kind で bus direct mix=0 + Piano→Default 切替で bus reset）
- [ ] 既存 `tests/fixtures/phase4a_default_c4_v08.rs` を Phase 4c でも継承し `test_default_n_strings_1_matches_phase4a` で使用
- [ ] **検証**:
  - 全 F59〜F63 (`cargo test -p dsp-core --test multi_string_tests`) pass

**コミット例**: `test(multi_string): add Phase 4c multi-string per voice tests + Phase 4a byte-match continuation`

#### Step 12. Sympathetic resonance テスト追加 (`tests/sympathetic_tests.rs`) + accessor 追加

- [ ] **test-only accessor の追加** (03 章 §7.5):
  - `SustainState::is_active_for_test(&self) -> bool`（`sustain_state.rs`、内部で `self.active` を返す。将来 `pub active: bool` を private 化する余地のため）
  - `ResonanceBus::feedback_gain_target_for_test(&self) -> f32` / `next_feedback_gain_for_test(&mut self) -> f32`（`resonance_bus.rs`）
  - 必要なら `SmoothedValue::target(&self) -> f32` も `#[doc(hidden)]` で追加
  - `Engine::sustain_active_for_test(&self) -> bool` / `resonance_feedback_target_for_test(&self) -> f32`（`engine.rs`）
- [ ] `crates/dsp-core/tests/sympathetic_tests.rs` を新規作成
- [ ] F64-a〜d / **F65-a〜i** テストを実装（F65-g: Default で bus_mix=0、F65-h: apply_instrument で bus reset、F65-i: CC#123 で bus reset）
- [ ] ResonanceBus に `#[doc(hidden)] pub fn buffer_max_amplitude_for_test(&self) -> f32` を追加（F65-h/i で bus reset 後の delay line がゼロクリアされていることを観測）
- [ ] **検証**:
  - 全 F64〜F65 pass

**コミット例**: `test(sympathetic): add ResonanceBus + Engine sustain integration tests`

#### Step 13. Hertz hammer + B(note) テスト追加 (`tests/hammer_hertz_tests.rs`)

- [ ] `crates/dsp-core/tests/hammer_hertz_tests.rs` を新規作成
- [ ] F66-a〜f / F67-a〜h テストを実装
- [ ] **検証**:
  - 全 F66〜F67 pass、特に F67-f (`test_b_curve_clamps_out_of_range`) で MIDI 範囲外動作確認

**コミット例**: `test(hammer_hertz): add Hertz raised cosine + B(note) LUT tests`

---

### フェーズ ι — 統合検証と Step 14 判断（2 ステップ）

#### Step 14. 統合 cargo test + WASM サイズ + cargo timing + 聴感判断 (D80, D73)

- [ ] `cargo test -p dsp-core` で **Phase 4b 既存 148 PASS + Phase 4c 新規 ~30 件 = 全 178 PASS** + 1 IGNORED が通ること
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` で warning ゼロ
- [ ] `cargo fmt --all` 実行、`pnpm fmt`（prettier）も実行
- [ ] `pnpm build:wasm` で WASM ビルド成功、`scripts/check-wasm-exports.mjs` で 19 required exports 確認
- [ ] gzip サイズ計測: < 22 KB を確認（F69-a）
- [ ] `cargo test --release -p dsp-core --test cpu_timing -- --nocapture` で:
  - Piano: < 0.15 ms / 128 frames (F69-b)
  - 非 Piano: < 0.05 ms / 128 frames (F69-c)
- [ ] `pnpm dev` + ブラウザで Piano プリセット試奏:
  - C4 / A4 / C5 / A5 / C2 / C7 等で実機聴感確認
  - **判断**:
    - 「Phase 4b より本物のピアノに近づいた」と確認できれば Step 15 へ進む（Step 15 はオプション化）
    - 「響板感が不足」「響きが薄い」と評価したら Step 15 で Modal M=16 拡張 + bridge coupling を追加
    - R44 (Piano 聴感未達) に該当したら R44 緩和策を順番に試す
- [ ] **検証**:
  - 全 F59〜F68 全 pass（multi_string + sympathetic + hammer_hertz + B(note) + apply_instrument 経路）
  - F69-a〜d 達成
  - 聴感確認結果を `docs/specs/2026-05-13-006-phase4c/step14-listening-notes.md`（新規）に記録

**コミット例**: `chore: Phase 4c integration verification at Step 14 (all tests pass, gzip 19.8KB, Piano 0.080ms)`

#### Step 15. (オプション) Modal M=16 拡張 + Bridge coupling 追加 (D73 / D80)

Step 14 の聴感判断で必要と決定した場合のみ実施:

- [ ] (Modal M=16 採用時) `crates/dsp-core/src/modal_body.rs` の `MAX_MODES` を 8 → 16 化、`ModeState` 配列拡張
- [ ] (Modal M=16 採用時) `params.json` の Piano エントリに 16 mode 係数を追加、`gen-params.mjs` で出力
- [ ] (Bridge coupling 採用時) `karplus_strong.rs` の `process_sample` に弦間 cross-feed (g≈0.05) を追加
- [ ] テスト追加: `test_modal_body_m16_piano` / `test_bridge_coupling_stability`
- [ ] **検証**:
  - 全 cargo test pass
  - WASM gzip < 22 KB 維持（Modal M=16 で +0.2 KB 想定）
  - Piano process < 0.15 ms 維持
- [ ] Step 14 と同じく実機聴感確認、改善を確認

**コミット例**: `feat(modal_body): expand Piano Modal Body to M=16 (Conklin) + add bridge coupling (Weinreich)`

---

### フェーズ κ — Factory Preset 更新と聴感チューニング（4 ステップ）

#### Step 16. `factory-presets.ts` の Piano エントリ初期値更新

- [ ] `web/src/lib/state/factory-presets.ts` の Piano エントリを Phase 4c 用に更新:
  - `createdAt: '2026-05-13T00:00:00.000Z'` (Phase 4c 着手日)
  - `params.damping / brightness / outputGain / pickPosition / bodyWet` は Phase 4b 値で開始（Step 19 で調整）
- [ ] 7 楽器 (Default / Guitar / Bass / Sitar 等) は変更なし
- [ ] localStorage v1 schema 互換維持
- [ ] **検証**: `pnpm check`, `pnpm --filter ./web check` 緑

**コミット例**: `chore(factory-presets): refresh Piano createdAt to Phase 4c date`

#### Step 17. 統合 cargo test + WASM サイズ + 実機 timing 取得

- [ ] `cargo test -p dsp-core` + clippy + fmt 緑
- [ ] `pnpm build` 成功、WASM gzip < 22 KB
- [ ] `pnpm preview` で本番ビルド確認、Piano + Sustain ペダル動作確認
- [ ] `__synthDev.measureProcessTime(5000)` で Piano timing 取得

**コミット例**: `chore: Phase 4c full verification before listening tuning`

#### Step 18. (聴感チューニング 1 回目) Piano プリセット値の反復調整 (D82 / R44 緩和)

- [ ] `pnpm dev` で実機聴感確認
- [ ] 不足感に応じて以下を反復調整（cargo / clippy / 互換性テストを壊さない範囲で）:
  - `damping` (0.995 〜 0.9995)
  - `brightness` (0.45 〜 0.60)
  - `bodyWet` (0.35 〜 0.65)
  - `params.json` の `unison_detune_cents` (1.0 〜 2.5)
  - `params.json` の `sympathetic_amount` (0.6 〜 1.2 ※内部 × 0.05 で clamp)
  - `params.json` の `hammer_cutoff_high_hz` (4500 〜 6500)
  - `params.json` の `inharmonicity_b_curve` の値（特に bass 領域）
- [ ] 各反復で `gen-params.mjs` 実行 → `cargo test -p dsp-core` で regression なし確認
- [ ] **検証**:
  - 全 F59〜F70 維持
  - 聴感で「Phase 4b より本物のピアノに近づいた」方向性確認

**コミット例**: `chore(piano-tuning): listening pass 1 — increase bodyWet to 0.55, raise hammer cutoff to 5800Hz`

#### Step 19. (聴感チューニング最終回) Piano 聴感達成確認 (D82)

- [ ] Step 18 を反復し、ユーザーが「Phase 4b より本物のピアノに近づいた」と判断するまで継続
- [ ] 既存 7 楽器 (Default / Guitar / Bass / Sitar 等) を試奏し regression なし確認
- [ ] Phase 1〜4b の全機能 (LFO / Mod Wheel / Sustain / Preset / VoiceMeter / MIDI CC / Pitch Bend) を動作確認
- [ ] **検証**:
  - 全 F59〜F70 pass
  - R44 回避達成
  - 実機聴感確認結果を `docs/specs/2026-05-13-006-phase4c/step19-listening-final.md`（新規）に記録

**コミット例**: `chore(piano-tuning): final listening pass — Piano achieves natural piano character`

---

### フェーズ λ — 完了処理（3 ステップ）

#### Step 20. F38b 実機計測（Phase 4c 完成後、F70-b）

- [ ] `pnpm dev` + ブラウザで Piano プリセット + `__synthDev.measureProcessTime(5000)` 実行
- [ ] 結果（avg / max / min / samples / bufferOverflow）を取得
- [ ] **判定**: Piano avg < 1.7 ms / max < 2.7 ms 達成
- [ ] 結果を `docs/specs/2026-05-13-006-phase4c/final-phase4c-timing.md`（新規）に記録（retrospective §8 で参照）
- [ ] iPhone Safari 実機でも Piano 動作確認 (F70-c、Phase 4a F9 継承)

**コミット例**: `chore: record Phase 4c F38b final timing (Piano X.XXms / Default X.XXms)`

#### Step 21. ドキュメント整備

- [ ] `CLAUDE.md` の「完了済みイテレーション」セクションに Phase 4c エントリ追加（05 章 §11 参照）
- [ ] `docs/retrospective/2026-05-13-006-phase4c.md` を新規作成（Phase 4b retrospective の章立てを踏襲、§1 概要 / §2 達成と未達 / §3 設計判断振り返り (D68-D85) / §4 躓きと教訓 / §5 既存負債 / §6 開発フロー上の改善 / §7 次イテレーション (Phase 4d) への引き継ぎ / §8 メトリクス / §9 メモリ更新案）
- [ ] `docs/specs/2026-05-13-006-phase4c/` 配下の仕様書群が最新であることを確認（実装中に発生した仕様書改訂を反映）

**コミット例**: `docs: add Phase 4c retrospective + update CLAUDE.md for Phase 4c completion`

#### Step 22. PR 作成 + main マージ

- [ ] ブランチを `phase4c-impl` から push
- [ ] `gh pr create` で PR 作成、タイトル例: `Phase 4c: Multi-string + Hertz hammer + Sympathetic resonance + B(note) LUT`
- [ ] PR 本文に以下を含める:
  - 主目的: Phase 4b の Piano 音色「弦楽器寄り」を構造拡張で解消
  - 実装した DSP 機能 (Multi-string 1/2/3 弦 / Hertz raised cosine hammer / Global resonance bus / 88 鍵 B(note) LUT)
  - Phase 4a / 4b 互換性: `n_strings = 1` で Phase 4a HEAD byte 一致継承、Phase 4b 互換 7 楽器 byte 一致
  - 性能: WASM gzip ~20 KB、Piano process < 0.15 ms、非 Piano < 0.05 ms
  - テスト: Phase 4b 148 PASS + 1 IGNORED + Phase 4c 新規 ~30 件
  - 受け入れ基準: 全 F59〜F70 達成、R44 回避、ユーザー実機聴感「本物のピアノに近づいた」確認
- [ ] CI 緑（build / test / lint）を確認
- [ ] レビュー後、main へマージ

**コミット例**: 既存コミット群、PR 経由

---

## ステップごとの所要時間目安

| フェーズ | ステップ | 所要時間 | 累積 |
|---|---|---|---|
| α (前処理) | Step 1 | 30 分 (ベースライン計測のみ) | 30 分 |
| β (params 拡張) | Step 2-3 | 1 時間 | 1.5 時間 |
| γ (Multi-string 基盤) | Step 4-5 | 3 時間 | 4.5 時間 |
| δ (Hertz hammer) | Step 6 | 2 時間 | 6.5 時間 |
| ε (process_sample 並列化) | Step 7 | 2 時間 | 8.5 時間 |
| ζ (B(note) 連携) | Step 8 | 1 時間 | 9.5 時間 |
| η (Sympathetic bus) | Step 9-10 | 2.5 時間 | 12 時間 |
| θ (テスト整備) | Step 11-13 | 3 時間 | 15 時間 |
| ι (統合 + Step 15 判断) | Step 14-15 | 2-4 時間 (Step 15 オプション) | 17-19 時間 |
| κ (聴感チューニング) | Step 16-19 | 3-6 時間 (反復回数次第) | 20-25 時間 |
| λ (完了処理) | Step 20-22 | 2 時間 | 22-27 時間 |

**合計**: **22-27 時間** (Phase 4b の 17-20 時間より大規模、Multi-string と聴感チューニングの反復が増分)。

## コミット数の目安

- Step 1: 1 commit（ベースライン記録）
- Step 2-3: 2 commits（params.json + gen-params.mjs）
- Step 4-7: 4 commits（karplus_strong 拡張、各 Step 1 つ）
- Step 8: 1 commit（dispersion B 引数化）
- Step 9-10: 2 commits（resonance_bus + engine 統合）
- Step 11-13: 3 commits（各テストファイル）
- Step 14: 1 commit（統合検証）
- Step 15: 0-2 commits（オプション、Modal M=16 / bridge coupling）
- Step 16: 1 commit（factory-presets）
- Step 17: 1 commit（中間検証）
- Step 18-19: 2-5 commits（聴感チューニング反復）
- Step 20: 1 commit（最終計測）
- Step 21: 1 commit（ドキュメント）
- Step 22: 0 commit（PR 経由）

**合計**: **約 20-25 commits**（Phase 4b の 17 commits より +3〜8、聴感チューニングと Multi-string 各 Step の独立コミット粒度のため）

## Phase 4c 実装時の禁忌（CLAUDE.md 制約）

- ❌ `process` ホットパスでヒープ確保（`Vec::resize` / `Box::new` / `String::from` 等）
- ❌ `wasm-bindgen` 使用
- ❌ 新規 C ABI 関数追加（D81、19 required exports 維持）
- ❌ 新規 ParamId 追加（D81）
- ❌ Phase 4a HEAD との byte 不一致（Default kind、`n_strings = 1`、`dispersion_active = false`、`feedback_gain = 0` 経路で必ず一致）
- ❌ `wasm-pack` 使用
- ❌ Svelte 5 `on:click` / `|preventDefault` 修飾子（小文字記法 `onclick` のみ）
- ❌ Look-ahead limiter / Pick fractional 化 / LFO 拡張等の Phase 4d 候補を Phase 4c に詰め込む（D84）
- ❌ C8 自己発振 / WASM SIMD を Phase 4c に詰め込む（D84）

## まとめ

Phase 4c は **22 ステップ・10 フェーズ**で構成し、Phase 4b の 18 ステップから +4 増加。実装規模感は **22-27 時間 / 20-25 commits**。Step 14 で **聴感判断** を行い Step 15 (Modal M=16 / Bridge coupling) を採否、Step 17-19 で **Piano プリセット聴感チューニング** を反復、Step 19 でユーザー実機聴感確認を **完了条件** に含める (D82)。Phase 4a HEAD byte 一致 (`n_strings = 1` 経路) と Phase 4b 7 楽器互換は Step 7 / Step 11 で機械保証 (D83)。`__synthDev.measureProcessTime` API を Step 1 (ベースライン) + Step 20 (Phase 4c 後) で使用、ユーザー操作必須。
