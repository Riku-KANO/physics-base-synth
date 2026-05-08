# Phase 3 実装セッション用プロンプト

新しい Claude Code セッションで Phase 3 の実装を開始する際、以下のプロンプトを冒頭に貼り付けてください。

---

## プロンプト本体（コピー用）

```
Phase 3 の実装を開始します。仕様書は `docs/specs/2026-05-07-003-phase3/` に揃っており、
合計 4156 行 / 8 ファイル（pre-research + 01〜07）。実装着手前に必ず以下を読んでください。

## 必読

1. `docs/specs/2026-05-07-003-phase3/07-implementation-checklist.md` — 14 ステップの実装順
2. `docs/specs/2026-05-07-003-phase3/01-overview.md` — D30〜D43 / D38b の設計判断 14 項目
3. `docs/specs/2026-05-07-003-phase3/03-dsp-core-spec.md` — DSP コアの全 API 仕様（最重要）
4. `docs/specs/2026-05-07-003-phase3/06-build-and-verify.md` — F26〜F38b 検証項目とリスク R24〜R30
5. `CLAUDE.md` — プロジェクト全体のルール / Phase 1-2 の制約

## 進め方

仕様書ドリブン開発（CLAUDE.md §「仕様書ドリブン開発」）に従い、**仕様書を主導**として実装を進める。
仕様書通りで冗長と感じる箇所も基本的にそのまま実装し、逸脱しない。

実装は **Step 1 から順に**、1 ステップ ≈ 1 コミット粒度（Phase 1 / 2 と同じ）。各 Step 完了時に
`cargo test -p dsp-core` で **Phase 2 既存 41 テストがすべて通ること**を確認しながら進める。

既存実装（Phase 2 までの `crates/dsp-core/src/`、`crates/wasm-audio/src/lib.rs`、
`web/src/lib/`）を破壊しないこと。

## Step 1 概要（最初の作業）

**Thiran allpass 試作評価**（F29 / D36 確定）:

1. `crates/dsp-core/src/fractional_delay.rs` に以下を追加:
   - `ThiranCoeffs` 構造体（`a1` / `z1_in` / `z1_out`、`set_fractional(d)` で `d.clamp(1e-4, 0.999)`）
   - `LagrangeCoeffs::set_fractional(&mut self, d: f32)` メソッド（中身は `*self = Self::new(d)`）
   - `FractionalDelay` enum（`Lagrange(LagrangeCoeffs)` / `Thiran(ThiranCoeffs)`）と
     `set_fractional` / `apply` / `reset` / `new_lagrange` / `new_thiran` メソッド

2. `KarplusStrong` の field を `lagrange: LagrangeCoeffs` から
   `fractional_delay: FractionalDelay` に置換（Phase 2 既存 process_sample の呼び出しも
   `self.fractional_delay.apply(...)` に統一）。**`use_thiran: bool` フラグや
   `if/else` 分岐は不採用**

3. test-only constructor 経路を追加:
   - `KarplusStrong::new_with_fractional_delay(fd: FractionalDelay)` （`#[doc(hidden)]`）
   - `VoicePool::new_with_fractional_delay_factory<F: Fn() -> FractionalDelay>(factory: F)`
   - `Engine::new_with_thiran()` （`#[doc(hidden)]`、`VoicePool` 経由で全 voice に Thiran 注入）

4. `crates/dsp-core/tests/pitch_accuracy.rs` に Thiran 版テストを追加:
   - `test_pitch_a1_thiran` / `_a2_thiran` / `_a4_thiran` / `_c6_thiran` / `_c8_thiran`
   - `test_pitch_c8_thiran_self_oscillates`（10 秒走らせて tail RMS > 0.01）
   - 各テストは `Engine::new_with_thiran()` を呼んで Thiran 経路で計測

