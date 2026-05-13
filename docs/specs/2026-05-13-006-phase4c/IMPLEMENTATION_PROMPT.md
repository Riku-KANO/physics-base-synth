# Phase 4c 実装セッション用プロンプト

新しい Claude Code セッションで Phase 4c の実装を開始する際、以下のプロンプトを冒頭に貼り付けてください。

---

## プロンプト本体（コピー用）

```
Phase 4c の実装を開始します。仕様書は `docs/specs/2026-05-13-006-phase4c/` に揃っており、
合計 9 ファイル（pre-research + 01〜07 + IMPLEMENTATION_PROMPT）。実装着手前に必ず以下を読んでください。

## 必読

1. `docs/specs/2026-05-13-006-phase4c/07-implementation-checklist.md` — 22 ステップの実装順
2. `docs/specs/2026-05-13-006-phase4c/01-overview.md` — D68〜D85 の設計判断 18 項目
3. `docs/specs/2026-05-13-006-phase4c/03-dsp-core-spec.md` — DSP コアの全 API 仕様（最重要、Multi-string + Hertz hammer + Sympathetic + B(note) LUT）
4. `docs/specs/2026-05-13-006-phase4c/06-build-and-verify.md` — F59〜F70 検証項目とリスク R40〜R44
5. `CLAUDE.md` — プロジェクト全体のルール / Phase 1-4b の制約

## 進め方

仕様書ドリブン開発（CLAUDE.md §「仕様書ドリブン開発」）に従い、**仕様書を主導**として実装を進める。
仕様書通りで冗長と感じる箇所も基本的にそのまま実装し、逸脱しない。

実装は **Step 1 から順に**、1 ステップ ≈ 1 コミット粒度（Phase 1 / 2 / 3 / 4a / 4b と同じ）。各 Step 完了時に
`cargo test -p dsp-core` で **Phase 4b 既存 148 PASS + 1 IGNORED がすべて通ること**を確認しながら進める。

既存実装（Phase 4b までの `crates/dsp-core/src/`、`crates/wasm-audio/src/lib.rs`、
`web/src/lib/`）を破壊しないこと。

## Phase 4c の主目的

**本格ピアノ音色 (Multi-string per voice 1/2/3 + Unison detuning ±1.5 cents + Hertz law raised cosine hammer + Global sympathetic resonance bus + 88 鍵 B(note) LUT + Piano プリセット聴感チューニング)** で Phase 4b の最大負債「Piano 音色が弦楽器寄り」を構造的に解消する。

## 重要な制約

- **新規 ParamId / C ABI 関数追加なし**（D81、required exports 19 を Phase 4a / 4b から維持）
- **Phase 4a HEAD byte 一致継承**: `n_strings = 1` 経路で `tests/fixtures/phase4a_default_c4_v08.rs` と ε=1e-6 一致（D83、Phase 4b の互換性保証を継承）
- **Phase 4b 7 楽器 byte 一致**: Default / Guitar / Ukulele / Mandolin / Bass / GuitarSteel / Sitar で Phase 4b 出力と完全一致
- **process ホットパスで heap alloc ゼロ**: Multi-string の `string_buffers` × 3 は `Engine::prepare` で一括確保、`StringState` は inline 配列（Phase 1 D4 継承）
- **依存ゼロ**: dsp-core / wasm-audio で外部 crate 追加禁止（Hertz hammer も自前、Sympathetic bus も自前）
- **Step 19 のユーザー実機聴感確認を完了条件に含める**（D82、cargo / clippy / 互換性テスト全 green でも聴感達成が必須）

## Phase 4d 候補（Phase 4c では着手しない、D84）

C8 ピッチ自己発振 / WASM SIMD / Pick position fractional 化 / Look-ahead limiter / LFO 波形拡張 (S&H / Square / Sawtooth) / LFO destinations 拡張 (Pick / Damping / BodyWet) / 楽器切替の fade-out (`PendingInstrumentChange` 状態機械) / Cross-tab preset 同期 / Preset JSON file import / export / F38b CI / E2E 自動化 / 複数 Piano 機種プリセット / Hammer Hardness UI 露出 / Una corda / Sympathetic を Sitar / Guitar へ適用

## チェックポイント

各 Step 完了時に以下を確認:

- [ ] `cargo test -p dsp-core` 全 pass (Phase 4b 既存 + Phase 4c 新規)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` warning ゼロ
- [ ] `pnpm fmt` / `cargo fmt --all` で format 整理
- [ ] commit message が conventional（feat / fix / chore / test / docs / refactor + 範囲）

