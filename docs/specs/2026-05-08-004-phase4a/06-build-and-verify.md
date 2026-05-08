# 06. ビルドと検証（Phase 4a）

## 目的

Phase 4a の検証項目（F38b の Phase 3 持ち越し計測 + F39 以降の新規）、リスク（R31 以降）、性能目標を定義する。Phase 1〜3 で確定した F1〜F38b および R1〜R30 はすべて維持し、本書では **Phase 4a で追加・更新する箇所のみ** 記述する。

## 他文書との関係

- 上流: [`pre-research.md`](./pre-research.md)（§2 F38b 手順 / §9 性能予算）、[`01-overview.md`](./01-overview.md)（D44 / D45）、[`03-dsp-core-spec.md`](./03-dsp-core-spec.md)（テスト方針）、[`04-wasm-audio-spec.md`](./04-wasm-audio-spec.md)（export 検証 / wasm-opt）、[`05-web-frontend-spec.md`](./05-web-frontend-spec.md)（実機確認項目）
- 下流: [`07-implementation-checklist.md`](./07-implementation-checklist.md)（Step ごとの検証達成ライン）
- 並行: Phase 3 [`06-build-and-verify.md`](../2026-05-07-003-phase3/06-build-and-verify.md) — F1〜F38b および R1〜R30 の継承

## 性能目標（Phase 4a）

