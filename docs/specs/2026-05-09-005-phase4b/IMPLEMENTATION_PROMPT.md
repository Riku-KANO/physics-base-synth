# Phase 4b 実装セッション用プロンプト

新しい Claude Code セッションで Phase 4b の実装を開始する際、以下のプロンプトを冒頭に貼り付けてください。

---

## プロンプト本体（コピー用）

```
Phase 4b の実装を開始します。仕様書は `docs/specs/2026-05-09-005-phase4b/` に揃っており、
合計 9 ファイル（pre-research + 01〜07 + IMPLEMENTATION_PROMPT）。実装着手前に必ず以下を読んでください。

## 必読

1. `docs/specs/2026-05-09-005-phase4b/07-implementation-checklist.md` — 18 ステップの実装順
2. `docs/specs/2026-05-09-005-phase4b/01-overview.md` — D56〜D67 の設計判断 12 項目
3. `docs/specs/2026-05-09-005-phase4b/03-dsp-core-spec.md` — DSP コアの全 API 仕様（最重要、Stretching all-pass cascade + Hammer model）
4. `docs/specs/2026-05-09-005-phase4b/06-build-and-verify.md` — F48〜F58 検証項目とリスク R37〜R39
5. `CLAUDE.md` — プロジェクト全体のルール / Phase 1-4a の制約

## 進め方

仕様書ドリブン開発（CLAUDE.md §「仕様書ドリブン開発」）に従い、**仕様書を主導**として実装を進める。
仕様書通りで冗長と感じる箇所も基本的にそのまま実装し、逸脱しない。

実装は **Step 1 から順に**、1 ステップ ≈ 1 コミット粒度（Phase 1 / 2 / 3 / 4a と同じ）。各 Step 完了時に
`cargo test -p dsp-core` で **Phase 4a 既存 120 テスト + 1 IGNORED がすべて通ること**を確認しながら進める。

既存実装（Phase 4a までの `crates/dsp-core/src/`、`crates/wasm-audio/src/lib.rs`、
`web/src/lib/`）を破壊しないこと。

## Step 1 概要（最初の作業）

**`.gitattributes` で改行 LF 統一**（D65 / F56、Phase 4b 着手最初の作業）:

1. リポジトリ root に `.gitattributes` を作成（仕様書 07 章 §Step 1 を参照）
2. `git add .gitattributes && git commit -m "chore: add .gitattributes for LF line endings (D65, F56)"`
3. `git add --renormalize .` で既存 file を LF 統一
4. **大量の CRLF→LF 差分が出るが内容変更ではない**、独立した commit で分離:
   `git commit -m "chore: normalize line endings to LF (D65, F56)"`
5. `git ls-files --eol | grep -v "i/lf"` で LF 以外の text file がゼロ
6. `pnpm fmt` を実行して差分が出ないこと（CRLF/LF 戦争の終結確認）

git commit を 2 つに分けることで、後続 Step での format 差分汚染を防ぐ。

## 重要な制約（仕様書から抜粋、絶対遵守）

- **`process` 中ヒープ確保ゼロ**: `Engine::prepare` で全バッファ事前確保、`process_sample` /
  `note_on` / `apply_instrument` 経路で `Vec::push` / `resize` を呼ばない。
  `dispersion_stages: [DispersionStage; 8]` は inline 配列（Vec ではない）、Hammer LPF も note_on 内 stack 変数のみ。Phase 1 D4 維持
- **C ABI のみ、wasm-bindgen 不使用**: `#[unsafe(no_mangle)] pub extern "C" fn`、
  `scripts/check-wasm-exports.mjs` の REQUIRED 配列で検証。**Phase 4b で C ABI 関数追加なし**、
  `synth_apply_instrument` の `kind` 値域が 0-6 → 0-7 に拡張されるのみ（既存関数の内部分岐）
- **依存ゼロ**: dsp-core / wasm-audio に外部 crate を追加しない（`microfft` / `heapless` 等も不可）。
  Phase 4a の binaryen は npm devDependency で継続
