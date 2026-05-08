# Phase 4a 実装セッション用プロンプト

新しい Claude Code セッションで Phase 4a の実装を開始する際、以下のプロンプトを冒頭に貼り付けてください。

---

## プロンプト本体（コピー用）

```
Phase 4a の実装を開始します。仕様書は `docs/specs/2026-05-08-004-phase4a/` に揃っており、
合計 9 ファイル（pre-research + 01〜07 + IMPLEMENTATION_PROMPT）。実装着手前に必ず以下を読んでください。

## 必読

1. `docs/specs/2026-05-08-004-phase4a/07-implementation-checklist.md` — 17 ステップの実装順
2. `docs/specs/2026-05-08-004-phase4a/01-overview.md` — D44〜D55 の設計判断 12 項目
3. `docs/specs/2026-05-08-004-phase4a/03-dsp-core-spec.md` — DSP コアの全 API 仕様（最重要）
4. `docs/specs/2026-05-08-004-phase4a/06-build-and-verify.md` — F38b + F39〜F47 検証項目とリスク R31〜R36
5. `CLAUDE.md` — プロジェクト全体のルール / Phase 1-3 の制約

## 進め方

仕様書ドリブン開発（CLAUDE.md §「仕様書ドリブン開発」）に従い、**仕様書を主導**として実装を進める。
仕様書通りで冗長と感じる箇所も基本的にそのまま実装し、逸脱しない。

実装は **Step 1 から順に**、1 ステップ ≈ 1 コミット粒度（Phase 1 / 2 / 3 と同じ）。各 Step 完了時に
`cargo test -p dsp-core` で **Phase 3 既存 94 テスト + 1 IGNORED がすべて通ること**を確認しながら進める。

既存実装（Phase 3 までの `crates/dsp-core/src/`、`crates/wasm-audio/src/lib.rs`、
`web/src/lib/`）を破壊しないこと。

## Step 1 概要（最初の作業）

**F38b 実機計測**（D44 / Phase 3 完成判定の最終案件）:

1. `pnpm build && pnpm preview` で本番ビルド + 4173 ポート起動
2. Chrome 最新版で `http://localhost:4173/physics-base-synth/` を開く
3. F12 → Performance タブ → ⚙ 歯車 → CPU: "No throttling"
4. ⏺ Record 開始
5. ブラウザ上で **8 voice 同時押下** + Pitch Bend + CC#7 操作 + Sustain Pedal を 10 秒間維持
6. ⏹ Record 停止、"Audio Worklet" レーンの各 task の self time を確認
7. **平均** と **最大** を記録（target: avg < 1.5 ms / max < 2.5 ms）

**結果記録**:
- `docs/retrospective/2026-05-07-003-phase3.md` §5 の `F38b 実機計測` 項目に追記（計測日時 / avg / max / 達成判定）
- target 達成 → ✅ Step 2 へ
- target 超過（avg ≥ 2.0 ms or max ≥ 3.0 ms）→ R30 対策（VOICE_STATE_STRIDE_FRAMES 4096 化等）を Phase 4a 内で適用、scope が変わる可能性あり

git commit `chore(retrospective): F38b 実機計測結果を追記 (D44)`

## 重要な制約（仕様書から抜粋、絶対遵守）

- **`process` 中ヒープ確保ゼロ**: `Engine::prepare` で全バッファ事前確保、`process_sample` /
  `note_on` / `apply_instrument` 経路で `Vec::push` / `resize` を呼ばない（LFO 状態 / 楽器係数も既存
  領域内で更新）。Phase 3 D4 維持
- **C ABI のみ、wasm-bindgen 不使用**: `#[unsafe(no_mangle)] pub extern "C" fn`、
  `scripts/check-wasm-exports.mjs` の REQUIRED 配列で検証（Phase 4a で 4 関数追加）
- **依存ゼロ**: dsp-core / wasm-audio に外部 crate を追加しない（`microfft` / `heapless` 等も不可）。
  binaryen は npm devDependency（build-time tooling、Cargo 依存ではない）として許容
- **ring buffer 不変条件**: write_index / read 位置は `% buf_len` で剰余、`% length_int` には
  しない（LFO Pitch destination で length_int が動的になるため、03 章 §統合フロー参照）
- **Phase 3 既存 API 完全互換**: C ABI 既存 15 関数 / Voice trait の `set_pitch_bend` /
  `synth_midi_cc` 等のシグネチャは不変。CC#1 (Mod Wheel) 分岐の有効化は内部実装の変更のみ