Phase α (Step 1) は **F38b 実機ベースライン取得**で、ユーザー操作必須（`pnpm dev` + Console から
`window.__synthDev.measureProcessTime(5000)` 呼び出し）。Auto mode では完結不可、結果取得後に Step 2 へ。

Phase ι (Step 14) は **統合検証 + 聴感判断**で、ここで Step 15 (Modal M=16 / Bridge coupling) の採否を決める。
Step 14 で「Phase 4b より本物のピアノに近づいた」と確認できれば Step 16 へ、不足があれば Step 15 を追加。

Phase κ (Step 17-19) は **Piano プリセット聴感チューニング反復**で、ユーザー実機聴感達成までは Phase 4c 完了とみなさない (D82)。
聴感達成までに R44 (Piano 聴感未達) が発生したら緩和策を順番に試す。

それでは Step 1 から開始してください。
```

---

## セッション開始時の確認事項（ユーザー向け）

実装セッションを開始する前に、以下を確認しておいてください:

1. **Phase 4b の retrospective が完成している**
   - `docs/retrospective/2026-05-09-005-phase4b.md` が存在する
   - Phase 4b の PR #5 が main にマージ済 (commit 6201814)
   - main ブランチが branch protected
2. **Phase 4c の主目的が「本格ピアノ音色」に確定している**（pre-research §12 #1）
3. **`pnpm dev` で Piano プリセットが動作する状態**（Phase 4b 完成、Piano kind が `kind=7` として動作）
4. **F38b 計測 API (`__synthDev.measureProcessTime`) が動作する状態**（Phase 4b 完成）
5. **iPhone Safari 等の実機での Phase 4b 動作が確認できる**（HTTPS 環境、Cloudflare Tunnel 等）
6. **`docs/specs/2026-05-13-006-phase4c/` 配下の 9 ファイルがすべて存在**:
   - `pre-research.md`
   - `01-overview.md`
   - `02-architecture.md`
   - `03-dsp-core-spec.md`
   - `04-wasm-audio-spec.md`
   - `05-web-frontend-spec.md`
   - `06-build-and-verify.md`
   - `07-implementation-checklist.md`
   - `IMPLEMENTATION_PROMPT.md` (本ファイル)

---

## Phase 4c の特徴と注意点

### Phase 4b との違い

| 観点 | Phase 4b | Phase 4c |
|---|---|---|
| 主目的 | Piano 音色を Stretching all-pass + Hammer model + Modal Body で実装 | Phase 4b の Piano が「弦楽器寄り」だった構造的限界を解消 |
| Step 数 | 18 | 22 |
| 設計判断 (D タグ) | D56〜D67 (12 項目) | D68〜D85 (18 項目) |
| 検証項目 (F タグ) | F48〜F58 (11 件) | F59〜F70 (12 件) |
| リスク (R タグ) | R37〜R39 (3 件) | R40〜R44 (5 件) |
| 新規ファイル | 6 (dispersion / dispersion_tests / karplus_strong_dispersion_tests / fixtures / __synthDev.ts / .gitattributes) | 5 (resonance_bus.rs / multi_string_tests / sympathetic_tests / hammer_hertz_tests / step19-listening-final.md) |
| 主要拡張ファイル | karplus_strong.rs / engine.rs / params.rs / synth-processor.ts | karplus_strong.rs / engine.rs / dispersion.rs / params.rs / params.json / gen-params.mjs |
| C ABI 追加関数 | 0 (D64) | 0 (D81) |
| 新規 ParamId | 0 | 0 (D81) |
| 互換性保証 | Phase 4a HEAD byte 一致 (`dispersion_active = false`) | Phase 4a HEAD byte 一致 (`n_strings = 1`) + Phase 4b 7 楽器互換 |
| 聴感達成 (D82) | プリセット聴感調整は実施せず Phase 4c 送り | **Step 17-19 で必須実施、ユーザー実機聴感確認が完了条件** |
| Step 規模 (時間) | 17-20 時間 | 22-27 時間 |

### Phase 4c の核心リスク

1. **R44: Piano 聴感が「Phase 4b より本物のピアノに近づいた」と確認できない**
   - 最大の Phase 4c 失敗シナリオ。Multi-string + Hertz hammer + Sympathetic + B(note) を全て実装しても、ユーザー実機聴感で「変化はあるが本物のピアノではない」「むしろ Phase 4b の方が良い」と評価される可能性
   - 緩和策（順番に試す）:
     1. Piano プリセット聴感調整 (`damping` / `brightness` / `bodyWet` / `unison_detune_cents` / `sympathetic_amount` / `hammer_cutoff_*` の反復) → Step 18-19
     2. Modal Body M=16 拡張 → Step 15 で追加
     3. Bridge coupling (Multi-string 案 B) → Step 15 で追加
     4. B(note) LUT 値の見直し (Young 1952 / Conklin 1996 fitting の精密化)
     5. これでも未達なら Phase 4d で「Two-stage decay 明示実装」「複数 Piano 機種」「Una corda」等の追加検討

2. **R41: `process` per call が 0.25 ms 警戒ライン超過**
   - Multi-string 3 弦 + Sympathetic で CPU が想定 (0.082 ms) を超える可能性
   - 緩和策: Multi-string buffer 案 1 → 案 2 (共有 buffer + 3 read 位置) で memory cache 圧迫を回避 / bridge coupling 撤回 / Modal M=16 撤回

3. **R42: WASM memory heap が 512 KB 超過**
   - Multi-string buffer 案 1 で +112 KB、累積で 233 KB 想定。iPhone Safari の制約を超える可能性
   - 緩和策: Multi-string buffer 案 1 → 案 2 (共有 buffer)

4. **R43: Sympathetic bus の数値発散**
   - feedback gain が過大 or LPF cutoff が高すぎると bus が発振
   - 緩和策: `FEEDBACK_GAIN_MAX` を 0.05 → 0.03 に強化、`BUS_INTERNAL_DECAY` を 0.95 → 0.90 に強化、LPF cutoff を低下

5. **R40: WASM gzip サイズが 25 KB 警戒ライン超過**
   - B(note) LUT 88 値 + Multi-string コード + Sympathetic bus で +1.2 KB 想定だが、それを超える可能性
   - 緩和策: B(note) LUT を 88 値 → 22 値 (4 半音ごと) に削減 / Modal M=16 撤回

### Auto mode で完結しない箇所

Phase 4b と同様、以下は **ユーザー操作必須** で Auto mode 完結不可:

- **Step 1**: F38b ベースライン取得 (`__synthDev.measureProcessTime`)
- **Step 14 / Step 18-19**: 実機聴感確認（`pnpm dev` + ブラウザ + Piano プリセット試奏）
- **Step 20**: Phase 4c 完成後の F38b 計測
- **iPhone Safari 実機での動作確認**（Phase 4a F9 継承）

これらの Step は仕様書策定時から「先送り可能なステップ」として明示してから Auto 進行する判断が妥当（retrospective §6 教訓）。

### `/simplify` の活用

Phase 4b retrospective §6 で確立した「retrospective 前に `/simplify` を実行する習慣化」を Phase 4c でも踏襲。Step 21 (ドキュメント整備) の前に `/simplify` をかけて、コード品質改善 + 重複テスト削除 + rot コメント削除を行う。`/simplify` レビュー結果は Phase 4c retrospective §3 / §5 に記録。

### 仕様書改訂 + 実装を同 commit で

Phase 3 D36 / Phase 4a Triangle 式 typo / Phase 4b `test_dispersion_b_zero_limit` 改訂で 3 回連続発生した「軽微な記述ミス / 数値判定基準の修正は実装と同 commit」を Phase 4c でも踏襲。実装段階で B(note) LUT 値 / Multi-string detune 値 / Sympathetic feedback 値などの数値が実測との乖離を起こした場合、仕様書改訂 + 実装を同 commit で行う。

---

## 実装ブランチ命名

- **メインブランチ**: `phase4c-impl` （Phase 4b の `phase4b-impl` 命名を踏襲）
- **PoC ブランチ（オプション、§10.4 (pre-research)）**: `phase4c-simd-poc` （Phase 4c 仕様書本体とは独立、Modal Body の SIMD 化を半日〜1 日で計測）

---

## まとめ

Phase 4c の実装プロンプトは Phase 4b と同形式で、**仕様書ドリブン開発 + 22 ステップ順次実装 + Step 19 ユーザー実機聴感達成を完了条件**を中核とする。Phase 4b との最大の違いは **「聴感達成 (D82) を完了条件に含めた」点**で、cargo / clippy / 互換性テスト全 green でも聴感未達なら Phase 4c 未完了とみなす（retrospective §6 教訓の組込み）。R44 (Piano 聴感未達) に至った場合は緩和策 5 段階を順番に試す運用。新規 ParamId / C ABI 関数追加なしの制約 (D81) は Phase 4a / 4b から完全継承、Phase 4a HEAD byte 一致は `n_strings = 1` 経路で機械保証 (D83)。
