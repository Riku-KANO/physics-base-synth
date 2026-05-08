# 02. Phase 4a アーキテクチャ

## 目的

Phase 1 / 2 / 3 で確立した 4 レイヤ構成（Svelte UI → AudioWorkletProcessor → wasm-audio → dsp-core）に対し、Phase 4a で追加する責務（LFO / Mod Wheel / プリセット保存・ロード / 多楽器切替 / wasm-opt）の配置を明確化する。Phase 3 までの構成は崩さず、新規責務をどのレイヤに置くかと既存コンポーネントの拡張点を定義する。

## 他文書との関係

- 上流: [`01-overview.md`](./01-overview.md)（Phase 4a の確定事項 / D44-D55 / 完成像）
- 下流: 本書のレイヤ責務に従って 03〜05 章で具体的な API / モジュール / コンポーネントを定義
- 並行: Phase 1 / 2 / 3 [`02-architecture.md`] — 同じ章番号 / 構造、Phase 4a は差分のみ記述

## 4 レイヤ構成（Phase 3 から不変）

```
Svelte UI (main thread) ──MessagePort──▶ AudioWorkletProcessor
                                              │ FFI (C ABI, raw exports)
                                              ▼
                                         wasm-audio (cdylib)
                                              │
                                              ▼
                                         dsp-core (rlib)
```

各レイヤの責務と禁止事項は CLAUDE.md と Phase 1 / 2 / 3 02 章で定義済み。Phase 4a でも変更しない。

## Phase 4a で追加される責務の配置

### Svelte UI 層（main thread）

| 責務 | 配置 | 既存パターン参照 |
|---|---|---|
| **Mod Wheel スライダー** | `web/src/lib/components/ModWheel.svelte`（新規） | `ParamSlider.svelte` の input range UI パターンを踏襲 |
| **LFO controls UI**（rate / waveform / 3 destinations の depth）| `web/src/lib/components/LfoSection.svelte`（新規）または既存 `<section class="params">` への追加 | Phase 3 D42 で追加した `PolyphonyToggle` と同じく `<section>` 直下に配置 |
| **Preset セレクター UI**（Factory + User）| `web/src/lib/components/PresetSelector.svelte`（新規） | `MidiSelect.svelte` の `<select>` ドロップダウンパターン |
| **Instrument ピッカー UI**（6 種ドロップダウン）| Preset 経由で適用、独立 UI は不要（プリセット選択時に楽器も切り替わる） | — |
| **Preset 保存・削除ボタン** | `PresetSelector.svelte` 内に配置 | Phase 3 既存 `<button onclick>` パターン |
| **localStorage 操作レイヤ** | `web/src/lib/state/preset-store.svelte.ts`（新規） | `synth.svelte.ts` / `ui.svelte.ts` の Svelte 5 runes (`$state`) パターン |
| **Preset apply 経路** | `preset-store.svelte.ts → engine.applyPreset(preset)` | `engine.ts` 既存の個別 setter (`setParam`, `sendMidiCc`) の集約呼出 |
| **WebMIDI Mod Wheel ハンドラ** | `web/src/lib/input/midi-cc.ts`（既存）に CC#1 → `engine.sendMidiCc(1, value)` 経路を追加 | Phase 3 D38 で実装済の他 CC と同経路 |

### AudioWorkletProcessor 層（音声スレッド）

| 責務 | 配置 | 既存パターン参照 |
|---|---|---|
| **LFO message dispatch** | `synth-processor.ts` の `onMessage` switch に 4 ケース追加（`lfoSetRate` / `lfoSetWaveform` / `lfoSetDepth` / `applyInstrument`） | Phase 3 D38 で追加した `midiCC` / `pitchBend` の dispatch パターン |
| **WasmExports interface 拡張** | `synth-processor.ts` 冒頭の `interface WasmExports` に 4 関数追加 | Phase 3 で 11 → 14 C ABI 関数に拡張した既存パターン（memory export を含めると 15 → 必須 export 数） |
| **Voice State stride push** | 変更なし（Phase 3 D41 の 1024 sample stride 維持） | — |
| **WASM ロード** | 変更なし（`WebAssembly.instantiate(bytes, { env: {} })`、`wasm-opt -O3` でサイズ削減後も同じバイナリを読む） | — |