| 指標 | Phase 3 後実測 | Phase 4a target | 警戒ライン | 撤退ライン |
|---|---|---|---|---|
| WASM gzip サイズ | 27.78 KB | **目標 < 15 KB**（想定 ~13 KB、wasm-opt -O3 込み） | **警戒 < 18 KB**（要調査） | **撤退 < 30 KB**（R32: Modal 係数削減 / 楽器を 4 種に） |
| Worklet バンドルサイズ | ~3 KB | < 10 KB（LFO + applyInstrument 経路追加で +0.5 KB 想定） | > 12 KB で esbuild 設定見直し |
| `process` per call (8 voice + Body + LFO + Pitch Bend + CC#7、release cargo timing) | 0.012 ms | < 1.7 ms | > 2.0 ms で R31（LFO 簡素化） |
| Worklet `process` self time avg (実機計測 F38b) | 未計測 | < 1.5 ms（Phase 3 持ち越し） | > 2.0 ms で R30 (stride 4096 化等) |
| Worklet `process` self time max (実機計測 F38b) | 未計測 | < 2.5 ms | > 3.0 ms で R30 |
| ヒープ確保 in `process` | 0 | 0（Phase 1 D4 維持） | > 0 で R29（debug build で alloc 検査） |

## 検証項目（F-tag）

Phase 1〜3 の F1〜F38b に加え、Phase 4a で **F38b（Phase 3 持ち越し）+ F39〜F47 の 9 件**を追加。

### F38b — Worklet process 時間の実機計測（Phase 3 持ち越し、Phase 4a §0 として実施）

**実施タイミング**: Phase 4a Step 1（実装着手の最初）

**目的**: Phase 3 の仕様書想定 1.5 ms / 2.5 ms に対し、Chrome DevTools Performance タブで Worklet `process` の self time が実機で達成できているか確認。

**手順**:
1. `pnpm build && pnpm preview` で本番ビルド + 4173 ポート起動
2. Chrome 最新版で `http://localhost:4173/physics-base-synth/` を開く
3. F12 → Performance タブ → ⚙ 歯車 → CPU: "No throttling"
4. ⏺ Record 開始
5. ブラウザ上で **8 voice 同時押下**（PC キーボード a-k で 8 鍵）+ Pitch Bend wheel 操作 + CC#7 操作 + Sustain Pedal 操作の **最悪ケース** を 10 秒間維持
6. ⏹ Record 停止
7. タイムライン下部の "Audio Worklet" レーンを展開、各 task の self time を確認
8. **平均** と **最大** を集計（recordable な task が ~2300 個 = 10 秒 × 230 quanta/sec）

**達成基準**:
- avg < 1.5 ms かつ max < 2.5 ms → ✅ 成功、`docs/retrospective/2026-05-07-003-phase3.md` §5 に結果追記
- avg ≥ 2.0 ms または max ≥ 3.0 ms → R30 対策を Phase 4a 内で適用（`VOICE_STATE_STRIDE_FRAMES` を 1024 → 4096 等）

**計測再現性のための注意点**:
- 他タブを閉じる（特に YouTube / Slack 等の常駐 audio）
- DevTools Console の WebMIDI ログ抑制
- Chrome の "Energy Saver" 設定を無効化
- コンセント給電で計測（バッテリ駆動だと CPU governor が下がる）

### F39 — `wasm-opt -O3` 適用後の WASM gzip サイズ削減（D45）

**確認方法**:
```bash
# Git Bash で
gzip -kc web/build/_app/immutable/assets/wasm_audio.*.wasm | wc -c
```

**達成基準（3 段階）**:
- **目標**: gzip < 15 KB（想定 ~13 KB、Phase 3 27.78 KB から大幅削減）
- **警戒**: gzip < 18 KB（要調査、`wasm-opt --print-stats` で各 pass の効果を分析、`--symbols-only` で名前圧縮確認）
- **撤退**: gzip < 30 KB（Phase 1-3 継承 target、超過で R32 = 楽器を 4 種に削減 / Modal Body M=8 → M=5）

**サブ確認**:
- `pnpm build:wasm:dev` で wasm-opt がスキップされる（ビルド時間 < 5s 維持）
- `pnpm build:wasm` で wasm-opt 適用される（ビルド時間 < 30s）
- wasm-opt 不在時に warning が出て素コピーで続行する（CI 環境差分の吸収）

### F40 — LFO 動作（D46 / D47 / D48）

**cargo test レベル** (`crates/dsp-core/tests/lfo_tests.rs` + `lfo_destinations_tests.rs`):

| サブ項目 | 検証内容 |
|---|---|
| F40-a | `Lfo::process_sample` が sine / triangle で [-1, 1] 範囲に収まる |
| F40-b | rate=5Hz で 9600 sample 後に 1 周期 |
| F40-c | LFO Pitch destination が voice の `cached_length` を周期変動させる |
| F40-d | LFO Brightness destination が voice の filter 出力を変調 |
| F40-e | LFO Volume destination が `output_l/r` の RMS を周期変動させる |
| F40-f | Mod Wheel = 0 で LFO 効果ゼロ（Phase 3 互換挙動） |
| F40-g | LFO + 8 voice + Pitch Bend で `process` のヒープ確保ゼロ |

**実機確認** (`pnpm dev`):
- vibrato（Pitch Depth=1.0 + Mod Wheel=1.0、rate 5Hz）で音程の周期変動が聞こえる
- tremolo（Volume Depth=1.0 + Mod Wheel=1.0、rate 5Hz）で音量の周期変動が聞こえる
- wah（Brightness Depth=1.0 + Mod Wheel=1.0、rate 1Hz）で音色の周期変動が聞こえる

### F41 — Mod Wheel CC#1 動作（D49）

**cargo test レベル** (`midi_cc_tests.rs` 拡張):

| サブ項目 | 検証内容 |
|---|---|
| F41-a | `synth_midi_cc(handle, 1, 0.5)` で `mod_wheel.target() == 0.5` |
| F41-b | CC#1 value 1.5 / -0.5 で 0..1 に clamp |
| F41-c | CC#1 = 0 で全 LFO destination が出力に影響しない |

**実機確認**:
- WebMIDI 物理 wheel を動かすと UI スライダーが追従する（同経路）
- UI スライダーを動かすと音に LFO 効果が変調される

### F42 — プリセット保存・ロード（D50 / D51）

**TypeScript レベル** (`pnpm --filter ./web check`):

| サブ項目 | 検証内容 |
|---|---|
| F42-a | `isValidPresetV1(getDefaultPreset())` が true |
| F42-b | `isValidPresetV1({})` が false |
| F42-c | `isValidPresetV1({version: 2, ...})` が false（未知 version） |
| F42-d | `isValidPresetV1` が NaN / Infinity / 値域外（damping=2.0、rate=999.0 等）を reject |
| F42-e | `presetStore.save({ name: 'Default', ... })` が Factory 名衝突で errorMessage を設定し保存しない |
| F42-f | `isValidPresetV1` が name.length > 64 を reject |

**実機確認**:
- プリセット名 "Test 1" で Save → リロード → User Preset として読み込める
- 32 件保存後に 33 件目を試行 → `errorMessage` に "Preset slot full" 表示
- localStorage の容量超過 (DevTools で localStorage を満杯にして試行) → `errorMessage` 表示
- `physbase.preset.v1.list` を手動破壊（`localStorage.setItem('physbase.preset.v1.list', '{invalid')`）→ User Preset は空扱い (`userPresets = []`)、Factory プリセット 7 種は影響なし、`STORAGE_KEY_LAST` が valid なら維持、stale なら `'Default'` fallback
- `physbase.preset.v1.list` を削除 + `physbase.preset.v1.last = 'Sitar'` のみ残した状態でリロード → Factory の Sitar が `currentPresetName` に復元される（LIST 不在でも LAST が読まれる経路の確認）
- `physbase.preset.v1.last = 'Deleted Preset Name'` のような stale 値でリロード → `currentPresetName === 'Default'` に fallback、`<select>` の value と option が一致して空白選択にならない
- Factory プリセットは Delete ボタン disabled

### F43 — 楽器切替（D52 / D53 / D54）

**cargo test レベル** (`instrument_tests.rs`):

| サブ項目 | 検証内容 |
|---|---|
| F43-a | `apply_instrument(Ukulele)` で modal coeffs が GuitarClassical と異なる |
| F43-b | `apply_instrument` で全 voice release（active_count == 0） |
| F43-c | `apply_instrument` で sustain pending bitmap = 0 |
| F43-d | Default kind の係数が Phase 3 既存値と完全一致（後方互換） |
| F43-e | 楽器ごとの `stereo_spread` が異なる値を返す |

**実機確認**:
- プリセット選択で楽器が切り替わり、音色が明確に変化する
- 演奏中の音は即時 release（楽器切替で音切れ）
- Default プリセットで Phase 3 と同じ音が出る

### F44 — 既存負債解消（D45）

**cargo test レベル**:

| サブ項目 | 検証内容 |
|---|---|
| F44-a | `excitation_snapshot` が `#[cfg(test)]` でガードされ、production builder で除外される |
| F44-c | 該当 integration test (Phase 3 既存 7 箇所) が `karplus_strong.rs` 内の `#[cfg(test)] mod` に移動済み、`cargo test -p dsp-core` の合計テスト件数が Phase 3 と同じ |
| F44-b | Phase 3 既存 94 PASS + Phase 4a 新規テストすべて通る |

**確認方法**:
- `cargo build --target wasm32-unknown-unknown --release` で `excitation_snapshot` symbol が含まれない（`wasm-objdump -x` 等で確認、または gzip サイズ削減で間接確認）

### F45 — メモリ確保ゼロ（Phase 4a 拡張、D4 / D44 維持）

**cargo test レベル** (`tests/no_alloc_tests.rs` 拡張):

| サブ項目 | 検証内容 |
|---|---|
| F45-a | `test_no_allocation_with_lfo_and_instrument`: 8 voice + LFO active + Mod Wheel + 楽器切替 1 回で voice buffer / LFO 状態 / modal_body coeffs の capacity 不変 |
| F45-b | LFO process_sample 1000 回呼出で alloc 0 |

**実装パターン**（Phase 3 既存と同じ）:
```rust
let cap_before = engine.voice_state_capacity();  // 内部 vec の capacity 合計
// ...各種操作（LFO + 楽器切替 + Pitch Bend + CC#7 連打）...
let cap_after = engine.voice_state_capacity();
assert_eq!(cap_before, cap_after);
```

### F46 — リアルタイム性能 release cargo timing（F37 拡張）

**cargo test レベル** (`tests/dsp_core_tests.rs` 拡張):

```rust
#[cfg(not(debug_assertions))]
#[test]
fn test_engine_process_block_timing_phase4a() {
    // 8 voice + Body + LFO + Pitch Bend + CC#7 + Mod Wheel = 1.0 の最悪ケース
    // 1000 回 process の平均が < 1.7 ms
    // ...
    assert!(per_block_ms < 1.7, "Phase 4a target: avg < 1.7 ms, got {}", per_block_ms);
}
```

**達成基準**: avg < 1.7 ms（Phase 3 1.5 ms + 0.2 ms 余裕）

### F47 — Phase 3 全機能の互換性（regression baseline）

**cargo test レベル**:

| サブ項目 | 検証内容 |
|---|---|
| F47-a | Phase 3 既存 94 PASS + 1 IGNORED（C8 ピッチテスト）すべて維持 |
| F47-b | Phase 3 の `test_pitch_a4`（A4 で誤差 < 0.5%）が Phase 4a でも通る（LFO depth=0 の通常経路） |
| F47-c | Phase 3 の `test_engine_voice_state_buffer_format` が Phase 4a でも通る |
| F47-d | Phase 3 の `test_engine_midi_cc_sustain_defers` が Phase 4a でも通る |

**実機確認**:
- Default プリセットで Mod Wheel = 0 にすると Phase 3 と同じ音が出る
- Pitch Bend / Sustain Pedal / Channel Volume / All Notes Off の挙動が Phase 3 と同じ
- mono / poly トグルが Phase 3 と同じく動作

## リスク（R-tag）

Phase 1〜3 の R1〜R30 を維持。Phase 4a で R31〜R36 を追加。

### R31 — LFO 切替時のクリック

**シナリオ**: LFO Volume Depth を 0.0 → 1.0 に瞬時変更したとき、または LFO Rate を 0.1 → 8.0 Hz に瞬時変更したときに音量段差が出る。

**対策**:
- LFO depth は SmoothedValue tau=0.05s で平滑化（既に 03 章 §Engine 内で実装）
- LFO rate も SmoothedValue tau=0.05s で平滑化（03 章 §Lfo 内で実装）
- 波形切替 (sine ↔ triangle) は SmoothedValue 不可（discrete）、ただし phase は維持されるため切替直後の値変動は < 1.0（Sine と Triangle の最大差）

**検証**: F40-a / F40-b の cargo test、実機での聴感確認（クリックなし）

### R32 — WASM gzip サイズ超過

**シナリオ**: `wasm-opt -O3` 適用後でも 30 KB target を超えてしまう（楽器係数 144 値追加で +1 KB、LFO で +0.5 KB の想定が崩れる）。

**対策**（pre-research §9.2 早期検証ライン）:
1. **第 1 段階** (gzip > 18 KB after wasm-opt): wasm-opt の `--print-stats` で重い pass を特定
2. **第 2 段階** (gzip > 22 KB): 楽器を 6 種 → 4 種に削減（Sitar / Mandolin を Phase 4b 送り）
3. **第 3 段階** (gzip > 25 KB): Modal Body M=8 → M=5 に削減（D30 R29 撤退ライン）

**検証**: F39 のサイズ計測で early detection

### R33 — Preset 互換性破壊

**シナリオ**: Phase 4a で v1 schema を変更（field 追加 / 型変更）した後、ユーザーが古い localStorage データを持っていてロード失敗。

**対策**:
- v1 は **本仕様書で凍結**（`PresetV1` interface 確定後は変更不可）
- 将来の field 追加は v2 として `PresetV2` を新規定義し、`migrateV1ToV2(p: PresetV1): PresetV2` を書く
- Phase 4a 中に v1 を変更したくなったら、仕様書改訂 + 全 migration の再考が必要（「仕様書から逸脱しない」原則）
- `isValidPresetV1` でバリデーション失敗時は console.warn + skip（throw しない、UX 配慮）

**検証**: F42 の TypeScript test、実機での localStorage 破壊テスト

### R34 — 楽器切替時の polyphony 影響

**シナリオ**: 演奏中に楽器を切り替えると全 voice が即時 release され、音切れがユーザー体験を損なう。

**対策**（Phase 4a 採用方針）:
- D53 で「即時 release」を確定、UI に注意書きを表示（`PresetSelector.svelte` に hint 文言追加検討）
- fade-out（短時間 release）は Phase 4b 以降の UX 改善で再評価

**検証**: 実機確認、ユーザーレビューで体験評価

### R35 — localStorage 容量超過 / 破壊

**シナリオ**:
- 32 件まで User Preset を保存しても 1 件 ~350 byte で 11.2 KB、5 MB 上限の 0.22% で容量超過は考えにくいが、他のアプリと共有の origin で枯渇する可能性
- DevTools で localStorage を直接編集した結果、不正な JSON が混入

**対策**:
- `try/catch` で QuotaExceededError を捕捉、UI に "Failed to save preset (storage quota exceeded?)" 表示
- ロード時に `isValidPresetV1` で 1 件ずつバリデーション、失敗は skip（他のプリセットは生存）

**検証**: F42-c / F42 実機テスト

### R36 — LFO destination の Brightness offset でクリップ

**シナリオ**: LFO Brightness depth=1.0 + Mod Wheel=1.0 で `brightness + lfo_offset` が [0, 1] 範囲を逸脱、内部 LPF が不安定化。

**対策**（03 章 §karplus_strong.rs::process_sample 内で実装）:
```rust
let brightness_v = (self.brightness.next_sample() + self.lfo_brightness_offset).clamp(0.0, 1.0);
```
clamp で Phase 3 と同じ [0, 1] 範囲を保証。

**検証**: cargo test で LFO Brightness Depth=1.0 を 60 秒走らせて NaN / 異常値が出ないこと

## トラブルシューティング tips

### `pnpm build` が wasm-opt エラーで失敗

- `node_modules/.bin/wasm-opt` の存在確認: `ls node_modules/.bin/wasm-opt*`
- `pnpm install` を再実行（`binaryen` の再インストール）
- それでも失敗するなら `NODE_ENV=development pnpm build` で wasm-opt をスキップして暫定回避（target サイズに影響）

### `cargo test -p dsp-core test_no_allocation_with_lfo_and_instrument` が fail

- `Engine::apply_instrument` 内で `Vec::push` / `Vec::resize` を呼んでいないか確認
- `lfo.rs` の `Lfo::process_sample` で `vec` 系操作がないか確認
- 既存の `test_no_allocation_with_modal_body_and_midi_cc` が通れば回帰、Phase 4a の差分のみ確認すれば良い

### LFO 効果が音に出ない

- Mod Wheel = 0 の状態で LFO depth を上げても **意図的に効果ゼロ**（D49 の master 制御）。Mod Wheel を 0.5 以上に上げて再確認
- depth=1.0 でも `LFO_*_SCALE` 定数 0.5 で実効深さは ±0.5 に制限されている（仕様通り）

### プリセット保存後にリロードで読み込めない

- DevTools Application タブ → Local Storage → `physbase.preset.v1.list` の内容確認
- `physbase.preset.v1.<name>` の内容が valid JSON か確認
- `isValidPresetV1` の console.warn メッセージを確認

### 楽器切替後に音が出ない

- `apply_instrument` が `pool.all_notes_off()` を呼ぶ仕様通り、切替直後の最初の note_on で音が復活する
- DevTools Console で `synth.engine.applyInstrument('default')` を実行して Default に戻れるか確認

## ビルドコマンド一覧

| コマンド | 用途 |
|---|---|
| `pnpm dev` | dev WASM ビルド + Vite dev server (5173) |
| `pnpm build:wasm` | release WASM ビルド + wasm-opt -O3 適用 |
| `pnpm build:wasm:dev` | dev WASM ビルド (wasm-opt スキップ) |
| `pnpm build` | release WASM + SvelteKit static build → `web/build/` |
| `pnpm preview` | 本番ビルドをプレビュー (4173)、F38b 計測用 |
| `pnpm check` | `cargo check --workspace` + `svelte-check` + params-sync |
| `pnpm lint` | `cargo clippy --workspace --all-targets -- -D warnings` |
| `pnpm fmt` | `cargo fmt --all` + prettier |
| `pnpm gen:params` | `params.json` から Rust / TS の params.rs / params.ts を生成 |
| `cargo test -p dsp-core` | dsp-core ユニットテスト全件 |
| `cargo test -p dsp-core test_lfo_` | LFO テストのみ |
| `cargo test -p dsp-core test_apply_instrument_` | 楽器切替テストのみ |
| `cargo test --release -p dsp-core test_engine_process_block_timing_phase4a` | release timing test |
| `pnpm --filter ./web check` | svelte-check 単独 |
| `pnpm --filter ./web lint` | prettier + eslint |

## サイズ計測手順

### WASM gzip サイズ

```bash
# Git Bash で
pnpm build
gzip -kc web/build/_app/immutable/assets/wasm_audio.*.wasm | wc -c
```

期待値: ~13000 bytes (target 30 KB の 43%)

### Worklet バンドルサイズ

```powershell
# PowerShell で
Get-ChildItem web\build\_app\immutable\assets\synth-processor*.js | Select-Object Name, Length
```

期待値: < 4 KB

### `__synthDev` 検証（Phase 3 F22 拡張）

```bash
# Git Bash で
grep -r "__synthDev" web/build/_app/immutable/ | wc -l
# 期待値: 0
```

## 達成ライン早見表（Step 別、07 章への種）

| ステップ完了 | 達成する F-tag |
|---|---|
| Step 1 (F38b 計測) | F38b |
| Step 2 (wasm-opt -O3) | F39 |
| Step 3 (excitation_snapshot cfg(test)) | F44 |
| Step 4 (params.json 拡張) | F39（部分）、F43 の準備 |
| Step 5 (lfo.rs + Engine 統合) | F40-a / F40-b / F40-g（部分） |
| Step 6 (Mod Wheel CC#1) | F41 |
| Step 7 (LFO destinations 統合) | F40-c / F40-d / F40-e / F40-f / F40-g |
| Step 8 (多楽器係数 6 種) | F43-a / F43-d / F43-e |
| Step 9 (Engine::apply_instrument) | F43-b / F43-c |
| Step 10 (C ABI 4 関数) | F39 |
| Step 11 (messages.ts + WasmExports + SynthEngine) | F40〜F43（経路） |
| Step 12 (preset-store) | F42 |
| Step 13 (PresetSelector + ModWheel + LfoSection UI) | F40 / F41 / F42 / F43 実機 |
| Step 14 (統合 cargo test + alloc ゼロ + release timing) | F45 / F46 / F47 |
| Step 15 (実機確認 + F38b 再計測) | F38b 再確認、Phase 4a 全機能の実機確認 |
| Step 16 (ドキュメント整備) | Phase 4a 完成 |
| Step 17 (PR 作成 + main マージ) | Phase 4a リリース |

すべての F38b + F39〜F47 が達成された時点で Phase 4a 完成。F46（release timing）と F45（alloc ゼロ）は **Step 14 で必須化**、F38b は **Step 1 + Step 15 の 2 回計測**（Phase 3 検証 + Phase 4a 後の regression 確認）。
