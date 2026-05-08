# 06. Phase 3 ビルド・実行・検証

## 目的

Phase 1 [06 章](../2026-05-06-001-mvp/06-build-and-verify.md) と Phase 2 [06 章](../2026-05-07-002-phase2/06-build-and-verify.md) を起点に、Phase 3 で発生する **ビルド手順の差分**（既存スクリプト不変、`params.json` 拡張のみ）、**追加検証項目 F26〜F38** の判定基準と検証手順、**追加リスク R24〜R29**、**性能目標** を定義する。Phase 1 / 2 セットアップ手順は完全継承する。

## 他文書との関係

- 上流: [`02-architecture.md`](./02-architecture.md)（プロジェクト構造、ビルドスクリプト）、[`05-web-frontend-spec.md`](./05-web-frontend-spec.md)（Worklet ビルド経路、VoiceMeter / PolyphonyToggle）
- 下流: [`07-implementation-checklist.md`](./07-implementation-checklist.md)（実装手順、Step 1 が Thiran 試作）
- 参考: Phase 1 / 2 06 章（初回セットアップ、開発時のコマンドフロー、F1〜F25 検証手順、リスク表 R1〜R23、トラブルシューティング、性能目標、デプロイ）— **本書で明示的に変更しない部分はすべて Phase 1 / 2 の記述を継承**

## 初回セットアップ