- **Mod Wheel = 0 で Phase 3 互換**: LFO depth が 1.0 でも Mod Wheel = 0 (デフォルト) で
  LFO 効果ゼロ、Phase 3 と完全に同じ音が出ること。`test_mod_wheel_zero_disables_lfo` で保証
- **Default プリセット = Phase 3 既存ギターボディ係数**: 楽器 enum kind=0 の Modal 係数が
  Phase 3 の `BODY_MODES_L/R` と完全一致（後方互換）
- **生成物 git commit**: `params.rs` / `params.ts` は `pnpm gen:params` で生成 → commit
  （Phase 1-3 の D25 継承）

## 開発コマンド

```powershell
pnpm dev                                      # dev WASM ビルド + Vite dev server
pnpm build                                    # release WASM (wasm-opt -O3) + SvelteKit build
pnpm preview                                  # 本番プレビュー (4173)、F38b 計測用
pnpm build:wasm                               # release WASM のみ (wasm-opt 適用)
pnpm build:wasm:dev                           # dev WASM のみ (wasm-opt スキップ)
cargo test -p dsp-core                        # dsp-core ユニットテスト全件
cargo test -p dsp-core test_lfo_              # LFO テストのみ
cargo test -p dsp-core test_apply_instrument_ # 楽器切替テストのみ
cargo test --release -p dsp-core test_engine_process_block_timing_phase4a  # release timing
pnpm check                                    # cargo check + svelte-check + params-sync
pnpm lint                                     # cargo clippy（warnings = errors）
pnpm fmt                                      # 整形
pnpm gen:params                               # params.json から生成
```

## ブランチ運用

- 現在 `phase4a-spec` ブランチ（仕様書のみ、本セッションで main にマージ済み想定）
- 実装は **`phase4a-impl` ブランチを切って進める**: `git checkout -b phase4a-impl`
- `main` は branch protected、PR 必須、CI 緑必須
- Step 単位でコミット、Step 17 完了時に PR 作成（`gh pr create`）→ CI 緑確認 → main マージ
- Phase 3 では Step 単位コミット → 一括 PR の流儀、Phase 4a でも同じ

## まず最初に行うこと

1. `phase4a-impl` ブランチを切る: `git checkout -b phase4a-impl`
2. `cargo test -p dsp-core` を一度実行して、Phase 3 既存 94 件 + 1 IGNORED すべてパスを確認（regression baseline）
3. 仕様書 4 件（07 / 01 / 03 / 06）を必読
4. Step 1 (F38b 実機計測) に入る

途中でレビュー観点や仕様書との不整合に気づいたら、その時点で報告してください。
仕様書改訂が必要なら 01〜07 を編集してから実装を進めます（Phase 3 では D36 案 D
の追加で 1 度仕様書を改訂しています）。
```

---

## 補足メモ

### Phase 3 からの差分理解

- Phase 3 で `engine.rs:171` の CC#1 (Mod Wheel) は no-op だった経路が、Phase 4a Step 6 で
  `self.mod_wheel.set_target(v)` に変更される（D49）
- Phase 3 で `BODY_MODES_L` / `BODY_MODES_R` がグローバル const だったのが、Phase 4a Step 4 で
  楽器ごとに 12 配列化される（Default の alias として既存名は維持、後方互換）
- Phase 3 で `excitation_snapshot` が `#[doc(hidden)]` だったのが、Phase 4a Step 3 で
  `#[cfg(test)]` ガードに変更される（既存負債解消、D45）

### Step 1 (F38b 計測) で target 超過時の判断

- avg < 1.5 ms かつ max < 2.5 ms → ✅ Step 2 以降へ進む
- avg in [1.5, 2.0) ms または max in [2.5, 3.0) ms → 注意、Phase 4a 後に再計測する前提で進む
- avg ≥ 2.0 ms または max ≥ 3.0 ms → R30 対策を **Phase 4a §1 として組み込み**:
  - `VOICE_STATE_STRIDE_FRAMES = 1024` → `4096` に変更（push 頻度 1/4、UI 更新 ~85ms）
  - VoiceMeter UI を Phase 4b 送り（`maybePushVoiceState` を skip、Voice Meter 削除）
  - これらは仕様書策定時の想定外なので、ユーザー判断を仰ぐ

### Step 4 (params.json 拡張) で生成物の重要性

- `gen-params.mjs` は Phase 4a で大幅拡張される。**Default kind = Phase 3 既存値の完全一致**
  が後方互換性の生命線。テスト `test_default_instrument_matches_phase3_modes` で機械的に保証
- `applyStereoSpread(modes, spread)` の純粋関数が L → R 係数を生成する設計（Phase 3 で確立）
- 楽器ごとに `STEREO_SPREAD_<INSTRUMENT>` を定義、Phase 3 のグローバル `STEREO_SPREAD = 0.05`
  も維持（Default の alias）