**禁止事項**: `process()` 内での新規 alloc は Phase 3 と同じく禁止。LFO 用の新規 view も `init` で 1 度確保、以降 `memory.buffer` 不変前提（Phase 3 D9 維持）。

### wasm-audio 層（C ABI 境界）

| 責務 | 配置 | 既存パターン参照 |
|---|---|---|
| **C ABI 4 関数追加** | `crates/wasm-audio/src/lib.rs` に `synth_apply_instrument` / `synth_lfo_set_rate` / `synth_lfo_set_waveform` / `synth_lfo_set_depth` を追加 | Phase 3 D38 / D41 で追加した `synth_midi_cc` / `synth_pitch_bend` / `synth_voice_state_ptr` と同じ `#[unsafe(no_mangle)] pub extern "C" fn` パターン |
| **ポインタ deref 安全性** | 既存 `unsafe { &mut *handle }` パターンを継承、`#![allow(clippy::not_unsafe_ptr_arg_deref)]` も継続 | — |
| **`SynthHandle` struct** | 変更なし（`Engine` を保持、scratch_l/r も Phase 1 から継承） | — |

### dsp-core 層（純粋 DSP）

| 責務 | 配置 | 既存パターン参照 |
|---|---|---|
| **`Lfo` 型定義** | `crates/dsp-core/src/lfo.rs`（新規） | `loss_filter.rs` / `modal_body.rs` と同じく単一責務モジュール、`#[inline(always)] pub fn process_sample()` パターン |
| **`InstrumentKind` enum** | `crates/dsp-core/src/params.rs`（生成ファイル）または手書き `crates/dsp-core/src/instrument.rs`（新規） | params.rs は `gen-params.mjs` 生成、enum は drift 防止のため codegen が望ましい |
| **多楽器 Modal 係数定数** | `crates/dsp-core/src/params.rs`（生成ファイル、`gen-params.mjs` 拡張） | Phase 3 の `BODY_MODES_L` / `BODY_MODES_R` を `BODY_MODES_<INSTRUMENT>_L` / `<INSTRUMENT>_R` の 6 種 × stereo 2 = 12 配列に拡張 |
| **`Engine::apply_instrument(kind)`** | `crates/dsp-core/src/engine.rs` の inherent method | Phase 3 既存 `set_param` / `set_mode` と同じ Engine 状態変更 API パターン |
| **Engine 内 LFO 状態** | `Engine` struct に `lfo: Lfo` / `mod_wheel: SmoothedValue` を追加 | Phase 3 で追加した `modal_body` / `sustain_state` / `channel_volume` と同じく Engine の保有フィールド |
| **LFO の destinations 適用** | `Engine::process` の per-sample loop 内で `lfo.process_sample()` → 値を pool / engine の SmoothedValue target に offset として加算 | Phase 3 の `modal_body.process_sample()` 呼出位置と同じく、`pool.process_sample()` 前後に挿入 |
| **denormal flush** | LFO 値にも適用検討（pre-research §3.7、process_sample 内で `+1e-25 -1e-25`） | Phase 3 D6 維持 |

**禁止事項**: `process` ホットパス中のヒープ確保ゼロ（Phase 1 D4 維持）。LFO 状態は Engine 内固定 size、楽器切替時の Modal 係数差し替えも `prepare` で確保した固定領域内（既存 `coeffs_l` / `coeffs_r` 配列を上書き）。

## ビルドパイプラインの差分

### Phase 3 までのパイプライン

```
params.json (単一ソース)
   │
   │ scripts/gen-params.mjs
   ▼
crates/dsp-core/src/params.rs (生成)
web/src/lib/audio/generated/params.ts (生成)
   │
   │ scripts/check-params-sync.mjs (CI 検証)
   ▼
PR diff で drift 検知

cargo build --target wasm32-unknown-unknown --release
   │
   │ scripts/copy-wasm.mjs (素のコピー)
   ▼
web/static/wasm-audio.wasm (Phase 3 実測 27.78 KB gzip)
   │
   │ scripts/check-wasm-exports.mjs (REQUIRED 配列 14 C ABI 関数 + memory export = 15 entry)
   ▼
PR で export 名 drift 検知
```

### Phase 4a でのパイプライン拡張