- **ring buffer 不変条件**: write_index / read 位置は `% buf_len` で剰余、`% length_int` には
  しない（dispersion で length_int の動的変動はないが、Phase 4a の LFO Pitch destination 経路は維持）
- **Phase 4a 既存 API 完全互換**: C ABI 既存 18 関数 + memory export = 19 required exports / Voice trait の
  既存メソッドシグネチャは不変、`set_dispersion_active(bool)` のみ追加
- **D67 互換性のバイト一致保証**: Default + Mod Wheel = 0 + LFO depth = 0 で Phase 4a と Phase 4b の
  process 出力が **ε=1e-6 でバイト一致**。`test_dispersion_disabled_matches_phase4a` で機械保証
- **`InstrumentKind::Default = 0` = Phase 3 既存ギターボディ係数**: Phase 4a と完全一致を維持、
  Piano は新規 7 番目として追加（既存 0-6 を保持）
- **生成物 git commit**: `params.rs` / `params.ts` は `pnpm gen:params` で生成 → commit
  （Phase 1-4a の D25 継承）

## 開発コマンド

```powershell
pnpm dev                                      # dev WASM ビルド + Vite dev server (DEV_MODE=true)
pnpm build                                    # release WASM (wasm-opt -O3) + SvelteKit build (DEV_MODE=false)
pnpm preview                                  # 本番プレビュー (4173)、F38b 計測用
pnpm build:wasm                               # release WASM のみ (wasm-opt 適用)
pnpm build:wasm:dev                           # dev WASM のみ (wasm-opt スキップ)
cargo test -p dsp-core                        # dsp-core ユニットテスト全件
cargo test -p dsp-core test_dispersion_       # Dispersion テストのみ
cargo test -p dsp-core test_apply_instrument_piano  # Piano 楽器切替テスト
cargo test -p dsp-core test_dispersion_disabled_matches_phase4a  # D67 互換性
cargo test --release -p dsp-core test_engine_process_block_timing_phase4b_  # release timing
pnpm check                                    # cargo check + svelte-check + params-sync
pnpm lint                                     # cargo clippy（warnings = errors）
pnpm fmt                                      # 整形
pnpm gen:params                               # params.json から生成
wasm-opt --print-stats web/static/wasm-audio.wasm  # Phase 4b Step 3、Phase 4a baseline 記録
```

## ブランチ運用

- 現在 `phase4b-spec` ブランチ（仕様書のみ、本セッションで main にマージ済み想定）
- 実装は **`phase4b-impl` ブランチを切って進める**: `git checkout -b phase4b-impl`
- `main` は branch protected、PR 必須、CI 緑必須
- Step 単位でコミット、Step 18 完了時に PR 作成（`gh pr create`）→ CI 緑確認 → main マージ
- Phase 4a では Step 単位コミット → 一括 PR の流儀、Phase 4b でも同じ

## まず最初に行うこと

1. `phase4b-impl` ブランチを切る: `git checkout -b phase4b-impl`
2. `cargo test -p dsp-core` を一度実行して、Phase 4a 既存 120 件 + 1 IGNORED すべてパスを確認（regression baseline）
3. 仕様書 4 件（07 / 01 / 03 / 06）を必読
4. Step 1（`.gitattributes` LF 統一）に入る