### Step 7 (LFO destinations 統合) の核心テスト

```rust
// crates/dsp-core/tests/lfo_destinations_tests.rs
#[test]
fn test_mod_wheel_zero_disables_lfo() {
    let mut engine = Engine::new();
    engine.prepare(48000.0, 128);

    // LFO depth を全 destination で 1.0 に設定
    engine.lfo_set_depth(LfoDestination::Pitch, 1.0);
    engine.lfo_set_depth(LfoDestination::Brightness, 1.0);
    engine.lfo_set_depth(LfoDestination::Volume, 1.0);

    // Mod Wheel は 0 (デフォルト) のまま
    // → LFO 効果は出ない (Phase 3 互換挙動)

    engine.note_on(60, 0.8);
    let mut buf_l = vec![0.0; 256];
    let mut buf_r = vec![0.0; 256];
    engine.process(&mut buf_l, &mut buf_r);

    // 同じ条件で Mod Wheel = 0 と LFO depth = 0 の出力が同一になることを確認
    let mut engine_no_lfo = Engine::new();
    engine_no_lfo.prepare(48000.0, 128);
    engine_no_lfo.note_on(60, 0.8);
    let mut buf_l2 = vec![0.0; 256];
    let mut buf_r2 = vec![0.0; 256];
    engine_no_lfo.process(&mut buf_l2, &mut buf_r2);

    // 出力が等しい (Mod Wheel = 0 で LFO 効果ゼロ)
    for i in 0..256 {
        assert!((buf_l[i] - buf_l2[i]).abs() < 1e-6,
                "LFO depth=1.0 + Mod Wheel=0 should produce same output as no LFO at frame {}", i);
    }
}
```

### Step 12 (preset-store) で bug-prone な箇所

- `localStorage.setItem` の QuotaExceededError は **必ず try/catch**
- `isValidPresetV1` のバリデーション失敗は **console.warn + skip**（throw しない、UX 配慮）
- `physbase.preset.v1.list` の name 配列は save / delete で **必ず更新** （個別キーと name 配列の同期）

### Step 14 (release timing test) の閾値判断

- target 1.7 ms は Phase 3 の 1.5 ms + 0.2 ms 余裕（LFO + Mod Wheel + 楽器切替の追加コスト分）
- CI で flaky なら 2.0 ms に緩めるが、ローカルで 1.7 ms 達成を目標
- F38b の実機計測（Step 15）と F46 の cargo timing test の両方が target 達成すれば Phase 4a 完成

### 仕様書改訂が必要になったときの判断

- Phase 3 では D36 で仕様書原案の前提（C8 自己発振）が崩れ、案 D を追加して Step 1 完了時に
  仕様書改訂をコミットした経緯あり
- Phase 4a で同様の発見があれば、**仕様書側を改訂してから実装を進める**（CLAUDE.md §「仕様書ドリブン開発」）
- 軽微な記述ミス（typo / 章番号ずれ）は実装と同じ commit で修正可能、設計判断の変更は別 commit で
- ユーザーに報告して承認を得てから改訂

### Phase 4b 予告

Phase 4a 実装完了 + retrospective 後に別計画で着手:
- ピアノ音色 (Stretching all-pass for inharmonicity B≈10⁻³ + impact model)
- 仕様書ディレクトリ: `docs/specs/<YYYY-MM-DD>-005-phase4b/`
- CPU コスト PoC を pre-research §1 に組み込む想定
- Phase 4a の実装完了が前提条件、本仕様書には含めない

### コミット粒度の参考

Phase 3 (14 Step) では実際は Step 単位 + 仕様書改訂 + 中間 fix で 16 commits、PR 1 本でマージ。
Phase 4a (17 Step) も同程度の commit 数を想定。

| Step | 概算 commit 数 |
|---|---|
| 1〜3 (F38b + 既存負債) | 3 |
| 4 (params + gen) | 1（生成物 commit 込み） |
| 5〜7 (LFO + Mod Wheel + 統合) | 3 |
| 8〜9 (楽器切替) | 2 |
| 10〜11 (C ABI + Worklet) | 2 |
| 12〜13 (Preset + UI) | 2 |
| 14 (統合テスト) | 1 |
| 15 (実機確認) | 1（チェックリスト埋めるのみ、コード変更なし可） |
| 16〜17 (ドキュメント + PR) | 2 |
| **合計** | **17 commits** |

各 Step は仕様書 07 章で詳細手順 + テスト追加内容が明示されているため、実装中は仕様書を逐次参照すれば判断は最小限で済む。