```
params.json (Phase 4a で LFO + 楽器 6 種定義 + stereo_spread を楽器ごとに保持に拡張)
   │
   │ scripts/gen-params.mjs (拡張: BodyMode 6 種 + InstrumentKind enum + LFO ParamId 出力)
   ▼
crates/dsp-core/src/params.rs (Phase 4a で BODY_MODES_<INSTRUMENT>_L/R 12 配列 + InstrumentKind 出力)
web/src/lib/audio/generated/params.ts (Phase 4a で InstrumentKind enum + STEREO_SPREAD_<INSTRUMENT> 出力)
   │
   │ scripts/check-params-sync.mjs (拡張: 楽器配列 12 件の同期検証)
   ▼
PR diff で drift 検知

cargo build --target wasm32-unknown-unknown --release
   │
   │ scripts/copy-wasm.mjs (Phase 4a で wasm-opt -O3 --strip-debug を呼出)  ← D45 差分
   ▼
web/static/wasm-audio.wasm (~13 KB gzip target、wasm-opt -O3 効果込み)
   │
   │ scripts/check-wasm-exports.mjs (REQUIRED 配列 18 C ABI 関数 + memory export = 19 entry)
   ▼
PR で export 名 drift 検知
```

### `wasm-opt` 統合の詳細（D45）

**配置**: `scripts/copy-wasm.mjs` の本体に追加（`pnpm build:wasm` / `pnpm build:wasm:dev` の両方で動作）。

**実装方針**: 既存 `scripts/copy-wasm.mjs` は `node scripts/copy-wasm.mjs <profile>` で `process.argv[2]` から `'release'` / `'debug'` を受け取る設計（`package.json` の `build:wasm` / `build:wasm:dev` で profile を渡している）。Phase 4a でもこの引数渡しの規約を維持し、**`profile === 'release'` のときのみ wasm-opt を適用**する:

```javascript
// scripts/copy-wasm.mjs（疑似コード、03 章で詳細）
import { execFileSync } from 'node:child_process';
import { existsSync, copyFileSync } from 'node:fs';

const profile = process.argv[2] === 'release' ? 'release' : 'debug';
const wasmOptBin = resolveWasmOpt();  // node_modules/.bin/wasm-opt or system wasm-opt

if (profile === 'release' && wasmOptBin && existsSync(wasmOptBin)) {
  execFileSync(wasmOptBin, ['-O3', '--strip-debug', srcPath, '-o', dstPath], { stdio: 'inherit' });
  console.log(`[copy-wasm] wasm-opt -O3 applied: ${srcSize} → ${dstSize} bytes`);
} else {
  copyFileSync(srcPath, dstPath);
  if (profile === 'release') {
    console.warn('[copy-wasm] wasm-opt not found, skipping optimization');
  }
}
```

**dev ビルドでの扱い**: `pnpm build:wasm:dev` (profile=`debug`) では wasm-opt をスキップ（dev ビルド時間短縮、debug 情報保持）。production ビルド (`pnpm build:wasm`、profile=`release`) でのみ適用。`package.json` の script 定義は不変、profile 引数渡しの既存規約を継承。

**依存追加**: `package.json` の `devDependencies` に `binaryen` を追加（npm パッケージ、build-time のみ）。Cargo の依存ではないため Phase 1〜3 の「dsp-core / wasm-audio に外部 crate 追加禁止」制約に抵触しない。

**フォールバック**: wasm-opt が見つからない場合は警告ログを出して素コピーで続行（CI 失敗にしない、開発者の環境差分を吸収）。

## メッセージプロトコル拡張

### `ToWorkletMessage`（main → Worklet）

Phase 3 既存:
```typescript
type ToWorkletMessage =
  | { type: 'init'; wasmBytes: ArrayBuffer; sampleRate: number }
  | { type: 'noteOn'; midi: number; velocity: number }
  | { type: 'noteOff'; midi: number }
  | { type: 'setParam'; id: number; value: number }
  | { type: 'setMode'; mode: 'mono' | 'poly' }
  | { type: 'midiCC'; cc: number; value: number }
  | { type: 'pitchBend'; semitones: number }
  | { type: 'reset' }
  | { type: 'dispose' };
```

Phase 4a で追加:
```typescript
type ToWorkletMessage =
  | ...既存...
  | { type: 'lfoSetRate'; hz: number }
  | { type: 'lfoSetWaveform'; kind: 'sine' | 'triangle' }
  | { type: 'lfoSetDepth'; dest: 'pitch' | 'brightness' | 'volume'; depth: number }
  | { type: 'applyInstrument'; kind: InstrumentKind };  // 'default' | 'guitar_classical' | ...
```