5. cargo test を実行して結果を整理し、**D36 確定**:
   - すべて誤差 < 0.5% + C8 自己発振成立 → **案 A 採用**: `Engine::new` を Thiran 切替、
     enum を解消して `fractional_delay: ThiranCoeffs` 単一型 field に置換
   - A1〜C6 で誤差悪化 +0.1% 超 → **案 B**: 高域のみ Thiran (note_on で midi に応じて enum variant 選択)
   - C8 すら改善されない → **案 C**: Lagrange 維持、enum を解消して `LagrangeCoeffs` 単一 field
     化、`test_pitch_c8` ignore 継続

6. 採用案を `01-overview.md` の D36 に反映してコミット（仕様書改訂版）

## 重要な制約（仕様書から抜粋、絶対遵守）

- **`process` 中ヒープ確保ゼロ**: `Engine::prepare` で全バッファ事前確保、`process_sample` /
  `note_on` 経路で `Vec::push` / `resize` を呼ばない
- **C ABI のみ、wasm-bindgen 不使用**: `#[unsafe(no_mangle)] pub extern "C" fn`、
  `scripts/check-wasm-exports.mjs` の REQUIRED 配列で検証
- **依存ゼロ**: dsp-core / wasm-audio に外部 crate を追加しない（`microfft` / `heapless` 等も不可）
- **ring buffer 不変条件**: write_index / read 位置は `% buf_len` で剰余、`% length_int` には
  しない（Pitch Bend で length_int が動的になるため、03 章 §統合フローを厳守）
- **Phase 2 既存 API 完全互換**: `KarplusStrong::note_on(freq, vel)` /
  `KarplusStrong::note_on_with_id(midi, freq, vel)` / `VoicePool::note_on(midi, freq, vel) -> usize`
  は引数順含めて Phase 2 と同じ
- **共通ヘルパ**: `KarplusStrong::note_on_internal(note_id: Option<u8>, freq, vel)` に集約、
  `Some(0)` と `None` を区別すること

## 開発コマンド

```powershell
pnpm dev            # dev WASM ビルド + Vite dev server
cargo test -p dsp-core   # dsp-core ユニットテスト全件
cargo test -p dsp-core test_pitch_  # ピッチテストだけ
cargo test --release -p dsp-core test_pitch_  # release ビルドで実行（Step 13 で必須）
pnpm check          # cargo check + svelte-check + params-sync
pnpm lint           # cargo clippy（warnings = errors）
pnpm fmt            # 整形
```

## ブランチ運用

- 現在 `phase3-spec` ブランチ（仕様書のみ）。実装は **`phase3-impl` ブランチを切って進める**
- `main` は branch protected、PR 必須、CI 緑必須
- Step 1 完了時に PR を立てるか、Step 14 完了まで一括 PR にするかは状況判断
  （Phase 2 では一括 PR `#1`、Phase 1 では Step 単位コミット → 一括 PR）

## まず最初に行うこと

1. `phase3-impl` ブランチを切る: `git checkout -b phase3-impl`
2. `cargo test -p dsp-core` を一度実行して、Phase 2 既存 41 件すべてパスを確認（regression baseline）
3. 仕様書 4 件（07 / 01 / 03 / 06）を必読
4. Step 1 の実装に入る

途中でレビュー観点や仕様書との不整合に気づいたら、その時点で報告してください。
仕様書改訂が必要なら 01〜07 を編集してから実装を進めます。
```

---

## 補足メモ

- Phase 2 は `crates/dsp-core/src/karplus_strong.rs` の `note_on_with_id` / `process_sample`
  を読んで、引数順や ring buffer 不変条件を実装前に再確認すること。
- Step 1 の試作で **D36 が確定するまで Step 2 以降は着手しない**。Step 1 結果次第で
  KarplusStrong の `fractional_delay` field 型が変わる（enum 解消 → 単一型）ため。
- レビュー指摘は本仕様書群の側を改訂で対応してきた経緯あり（10+ ラウンド）。実装中に気づいた
  仕様の曖昧さや矛盾は遠慮なく仕様書側を修正する方針で OK。
