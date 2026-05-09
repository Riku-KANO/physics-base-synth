# 07. Phase 4b 実装順序チェックリスト

## 目的

Phase 4b 仕様書承認後、本仕様を実装する際の作業順序を Phase α〜ι の 9 フェーズ・全 18 ステップで定義する。各ステップは独立して進捗確認でき、検証チェックリスト（F48〜F58 + Phase 4a 既存）の充足ポイントを明示する。Phase 1 / 2 / 3 / 4a の `07-implementation-checklist.md` と同じ粒度（1 ステップ ≈ 1 コミット）で構成する。

## 他文書との関係

- 上流: 全ての仕様書（pre-research、01〜06）
- 参考: Phase 1 / 2 / 3 / 4a [07 章] — **Phase 4b ステップは Phase 4a の構成パターンを踏襲**
- このドキュメントは **実装専用のチェックリスト** であり、各ステップの設計詳細は対応する仕様書を参照する

## 前提条件

Phase 4b 実装着手前に以下を確認:

1. **Phase 4a の 120 PASS + 1 IGNORED テストがすべて維持されている**
   - `cargo test -p dsp-core` で確認
2. **`pnpm dev` でブラウザでの 8 音ポリフォニック + Modal Body + LFO + Mod Wheel + Preset + 7 楽器プリセットが動作確認できる状態**
3. **Phase 4a の retrospective が完了している**（`docs/retrospective/2026-05-08-004-phase4a.md`）
4. **Phase 4 を 4a / 4b / 4c に段階分割するユーザー承認が完了**（2026-05-09 時点で確定）

## 実装ステップ（全 18 段階、Phase α〜ι の 9 フェーズ）

### フェーズ α — 既存負債の前処理（3 ステップ）

#### Step 1. `.gitattributes` で改行 LF 統一（D65、F56）