### `FromWorkletMessage`（Worklet → main）

**変更なし**（Phase 3 の `ready` / `error` / `debug` / `voiceState` を維持）。プリセット保存・ロードは UI 層完結のため Worklet 経由不要。

## プリセット適用フロー

```
ユーザー操作: PresetSelector で "My Sound 1" を選択
   ▼
PresetSelector.svelte: presetStore.applyPreset('My Sound 1', engine)
   ▼
preset-store.svelte.ts: applyPreset(preset, engine):
   1. engine.applyInstrument(preset.instrument)
        → MessagePort: { type: 'applyInstrument', kind }
        → Worklet: synth_apply_instrument(handle, kindNumeric)
        → dsp-core::Engine::apply_instrument(kind):
             - pool.all_notes_off()
             - modal_body.coeffs_l/r を新楽器の BODY_MODES に差し替え
             - modal_body.prepare(sample_rate) で係数再計算
             - modal_body.reset() で z1/z2 状態クリア
             - stereo_spread を新楽器値に更新（params.rs 経由）
   2. engine.setParam(Damping, preset.params.damping)
   3. engine.setParam(Brightness, preset.params.brightness)
   4. engine.setParam(OutputGain, preset.params.outputGain)
   5. engine.setParam(PickPosition, preset.params.pickPosition)
   6. engine.setParam(BodyWet, preset.params.bodyWet)
   7. engine.lfoSetRate(preset.lfo.rate)
   8. engine.lfoSetWaveform(preset.lfo.waveform)
   9. engine.lfoSetDepth('pitch', preset.lfo.pitchDepth)
  10. engine.lfoSetDepth('brightness', preset.lfo.brightnessDepth)
  11. engine.lfoSetDepth('volume', preset.lfo.volumeDepth)
   ▼
synth.svelte.ts: $state を更新（ParamSlider が反応して UI に反映）
```

**重要**: 各 step は MessagePort で個別送信、Worklet 側で順次 `synth_*` を呼ぶ。**`applyPreset` 全体を 1 message で送る最適化は不要**（main → Worklet rAF スロットルが既に存在、UX 影響なし）。

## 楽器切替フロー（プリセット選択の subset）

```
ユーザー操作: InstrumentPicker または PresetSelector で楽器変更
   ▼
engine.applyInstrument('ukulele')
   ▼
MessagePort: { type: 'applyInstrument', kind: 'ukulele' }
   ▼
Worklet: synth_apply_instrument(handle, 2)  // 2 = Ukulele
   ▼
dsp-core::Engine::apply_instrument(InstrumentKind::Ukulele):
   1. self.pool.all_notes_off()        // 演奏中の音を即時 release
   2. self.current_instrument = kind
   3. self.modal_body.set_instrument(kind, sample_rate):
        - coeffs_l[i] = calc_coeffs(BODY_MODES_UKULELE_L[i], sample_rate)
        - coeffs_r[i] = calc_coeffs(BODY_MODES_UKULELE_R[i], sample_rate)
        - states_l/r をゼロクリア（reset）
   4. self.stereo_spread = STEREO_SPREAD_UKULELE  // 楽器ごとの値（D54）
```

**`pool.all_notes_off()` の理由**: Modal 係数差し替えと voice 内の Karplus-Strong は独立だが、楽器が変わった瞬間に音色が急変するため、UI 体験として全 voice release が自然。fade-out は Phase 4b 以降の検討。

## LFO 配置フロー（per sample）