途中でレビュー観点や仕様書との不整合に気づいたら、その時点で報告してください。
仕様書改訂が必要なら 01〜07 を編集してから実装を進めます（Phase 3 では D36 案 D
の追加で 1 度仕様書を改訂、Phase 4a では Triangle 式の typo 修正で仕様書を改訂しています）。
```

---

## 補足メモ

### Phase 4a からの差分理解

- Phase 4a で `engine.rs::apply_instrument` が `pool.all_notes_off()` + Modal 差し替え + reset の即時 release だったが、Phase 4b Step 11 で **末尾に `pool.set_dispersion_active(matches!(kind, Piano))` の 1 行を追加**（D67）。当初 D63 で「5 ms fade-out」を提案したが SmoothedValue 同期 set_target の実現不能性により撤回（指摘事項 #3 反映）、Phase 4a D53 即時 release を完全継承
- Phase 4a で `KarplusStrong::process_sample` の `read_z` 値が `thiran.process(buffer[read_z])` で直接 Thiran に渡っていたが、Phase 4b Step 7 で **`dispersion_active = true` のとき 8 段 dispersion cascade を経由** する分岐を追加（D60）。`dispersion_active = false` (Phase 4a 既存 7 楽器) では Phase 4a と完全一致
- Phase 4a で `note_on_internal` の buffer 初期化が pluck noise burst + pick comb のみだったが、Phase 4b Step 8 で **`dispersion_active = true` で hammer 経路 (impulse + velocity LPF)** に分岐（D61）
- Phase 4a で `InstrumentKind` が 0-6 (Default + 6 楽器) だったが、Phase 4b Step 4 で **Piano = 7 を追加**、`gen-params.mjs` 拡張で BODY_MODES_PIANO_L/R / 専用フィールド (inharmonicity_b / hammer_cutoff_*) を出力（D62）
- Phase 4a で Worklet `process` self time の実機計測が手動 (Chrome DevTools Performance タブ) だったが、Phase 4b Step 14 で **`__synthDev.measureProcessTime(durationMs)` で Console から呼べる API に自動化** （D66）。計測は **AudioWorkletGlobalScope の `performance.now()` 差分**で self time を実測（指摘事項 #1 反映: `currentFrame` は callback 内で進まないため self time 計測には使えない）
- Phase 4a で頻発した CRLF/LF 戦争が、Phase 4b Step 1 の **`.gitattributes` LF 統一** で解消（D65）

### Step 1 (.gitattributes) で大量差分が出る理由

- リポジトリ内の既存 file が Windows + git autocrlf で CRLF として staging area に入っていた可能性
- `.gitattributes` で `* text=auto eol=lf` を宣言すると、Git が working tree (CRLF) と repository (LF) の不一致を検出
- `git add --renormalize .` でこの不一致を解消、結果として「内容変更なし、改行コードのみ変更」の commit が作られる
- これは **Phase 4b 後続 Step での意図しない CRLF/LF 差分汚染を防ぐ** ための前処理であり、独立 commit で分離する意義がある
- 以降の `pnpm fmt` で改行コード差分が出なくなる（Phase 4a で 4-5 回繰り返し発生していた問題が解消）

### Step 4 (params.json + gen-params.mjs) で生成物の重要性

- `gen-params.mjs` の Phase 4b 拡張は Piano 専用フィールド (`inharmonicity_b` / `hammer_cutoff_low_hz` / `hammer_cutoff_high_hz`) を **kind === 'Piano' のときのみ必須**として validation
- `INSTRUMENT_KIND_COUNT` が 7 → 8 に変わるため、Phase 4a 既存テスト `test_default_instrument_matches_phase3_modes` が引き続き通ることを確認（Default kind の値は Phase 4a と完全一致）
- TS 側の `InstrumentKindKey` 型に `'piano'` が追加されるため、`preset-schema.ts` / `factory-presets.ts` 等の TypeScript 型推論が連鎖的に拡張される（Step 12-13 で型エラー解消）

### Step 5 (`dispersion.rs`) の核心実装

```rust
// Faust `piano_dispersion_filter` の Rust 移植 (D59)
pub fn compute_dispersion_a1(m: u32, b: f32, f0: f32, fs: f32) -> (f32, f32) {
    use core::f32::consts::PI;

    let m_f32 = m as f32;
    let trt = 2.0_f32.powf(1.0 / 12.0);
    let bc = b.max(1.0e-6);
    let log_bc = bc.ln();
    let ikey = ((f0 * trt) / 27.5_f32).ln() / trt.ln();

    // K1=-0.00179, K2=-0.0233, K3=-2.93 (文献値)
    let kd = (K1 * log_bc * log_bc + K2 * log_bc + K3).exp();
    // M1=0.0126, M2=0.0606, M3=-0.00825, M4=1.97 (文献値)
    let m_log = m_f32.ln();
    let cd = ((M1 * m_log + M2) * log_bc + M3 * m_log + M4).exp();

    let d = (cd - ikey * kd).exp();
    let a1 = ((1.0 - d) / (1.0 + d)).clamp(-0.999, 0.999);

    let wt = 2.0 * PI * f0 / fs;
    let sin_wt = wt.sin();
    let cos_wt = wt.cos();
    let polydel = |a: f32| -> f32 { (sin_wt / (a + cos_wt)).atan() / wt };
    let group_delay_per_stage = polydel(a1) - polydel(1.0 / a1);

    (a1, group_delay_per_stage)
}
```

マジック定数 `K1〜K3` / `M1〜M4` は Rauhala-Välimäki 2006 / Faust `misceffects.lib::piano_dispersion_filter` の文献値。`#![allow(clippy::approx_constant)]` をモジュール冒頭で適用（一部値が `approx_constant` lint に引っかかる可能性のため、Phase 4a で確立したパターン）。