[Phase 1 06 章 §初回セットアップ](../2026-05-06-001-mvp/06-build-and-verify.md#初回セットアップ) を **完全継承**。Phase 3 で追加のセットアップ手順なし。`scripts/gen-params.mjs` が Body Mode 24 値 + Stereo Spread 1 値を扱うが Node 標準ライブラリのみ使用するため追加 npm package 不要。

## 開発時のコマンドフロー

### 通常の開発サイクル（Phase 3 版、Phase 2 と不変）

```powershell
# Rust 側を変更したとき
pnpm build:wasm:dev

# UI/Worklet 側だけ変更したとき
pnpm --filter web dev

# まとめて起動
pnpm dev
```

`pnpm dev` 内で `gen:params` が前段に走り、Body Mode 係数を含む生成物が更新される。

### 動作確認の最初の一歩（Phase 3 版）

[Phase 1 / 2 06 章 §動作確認の最初の一歩](../2026-05-06-001-mvp/06-build-and-verify.md#動作確認の最初の一歩) を継承。Phase 3 では Step 1（Thiran 試作）後に F29 (C8 自己発振) と F26 (Modal Body) を追加検証する。

### 本番ビルドの確認

```powershell
pnpm build
# 内部で gen:params → cargo build --release → copy-wasm → check-wasm-exports → vite build が走る

pnpm --filter web preview
# http://localhost:4173 で本番バンドルを確認
```

## ビルドアーティファクトのパス一覧

[Phase 2 06 章 §ビルドアーティファクトのパス一覧](../2026-05-07-002-phase2/06-build-and-verify.md#ビルドアーティファクトのパス一覧) を継承、Phase 3 で内容のみ拡張。

| 種別 | パス | Phase 3 差分 |
|---|---|---|
| `params.json` | `params.json` | Body Mode 24 値 + Stereo Spread 追加（D32） |
| 生成 Rust | `crates/dsp-core/src/params.rs` | `BodyMode` struct + `BODY_MODES_L/R` + `STEREO_SPREAD` 含む |
| 生成 TS | `web/src/lib/audio/generated/params.ts` | `BodyMode` interface + 同 export 含む |
| WASM バイナリ（コピー後） | `web/src/lib/wasm/wasm_audio.wasm` | `synth_midi_cc` / `synth_pitch_bend` / `synth_voice_state_ptr` export 含む |
| Worklet バンドル | `web/static/worklet/synth-processor.js` | Voice State stride push、midi_cc / pitch_bend dispatch 含む |
| 静的サイト | `web/build/` | VoiceMeter / PolyphonyToggle / midi-cc.ts 含む |

## ParamDescriptor 同期チェック

Phase 2 06 章の `check-params-sync.mjs` を継承。Phase 3 で `params.json` に `body_modes` / `stereo_spread` セクションが追加されるが、`generateRustSource` / `generateTsSource` の出力に含まれるため文字列一致判定だけで drift 検知が成立する。

## C ABI 検証

`scripts/check-wasm-exports.mjs` の REQUIRED 配列に Phase 3 で追加した 3 関数（`synth_midi_cc` / `synth_pitch_bend` / `synth_voice_state_ptr`）を追記する。`pnpm build:wasm` 実行時に検証され、3 関数のいずれかが export されていなければ exit 1。

## 検証項目（Phase 3 追加 F26〜F38）

Phase 1 [F1〜F9](../2026-05-06-001-mvp/06-build-and-verify.md) と Phase 2 [F10〜F25](../2026-05-07-002-phase2/06-build-and-verify.md) を継承し、Phase 3 で 13 件追加。**F1〜F25 の実機検証は持ち越し継続**（Phase 1 retrospective §7、Phase 2 retrospective §7、ある程度動作確認済みのため Phase 3 着手前提条件としない）。

| # | 検証 | 手順 | 判定基準 |
|---|---|---|---|
| **F26** | Modal Body Resonator のフィルタ応答 | `cargo test -p dsp-core test_single_biquad_*`（単体: DC blocking / peak / bandwidth、係数仕様の厳密検証）+ `test_modal_body_*`（aggregate: DC blocking / peak at modes / inter-mode attenuation / stereo / no alloc / reset、隣接干渉を許容した定性検証） | 全テストパス（単体: DC < 0.001、ピーク `mode.gain` ± 5%、aggregate: ピーク `mode.gain` の 0.5〜1.5 倍、stereo 差 3〜10%） |
| **F27** | Loss filter の周波数依存損失 | `cargo test -p dsp-core test_loss_filter_*` | DC ゲイン 1.0 ± 0.001、Nyquist 減衰 `(1-ρ)/(1+ρ)`、A6 では A4 より大きい ρ |
| **F28** | Pick position 励振 shaping の倍音減衰 | `cargo test -p dsp-core test_pick_min_beta_minimal_shape`（β=0.05 公開 API 最小値で comb 効果最小、外部から到達可能）/ `test_pick_position_node_at_beta_half`（β=0.5 で偶数倍音消失、FFT 検証）/ `test_pick_position_attenuates_kth_harmonic`（β=1/k で k 番目倍音減衰）/ `test_pick_position_no_extra_alloc`（β 変えて連打、buffer.len 不変）/ `test_pick_internal_k_zero_branch`（length_int=9 + β=0.05 で K=0 分岐を `#[cfg(test)]` 経由でテスト、外部 API 制約と内部分岐検証を分離）+ 実機聴感（β=0.05 / 0.125 / 0.5 で音色が変わる） | 全テストパス、β=0.5 で偶数倍音 < 0.1 倍、内部 K=0 分岐 panic なし、buffer.len() 不変 |
| **F29** | Thiran allpass の C8 自己発振 | **Step 1 で実施**: `cargo test -p dsp-core test_pitch_c8_thiran_self_oscillates` で C8 自己発振が 10 秒継続して tail RMS > 0.01 に収束 + A1〜C6 の精度劣化が +0.1% 以下 | テストパス → 案 A 採用、劣化なら案 B/C 検討（D36 確定） |
| **F30** | Brightness 群遅延補正後のピッチ偏移 | `cargo test -p dsp-core test_engine_brightness_pitch_correction`（A4 で brightness=0.5 設定、measure_f0 で誤差 < 0.5%） | Phase 2 の 0.89% 偏移が 0.5% 以下に解消 |
| **F31** | MIDI CC dispatch の動作 | `cargo test -p dsp-core test_engine_midi_cc_*`（CC#7 で `channel_volume` 独立更新 + `output_gain` 不変 (D38b 直交)、CC#64 で Poly のみ deferred note_off と pending bitmap、CC#123 で全 voice + hold_stack + sustain_state reset、未対応 CC で panic / alloc なし） | 全テストパス（CC#1 Mod Wheel は Phase 4 送りのため対象外、CC#7 直交 / CC#123 sustain reset / Mono+Sustain は no-op (Phase 2 D29 既存挙動継承) / pending clear 等の Sustain 統合テストすべて含む） |
| **F32** | Pitch Bend の滑らかな遷移 | `cargo test -p dsp-core test_pitch_bend_*`（5ms tau で +2 → 0 → -2 の遷移、frequency が連続変化、ring buffer 不変条件 `% buf_len` 維持） + 実機聴感（PC キーボード A 押しながら +/- でピッチ可変） | クリック音なし、5ms 遷移で frequency 中間値が補間、ring buffer 整合 |
| **F33** | Sustain Pedal の note_off 保留と相互作用（**Poly mode のみ Sustain 適用、Mono は無視**） | `cargo test -p dsp-core test_sustain_*` および `test_engine_midi_cc_sustain_*` / `test_engine_mono_sustain_no_op` / `test_engine_mode_switch_clears_sustain` + dev console での確認: **Poly mode** で CC#64=127 → A 押す → A 離す → 音が継続 → CC#64=0 で release。**Mono mode では Sustain は no-op**（Phase 2 D29 既存 last-note priority を完全継承、CC#64 値に関係なく即時 release / hold_stack 復帰、Mono+Sustain は Phase 4 で再評価）。Poly→Mono mode 切替時は `set_mode` で `sustain_state.reset()` され pending を全 release | 全 cargo test パス、実機で Poly Sustain on/off / 同一ノート再打鍵で pending bit 適切にクリア / Mono では Sustain 無視を確認 |
| **F34** | Voice Meter UI 表示更新 | 実機: `pnpm dev` でブラウザ起動、PC キーボード 8 鍵同時押下 → 全 8 セルが active 表示、振幅で輝度変化 | 21ms 周期で更新（ブラウザ描画と整合）、active セル数が押下数と一致 |
| **F35** | Soft clip の閾値挙動 | `cargo test -p dsp-core test_soft_clip_*`（linear in safe range / bounded / continuous / extreme）+ 実機（OutputGain=1.5 + 8 鍵全力 + Body Wet=1.0）で出力が ±1.0 以内 | 全テストパス（|x|≤0.95 で `assert_eq!(soft_clip(x), x)`、任意 x で `\|y\| < 1.0`） |
| **F36** | WASM gzip < 30 KB 維持 | `pnpm build` 後に gzip 計測 | < 30 KB（target）、想定 12.9 KB |
| **F37** | process 時間 < 1.5 ms 維持（**必須化**） | **`cargo test --release -p dsp-core test_engine_process_block_timing -- --nocapture` で 128 frame × 1000 回 process の平均時間を計測**、`< 1.5 ms / process` を `assert!`（Step 13 で実装、CI 必須）。Chrome DevTools Performance タブ計測は補助検証 | 平均 < 1.5 ms / process（128 frames @ 48kHz）。CI flaky 対策で許容を `< 2.0 ms` に設定可能だが目標は 1.5 ms |
| **F38** | Phase 3 全機能 ON で alloc ゼロ（Rust 側） | `cargo test -p dsp-core test_no_allocation_with_modal_body_and_midi_cc`（VoicePool prepare → 全 voice note_on → CC dispatch 連続 → Pitch Bend → process_sample 1 秒 → length 不変）+ Worklet 側 `process()` 内で `new` を呼ばない（コードレビューで確認） | テストパス + JS 側スクラッチ事前確保確認 |
| **F38b** | Worklet `process()` 全体の実機 process 時間 < 1.5 ms（**Phase 3 完成判定で必須**） | Chrome DevTools Performance タブで `pnpm preview`（本番ビルド）を 8 voice 同時 + Body Resonator + Pitch Bend 動作中に Record（10 秒）→ Stop。"Audio Worklet" レーンで `process` 関数の self time 平均と max を計測。**postMessage コストを含む実時間を測ること**。Step 14 完了前に必ず実施 | 平均 < 1.5 ms / max < 2.5 ms。超過時は R30 の対策案 (1)〜(4) のいずれかを適用 |

### 検証手順の補足

#### F29（Thiran allpass 試作評価）の詳細手順

Step 1 で実施する重要検証。pre-research §4.4 の方針:

```rust
// crates/dsp-core/tests/pitch_accuracy.rs に追加

#[test]
fn test_pitch_a1_thiran() {
    let f0 = measure_f0_thiran(33, 48000.0);  // Thiran builder を使う変種
    assert!((f0 - 55.0).abs() / 55.0 < 0.005, "A1 thiran error too large: {}", f0);
}

// 同様に test_pitch_a2_thiran / test_pitch_a4_thiran / test_pitch_c6_thiran / test_pitch_c8_thiran を追加

#[test]
fn test_pitch_c8_thiran_self_oscillates() {
    // 10 秒走らせて RMS が定常値 > 0.01 に収束することを確認
    let mut engine = ...;
    engine.note_on(108, 0.8);  // C8
    let mut samples = vec![0.0; 480000];  // 10 秒
    for s in &mut samples { *s = engine.process_sample(); }
    let tail_rms = (samples[..480000].iter().rev().take(48000).map(|x| x*x).sum::<f32>() / 48000.0).sqrt();
    assert!(tail_rms > 0.01, "C8 not self-oscillating: tail_rms = {}", tail_rms);
}
```

実装方法:
1. `crates/dsp-core/src/fractional_delay.rs` に `ThiranCoeffs` を Step 1 で追加（既存 `LagrangeCoeffs` と並列）+ `LagrangeCoeffs::set_fractional` を追加 + **`FractionalDelay` enum で統合**（03 章 §Fractional delay 拡張、`set_fractional` / `apply` / `reset` / `new_lagrange` / `new_thiran`）
2. `KarplusStrong` の field は `fractional_delay: FractionalDelay` の単一名で統一。`Engine::new` がデフォルト `new_lagrange()`、**`Engine::new_with_thiran()` を test-only constructor として追加**して `new_thiran()` を選択する経路を提供（07 章 Step 1）。`use_thiran: bool` フラグや `if/else` 分岐は不採用
3. Thiran 版テストを A1〜C8 で 5 件 + C8 自己発振 1 件実施、結果を `println!` で出力
4. 結果:
   - **すべて誤差 < 0.5%** + C8 自己発振成立 → 案 A 採用、`Engine::new` の選択を `FractionalDelay::new_thiran()` に切替え、enum 解消で `fractional_delay: ThiranCoeffs` 単一型 field 化（enum dispatch 除去）
   - **A1〜C6 で誤差悪化、C8 のみ改善** → 案 B（高域のみ Thiran、note_on で midi に応じて enum variant を選ぶ）採用
   - **C8 すら改善されない** → 案 C（Lagrange 維持、enum も解消して `LagrangeCoeffs` 単一 field 化、`test_pitch_c8` ignore 継続）

#### F37（process 時間）の詳細手順（**Phase 3 で必須化**）

Phase 2 06 章 §F16 では持ち越し可だったが、**Phase 3 では Step 13 で `cargo test --release` ベースの timing test を必須化**。リアルタイム DSP では process 時間の保証がないと完成判定できないため。

```rust
// crates/dsp-core/tests/dsp_core_tests.rs に追加（Step 13）
#[test]
#[cfg(not(debug_assertions))]  // release ビルドでのみ計測
fn test_engine_process_block_timing() {
    use std::time::Instant;
    let mut engine = Engine::new();
    engine.prepare(48000.0, 128);

    // 8 voice 全 active + MIDI CC + Pitch Bend で最悪ケース近似
    for i in 0..8 {
        engine.note_on(60 + i, 0.8);
    }
    engine.handle_pitch_bend(1.0);
    engine.handle_midi_cc(7, 0.8);

    let mut output_l = vec![0.0; 128];
    let mut output_r = vec![0.0; 128];
    const ITERATIONS: u32 = 1000;
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        engine.process(&mut output_l, &mut output_r);
    }
    let elapsed = start.elapsed();
    let per_block_us = elapsed.as_micros() as f64 / ITERATIONS as f64;
    let per_block_ms = per_block_us / 1000.0;
    println!("F37: process_block timing = {:.3} ms / 128 frames", per_block_ms);
    assert!(per_block_ms < 1.5, "F37 fail: {:.3} ms >= 1.5 ms", per_block_ms);
}
```

実装ガイドライン:
- `#[cfg(not(debug_assertions))]` で release ビルド限定（debug ビルドでは数倍遅いため flaky になる）
- CI は `pnpm check` の中で `cargo test --release -p dsp-core test_engine_process_block_timing` を必須化（PR ブロック条件）
- CI 環境（GitHub Actions ubuntu-latest）と手元 PC では速度差が出るため、CI 用に `< 2.0 ms` の緩い閾値を別 attribute で許容するのも可。**最低でも release ビルドで実測値を記録し、撤退判定に使う**

Chrome DevTools Performance タブ計測は **F38b として必須化**（Worklet 全体の process 時間 = JS dispatch + WASM 処理 + postMessage コスト を測る、F37 では測れない）。詳細は次節。

#### F38b（Worklet process 時間の実機計測、Phase 3 完成後必須）

F37 の cargo timing test は **Rust DSP 内部のみ**を測るため、AudioWorkletProcessor の JS dispatch / `postMessage`（Voice State push）/ `WebAssembly.Memory` view 操作のコストは含まれない。R30（Worklet postMessage の structured clone コスト不明）への対策として、Phase 3 完成判定の前に Chrome DevTools Performance タブで以下を実施:

1. `pnpm build && pnpm preview` で本番ビルドを `http://localhost:4173` に立ち上げる
2. Chrome 最新版で開き、Start Audio をクリック → AudioContext を resume
3. PC キーボードで 8 鍵同時押下、CC Sustain (dev console: `__synthDev` 経由 or MIDI keyboard)、Pitch Bend を動かす状態を作る
4. Chrome DevTools (F12) → Performance タブ → 設定で "CPU: 4× slowdown" は **無効**（実速度計測）
5. ⏺ Record を開始 → 10 秒待つ → ⏹ Stop
6. タイムラインで "Audio Worklet" レーンを展開、`process` 関数の **Self time** の avg / max を確認
7. 判定:
   - **avg < 1.5 ms かつ max < 2.5 ms** → F38b 達成、Phase 3 完成
   - **avg ≥ 1.5 ms** → R30 の対策案を適用（stride を 4096 に下げる / Voice Meter を削除 等）
   - **max ≥ 2.5 ms（オーディオドロップアウトリスク）** → 同上

Linux ベースの開発機なら Edge / Brave / Vivaldi 等の Chromium 派生ブラウザでも代替可。Firefox は profiling フォーマット異なるため Chrome 推奨。

#### F38（メモリ確保ゼロ）の詳細手順

Phase 2 F17 を Phase 3 用に拡張。`test_no_allocation_with_modal_body_and_midi_cc` で:

1. `Engine::new()` + `Engine::prepare(48000, 128)`
2. baseline = `engine.pool.voices[0].buffer.len()`（Phase 2 既存パターン）
3. 全 8 voice に note_on
4. `synth_midi_cc(handle, 7, 0.5)` (Volume) / `synth_midi_cc(handle, 64, 1.0)` (Sustain) / `synth_pitch_bend(handle, 1.0)` をそれぞれ 100 回ずつ呼ぶ。**追加で `synth_midi_cc(handle, 1, 0.5)` (Mod Wheel = Phase 4 送り、未対応 CC) も呼んで panic / alloc が発生しないことも確認**（CC#1 は no-op として無視されるべき）
5. `process_block` を 100 回（≈ 1 秒分）呼ぶ
6. baseline と比較、length 不変 → alloc ゼロ判定

### 既存検証項目の Phase 3 での扱い

| 項目 | Phase 1 / 2 | Phase 3 |
|---|---|---|
| F1〜F9 | Phase 1 実機検証 | **持ち越し継続**（ある程度動作確認済み） |
| F10〜F15 | Phase 2 cargo test レベルで達成、実機未検証 | **持ち越し継続**（cargo test は維持、実機は次フェーズ） |
| F16 | process 時間 < 1.5 ms 未検証 | **F37 で必須化**（release cargo timing test、Step 13 で実装、CI ブロック条件） |
| F17 | Phase 2 メモリ確保ゼロ | **F38 に統合** |
| F18〜F25 | Phase 2 mono / voice meter / サイズ等 | **持ち越し継続** |
| F25 | Phase 1 retrospective §2 達成記載 | 継続して未達（実機検証持ち越しのため） |

## リスクと対策表（Phase 3 追加 R24〜R29）

[Phase 1 06 章 §リスクと対策表](../2026-05-06-001-mvp/06-build-and-verify.md#リスクと対策表) の R1〜R16 と Phase 2 R17〜R23 を継承、Phase 3 で 6 件追加。

| # | リスク | 影響 | 対策 |
|---|---|---|---|
| **R24** | Modal Body の biquad で denormal 発生で CPU 急増 | F37 失敗（process 時間超過）、ARM/x86 共通の問題 | 各 biquad の出力に `+1e-25 -1e-25` トリック（D6 拡張、`modal_body.rs::process_sample` で実装）。実装時に `cargo test test_modal_body_no_denormal_cpu_spike` を追加して測定（参考） |
| **R25** | Thiran allpass の極が単位円上 / 外で発散 | F29 失敗、KS が無音化または無限大化 | `ThiranCoeffs::set_fractional` で **`d.clamp(1e-4, 0.999)`** を適用（下限 1e-4、上限 0.999）。これにより `a₁ = (1-d)/(1+d) ∈ [5e-4, 0.9998]` で **極が常に単位円内**（極 z = -a₁ で `|z| < 1`）を保証。**d=0 を除外する理由**: a₁=1.0 で極が単位円上 z=-1 となり境界（実装上は極零相殺で動作するが数値誤差で発散リスク）、d=1e-4 への clamp で fractional 誤差は 1e-4 サンプル ≈ 0.002% @ A4 で実用上無視可能。Step 1 試作で `test_thiran_pole_stability` を追加して d ∈ {0, 0.5, 0.999, 1.0} で発散しないことを検証 |
| **R26** | Pitch Bend で fractional delay の係数を毎サンプル再計算 → process 時間超過 | F37 失敗 | Pitch Bend が動いていないとき (length_target が定常) は係数再計算をスキップ（`length_frac` 差分が ≈ 0 なら skip）。Lagrange なら 12 演算 / sample 増、Thiran なら 3 演算 / sample 増、いずれも予算内見積 |
| **R27** | Voice State 共有メモリ buffer の write/read race で UI が中途半端な値を見る | F34 で active 数が瞬間的に間違う | Worklet が `process_block` 終端で書き、postMessage で main が受信。memory.buffer の atomic 不要（postMessage が happens-before 関係を保証）。Phase 4 で SharedArrayBuffer 化検討時は atomic 必須 |
| **R28** | MIDI CC が連続値で flooding し postMessage キューが詰まる | UI レスポンス低下、process スレッド遅延 | SynthEngine 側で前値一致なら送信スキップ（`_lastPitchBend` キャッシュ、`midi-cc.ts` での dedupe）。CC#64 / #123 はスロットルしないが頻度が低いため問題なし |
| **R29** | Modal Body 8 モードで CPU 余裕が見積より少なく F37 fail | 想定 +50 演算/sample が実測 +200 演算 / sample | (1) M=8 から M=5 に削減（高域モードは聴感差小）、(2) stereo を mono 化して計算量半減、(3) `wasm-opt -O3` 必須化、(4) どうしても駄目なら Body Resonator を Phase 4 送り（D30 撤退） |
| **R30** | Worklet `postMessage` の structured clone コストが render thread で支配的になり Voice State push が `process()` 全体の予算を圧迫（F37 cargo timing は Rust 内部のみで Worklet 全体の process 時間を測れない） | F38b（実機計測）で発覚、UI Voice Meter による音質劣化 | (1) F38b（Chrome DevTools Performance タブの実機 Worklet process 計測）を Phase 3 完成判定に必須化、(2) 1024 サンプル毎が重ければ 4096 サンプル毎（85 ms 周期）に下げる、(3) どうしても駄目なら Voice Meter UI を Phase 4 送り（UI から削除して `__synthDev` のみ残す）、(4) Phase 4 で `SharedArrayBuffer` + Atomics に移行（COOP/COEP ヘッダ必須、GitHub Pages から別ホスティングへ移行）|

## トラブルシューティング Tips

[Phase 1 / 2 06 章 §トラブルシューティング Tips](../2026-05-06-001-mvp/06-build-and-verify.md#トラブルシューティング-tips) を継承し、Phase 3 で追加。

### 「Modal Body が鳴らない / dry のみ」

- `params.json` の `BodyWet` が 0.0 になっていないか（デフォルト 0.5）
- `Engine::process` の per-sample loop で `wet * body_l + (1 - wet) * dry` 形式になっているか確認
- `modal_body.prepare(sample_rate)` が `synth_new` 内で呼ばれているか
- bandpass biquad の係数（特に `b0` / `b2` / `a0_raw` 正規化）が正しいか debug print で確認
- `BODY_MODES_L/R` の係数が NaN を含んでいないか（`gen-params.mjs` の出力を grep で確認）

### 「Modal Body の DC が漏れる」

- bandpass biquad は `b0 + b2 = 0`（DC ゲインゼロ）が必須。`calc_coeffs` の正規化で `b0 = +α·gain/a0_raw`、`b2 = -α·gain/a0_raw` になっているか確認
- 旧版仕様（resonator `H(z) = b0 / (1 + a1·z⁻¹ + a2·z⁻²)`）は DC 漏れの原因。bandpass 形に変更されていることを再確認

### 「C8 で Thiran 試作が安定しない」

- `ThiranCoeffs::set_fractional(d)` の `d` が **`[1e-4, 0.999]`** 範囲に clamp されているか（**`[0.0, 0.999]` ではない**、d=0 で a₁=1.0 となり極が単位円上 z=-1 に乗って境界バグの原因。R25 / D36）
- `a₁ = (1-d)/(1+d) ∈ [5e-4, 0.9998]` の範囲に収まっているか debug print で確認
- 状態 `z1_in` / `z1_out` が note_on で reset されているか（または前 note の状態を引き継いでいるか、設計方針と整合）
- `KarplusStrong::process_sample` で Lagrange と Thiran のどちらが使われているか debug print で確認（`self.fractional_delay` の enum variant を `match` で出す、`Engine::new` か `Engine::new_with_thiran` のどちらで構築されたかが判別する根拠）
- F29 fail 時は案 B（高域のみ Thiran）または案 C（Lagrange 維持）にフォールバック検討

### 「Pitch Bend で音が壊れる」

- `length_target` が `[2.0, max_len-3]` 範囲に clamp されているか（buffer overrun 回避）
- SmoothedValue の tau が 5ms（240 sample @ 48kHz）になっているか、長すぎると遅延、短すぎるとクリック
- `set_pitch_bend(±2.0)` で `freq * 2^(±1/6) ≈ ±12%` の delay 変化が実測で見えるか debug print

### 「Voice Meter が更新されない」

- Worklet 側で `frame_counter >= 1024` の条件分岐が走っているか
- `synth_voice_state_ptr` が non-null を返しているか
- `Uint8Array` view が `memory.buffer` の正しいオフセットを指しているか（grow 時の detach に注意）
- main 側 `voiceState.activeMask` の `$state` が `<VoiceMeter>` 内で再描画をトリガーしているか

### 「MIDI CC が効かない」

- WebMIDI API で `selectedInput.onmidimessage` が `handleMidiMessage` を呼んでいるか
- ステータスバイト 0xb0 の判定が正しいか（`data[0] & 0xf0 === 0xb0`）
- CC 番号の switch case で対象 CC（1 / 7 / 64 / 123）に case が用意されているか
- `engine.sendMidiCc(cc, value)` の `value` が 0..127 の整数か（小数で渡すと WASM 側 normalize で 0 になる）

### 「Soft clip で音が詰まる / 歪みが大きい」

- 閾値 0.95 で常用範囲はほぼ通過するはず。歪みが大きいなら入力 RMS が想定より大きい（OutputGain 過大）
- 区間関数の境界 `|x| = 0.95` で連続になっているか確認（`assert_eq!(soft_clip(0.95), 0.95)` でパスすべき）
- `|x| > 0.95` 分岐の式 `signum(x) · (0.95 + 0.05·e/(e+0.05))` が正しく実装されているか（`e = |x| − 0.95`、e=0 で +0、e→∞ で +0.05）
- soft clip を OutputGain の **後**に置いているか（前に置くと output_gain で増幅された値が `|x| > 0.95` 分岐に入って歪みが増える）

### 「`pnpm gen:params` が body_modes を出さない」

- `params.json` のトップレベルに `body_modes` セクションが書かれているか
- `gen-params.mjs` の `generateRustSource` / `generateTsSource` が `paramsJson.body_modes` を読んでいるか
- `applyStereoSpread(modes, spread)` 純粋関数が定義されているか
- 生成された `params.rs` を grep `BODY_MODES` で確認

## 性能目標（Phase 3）

| 指標 | Phase 2 実績 / 目標 | Phase 3 目標 | 備考 |
|---|---|---|---|
| `process` 時間（128 frames @ 48kHz、CPU 予算 2.67 ms）| < 1.5 ms 想定（未検証） | **< 1.5 ms（必須）** | F37 で計測（release cargo timing test、Step 13）。Phase 3 加算 +0.05 ms 想定（pre-research §9.3） |
| 起動から最初の音まで | < 2 秒 | < 2 秒（維持） | WASM サイズ +2.4 KB gzip 程度なら影響軽微 |
| WASM gzip サイズ | 10.56 KB | **< 30 KB（target）、想定 12.9 KB** | F36 で計測 |
| WASM raw サイズ | 24.24 KB | < 60 KB 想定 | wasm-opt 任意のまま |
| Worklet 本番バンドル | 5.04 KB | **< 10 KB**（維持） | F22 拡張、Voice State push ロジック +0.5 KB 想定 |
| ヒープ確保回数（process 中） | 0 回 | 0 回（維持） | F38 で検証 |
| ピッチ精度（A1〜C6） | ± 0.5% 以内 | **± 0.5% 以内（維持）+ ブライトネス補正で 0.89% → < 0.5%** | F30 / Step 1 試作で確認 |
| ピッチ精度（C8） | 物理限界、test_pitch_c8 ignore | **Step 1 で Thiran 採用なら ± 0.5% 達成、不採用なら継続 ignore** | F29 / D36 |
| 最悪ケース歪み（8 鍵全力 + OutputGain=1.5 + Body Wet=1.0）| 許容 | **soft clip でクランプ < 1.0** | F35 / D43 |
| Voice Meter 更新レート | — | 21 ms 周期（48 Hz、48 kHz / 1024 stride） | F34 |

## デプロイ

[Phase 1 06 章 §デプロイ](../2026-05-06-001-mvp/06-build-and-verify.md#デプロイ参考mvp の必須ではない) を継承。GitHub Pages の自動デプロイ（`main` ブランチへの push で `.github/workflows/deploy.yml` 発火）を維持。Phase 3 では `params.json` の `body_modes` セクション拡張で `gen:params` の出力が変わるが、CI 内 `pnpm build` で自動的に反映される。

### CI ワークフロー

Phase 2 [`.github/workflows/ci.yml`](../2026-05-07-002-phase2/06-build-and-verify.md#ci-ワークフローの追加チェック) を継承。Phase 3 で追加変更なし（`pnpm check` が `check:params-sync` を含むため、`body_modes` 追加でも drift 検知は機能する）。

CI 上で実行されるテストは Phase 2 の 41 件 + Phase 3 追加 25 件（[03 章 §テスト方針](./03-dsp-core-spec.md#テスト方針phase-3-新規追加分)）= 66 件目標。すべて `cargo test -p dsp-core` でパスすることが PR ブロック条件。
