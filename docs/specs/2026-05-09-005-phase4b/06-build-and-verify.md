# 06. ビルドと検証（Phase 4b）

## 目的

Phase 4b の検証項目（F38b の Phase 4b 後再計測、F48 以降の新規）、リスク（R37 以降）、性能目標を定義する。Phase 1〜4a で確定した F1〜F47 および R1〜R36 はすべて維持し、本書では **Phase 4b で追加・更新する箇所のみ** 記述する。

## 他文書との関係

- 上流: [`pre-research.md`](./pre-research.md)（§9 既存負債整理 / §10 性能予算）、[`01-overview.md`](./01-overview.md)（D56-D67）、[`03-dsp-core-spec.md`](./03-dsp-core-spec.md)（テスト方針）、[`04-wasm-audio-spec.md`](./04-wasm-audio-spec.md)（export 検証）、[`05-web-frontend-spec.md`](./05-web-frontend-spec.md)（実機確認項目）
- 下流: [`07-implementation-checklist.md`](./07-implementation-checklist.md)（Step ごとの検証達成ライン）
- 並行: Phase 4a [`06-build-and-verify.md`](../2026-05-08-004-phase4a/06-build-and-verify.md) — F1〜F47 および R1〜R36 の継承

## 性能目標（Phase 4b）