### Step 7 (D67 互換性核心テスト) の核心テスト

```rust
// crates/dsp-core/tests/karplus_strong_dispersion_tests.rs
#[test]
fn test_dispersion_disabled_matches_phase4a() {
    let mut engine = Engine::new();
    engine.prepare(48000.0, 128);
    // Default kind / Mod Wheel=0 / LFO depth=0 / 全パラメータ Phase 4a 既定値
    engine.note_on(60, 0.8);  // C4

    let mut buf_l = vec![0.0; 256];
    let mut buf_r = vec![0.0; 256];
    engine.process(&mut buf_l, &mut buf_r);

    // Phase 4a の golden 値 (test fixture または直接埋め込み定数)
    // → Phase 4a HEAD で `cargo test -- --nocapture` を一度実行して取得
    let golden_l = phase4a_golden_buf_l_default_c4_v08();
    let golden_r = phase4a_golden_buf_r_default_c4_v08();

    for i in 0..256 {
        assert!((buf_l[i] - golden_l[i]).abs() < 1e-6,
            "L mismatch at frame {}: phase4b={} vs phase4a={}",
            i, buf_l[i], golden_l[i]);
        assert!((buf_r[i] - golden_r[i]).abs() < 1e-6,
            "R mismatch at frame {}: phase4b={} vs phase4a={}",
            i, buf_r[i], golden_r[i]);
    }
}
```

これが **Phase 4b の Phase 4a 互換性保証の中核**。Default kind では `dispersion_active = false`、`process_sample` で `else` 分岐に入って Phase 4a と完全一致した経路を通る。

### Step 11 (Engine::apply_instrument 末尾に set_dispersion_active 呼出を追加) の重要点（D63 改訂後）

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

- **指摘事項 #3 反映**: 当初 D63 で「5 ms fade-out」を提案していたが、`SmoothedValue::set_target` は target 代入のみで `current` は `next_sample()` でしか進まないため、同期メソッド内で `set_target(0.0)` → `set_target(prev_value)` を実行しても fade-out は発生しない。状態機械（`PendingInstrumentChange`）導入も検討したが Phase 4b 主目的（ピアノ音色）に対する実装複雑度が大きいため、**Phase 4a D53「即時 release」を完全継承**
- **Phase 4b 新規追加は 2 行のみ**: `let active = matches!(kind, InstrumentKind::Piano); self.pool.set_dispersion_active(active);`
- **pop noise 軽減なし**: 楽器切替時の Body z1/z2 不連続による pop noise は Phase 4a と同レベルで残る。fade-out / cross-fade（`PendingInstrumentChange` 状態機械）は Phase 4c 送り

### Step 14 (dev-only timing 集約) の DEV_MODE フラグ（指摘事項 #2 反映）

```typescript
// synth-processor.ts 冒頭
declare const DEV_MODE: boolean;
//                       ^^^^^^^ esbuild --define で `true` / `false` に置換される識別子
//                       ローカル `const DEV_MODE = ...` は禁止 (define 対象外、置換されない)
```