```
Engine::process(output_l, output_r):
   for i in 0..n {
     # Phase 4a 追加: LFO 値を取得
     let lfo_value = self.lfo.process_sample();  // ∈ [-1, 1]
     let mod_wheel = self.mod_wheel.next_sample();  // ∈ [0, 1]

     # Phase 4a 追加: LFO Pitch destination を pool に伝播
     # Engine 側で exp2 を 1 回計算し、factor を fan-out（per voice exp2 を回避、03 章で詳細）
     let pitch_offset_semitones = lfo_value * self.lfo_pitch_depth.next_sample() * mod_wheel * 0.5;
     let pitch_factor = (-pitch_offset_semitones / 12.0).exp2();
     self.pool.set_lfo_pitch_factor(pitch_factor);

     # Phase 4a 追加: LFO Brightness destination を pool に伝播
     let brightness_offset = lfo_value * self.lfo_brightness_depth.next_sample() * mod_wheel * 0.5;
     self.pool.set_lfo_brightness_offset(brightness_offset);

     # Phase 4a 追加: LFO Volume destination は Engine 単位で適用
     let volume_multiplier = 1.0 + lfo_value * self.lfo_volume_depth.next_sample() * mod_wheel * 0.5;

     # Phase 3 既存パス
     let dry = self.pool.process_sample();
     let (body_l, body_r) = self.modal_body.process_sample(dry);
     let wet = self.body_wet.next_sample();
     let dry_amount = 1.0 - wet;
     let mixed_l = dry_amount * dry + wet * body_l;
     let mixed_r = dry_amount * dry + wet * body_r;

     # Phase 4a 追加: volume_multiplier を combined gain に乗算
     let combined = self.output_gain.next_sample() * self.channel_volume.next_sample() * volume_multiplier;

     output_l[i] = soft_clip(mixed_l * combined);
     output_r[i] = soft_clip(mixed_r * combined);
   }
```

**注意**: `pool.set_lfo_pitch_factor` / `set_lfo_brightness_offset` は per-sample 呼出。Engine 側で exp2 を 1 回計算し factor を fan-out する設計のため、KarplusStrong 側は乗算のみで処理（per voice exp2 を回避、03 章で詳細）。コストは Engine exp2 (7) + fan-out (16) + per voice 適用 (24) = 47 演算/sample で許容。

## ファイル変更リスト

### 新規作成

#### dsp-core
- `crates/dsp-core/src/lfo.rs` — LFO 型定義（D46 / D47）
- `crates/dsp-core/src/instrument.rs`（任意、enum を手書きする場合）

#### web
- `web/src/lib/components/ModWheel.svelte` — Mod Wheel スライダー UI
- `web/src/lib/components/LfoSection.svelte` — LFO controls UI
- `web/src/lib/components/PresetSelector.svelte` — プリセット選択 UI
- `web/src/lib/state/preset-store.svelte.ts` — localStorage 操作レイヤ
- `web/src/lib/state/preset-schema.ts` — `PresetV1` interface 定義
- `web/src/lib/state/factory-presets.ts` — Factory Preset 7 種の const テーブル

### 主要変更

#### dsp-core
- `crates/dsp-core/src/lib.rs` — `pub mod lfo;` 追加
- `crates/dsp-core/src/engine.rs` — `lfo` / `mod_wheel` / `current_instrument` / `lfo_*_depth` フィールド追加、`apply_instrument` / `lfo_set_*` メソッド追加、`process` 内で LFO 適用、`handle_midi_cc` の CC#1 分岐有効化
- `crates/dsp-core/src/params.rs`（生成ファイル）— `InstrumentKind` enum、`BODY_MODES_<INSTRUMENT>_L/R` 12 配列、`STEREO_SPREAD_<INSTRUMENT>` 6 値
- `crates/dsp-core/src/modal_body.rs` — `set_instrument(kind, sample_rate)` メソッド追加
- `crates/dsp-core/src/voice_pool.rs` — `set_lfo_pitch_factor` / `set_lfo_brightness_offset` 追加（全 voice fan-out、factor は Engine 側で exp2 済）
- `crates/dsp-core/src/karplus_strong.rs` — `lfo_pitch_offset` / `lfo_brightness_offset` フィールド追加（既存 `set_pitch_bend` の SmoothedValue に offset として加算する設計、03 章で詳細）
- `crates/dsp-core/src/karplus_strong.rs` の `excitation_snapshot` を `#[cfg(test)]` でガード（D45 既存負債解消）

#### wasm-audio
- `crates/wasm-audio/src/lib.rs` — `synth_apply_instrument` / `synth_lfo_set_rate` / `synth_lfo_set_waveform` / `synth_lfo_set_depth` の 4 関数追加