| 指標 | Phase 4a 後実測 | Phase 4b target | 警戒ライン | 撤退ライン |
|---|---|---|---|---|
| WASM gzip サイズ | 18.42 KB | **目標 < 22 KB**（想定 ~19 KB、Phase 4a + Phase 4b 純増 0.6 KB） | **警戒 < 25 KB**（要調査） | **撤退 < 30 KB**（R32: Modal 係数削減 / 楽器を 4 種に） |
| Worklet バンドルサイズ (production) | 8.17 KB | < 12 KB（Phase 4a < 10 KB から余裕、Piano + INSTRUMENT_KIND_MAP 拡張で +0.3 KB） | > 14 KB で esbuild 設定見直し |
| `process` per call (8 voice + Body + LFO + Pitch Bend + CC#7 + **Piano dispersion 8 段**、release cargo timing) | 0.023 ms | < 1.7 ms（Phase 4a 0.023 ms + Phase 4b dispersion +0.033 ms = 0.056 ms 想定） | > 2.0 ms で R37（dispersion M=8 → M=4） |
| `process` per call (Piano 以外、Phase 4a 同条件) | 0.023 ms | < 1.0 ms（dispersion skip で Phase 4a 同等） | > 1.5 ms で regression 調査 |
| Worklet `process` self time avg (`__synthDev.measureProcessTime`、Piano kind) | 未計測 | < 1.7 ms | > 2.0 ms で R30 (stride 4096 化等) |
| Worklet `process` self time max (`__synthDev.measureProcessTime`、Piano kind) | 未計測 | < 2.7 ms | > 3.0 ms で R30 |
| ヒープ確保 in `process` | 0 | 0（Phase 1 D4 維持、`dispersion_stages` は inline 配列） | > 0 で R29（debug build で alloc 検査） |
| Phase 4a 互換性 (Default + Mod Wheel=0 + LFO depth=0) | — | Phase 4a 出力との **ε=1e-6 バイト一致**（D67 機械保証） | バイト不一致は実装誤り、即修正 |

## 検証項目（F-tag）

Phase 1〜4a の F1〜F47 に加え、Phase 4b で **F48〜F58 の 11 件**を追加。

### F48 — `__synthDev.measureProcessTime` 計測自動化（D66）

**実施タイミング（指摘事項 #4 反映で改訂）**:
- **Step 2**: 型定義 + Web 側 API 雛形作成（`__synthDev.ts` の `measureProcessTime` 関数本体、messages.ts の variant 追加）
- **Step 14**: Worklet 側 dev-only timing 集約コード実装（`synth-processor.ts` の DEV_MODE ガードコード + `synth.svelte.ts` への dev export 追加）→ **F48 完成**
- **Step 16**: 実機計測（`pnpm dev` でブラウザ起動、Console から `await window.__synthDev.measureProcessTime(10000)` 実行）

**目的**: Phase 4a で実機計測の操作習熟が要因で実測値取得を持ち越していた F38b について、Console から呼べる API で自動集計できるようにする。

**手順**:
1. `pnpm dev` で dev server 起動
2. ブラウザで `http://localhost:5173/physics-base-synth/` を開き Start ボタンクリック
3. 8 voice 押下状態（PC キーボード a-k 押下持続）
4. DevTools Console で:
   ```javascript
   const result = await window.__synthDev.measureProcessTime(10000);
   console.log(result);
   // → { avg: 0.045, max: 0.087, min: 0.012, samples: [...], bufferOverflow: false, durationMs: 10000 }
   ```
5. avg / max を記録、`bufferOverflow: true` なら 4096 entry を超えた（48kHz/128 frames で 375 quanta/sec、4096 entry で約 10.92 秒分を保持。`durationMs ≤ 10000` なら overflow なしの想定）

**達成基準**:
- avg / max を取得できる（API がエラー throw しない）
- target: avg < 1.7 ms / max < 2.7 ms（Piano kind の最悪ケース）
- target 超過時は R37 対策（M=4 削減 / Piano kind 撤退）

### F49 — Phase 4b WASM gzip サイズ（D62 + D57 + D61）

**確認方法**:
```bash
# Git Bash で
pnpm build
gzip -kc web/build/_app/immutable/assets/wasm_audio.*.wasm | wc -c
```

**達成基準（3 段階）**:
- **目標**: gzip < 22 KB（想定 ~19 KB、Phase 4a 18.42 KB + Phase 4b 純増 0.6 KB）
- **警戒**: gzip < 25 KB（要調査、`wasm-opt --print-stats` で各 pass の効果を分析）
- **撤退**: gzip < 30 KB（Phase 1-4a 継承 target、超過で R32 = Modal 係数削減 / 楽器 4 種に）

**サブ確認**:
- `pnpm build:wasm:dev` で wasm-opt がスキップされる（ビルド時間 < 5s 維持、Phase 4a 既存）
- `pnpm build:wasm` で wasm-opt 適用される（Phase 4a 既存）
- Phase 4b Step 3 で `wasm-opt --print-stats` を一時的に実行、Phase 4a 18.42 KB の各 pass の内訳を retrospective §5 へ追記

### F50 — リアルタイム性能 release cargo timing（F46 拡張、Piano kind 含む）

**cargo test レベル** (`tests/dsp_core_tests.rs` 拡張):

```rust
#[cfg(not(debug_assertions))]
#[test]
fn test_engine_process_block_timing_phase4b_piano() {
    // 8 voice 全 active + Piano kind + LFO depths 全 1.0 + Mod Wheel=1.0 + Pitch Bend
    // + CC#7 動的変化の最悪ケース
    // 1000 回 process の平均が < 1.7 ms
    // ...
    assert!(per_block_ms < 1.7, "Phase 4b Piano target: avg < 1.7 ms, got {}", per_block_ms);
}

#[cfg(not(debug_assertions))]
#[test]
fn test_engine_process_block_timing_phase4b_other() {
    // 8 voice 全 active + Default kind (dispersion skip) + LFO + Pitch Bend + CC#7
    // 1000 回 process の平均が < 1.0 ms (Phase 4a 0.023 ms と同等期待)
    assert!(per_block_ms < 1.0, "Phase 4b non-Piano target: avg < 1.0 ms, got {}", per_block_ms);
}
```

**達成基準**:
- Piano kind の avg < 1.7 ms（Phase 4a 0.023 ms + dispersion +0.033 ms = 0.056 ms 想定の 30× 余裕）
- 非 Piano の avg < 1.0 ms（Phase 4a 互換性、dispersion skip で CPU 増加なし）

### F51 — Phase 4b ピアノ音色の DSP 動作（D57 / D58 / D59）

**cargo test レベル** (`crates/dsp-core/tests/dispersion_tests.rs` + `karplus_strong_dispersion_tests.rs`):

| サブ項目 | 検証内容 |
|---|---|
| F51-a | `compute_dispersion_a1(8, 7.5e-4, 440.0, 48000.0)` の `a1.abs() < 1.0`（極の単位円内安定性） |
| F51-b | B が 1e-6 → 1e-2 で `|a1|` 単調増加 |
| F51-c | A0 (27.5Hz) と C8 (4186Hz) で a1 が異なる（`Ikey(f0)` 補正が効いている） |
| F51-d | `DispersionStage::process` で a1=0 で passthrough、a1≠0 で位相変調 |
| F51-e | M=8 cascade を 1024 サンプル走らせて NaN/Inf なし、|y| < 100 |
| F51-f | A4 / B=7.5e-4 で `gd_per_stage > 0`（dispersion で位相遅延が生じる） |
| F51-g | Piano kind の `note_on` 後、`dispersion_stage_a1(0)` が `compute_dispersion_a1` 戻り値と一致 |

**実機確認** (`pnpm dev`):
- Piano プリセットで A4 押下 → Default プリセットの A4 と比べて高音域に「stretched」な倍音（やや上ずれた整数倍音）が聞こえる
- 同じ周波数で hammer 風 attack（impulse から velocity 依存 LPF で smoothing された立ち上がり）が認識できる

### F52 — Hammer model（D61）

**cargo test レベル** (`tests/karplus_strong_dispersion_tests.rs`):

| サブ項目 | 検証内容 |
|---|---|
| F52-a | `dispersion_active = true` 状態で note_on 後、buffer[0] が単位 impulse 由来（pluck noise burst でない、autocorr 確認） |
| F52-b | `dispersion_active = false` で従来 pluck noise burst（Phase 4a 互換、autocorr 異なる） |
| F52-c | velocity=0.1 と velocity=1.0 で buffer の高域成分（前半 sample の RMS / 後半 RMS 比）が異なる |
| F52-d | velocity=1.0 の方が cutoff_high (4000 Hz) が高く、buffer の高域成分が多い |

**実機確認**:
- Piano プリセットで弱打鍵 (velocity 30) → 柔らかい音、強打鍵 (velocity 120) → 明るい音
- Default プリセットでは velocity 変動でこのような明確な音色変化はない（pluck 経路は velocity 振幅変化のみ）

### F53 — Piano Modal Body 係数（D62）

**cargo test レベル** (`tests/modal_body_tests.rs` 拡張 + `tests/instrument_tests.rs`):

| サブ項目 | 検証内容 |
|---|---|
| F53-a | `BODY_MODES_PIANO_L[0].freq == 55.0`（soundboard mode 1, Conklin 1996） |
| F53-b | `STEREO_SPREAD_PIANO == 0.05` |
| F53-c | `apply_instrument(Piano)` 後、`modal_body.coeff_l_b0(0)` が `BODY_MODES_PIANO_L[0]` ベースの計算値と一致 |
| F53-d | `body_modes_for_instrument(InstrumentKind::Piano)` が `(&BODY_MODES_PIANO_L, &BODY_MODES_PIANO_R)` を返す |
| F53-e | `INSTRUMENT_KIND_COUNT == 8`（Phase 4a 7 → Phase 4b 8） |
| F53-f | `INHARMONICITY_B_PIANO == 7.5e-4` / `HAMMER_CUTOFF_LOW_PIANO == 800.0` / `HAMMER_CUTOFF_HIGH_PIANO == 4000.0` |

**実機確認**:
- Piano プリセット選択で `synth.engine.applyInstrument('piano')` が dispatch される
- Piano は他楽器より低音域の共鳴感が強く（55 Hz, Q=10）、ピアノっぽい響板感

### F54 — `Engine::apply_instrument` の即時 release + `set_dispersion_active` 切替（D63 改訂後 / D67）

**仕様変更（指摘事項 #3 反映）**: 当初 D63 で「5 ms fade-out」を提案していたが、`SmoothedValue::set_target` は target 代入のみで `current` は `next_sample()` でしか進まないため、同期メソッド内では fade-out が発生しない。状態機械（`PendingInstrumentChange`）導入も検討したが Phase 4b の主目的（ピアノ音色）に対する実装複雑度が大きいため、**Phase 4a D53「即時 release」を継承**し、Phase 4b 新規追加は `pool.set_dispersion_active(piano)` の 1 行のみとした。

**cargo test レベル** (`tests/instrument_tests.rs`):

| サブ項目 | 検証内容 |
|---|---|
| F54-a | `apply_instrument(Piano)` で `pool.active_count() == 0`（即時 release、Phase 4a D53 継承） |
| F54-b | `apply_instrument(Piano)` で `pool.voice(0..N).all(|v| v.dispersion_active() == true)` |
| F54-c | `apply_instrument(Default)` で `pool.voice(0..N).all(|v| v.dispersion_active() == false)`（Piano → Default で確実に dispersion_active が false に戻る） |
| F54-d | `apply_instrument(Piano)` 内で `output_gain.target()` が変更されない（D63 改訂後、fade-out 機構なし） |

**実機確認**:
- 楽器切替時の挙動は Phase 4a と同じ（即時 release、pop noise も Phase 4a と同レベル）。fade-out は Phase 4c 送り
- Piano プリセット選択 → 音が出る、`pool.set_dispersion_active(true)` が漏れなく fan-out されている

### F55 — Phase 4a 互換性のバイト一致保証（D67、最重要）

**cargo test レベル** (`tests/karplus_strong_dispersion_tests.rs`):

```rust
#[test]
fn test_dispersion_disabled_matches_phase4a() {
    // Default kind / Mod Wheel=0 / LFO depth 全 0 / 全パラメータ Phase 4a 既定値
    // → dispersion_active = false 経路で Phase 4a と完全一致を確認

    let mut engine_phase4b = Engine::new();
    engine_phase4b.prepare(48000.0, 128);
    engine_phase4b.note_on(60, 0.8);

    let mut buf_l_phase4b = vec![0.0; 256];
    let mut buf_r_phase4b = vec![0.0; 256];
    engine_phase4b.process(&mut buf_l_phase4b, &mut buf_r_phase4b);

    // Phase 4a の固定 golden 値（test fixtures に保存）または直接対照計算
    // Phase 4a の Engine::process に dispersion 分岐がない経路と等価
    let golden = phase_4a_golden_buffer_for_default_c4_velocity_08();

    for i in 0..256 {
        assert!((buf_l_phase4b[i] - golden.l[i]).abs() < 1e-6,
            "Frame {} L mismatch: phase4b={} vs phase4a={}", i, buf_l_phase4b[i], golden.l[i]);
        assert!((buf_r_phase4b[i] - golden.r[i]).abs() < 1e-6,
            "Frame {} R mismatch: phase4b={} vs phase4a={}", i, buf_r_phase4b[i], golden.r[i]);
    }
}
```

**達成基準**:
- Default kind / Mod Wheel=0 / LFO depth=0 で Phase 4a と Phase 4b の出力が **ε=1e-6 でバイト一致**
- 不一致は実装誤り、Phase 4b 完成 NG

**Golden 値の生成**: Phase 4b 着手時に Phase 4a の HEAD でテストを実行して `buf_l/r` の最初 256 frame を `cargo test -- --nocapture` で出力、`tests/fixtures/phase4a_default_c4.json` に保存（コメントに Phase 4a commit hash を記載）。

### F56 — `.gitattributes` LF 統一（D65）

**確認方法**:
```bash
# .gitattributes 適用後
git check-attr -a -- web/src/lib/components/Keyboard.svelte
# → web/src/lib/components/Keyboard.svelte: text: auto
# → web/src/lib/components/Keyboard.svelte: eol: lf

git ls-files --eol | head
# → i/lf    w/lf    attr/text=auto eol=lf  web/src/...
```

**達成基準**:
- 全 tracked text file が `i/lf w/lf` で LF 統一
- prettier format 後に CRLF 差分が出ない（Phase 4a で頻発した CRLF/LF 戦争が断たれる）
- `git status` が clean な状態で `pnpm fmt` を実行 → 差分なし（Phase 4a では LF/CRLF 差分が出ていた）

### F57 — Phase 4a 全機能の互換性（regression baseline）

**cargo test レベル**:

| サブ項目 | 検証内容 |
|---|---|
| F57-a | Phase 4a 既存 120 PASS + 1 IGNORED（C8 ピッチテスト）すべて維持 |
| F57-b | `test_default_instrument_matches_phase3_modes` (Phase 4a 既存) が Phase 4b でも通る |
| F57-c | `test_mod_wheel_zero_disables_lfo` (Phase 4a 既存) が Phase 4b でも通る |
| F57-d | Phase 4a 既存 LFO / Mod Wheel / Preset / 多楽器 6 種テストがすべてパス |

**実機確認**:
- Default プリセットで Mod Wheel = 0 にすると Phase 4a と同じ音が出る
- Phase 4a の 6 楽器（Guitar Classical / Ukulele / Mandolin / Bass / Steel Guitar / Sitar）でそれぞれ Phase 4a と同じ音が出る
- LFO / Mod Wheel / Preset / Pitch Bend / Sustain Pedal / Channel Volume / All Notes Off の挙動が Phase 4a と同じ
- mono / poly トグルが Phase 4a と同じく動作

### F58 — メモリ確保ゼロ（Phase 4a 拡張、D4 / D44 維持）

**cargo test レベル** (`tests/no_alloc_tests.rs` 拡張):

| サブ項目 | 検証内容 |
|---|---|
| F58-a | `test_no_allocation_with_piano_kind`: 8 voice + Piano kind active + LFO + Mod Wheel + Pitch Bend + 楽器切替 (Piano ↔ Default 1 回) で voice buffer / LFO 状態 / `dispersion_stages` capacity 不変 |
| F58-b | Piano kind の `note_on` 100 連打で alloc 0（hammer LPF は note_on 内 stack 変数のみ） |

**実装パターン**:
```rust
let cap_before = engine.voice_state_capacity();
// ...各種操作（Piano kind での Pitch Bend + LFO depth + 楽器切替 + 連打）...
let cap_after = engine.voice_state_capacity();
assert_eq!(cap_before, cap_after);
```

## リスク（R-tag）

Phase 1〜4a の R1〜R36 を維持。Phase 4b で R37〜R39 を追加。

### R37 — Piano kind での dispersion cascade CPU 超過

**シナリオ**: 8 voice × 8 段 dispersion で 256 演算/sample が想定より重く、Piano kind 演奏時に Worklet `process` self time が target 1.7 ms を超える（特に低スペックマシン）。

**対策**:
1. **第 1 段階** (Piano avg > 2.0 ms): dispersion を M=8 → M=4 に削減（pre-research §4.3、CPU 半減 / 表現力やや低下）
2. **第 2 段階** (Piano avg > 2.5 ms): Piano kind の最大同時発音を 8 → 4 に制限（voice_pool 経由）
3. **第 3 段階** (Piano avg > 3.0 ms): Piano kind を Phase 4b から外し、Phase 4c で SIMD 化と同時に再導入

**検証**: F50（cargo timing）+ F48（実機 `__synthDev.measureProcessTime`）で early detection

### R38 — Hammer LPF の cutoff 計算で `f32::exp` 精度

**シナリオ**: `1.0 - (-2.0 * PI * cutoff_hz / sample_rate).exp()` で cutoff が極端に低い（< 50 Hz）かつ sample_rate が高い（96 kHz など）場合、`exp` 引数が ~-0.0033 で `1 - exp(-0.0033) ≈ 0.0033` の精度劣化、buffer 初期化時の LPF 効果が想定より弱い。

**対策**: 
- Phase 4b の Piano cutoff_low = 800 Hz / cutoff_high = 4000 Hz / sample_rate ≤ 48 kHz では精度劣化は無視できる（α は 0.1 以上）
- 念のため `alpha.clamp(0.001, 0.999)` で安全側に制限（実装で対応）
- 96 kHz 対応や cutoff < 100 Hz の Hammer Hardness 実装は Phase 4c

**検証**: F52 cargo test、各 sample_rate / velocity で α の値域チェック

### R39 — Piano プリセット切替時の dispersion_active fan-out 漏れ

**シナリオ**: `Engine::apply_instrument(Piano)` で `pool.set_dispersion_active(true)` を fan-out したが、その後の `apply_instrument(Default)` で `set_dispersion_active(false)` の呼出が抜け、Default kind に戻したのに dispersion が残ったまま音が変調される。

**対策**: 
- `apply_instrument` 内で **kind に関わらず必ず `dispersion_active` を再評価**:
  ```rust
  let dispersion_active = matches!(kind, InstrumentKind::Piano);
  self.pool.set_dispersion_active(dispersion_active);
  ```
- `test_apply_instrument_default_disables_dispersion` で機械保証

**検証**: F54-b（cargo test）+ 実機での Piano → Default 切替後の音確認

## トラブルシューティング tips

### `pnpm build` が gen-params.mjs エラーで失敗 (Piano 専用フィールド検証)

- `params.json` の Piano エントリに `inharmonicity_b` / `hammer_cutoff_low_hz` / `hammer_cutoff_high_hz` がすべて存在することを確認
- `hammer_cutoff_high_hz > hammer_cutoff_low_hz` の制約に違反していないか確認
- `pnpm gen:params` を実行してエラーメッセージを直接確認

### `cargo test test_dispersion_disabled_matches_phase4a` が fail（D67 互換性）

- Phase 4b の `KarplusStrong::process_sample` で `if self.dispersion_active` 分岐の else 経路が Phase 4a と完全一致しているか確認
- `dispersion_active = false` 経路で `thiran.process(self.buffer[read_z])` の引数が Phase 4a と一致しているか
- Phase 4a の golden 値（`tests/fixtures/phase4a_default_c4.json`）が古い場合は Phase 4a HEAD で再生成

### Piano プリセット選択しても音色変化が小さい

- `synth.engine.applyInstrument('piano')` が dispatch されているか DevTools Console で確認
- `engine.currentInstrument` が `'piano'` になっているか
- `synth_apply_instrument(handle, 7)` が WASM 側で呼ばれているか（debug ビルドで `console.log` 追加）
- Piano プリセットの `params.damping = 0.998` が適用されているか（DAMPING スライダーで確認）

### `__synthDev.measureProcessTime` が timeout する

- AudioWorkletNode が初期化されているか確認（Start ボタンクリック必須）
- DEV_MODE フラグが true で build されているか（`pnpm dev` であって `pnpm preview` でない）
- `synth.engine.workletPort()` が `null` を返していないか

### Piano 演奏で音が出ない / 異常音

- `dispersion_active = true` 状態で `note_on` で `compute_dispersion_a1` が finite を返しているか確認
- `a1.abs() < 1.0` で極が単位円内になっているか（テスト F51-a）
- Hammer LPF の `alpha` が `0.001 < alpha < 0.999` の範囲か
- Piano プリセットの `damping = 0.998` で減衰がやや早いため、Default の 0.996 より持続が短いのは想定通り

### `pnpm fmt` で意図しない LF/CRLF 差分が出る (Phase 4a 既存問題)

- `.gitattributes` が repo root にあるか確認、`* text=auto eol=lf` が記載されているか
- `git add --renormalize .` を実行して既存 file を LF へ再正規化
- 個別 file が `i/lf w/lf` になっているか `git ls-files --eol` で確認

## ビルドコマンド一覧

| コマンド | 用途 |
|---|---|
| `pnpm dev` | dev WASM ビルド + Vite dev server (5173)、`__synthDev.measureProcessTime` 利用可 |
| `pnpm build:wasm` | release WASM ビルド + wasm-opt -O3 適用 |
| `pnpm build:wasm:dev` | dev WASM ビルド (wasm-opt スキップ) |
| `pnpm build` | release WASM + SvelteKit static build → `web/build/` |
| `pnpm preview` | 本番ビルドをプレビュー (4173)、F38b 計測用（dev_mode false で `__synthDev` 利用不可） |
| `pnpm check` | `cargo check --workspace` + `svelte-check` + params-sync |
| `pnpm lint` | `cargo clippy --workspace --all-targets -- -D warnings` |
| `pnpm fmt` | `cargo fmt --all` + prettier |
| `pnpm gen:params` | `params.json` から Rust / TS の params.rs / params.ts を生成 |
| `cargo test -p dsp-core` | dsp-core ユニットテスト全件 |
| `cargo test -p dsp-core test_dispersion_` | dispersion テストのみ |
| `cargo test -p dsp-core test_apply_instrument_piano` | Piano 楽器切替テストのみ |
| `cargo test --release -p dsp-core test_engine_process_block_timing_phase4b_` | release timing test |
| `cargo test -p dsp-core test_dispersion_disabled_matches_phase4a` | D67 互換性テスト |
| `pnpm --filter ./web check` | svelte-check 単独 |
| `pnpm --filter ./web lint` | prettier + eslint |
| `wasm-opt --print-stats web/static/wasm-audio.wasm` | Phase 4b Step 3、Phase 4a の WASM 各 pass 内訳調査 |

## サイズ計測手順

### WASM gzip サイズ

```bash
# Git Bash で
pnpm build
gzip -kc web/build/_app/immutable/assets/wasm_audio.*.wasm | wc -c
```

期待値: ~19000 bytes（target 22 KB の 86%、Phase 4a 18.42 KB から +0.6 KB）

### Worklet バンドルサイズ

```powershell
# PowerShell で
Get-ChildItem web\build\_app\immutable\assets\synth-processor*.js | Select-Object Name, Length
```

期待値: < 9 KB（production、`DEV_MODE = false` で timing 集約コード削除）

### `__synthDev` 検証（Phase 3 F22 / Phase 4a 既存 + Phase 4b 拡張、指摘事項 #1 反映）

```bash
# Git Bash で、production ビルドに __synthDev / measureProcessTime / DEV_MODE /
# timingBuffer が残っていないことを確認。
# Worklet bundle は static/worklet/ 由来で web/build/worklet/synth-processor.js 側に
# 出るため、web/build/_app/immutable/ だけでなく web/build/ 全体を grep する。
grep -r "__synthDev\|measureProcessTime\|DEV_MODE\|timingBuffer" web/build/ | wc -l
# 期待値: 0
```

### `wasm-opt --print-stats` ベースライン記録（Phase 4b Step 3）

```bash
# Phase 4b 着手時に Phase 4a の WASM の各 pass 内訳を取得
pnpm build:wasm  # まず最新 WASM を生成
wasm-opt --print-stats web/static/wasm-audio.wasm > /tmp/wasm-opt-stats.txt
# /tmp/wasm-opt-stats.txt の内容を docs/retrospective/2026-05-08-004-phase4a.md §5 へ追記
```

期待出力:
```
Functions: 18
Imports: 0
Globals: 12
Memories: 1 (initial: 17 pages = 1.0 MB)
...
```

## 達成ライン早見表（Step 別、07 章への種）

| ステップ完了 | 達成する F-tag |
|---|---|
| Step 1 (`.gitattributes` LF 統一 + `git add --renormalize`) | F56 |
| Step 2 (`__synthDev.measureProcessTime` 整備) | F48 準備（型定義 + Web 側 API のみ、Worklet 側集約は Step 14 で完成） |
| Step 3 (`wasm-opt --print-stats` ベースライン記録) | F49（部分、調査） |
| Step 4 (`params.json` Piano 楽器追加 + `gen-params.mjs` 拡張) | F49（生成パイプライン）、F53 の準備 |
| Step 5 (`dispersion.rs` 実装) | F51-a〜f |
| Step 6 (`KarplusStrong` に dispersion フィールド + `note_on` の cascade 初期化) | F51-g |
| Step 7 (`process_sample` で cascade 適用) | F51 cargo 通過 |
| Step 8 (`note_on_internal` で hammer 経路分岐) | F52 |
| Step 9 (Voice trait + VoicePool に `set_dispersion_active`) | （Step 11 で活用） |
| Step 10 (Piano Modal 係数 + `body_modes_for_instrument` の Piano 分岐) | F53-a〜e |
| Step 11 (`Engine::apply_instrument` 末尾に `set_dispersion_active` 呼出を追加、Phase 4a D53 即時 release を継承) | F54 |
| Step 12 (`messages.ts` + `synth-processor.ts` + `engine.ts` の InstrumentKindKey 拡張) | F49（経路）|
| Step 13 (`preset-schema.ts` + `factory-presets.ts` で Piano エントリ追加) | F53-c 経路 |
| Step 14 (`__synthDev.ts` + dev-only timing 集約コード) | F48 完成 |
| Step 15 (統合 cargo test + alloc + release timing + Phase 4a 互換性) | F50 / F55 / F57 / F58 |
| Step 16 (実機確認 + `__synthDev.measureProcessTime` で F48 計測) | F48 実機 / F51-F54 実機 / F49 / F57 実機 |
| Step 17 (ドキュメント整備 + retrospective 準備) | Phase 4b 完成 |
| Step 18 (PR 作成 + main マージ) | Phase 4b リリース |

すべての F48〜F58 + Phase 4a 既存 F1〜F47 が達成された時点で Phase 4b 完成。F50（release timing）と F58（alloc ゼロ）は **Step 15 で必須化**、F48 は **Step 2 で準備、Step 14 で完成、Step 16 で実機計測実施**（指摘事項 #4 反映で実施タイミングを 3 段階に分離）。