- **`web/package.json` の build:worklet:dev / build:worklet スクリプトに `--define:DEV_MODE=true` / `--define:DEV_MODE=false` を esbuild に渡す引数を追加** する仕様変更
- `declare const DEV_MODE: boolean;` で TS の型エラーを回避（ランタイムには存在しない型のみ宣言）
- `pnpm dev` (build:worklet:dev = `--define:DEV_MODE=true`) で `DEV_MODE` 識別子が `true` リテラルに置換
- `pnpm build` (build:worklet = `--define:DEV_MODE=false`) で `false` に置換 → `if (false) { ... }` ブロックが tree-shake で完全削除
- production bundle に `DEV_MODE` / `__synthDev` / `measureProcessTime` の文字列が **0 hits** であることを Step 15 の grep で確認

### Step 14 の計測方式（指摘事項 #1 反映）

- **`AudioWorkletGlobalScope.performance.now()` 差分で self time を実測**: `process` 開始時 / 終了時の `performance.now()` 値の差分を ms 単位で取得（精度 ~5μs、ブラウザ依存）
- **`currentFrame` は使わない**: callback 内では値が進まないため self time 計測には使えない（音声時間 128/sampleRate ≈ 2.67ms を返すだけ）
- **リングバッファ `Float32Array(4096)`**: 48kHz / 128 frames で 375 quanta/sec、4096 entry で約 10.92 秒分を保持。`durationMs ≤ 10000` なら overflow なし、超えると最新 ~10.92 秒分のみ有効サンプルとして残る（`bufferOverflow=true` で報告）
- **stop メッセージで時系列順に並べ直し**: wrap している場合は `[writeIndex..capacity)` ++ `[0..writeIndex)` の順に main へ送る（古いサンプル → 新しいサンプル）

### 仕様書改訂が必要になったときの判断

- Phase 3 では D36 で仕様書原案の前提（C8 自己発振）が崩れ、案 D を追加して Step 1 完了時に仕様書改訂をコミットした経緯あり
- Phase 4a では Triangle 式の typo を実装段階で発見、同 commit で仕様書側も修正
- Phase 4b で同様の発見があれば、**仕様書側を改訂してから実装を進める**（CLAUDE.md §「仕様書ドリブン開発」）
- 軽微な記述ミス（typo / 章番号ずれ）は実装と同じ commit で修正可能、設計判断の変更は別 commit で
- ユーザーに報告して承認を得てから改訂

### Phase 4c 予告

Phase 4b 実装完了 + retrospective 後に別計画で着手:
- 候補: C8 自己発振 / Pick fractional 化 / Look-ahead limiter / WASM SIMD / LFO 拡張 / Cross-tab preset 同期 / Preset import/export / Mono+Sustain 本実装 / 複数 Piano 機種 / Hammer Hardness UI / Sustain×sympathetic resonance / Hertz law hammer
- 仕様書ディレクトリ: `docs/specs/<YYYY-MM-DD>-006-phase4c/`
- Phase 4b の retrospective で候補から優先順位を確定する
- Phase 4b の実装完了が前提条件、本仕様書には含めない

### コミット粒度の参考

Phase 4a (17 Step) では実際は Step 単位 + 仕様書改訂 + 中間 fix で 16 commits、PR 1 本でマージ。
Phase 4b (18 Step) も同程度の commit 数を想定。

| Step | 概算 commit 数 |
|---|---|
| 1 (.gitattributes) | 2（独立 commit を 2 つに分離） |
| 2-3 (`__synthDev` 準備 + wasm-opt --print-stats) | 2 |
| 4 (params + gen) | 1（生成物 commit 込み） |
| 5-7 (dispersion 実装) | 3 |
| 8 (hammer 経路) | 1 |
| 9-11 (trait + Engine apply_instrument) | 3 |
| 12-14 (Web フロントエンド) | 3 |
| 15 (統合テスト) | 1 |
| 16 (実機確認) | 1（チェックリスト埋めるのみ、コード変更なし可） |
| 17-18 (ドキュメント + PR) | 2 |
| **合計** | **19 commits** |

各 Step は仕様書 07 章で詳細手順 + テスト追加内容が明示されているため、実装中は仕様書を逐次参照すれば判断は最小限で済む。