#### params.json + scripts
- `params.json` — `instruments` セクション（楽器 6 種 × {body_modes 8 件 + stereo_spread}）追加、LFO 関連 ParamDescriptor 追加
- `scripts/gen-params.mjs` — `InstrumentKind` enum 出力、`BODY_MODES_<INSTRUMENT>_L/R` 12 配列出力、`STEREO_SPREAD_<INSTRUMENT>` 6 値出力
- `scripts/check-params-sync.mjs` — 楽器 12 配列の同期検証
- `scripts/copy-wasm.mjs` — wasm-opt -O3 統合（D45）
- `scripts/check-wasm-exports.mjs` — REQUIRED 配列に 4 関数追加
- `package.json` — `devDependencies` に `binaryen` 追加

#### web
- `web/src/lib/audio/messages.ts` — `lfoSetRate` / `lfoSetWaveform` / `lfoSetDepth` / `applyInstrument` variant 追加
- `web/src/lib/audio/synth-processor.ts` — `WasmExports` interface に 4 関数追加、message dispatch に 4 ケース追加
- `web/src/lib/audio/engine.ts` — `lfoSetRate` / `lfoSetWaveform` / `lfoSetDepth` / `applyInstrument` / `applyPreset` メソッド追加
- `web/src/lib/state/synth.svelte.ts` — LFO 関連の `$state` (rate / waveform / 3 depths) 追加
- `web/src/routes/+page.svelte` — `<ModWheel>` / `<LfoSection>` / `<PresetSelector>` を配置

### 軽微な更新

- `README.md` — Phase 4a 機能追記、F38b 計測手順、F39+ 検証手順
- `CLAUDE.md` — 「現在のイテレーション」を Phase 4a へ更新、Phase 4b 予告追記
- `docs/retrospective/2026-05-07-003-phase3.md` §5 — F38b 計測結果追記

## レイヤ間の依存方向（Phase 3 から不変）

```
Svelte UI ──depends on──▶ Worklet messages.ts (型のみ)
Worklet ──depends on──▶ wasm-audio (FFI 経由、TS インポートなし)
wasm-audio ──depends on──▶ dsp-core (rlib 依存)
dsp-core ──depends on──▶ なし（依存ゼロ、Phase 1-3 制約継承）
```

Phase 4a でも依存方向を逆転させない。`preset-store.svelte.ts` は `engine.ts` に依存するが、`engine.ts` は `preset-store.svelte.ts` を知らない（applyPreset の呼出は UI が起点）。

## 設計判断の章間相互参照

- D45 (`wasm-opt -O3`) → 06 章 §性能目標（サイズ計測手順）
- D46 (LFO 配置) → 03 章 §Lfo / §Engine の per-sample loop
- D47 (LFO 波形) → 03 章 §Lfo の `process_sample` 実装
- D48 (LFO destinations) → 03 章 §Engine の destination 適用、05 章 §LfoSection UI
- D49 (Mod Wheel) → 03 章 §Engine::handle_midi_cc、05 章 §ModWheel UI
- D50 (Preset 形式) → 05 章 §preset-store.svelte.ts、§preset-schema.ts
- D51 (User Preset 上限) → 05 章 §PresetStore.save
- D52 (楽器 6 種) → 03 章 §multi-instrument modal coefficients
- D53 (楽器切替挙動) → 03 章 §Engine::apply_instrument
- D54 (stereo_spread 楽器別) → 03 章 §params.rs の STEREO_SPREAD_<INSTRUMENT>
- D55 (Mono+Sustain 現状維持) → 03 章 §Engine::note_off (Phase 3 D40 P1-2 継承)

## 性能目標の整理（03〜06 章への伝達）

| レイヤ | 目標 | 達成方式 | 検証 |
|---|---|---|---|
| dsp-core | LFO + 楽器切替で `process` per sample +28 演算 | inline / per-voice fan-out 最小化 | 03 章 §process_sample 計測、06 章 F37 release timing test |
| wasm-audio | C ABI 4 関数追加でも raw WASM +1 KB 程度 | 単純な extern "C" wrapper | 04 章 §関数定義 |
| Worklet | message dispatch 追加でも `process` 内 alloc ゼロ | switch case 追加のみ、init 時 view キャッシュ維持 | 06 章 F38 |
| UI | preset-store / LFO / ModWheel UI 追加で初期描画 +50 ms 程度 | runes ベース、$derived の局所化 | 05 章 §LfoSection / §PresetSelector |
| ビルド | WASM gzip 目標 15 KB / 警戒 18 KB / 撤退 30 KB（wasm-opt -O3 適用後の想定 ~13 KB） | binaryen を build スクリプトに統合 | 06 章 §性能目標、F39 |