- [ ] リポジトリ root に `.gitattributes` を作成（[`02-architecture.md` §ファイル変更リスト](./02-architecture.md#新規作成)）:
  ```
  * text=auto eol=lf
  *.md   text eol=lf
  *.svelte text eol=lf
  *.ts   text eol=lf
  *.js   text eol=lf
  *.mjs  text eol=lf
  *.rs   text eol=lf
  *.json text eol=lf
  *.toml text eol=lf
  *.lock text eol=lf
  *.yml  text eol=lf
  *.yaml text eol=lf

  *.png  binary
  *.jpg  binary
  *.jpeg binary
  *.gif  binary
  *.ico  binary
  *.wasm binary
  ```
- [ ] git add `.gitattributes` を **独立した commit** で先に確定:
  - [ ] `git add .gitattributes`
  - [ ] `git commit -m "chore: add .gitattributes for LF line endings (D65, F56)"`
- [ ] 既存 file を LF へ再正規化:
  - [ ] `git add --renormalize .`
  - [ ] **CRLF 由来の差分が大量に出ても通常**、内容変更ではない
  - [ ] `git diff --stat | head` で変更ファイル数を確認
- [ ] **独立した commit** で正規化結果を確定:
  - [ ] `git commit -m "chore: normalize line endings to LF (D65, F56)"`
- [ ] 検証:
  - [ ] `git ls-files --eol | grep -v "i/lf" | grep -v "binary"` で LF 以外の text file がゼロ
  - [ ] `pnpm fmt` を実行して差分が出ないこと（CRLF/LF 戦争の終結確認）
- **検証**: F56 達成、Phase 4b 後続 Step での format 差分汚染を防ぐ

#### Step 2. `__synthDev.measureProcessTime` 整備の準備 + 関連型定義（D66、F48 準備）

- [ ] `web/src/lib/audio/messages.ts` の `ToWorkletMessage` に `startTimingCapture` / `stopTimingCapture` を追加（dev only）:
  ```typescript
  // Phase 4b D66 (dev only)
  | { type: 'startTimingCapture' }
  | { type: 'stopTimingCapture' }
  ```
- [ ] `FromWorkletMessage` に `timing` variant を追加:
  ```typescript
  | { type: 'timing'; samples: number[]; bufferOverflow: boolean }
  ```
- [ ] `web/src/lib/audio/__synthDev.ts` を新規作成（[`05-web-frontend-spec.md` §__synthDev.ts](./05-web-frontend-spec.md#__synthdevts-phase-4b-新規d66) を参照）:
  - [ ] `measureProcessTime(port, durationMs)` 関数の実装
  - [ ] Promise + setTimeout + port.addEventListener で集約
- [ ] `web/src/lib/audio/engine.ts` に `workletPort()` getter を追加:
  ```typescript
  workletPort(): MessagePort | null {
    return this.node?.port ?? null;
  }
  ```
- [ ] **`synth-processor.ts` の dev-only timing 集約コードは Step 14 で実装**（dispersion / hammer の本体実装後にまとめて追加するため、本 Step では型定義 + Web 側 API のみ）
- [ ] `pnpm --filter ./web check` がパス
- [ ] git commit `feat(web): __synthDev.measureProcessTime 型定義 + API 追加 (D66, F48 準備)`
- **検証**: 型定義 + Web 側完成、Worklet 側集約は Step 14 で完成

#### Step 3. `wasm-opt --print-stats` ベースライン記録（F49 部分）

- [ ] `pnpm build:wasm` で Phase 4a の最新 WASM を生成:
  ```bash
  pnpm build:wasm
  ls -la web/static/wasm-audio.wasm  # サイズ確認 (~40 KB raw 想定)
  ```
- [ ] `wasm-opt --print-stats` で各 pass の内訳を取得:
  ```bash
  # Windows + Git Bash
  ./node_modules/.bin/wasm-opt --print-stats web/static/wasm-audio.wasm > /tmp/wasm-opt-stats-phase4a.txt 2>&1
  cat /tmp/wasm-opt-stats-phase4a.txt
  ```
- [ ] gzip サイズも記録:
  ```bash
  gzip -kc web/static/wasm-audio.wasm | wc -c
  ```
- [ ] `docs/retrospective/2026-05-08-004-phase4a.md` §5 の「WASM gzip 18.42 KB が警戒ライン超過」項目を更新:
  - [ ] `wasm-opt --print-stats` の出力（Functions / Imports / Globals / Memories の行数 + 主要 pass の効果）を追記
  - [ ] Phase 4b 着手時のベースラインとして記録
- [ ] git commit `chore(retrospective): Phase 4a WASM サイズベースライン (wasm-opt --print-stats) を追記 (F49)`
- **検証**: F49 ベースライン、Phase 4b 後の `pnpm build` で gzip 比較が可能になる

### フェーズ β — params.json 拡張と Piano 楽器係数（1 ステップ）

#### Step 4. `params.json` に Piano エントリ追加 + `gen-params.mjs` 拡張（D62、F49 / F53 準備）

- [ ] `params.json` の `instruments` 配列に Piano エントリ（8 番目）を追加:
  ```json
  {
    "kind": "Piano",
    "stereo_spread": 0.05,
    "inharmonicity_b": 7.5e-4,
    "hammer_cutoff_low_hz": 800.0,
    "hammer_cutoff_high_hz": 4000.0,
    "body_modes": [
      { "freq": 55.0,   "q": 10.0, "gain": 1.0  },
      { "freq": 110.0,  "q": 12.0, "gain": 0.85 },
      { "freq": 175.0,  "q": 15.0, "gain": 0.7  },
      { "freq": 280.0,  "q": 18.0, "gain": 0.55 },
      { "freq": 460.0,  "q": 22.0, "gain": 0.45 },
      { "freq": 750.0,  "q": 28.0, "gain": 0.35 },
      { "freq": 1300.0, "q": 35.0, "gain": 0.28 },
      { "freq": 2200.0, "q": 40.0, "gain": 0.22 }
    ]
  }
  ```
- [ ] `scripts/gen-params.mjs` を拡張（[`02-architecture.md` §gen-params.mjs の Phase 4b 拡張](./02-architecture.md#gen-paramsmjs-の-phase-4b-拡張d62) を参照）:
  - [ ] Piano 専用フィールド（`inharmonicity_b` / `hammer_cutoff_low_hz` / `hammer_cutoff_high_hz`）を validation
  - [ ] `kind === 'Piano'` のときに `INHARMONICITY_B_PIANO` / `HAMMER_CUTOFF_LOW_PIANO` / `HAMMER_CUTOFF_HIGH_PIANO` const を Rust + TS 出力
  - [ ] `BODY_MODES_PIANO_L` / `BODY_MODES_PIANO_R` を `applyStereoSpread` で生成
  - [ ] `STEREO_SPREAD_PIANO` を出力
  - [ ] `InstrumentKind::Piano = 7` を Rust enum に追加
  - [ ] `INSTRUMENT_KIND_COUNT` を 7 → 8 に変更
  - [ ] `body_modes_for_instrument` / `stereo_spread_for_instrument` のヘルパに Piano 分岐追加
  - [ ] TS 側の `InstrumentKindKey` に `'piano'` 追加、`INSTRUMENT_KIND_TO_NUMBER['piano'] = 7` 追加
- [ ] `pnpm gen:params` を実行、生成ファイルに以下が含まれることを確認:
  - [ ] Rust 側: `BODY_MODES_PIANO_L/R` + `STEREO_SPREAD_PIANO` + `INHARMONICITY_B_PIANO` + `HAMMER_CUTOFF_LOW_PIANO` + `HAMMER_CUTOFF_HIGH_PIANO` + `InstrumentKind::Piano = 7` + `INSTRUMENT_KIND_COUNT = 8`
  - [ ] TS 側: `'piano'` を含む `InstrumentKindKey` + `INSTRUMENT_KIND_TO_NUMBER['piano'] = 7` + Piano 専用 const
- [ ] `pnpm check:params-sync` がパス
- [ ] `cargo check -p dsp-core` がパス、`pnpm --filter ./web check` がパス
- [ ] git commit `feat(params): Piano 楽器プリセットを生成パイプラインに追加 (D62, F53 準備)`
- **検証**: 生成物 drift なし、PARAM_DESCRIPTORS は不変（5 件）、`InstrumentKind::Piano = 7` 追加

### フェーズ γ — Dispersion cascade 実装（3 ステップ）

#### Step 5. `dispersion.rs` 実装（D57 / D58 / D59、F51-a〜f）

- [ ] `crates/dsp-core/src/dispersion.rs` を新規作成（[`03-dsp-core-spec.md` §Dispersion](./03-dsp-core-spec.md#dispersion-dispersionrs--phase-4b-新規d57--d58--d59) を参照）:
  - [ ] `pub const DISPERSION_STAGES: usize = 8;`
  - [ ] `K1〜K3` / `M1〜M4` の magic constants（`#![allow(clippy::approx_constant)]` をモジュール冒頭で適用）
  - [ ] `DispersionStage` 構造体: `a1` / `z1_in` / `z1_out` フィールド + `new` / `reset` / `process` メソッド
  - [ ] `compute_dispersion_a1(m, b, f0, fs) -> (f32, f32)` 純粋関数: Faust 由来 closed-form 式の Rust 移植、`a1.clamp(-0.999, 0.999)` で安全側
- [ ] `crates/dsp-core/src/lib.rs` に追加:
  ```rust
  pub mod dispersion;
  pub use dispersion::{DispersionStage, compute_dispersion_a1, DISPERSION_STAGES};
  ```
- [ ] `crates/dsp-core/tests/dispersion_tests.rs` を新規作成（[`03-dsp-core-spec.md` §Dispersion テスト方針](./03-dsp-core-spec.md#dispersion-テスト方針) を参照）:
  - [ ] `test_dispersion_a1_in_safe_range`
  - [ ] `test_dispersion_a1_increases_with_b`
  - [ ] `test_dispersion_b_zero_limit`
  - [ ] `test_dispersion_a1_keyboard_dependence`
  - [ ] `test_dispersion_stage_reset`
  - [ ] `test_dispersion_stage_passthrough_when_a1_zero`
  - [ ] `test_dispersion_cascade_8_stages_stable`
  - [ ] `test_dispersion_group_delay_positive`
- [ ] `cargo test -p dsp-core test_dispersion_` がすべてパス
- [ ] `cargo clippy --workspace -- -D warnings` がパス
- [ ] git commit `feat(dsp-core): Stretching all-pass cascade 実装 (D57, D58, D59, F51-a/b/c/d/e/f)`
- **検証**: F51-a〜f 達成、`dispersion.rs` 単体動作確認

#### Step 6. `KarplusStrong` に dispersion フィールド追加 + `note_on` で a1 算出（D60、F51-g）

- [ ] `crates/dsp-core/src/karplus_strong.rs` の use 句拡張:
  ```rust
  use crate::dispersion::{compute_dispersion_a1, DispersionStage, DISPERSION_STAGES};
  use crate::params::{
      INHARMONICITY_B_PIANO, HAMMER_CUTOFF_LOW_PIANO, HAMMER_CUTOFF_HIGH_PIANO,
      // ...既存
  };
  ```
- [ ] `KarplusStrong` 構造体にフィールド追加（[`03-dsp-core-spec.md` §フィールド追加](./03-dsp-core-spec.md#karplusstrong-の-phase-4b-拡張) を参照）:
  ```rust
  dispersion_stages: [DispersionStage; DISPERSION_STAGES],
  dispersion_active: bool,
  ```
- [ ] `KarplusStrong::new()` で初期化:
  ```rust
  dispersion_stages: [DispersionStage::new(); DISPERSION_STAGES],
  dispersion_active: false,
  ```
- [ ] `set_dispersion_active(active: bool)` setter を `#[inline(always)]` で追加（`active = false` で全 stage を `reset()`）
- [ ] `dispersion_active()` / `dispersion_stage_a1(idx)` の `#[doc(hidden)]` getter を追加（テスト用）
- [ ] `note_on_internal` の `adjusted_length` 計算で **dispersion 群遅延補正を加算**:
  ```rust
  let dispersion_tau_g = if self.dispersion_active {
      let (a1, gd_per_stage) = compute_dispersion_a1(
          DISPERSION_STAGES as u32, INHARMONICITY_B_PIANO,
          freq_hz, self.sample_rate,
      );
      for stage in self.dispersion_stages.iter_mut() {
          stage.a1 = a1; stage.z1_in = 0.0; stage.z1_out = 0.0;
      }
      (DISPERSION_STAGES as f32) * gd_per_stage
  } else {
      0.0
  };
  let total_compensation = brightness_tau_g + dispersion_tau_g;
  let adjusted = (raw_len - total_compensation).max(3.0);
  ```
- [ ] `KarplusStrong::reset()` で `dispersion_active = false` + 全 stage reset
- [ ] **`note_on_internal` の buffer 初期化分岐 (hammer / pluck) は Step 8 で実装**（本 Step では adjusted_length と dispersion 係数算出のみ、buffer は既存 pluck 経路のまま）
- [ ] **`process_sample` で cascade 適用は Step 7 で実装**（本 Step では Phase 4a 既存の Thiran のみで動作させる、互換性維持）
- [ ] テスト追加:
  - [ ] `test_dispersion_a1_set_in_note_on` (`tests/karplus_strong_dispersion_tests.rs` 新規):
    - [ ] `set_dispersion_active(true)` → `note_on(440, 0.8)` 後、`dispersion_stage_a1(0)` が `compute_dispersion_a1(8, INHARMONICITY_B_PIANO, 440.0, sr).0` と一致
- [ ] `cargo test -p dsp-core test_dispersion_a1_set_in_note_on` がパス
- [ ] `cargo test -p dsp-core` で Phase 4a 既存 120 件すべてパス（regression なし）
- [ ] git commit `feat(karplus-strong): dispersion フィールド追加 + note_on で a1 算出 (D60, F51-g)`
- **検証**: F51-g 達成、Phase 4a regression なし（dispersion_active=false で Phase 4a 互換）

#### Step 7. `process_sample` で dispersion cascade 適用（D60、F51 完成）

- [ ] `KarplusStrong::process_sample` の `read_z` 値に対して dispersion cascade を適用（[`03-dsp-core-spec.md` §`process_sample` の拡張](./03-dsp-core-spec.md#process_sample-の拡張d60) を参照）:
  ```rust
  let read_value = if self.dispersion_active {
      let mut x = self.buffer[read_z];
      for stage in self.dispersion_stages.iter_mut() {
          x = stage.process(x);
      }
      self.thiran.process(x)
  } else {
      self.thiran.process(self.buffer[read_z])
  };
  ```
- [ ] **D67 互換性確認**: `dispersion_active = false` 経路が Phase 4a と完全一致（`thiran.process(self.buffer[read_z])` の引数が Phase 4a と同じ）
- [ ] テスト追加 (`tests/karplus_strong_dispersion_tests.rs` 拡張):
  - [ ] `test_dispersion_disabled_matches_phase4a` (D67 互換性核心テスト)
  - [ ] Default kind / Mod Wheel=0 / LFO depth=0 で 256 サンプル process した出力が **Phase 4a と ε=1e-6 でバイト一致** を保証
  - [ ] **Phase 4a の golden 値生成**: 本 Step 完了前に Phase 4a の HEAD で `cargo test -- --nocapture` を一度実行して buf_l/r の最初 256 frame を取得、`tests/fixtures/phase4a_default_c4.json` に保存（または直接定数として埋め込む）
- [ ] `cargo test -p dsp-core test_dispersion_disabled_matches_phase4a` がパス
- [ ] `cargo test -p dsp-core` で全テスト通過
- [ ] git commit `feat(karplus-strong): process_sample で dispersion cascade 適用 (D60, F55, F57)`
- **検証**: F51 完成、F55 (Phase 4a 互換性) を機械保証、F57 regression baseline 維持

### フェーズ δ — Hammer model（1 ステップ）

#### Step 8. `note_on_internal` で hammer 経路分岐（D61、F52）

- [ ] `KarplusStrong::note_on_internal` の buffer 初期化を pluck / hammer で分岐（[`03-dsp-core-spec.md` §`note_on_internal` の拡張](./03-dsp-core-spec.md#note_on_internal-の拡張d60--d61) を参照）:
  ```rust
  if self.dispersion_active {
      // === Hammer 経路 (Piano kind) ===
      for v in self.buffer.iter_mut() { *v = 0.0; }
      self.buffer[0] = velocity;  // 単位 impulse
      let cutoff_hz = HAMMER_CUTOFF_LOW_PIANO
          + velocity.clamp(0.0, 1.0) * (HAMMER_CUTOFF_HIGH_PIANO - HAMMER_CUTOFF_LOW_PIANO);
      let alpha = 1.0 - (-2.0 * core::f32::consts::PI * cutoff_hz / self.sample_rate).exp();
      let mut z = 0.0_f32;
      for i in 0..len_int {
          z = alpha * self.buffer[i] + (1.0 - alpha) * z;
          self.buffer[i] = z;
      }
      // Pick position は適用しない (hammer は固定位置)
  } else {
      // === Pluck 経路 (Phase 1〜4a 既存) ===
      // ...既存コードのまま...
  }
  ```
- [ ] テスト追加 (`tests/karplus_strong_dispersion_tests.rs` 拡張):
  - [ ] `test_note_on_with_dispersion_active_uses_hammer_excitation`
  - [ ] `test_note_on_with_dispersion_inactive_uses_pluck_excitation`
  - [ ] `test_hammer_velocity_affects_brightness`
- [ ] `cargo test -p dsp-core test_hammer_velocity` / `test_note_on_with_dispersion_` がパス
- [ ] `cargo test -p dsp-core test_dispersion_disabled_matches_phase4a` も再度パス（pluck 経路は Phase 4a と完全一致）
- [ ] git commit `feat(karplus-strong): hammer 励振経路 (Commuted impulse + velocity LPF) を実装 (D61, F52)`
- **検証**: F52 達成、Phase 4a 互換性維持（dispersion_active=false で pluck 経路）

### フェーズ ε — Voice trait + VoicePool 拡張（1 ステップ）

#### Step 9. `Voice::set_dispersion_active` 追加 + VoicePool fan-out（D67、F58 準備）

- [ ] `crates/dsp-core/src/traits.rs` の `Voice` trait に追加:
  ```rust
  fn set_dispersion_active(&mut self, active: bool);
  ```
- [ ] `crates/dsp-core/src/voice.rs` の `KarplusStrong` 向け委譲を追加:
  ```rust
  fn set_dispersion_active(&mut self, active: bool) {
      KarplusStrong::set_dispersion_active(self, active)
  }
  ```
- [ ] `crates/dsp-core/src/voice_pool.rs` に追加:
  ```rust
  pub fn set_dispersion_active(&mut self, active: bool) {
      for v in &mut self.voices {
          v.set_dispersion_active(active);
      }
  }
  ```
- [ ] `crates/dsp-core/src/note_allocator.rs` の `#[cfg(test)] mod` 内 MockVoice に空実装追加:
  ```rust
  fn set_dispersion_active(&mut self, _active: bool) {}
  ```
- [ ] `cargo build --workspace` がパス
- [ ] `cargo test -p dsp-core` で Phase 4a 既存テスト全件パス（trait 拡張による E0046 が出ないこと）
- [ ] git commit `feat(voice-pool): Voice trait に set_dispersion_active(bool) 追加 (D67)`
- **検証**: trait 拡張完了、Engine から fan-out 可能な状態

### フェーズ ζ — ModalBody Piano + Engine apply_instrument 拡張（2 ステップ）

#### Step 10. Piano Modal 係数の Engine 経由動作確認（D62、F53）

`gen-params.mjs` 拡張は Step 4 で完了済、本 Step は ModalBody が Piano kind の `BODY_MODES_PIANO_L/R` を正しく取得することを cargo test で確認する。

- [ ] `crates/dsp-core/src/modal_body.rs` は **コード変更なし**（Phase 4a 既存の `set_instrument(kind, sr)` が `body_modes_for_instrument(InstrumentKind::Piano)` で Piano BodyMode を取得）
- [ ] テスト追加 (`tests/modal_body_tests.rs` 拡張):
  - [ ] `test_piano_modal_first_mode_at_55hz`: `BODY_MODES_PIANO_L[0].freq == 55.0`
  - [ ] `test_piano_stereo_spread_default`: `STEREO_SPREAD_PIANO == 0.05`
- [ ] テスト追加 (`tests/instrument_tests.rs` 拡張):
  - [ ] `test_apply_instrument_piano_modal_coeffs`: `apply_instrument(Piano)` 後、`modal_body.coeff_l_b0(0)` が Piano 係数ベース
  - [ ] `test_default_disables_dispersion`: Default → Piano → Default で `dispersion_active` が false → true → false
- [ ] `cargo test -p dsp-core test_piano_modal_` / `test_apply_instrument_piano_modal_coeffs` / `test_default_disables_dispersion` がパス
- [ ] git commit `test(modal-body, instrument): Piano kind の Modal 係数経由動作確認 (D62, F53)`
- **検証**: F53-a〜e 達成、Piano kind が ModalBody 経由で動作確認

#### Step 11. `Engine::apply_instrument` 末尾に `set_dispersion_active` 呼出を追加（D63 改訂後 / D67、F54）

**仕様変更（指摘事項 #3 反映）**: 当初 D63 で「5 ms fade-out」を実装する Step だったが、SmoothedValue 同期 set_target の実現不能性により撤回。本 Step では Phase 4a 既存の `apply_instrument` 末尾に `pool.set_dispersion_active(matches!(kind, Piano))` の 1 行を追加するのみ。Phase 4a D53「即時 release」を完全継承する。

- [ ] `crates/dsp-core/src/engine.rs::apply_instrument` を拡張（[`03-dsp-core-spec.md` §`apply_instrument` は Phase 4a の即時 release を継承](./03-dsp-core-spec.md#apply_instrument-は-phase-4a-の即時-release-を継承d63-改訂後) を参照）:
  ```rust
  pub fn apply_instrument(&mut self, kind: InstrumentKind) {
      // Phase 4a 既存処理（即時 release、Phase 4b で変更なし）
      self.pool.all_notes_off();
      self.hold_stack.clear();
      self.sustain_state.reset();
      self.current_instrument = kind;
      self.stereo_spread = stereo_spread_for_instrument(kind);
      self.modal_body.set_instrument(kind, self.sample_rate);

      // Phase 4b D67 新規: dispersion_active を全 voice に fan-out
      let dispersion_active = matches!(kind, InstrumentKind::Piano);
      self.pool.set_dispersion_active(dispersion_active);
  }
  ```
- [ ] `Engine::reset` も拡張:
  ```rust
  // ...Phase 4a 既存...
  self.pool.set_dispersion_active(false);  // Default kind に戻る
  ```
- [ ] テスト追加 (`tests/instrument_tests.rs` 拡張):
  - [ ] `test_apply_instrument_piano_enables_dispersion` (F54-b)
  - [ ] `test_apply_instrument_default_disables_dispersion` (F54-c)
  - [ ] `test_apply_instrument_piano_no_alloc` (F58)
  - [ ] `test_apply_instrument_does_not_modify_output_gain_target` (F54-d): D63 改訂後、`apply_instrument(Piano)` 内で `output_gain.target()` が変更されないことを確認（fade-out 機構なし）
- [ ] `cargo test -p dsp-core test_apply_instrument_` がすべてパス
- [ ] git commit `feat(engine): apply_instrument 末尾に set_dispersion_active 呼出を追加 (D67, F54, F58)`
- **検証**: F54 達成、F58 alloc ゼロ準備、Engine 側で Piano 切替フロー完成（即時 release は Phase 4a D53 継承）

### フェーズ η — Web フロントエンド（3 ステップ）

#### Step 12. `messages.ts` + `synth-processor.ts` + `engine.ts` で `'piano'` 拡張（D62）

- [ ] `web/src/lib/audio/messages.ts` の `InstrumentKindKey` 型に `'piano'` 追加（D62）
- [ ] `web/src/lib/audio/synth-processor.ts` の `INSTRUMENT_KIND_MAP` に `piano: 7` 追加
- [ ] `web/src/lib/audio/engine.ts` は変更なし（型エラー解消の確認のみ、`InstrumentKindKey` 拡張で自動的に値域拡張）
- [ ] `pnpm --filter ./web check` がパス
- [ ] git commit `feat(web): InstrumentKindKey に 'piano' 追加 + INSTRUMENT_KIND_MAP 拡張 (D62)`
- **検証**: 型定義拡張完了、Step 13-14 で UI 経路完成

#### Step 13. `preset-schema.ts` + `factory-presets.ts` で Piano エントリ追加（D62、F53 経路）

- [ ] `web/src/lib/state/preset-schema.ts` の `InstrumentKindKey` 型 + `VALID_INSTRUMENTS` 配列に `'piano'` 追加（[`05-web-frontend-spec.md` §preset-schema.ts](./05-web-frontend-spec.md#preset-schemats-の-phase-4b-拡張d62)）
- [ ] `web/src/lib/state/factory-presets.ts` に Piano エントリ（8 番目）を追加（[`05-web-frontend-spec.md` §factory-presets.ts](./05-web-frontend-spec.md#factory-presetsts-の-phase-4b-拡張d62)）:
  ```typescript
  {
    version: 1,
    name: 'Piano',
    createdAt: '2026-05-09T00:00:00.000Z',
    instrument: 'piano',
    params: { damping: 0.998, brightness: 0.55, outputGain: 0.7, pickPosition: 0.13, bodyWet: 0.4 },
    lfo: { rate: 5.0, waveform: 'sine', pitchDepth: 0.0, brightnessDepth: 0.0, volumeDepth: 0.0 }
  }
  ```
- [ ] `pnpm --filter ./web check` がパス
- [ ] `pnpm --filter ./web lint` がパス
- [ ] git commit `feat(web): Piano プリセットを Factory Preset に追加 (D62)`
- **検証**: PresetSelector ドロップダウンに Piano が 8 番目として表示される（コード変更なし、自動反映）

#### Step 14. dev-only timing 集約コード（synth-processor.ts、D66、F48 完成）

- [ ] `web/src/lib/audio/synth-processor.ts` に dev-only timing 集約コード追加（[`05-web-frontend-spec.md` §synth-processor.ts の Phase 4b 変更点](./05-web-frontend-spec.md#synth-processorts-の-phase-4b-変更点) を参照）:
  - [ ] **`declare const DEV_MODE: boolean;`** を冒頭に追加（type-only declaration、ローカル `const DEV_MODE = ...` は禁止 / 指摘事項 #2 反映）
  - [ ] `timingBuffer: Float32Array | null` / `timingBufferCapacity` / `timingBufferWriteIndex` / `timingBufferCount` / `timingBufferWrapped` / `timingCaptureActive` フィールド追加（リングバッファ管理）
  - [ ] constructor で `if (DEV_MODE)` ガードで **`timingBufferCapacity = 4096; timingBuffer = new Float32Array(this.timingBufferCapacity)`** 初期化（48kHz/128 frames で 375 quanta/sec、4096 entry で約 10.92 秒分 / 指摘事項 #5 反映）
  - [ ] `onMessage` switch に `startTimingCapture` / `stopTimingCapture` ケース追加（DEV_MODE ガード）
    - [ ] `startTimingCapture`: writeIndex / count / wrapped をリセットして `timingCaptureActive = true`
    - [ ] `stopTimingCapture`: wrap 状態に応じて時系列順 (`[writeIndex..capacity)` ++ `[0..writeIndex)` または `[0..count)`) に並べ直して main へ `port.postMessage({type: 'timing', samples, bufferOverflow: wrapped})`
  - [ ] `process()` 内で `if (DEV_MODE && timingCaptureActive)` の場合に **`startMs = performance.now()`** を記録（指摘事項 #1 反映: `currentFrame` は callback 内で進まないため self time 計測には使えない）
  - [ ] process 後に **`elapsedMs = performance.now() - startMs`** を計算してリングバッファに書込、writeIndex/count/wrapped を更新
- [ ] **`web/package.json` の build:worklet:dev / build:worklet スクリプトに esbuild の `--define:DEV_MODE=true` / `--define:DEV_MODE=false` 引数を追加**（[`05-web-frontend-spec.md` §Worklet build script の Phase 4b 拡張](./05-web-frontend-spec.md#worklet-build-script-の-phase-4b-拡張d66) を参照、指摘事項 #2 反映）。`vite.config.ts` の define ではなく **worklet build 専用 esbuild 引数** で渡す（worklet bundle が独立 esbuild で生成されるため）
- [ ] `web/src/lib/state/synth.svelte.ts` に `__synthDev.measureProcessTime` 追加（[`05-web-frontend-spec.md` §synth.svelte.ts](./05-web-frontend-spec.md#synthsvelttests-の-phase-4b-拡張d66) を参照）:
  ```typescript
  if (import.meta.env.DEV) {
    w.__synthDev.measureProcessTime = async (durationMs: number) => {
      const port = synth.engine.workletPort();
      if (!port) throw new Error('Worklet not initialized, call StartButton first');
      const { measureProcessTime } = await import('$lib/audio/__synthDev');
      return measureProcessTime(port, durationMs);
    };
  }
  ```
- [ ] `pnpm dev` でブラウザ起動、DevTools Console で:
  ```javascript
  await window.__synthDev.measureProcessTime(5000)
  // → { avg: ..., max: ..., min: ..., samples: [...], bufferOverflow: false }
  // performance.now() 差分なので avg は ms 単位の self time（音声時間 2.67ms ではない）
  ```
  が動作することを確認
- [ ] `pnpm build` で production ビルド、`grep -r "DEV_MODE\|measureProcessTime\|timingBuffer" web/build/` で **0 hits**（worklet 側 `--define:DEV_MODE=false` + tree-shake で削除されている）
- [ ] git commit `feat(web): __synthDev.measureProcessTime + Worklet timing 集約 (D66, F48)`
- **検証**: F48 完成、`pnpm dev` で Console から API 呼出可能

### フェーズ θ — 統合検証（1 ステップ）

#### Step 15. 統合 cargo test + alloc ゼロ + release timing + Phase 4a 互換性（F50 / F55 / F57 / F58）

- [ ] `tests/no_alloc_tests.rs` に `test_no_allocation_with_piano_kind` を追加（[`03-dsp-core-spec.md` §テスト方針](./03-dsp-core-spec.md#テスト方針) を参照）:
  - [ ] 8 voice + Piano kind active + LFO + Mod Wheel + Pitch Bend + 楽器切替 (Piano ↔ Default) で voice buffer / LFO 状態 / dispersion_stages capacity 不変
- [ ] `tests/dsp_core_tests.rs` に Phase 4b release timing test を追加（[`06-build-and-verify.md` §F50](./06-build-and-verify.md#f50--リアルタイム性能-release-cargo-timingf46-拡張piano-kind-含む) を参照）:
  - [ ] `test_engine_process_block_timing_phase4b_piano`: `#[cfg(not(debug_assertions))]`、Piano kind 最悪ケースで avg < 1.7 ms
  - [ ] `test_engine_process_block_timing_phase4b_other`: 非 Piano (Default) で avg < 1.0 ms（Phase 4a 互換）
- [ ] **`cargo test --release -p dsp-core` で全テスト**（Phase 4a 既存 120 + Phase 4b 新規 ~25 = 145 件目標）がパス、特に F50 timing test が release で通ること
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` がパス
- [ ] `pnpm fmt` で全コードフォーマット（`.gitattributes` LF 統一済のため CRLF 差分は出ない想定）
- [ ] `pnpm build` で本番ビルド成功
- [ ] WASM gzip サイズ計測:
  ```bash
  gzip -kc web/build/_app/immutable/assets/wasm_audio.*.wasm | wc -c
  ```
  - [ ] **目標**: gzip < 22 KB（想定 ~19 KB）
  - [ ] **警戒**: gzip < 25 KB（超えたら `wasm-opt --print-stats` で再調査）
  - [ ] **撤退**: gzip < 30 KB（超えたら R32 適用 = 楽器 4 種に削減 / Modal M=5）
- [ ] Worklet 本番バンドルサイズ計測（< 12 KB target）:
  ```powershell
  Get-ChildItem web\build\_app\immutable\assets\synth-processor*.js | Select-Object Name, Length
  ```
- [ ] `__synthDev` / `DEV_MODE` / `timingBuffer` / `measureProcessTime` が production bundle に 0 hits（grep で検証、worklet bundle は `web/build/worklet/synth-processor.js` 由来のため `web/build/` 全体を対象とする / 指摘事項 #1 反映）:
  ```bash
  grep -r "__synthDev\|measureProcessTime\|DEV_MODE\|timingBuffer" web/build/ | wc -l
  # 期待値: 0
  ```
- [ ] git commit `test: Phase 4b 統合テスト (alloc / release timing / Phase 4a 互換性) (F50, F55, F57, F58)`
- **検証**: F50 / F55 / F57 / F58 達成、Phase 4a 既存テスト regression なし

### フェーズ ι — 実機確認 + ドキュメント整備（3 ステップ）

#### Step 16. 実機確認（pnpm dev + `__synthDev.measureProcessTime`）（F48 / F49 / F51-F54 / F57 実機）

- [ ] `pnpm dev` でブラウザ起動、F48〜F57 の実機確認:
  - [ ] **F48 計測自動化**: DevTools Console で `await window.__synthDev.measureProcessTime(10000)` 実行 → avg / max を取得（target avg < 1.7 ms / max < 2.7 ms）
  - [ ] **F49 サイズ**: `pnpm build` の gzip 計測 < 22 KB
  - [ ] **F51 Stretching all-pass 実機**: Piano プリセットで A4 押下 → Default の A4 と比べて高音域に「stretched」な倍音が聞こえる
  - [ ] **F52 Hammer model 実機**: Piano プリセットで弱打鍵 (velocity 30) と強打鍵 (velocity 120) で音色変化（弱: 柔らかい / 強: 明るい）
  - [ ] **F53 Piano Modal 実機**: Piano プリセット選択で楽器が切り替わり、低音域の響板感が強い
  - [ ] **F54 即時 release + dispersion_active 実機**: Piano 選択で `pool.set_dispersion_active(true)` が漏れなく fan-out（音が出てピアノっぽく変調される）、Default 選択で `false` に戻る。pop noise の聴感は Phase 4a と同レベル（D63 改訂後、fade-out なし）
- [ ] **F57 Phase 4a 互換性 実機**:
  - [ ] Default プリセットで Mod Wheel = 0 にすると Phase 4a と同じ音
  - [ ] Phase 4a の 6 楽器（Guitar Classical / Ukulele / Mandolin / Bass / Steel Guitar / Sitar）でそれぞれ Phase 4a と同じ音
  - [ ] LFO / Mod Wheel / Preset / Pitch Bend / Sustain Pedal / Channel Volume の挙動が Phase 4a と同じ
  - [ ] mono / poly トグルが Phase 4a と同じく動作
- [ ] **F38b 再計測** (Phase 4b 後の実機 Worklet `process` 時間):
  - [ ] Piano kind: avg < 1.7 ms / max < 2.7 ms
  - [ ] 非 Piano (Default + 6 楽器): Phase 4a と同等 (~0.045 ms 想定)
  - [ ] 計測値を retrospective 準備として記録
- [ ] iOS Safari (HTTPS URL or Pages preview) での動作確認（持ち越し可、可能なら実施）
- [ ] git commit `chore: Phase 4b 実機確認完了 (F48, F49, F51-F54, F57 実機, F38b 再計測)`
- **検証**: F48 / F49 / F51-F54 / F57 の実機達成、F38b 再計測完了

#### Step 17. ドキュメント整備 + retrospective 準備

- [ ] `README.md` を Phase 4b 用に更新:
  - [ ] Piano プリセット使用方法
  - [ ] `__synthDev.measureProcessTime` の使い方
  - [ ] F48〜F58 の自己検証手順
- [ ] `CLAUDE.md` の「現在のイテレーション」を Phase 4b 完了に更新、「次は Phase 4c（C8 自己発振 / WASM SIMD / Sustain×sympathetic 等から選定）」を追記、Phase 4a の `(Phase 4a) — wasm-opt -O3 / ...` 列に Phase 4b の項目を追加
- [ ] 仕様書群 `docs/specs/2026-05-09-005-phase4b/` の各章でリンク切れがないか確認
- [ ] retrospective テンプレートを準備（`/retrospective` カスタムコマンドを Phase 4b 完了後に発火）
- [ ] git commit `docs: Phase 4b 完了反映 (README / CLAUDE.md / retrospective 準備)`
- **検証**: Phase 4b 完成、Phase 4c への申し送りが文書化される

#### Step 18. PR 作成 + main マージ

- [ ] `cargo test --release -p dsp-core` 最終確認、全件パス
- [ ] `pnpm check` / `pnpm lint` / `pnpm fmt` 最終確認
- [ ] `pnpm build` 最終ビルド成功確認、gzip < 22 KB 目標 / < 25 KB 警戒 / < 30 KB 撤退ラインを確認
- [ ] PR 作成（`gh pr create`）:
  - [ ] PR タイトル: `Phase 4b: Piano (Stretching all-pass + Hammer model) + .gitattributes LF + __synthDev.measureProcessTime`
  - [ ] PR ボディに Phase 4b スコープサマリ + 検証結果（F48 avg/max、gzip サイズ、テスト件数、Phase 4a 互換性確認）
- [ ] CI 緑を確認（build / test / lint / params-sync / wasm-exports）
- [ ] main ブランチへマージ
- [ ] retrospective 着手（`/retrospective 2026-05-09-005-phase4b`）
- **検証**: Phase 4b が main にマージされる、Phase 4c 着手準備完了

## ステップごとの依存関係

```
Step 1 (.gitattributes LF)
  └─ 後続 Step での format 差分汚染防止
      ▼
Step 2 (__synthDev 型定義 + Web 側 API) ─ Step 3 (wasm-opt --print-stats)
  └─ 既存負債解消、独立、並列可
      ▼
Step 4 (params.json + gen-params.mjs 拡張)
  └─ Piano kind enum + Modal 係数 + 専用フィールド生成
      ▼
Step 5 (dispersion.rs 実装)
      ▼
Step 6 (KarplusStrong に dispersion フィールド + note_on で a1 算出)
      ▼
Step 7 (process_sample で cascade 適用) ← Step 6 完了が前提
  └─ D67 Phase 4a 互換性をここで機械保証 (test_dispersion_disabled_matches_phase4a)
      ▼
Step 8 (note_on_internal で hammer 経路分岐)
      ▼
Step 9 (Voice trait + VoicePool に set_dispersion_active)
      ▼
Step 10 (Piano Modal 係数の Engine 経由動作確認) ← Step 4 完了が前提
      ▼
Step 11 (Engine::apply_instrument 末尾に set_dispersion_active 呼出を追加、Phase 4a D53 即時 release を継承) ← Step 9 + Step 10 完了が前提
      ▼
Step 12 (messages.ts + synth-processor.ts で 'piano' 追加) ← Step 11 完了が前提
      ▼
Step 13 (preset-schema.ts + factory-presets.ts で Piano エントリ) ← Step 12 完了が前提
      ▼
Step 14 (synth-processor.ts に dev-only timing 集約 + synth.svelte.ts に measureProcessTime) ← Step 2 + Step 12 完了が前提
      ▼
Step 15 (統合 cargo test + alloc + release timing + Phase 4a 互換性) ← Step 14 まで完了が前提
      ▼
Step 16 (実機確認 + __synthDev.measureProcessTime で F48 計測)
      ▼
Step 17 (ドキュメント整備)
      ▼
Step 18 (PR 作成 + main マージ)
```

並列実装可能なポイント:

- Step 2（`__synthDev` 型定義）と Step 3（`wasm-opt --print-stats`）は独立、並列可
- Step 5（`dispersion.rs`）は他に依存なし、Step 4 と並列可（Piano const は import せず B 値を引数で受ける純粋関数）
- Step 9（trait 拡張）と Step 10（Modal 動作確認）は Step 4 + Step 6 完了後ならどちらが先でも可

## 達成ライン早見表

| ステップ完了 | 達成する F-tag |
|---|---|
| Step 1 | F56（`.gitattributes` LF 統一） |
| Step 2 | F48 準備（型定義 + Web 側 API） |
| Step 3 | F49 部分（baseline 記録） |
| Step 4 | F49（生成パイプライン）、F53 準備 |
| Step 5 | F51-a〜f（Dispersion 単体） |
| Step 6 | F51-g（KarplusStrong 統合） |
| Step 7 | F51 完成、F55 (Phase 4a 互換性 D67) |
| Step 8 | F52（Hammer model） |
| Step 9 | F58 準備（trait 拡張） |
| Step 10 | F53-a〜e（Piano Modal 係数） |
| Step 11 | F54（即時 release + dispersion_active 切替、Phase 4a D53 継承） |
| Step 12 | InstrumentKindKey 拡張完了 |
| Step 13 | Piano プリセット UI 反映 |
| Step 14 | F48 完成（dev-only timing 集約 + 計測 API） |
| Step 15 | F50 / F55 / F57 / F58（alloc / timing / 互換性 / regression） |
| Step 16 | F48 / F49 / F51-F54 / F57 実機 + F38b 再計測 |
| Step 17 | ドキュメント完成 |
| Step 18 | Phase 4b 完成 |

すべての F48〜F58 + Phase 4a 既存 F1〜F47 が達成された時点で Phase 4b 完成。F50（release timing）と F58（alloc ゼロ）は **Step 15 で必須化**、F48 は **Step 14 で完成 + Step 16 で実機計測**。F55（Phase 4a 互換性）は **Step 7 で機械保証 + Step 16 で実機確認**。

## 実装着手者へのメモ

- **Step 1（`.gitattributes` LF 統一）が Phase 4b 着手最初の作業**。Phase 4a で頻発した CRLF/LF 戦争を断つ。`git add --renormalize` で大量差分が出るが、独立した commit で分離すれば後続 Step が clean に進む
- **Step 4（params.json 拡張）の生成物は git commit する**（Phase 1〜4a の D25 継承）。生成物のレビュー観点は「Default 〜 Sitar 7 楽器の値が Phase 4a 既存値と完全一致しているか」が最重要（regression 防止）
- **Step 5（`dispersion.rs`）の `compute_dispersion_a1` は Faust 由来 closed-form 式の Rust 移植**。マジック定数 `K1〜K3` / `M1〜M4` は文献値、`#![allow(clippy::approx_constant)]` をモジュール冒頭で適用
- **Step 7（`process_sample` で cascade 適用）が Phase 4b 互換性の核心**。`dispersion_active = false` 経路が Phase 4a と完全一致（`thiran.process(self.buffer[read_z])` の引数）、`test_dispersion_disabled_matches_phase4a` が ε=1e-6 でバイト一致を機械保証
- **Step 7 の Phase 4a golden 値生成**: Step 7 着手前に Phase 4a HEAD で `cargo test -- --nocapture` を一度実行して buf_l/r の最初 256 frame を取得、`tests/fixtures/phase4a_default_c4.json` に保存（または Rust 定数として埋め込む）
- **Step 8（hammer 経路）の cutoff 計算**: `1.0 - exp(-2π·fc/fs)` で α を算出、`α.clamp(0.001, 0.999)` で安全側に制限。velocity = 0 でも cutoff_low (800 Hz) は適用（completely silent にはならない）
- **Step 11（apply_instrument 末尾に set_dispersion_active 呼出）の重要点**: 当初 D63 で 5 ms fade-out を提案していたが指摘事項 #3 反映で撤回（SmoothedValue 同期 set_target で fade-out は実現不能）。Phase 4b では Phase 4a D53 即時 release を完全継承し、`apply_instrument` 末尾に `let active = matches!(kind, InstrumentKind::Piano); self.pool.set_dispersion_active(active);` の 2 行を追加するのみ
- **Step 14（dev-only timing 集約）の DEV_MODE フラグ**: esbuild の `define: { DEV_MODE: 'true' / 'false' }` で build 時に置換、production `if (false)` ブロックは tree-shake で完全削除される。`pnpm dev` で `true`、`pnpm build` で `false`
- **Step 15（release timing test）の閾値**: Piano kind avg < 1.7 ms（Phase 4a 0.023 ms + dispersion +0.033 ms = 0.056 ms 想定の 30× 余裕）、非 Piano avg < 1.0 ms（Phase 4a 互換性、dispersion skip で CPU 増加なし）
- **Step 16（実機確認）の Phase 4a 互換性**: Default プリセット + Mod Wheel = 0 で Phase 4a と同じ音が出ることが最重要 regression check。Phase 4a の 6 楽器すべてで音色変化なしも実機確認
- **各ステップで コミットを分ける** ことを推奨（Phase 1 / 2 / 3 / 4a と同じ）。問題発生時に二分探索しやすい。コミットメッセージは `feat(dispersion): step 5 - Stretching all-pass cascade 実装 (D57, D58, D59, F51)` のように Step 番号 + D-tag + F-tag を含める
- **Phase 4a 既存テスト 120 PASS が各ステップで壊れないこと**を最優先。Step 6 / 7 / 8 / 11 で KarplusStrong / VoicePool / Engine を書き換えるため、各 Step 完了時に `cargo test -p dsp-core` を回す
- 詰まったら [`06-build-and-verify.md` §トラブルシューティング](./06-build-and-verify.md#トラブルシューティング-tips) を参照
