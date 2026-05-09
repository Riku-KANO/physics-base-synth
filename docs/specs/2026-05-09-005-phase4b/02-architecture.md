# 02. Phase 4b アーキテクチャ

## 目的

Phase 1 / 2 / 3 / 4a で確立した 4 レイヤ構成（Svelte UI → AudioWorkletProcessor → wasm-audio → dsp-core）に対し、Phase 4b で追加する責務（Stretching all-pass cascade / Hammer model / Piano Modal Body / `__synthDev` 計測自動化 / `.gitattributes` LF 統一）の配置を明確化する。Phase 4a までの構成は崩さず、新規責務をどのレイヤに置くかと既存コンポーネントの拡張点を定義する。

## 他文書との関係

- 上流: [`01-overview.md`](./01-overview.md)（Phase 4b の確定事項 / D56-D67 / 完成像）
- 下流: 本書のレイヤ責務に従って 03〜05 章で具体的な API / モジュール / コンポーネントを定義
- 並行: Phase 1 / 2 / 3 / 4a [`02-architecture.md`] — 同じ章番号 / 構造、Phase 4b は差分のみ記述

## 4 レイヤ構成（Phase 4a から不変）

```
Svelte UI (main thread) ──MessagePort──▶ AudioWorkletProcessor
                                              │ FFI (C ABI, raw exports)
                                              ▼
                                         wasm-audio (cdylib)
                                              │
                                              ▼
                                         dsp-core (rlib)
```

各レイヤの責務と禁止事項は CLAUDE.md と Phase 1 / 2 / 3 / 4a の 02 章で定義済み。Phase 4b でも変更しない。

## Phase 4b で追加される責務の配置

### Svelte UI 層（main thread）

| 責務 | 配置 | 既存パターン参照 |
|---|---|---|
| **Piano Factory Preset 1 件追加** | `web/src/lib/state/factory-presets.ts` の const 配列に Piano エントリを 8 番目として追加 | Phase 4a 7 種定義の延長、JSON 形式は `PresetV1` で一致 |
| **`PresetSelector` での Piano 表示** | 既存 `PresetSelector.svelte` のドロップダウンに自動的に Piano が追加される（`presetStore.factoryPresets` を反復、新規コード不要） | Phase 4a の Factory ループ反復が Piano エントリを自然に取り込む |
| **InstrumentKindKey に `'piano'` 追加** | `web/src/lib/state/preset-schema.ts` の `InstrumentKindKey` 型 + `VALID_INSTRUMENTS` 配列に `'piano'` を追記、`gen-params.mjs` の TS 出力にも `'piano'` を含める | `InstrumentKindKey` の Phase 4a 既存 7 値 (`'default'` / `'guitar_classical'` / `'ukulele'` / `'mandolin'` / `'bass'` / `'guitar_steel'` / `'sitar'`) と同形式 |
| **`__synthDev.measureProcessTime` API** | `web/src/lib/audio/__synthDev.ts`（新規 dev-only モジュール、`import.meta.env.DEV` でガード） | Phase 3 既存 `__synthDev` の dev API パターンを継承 |
| **`__synthDev` 公開** | `web/src/lib/state/synth.svelte.ts` で既存の `__synthDev` 公開経路に `measureProcessTime` を追加 | Phase 4a 既存 `if (import.meta.env.DEV) { ... }` の dev-only export パターン |

### AudioWorkletProcessor 層（音声スレッド）

| 責務 | 配置 | 既存パターン参照 |
|---|---|---|
| **`process` 開始/終了の self time 記録** | `synth-processor.ts` の `process(inputs, outputs)` メソッド冒頭/末尾で **`performance.now()` を記録し差分 (ms) を計測**（dev only、`if (DEV_MODE)` ガード）。指摘事項 #1 反映: 当初 `currentFrame` ベースを案としていたが callback 内で進まないため self time 計測には使えず、`performance.now()` 方式に変更 | Phase 3 / 4a までは未実装、Phase 4b D66 で初出 |
| **Timing message 集約** | Worklet 側でリングバッファ（`Float32Array(4096)` の 1 度確保、48kHz/128 frames で 375 quanta/sec、約 10.92 秒分）に self time を蓄積、**`stopTimingCapture` 受信時にまとめて時系列順に `port.postMessage({type: 'timing', samples, bufferOverflow})` で main へ送信**（一括送信、`startTimingCapture` 〜 `stopTimingCapture` の区間 1 回） | Phase 3 D41 の Voice State buffer の stride push とは異なる「区間終端で一括送信」パターン |
| **WasmExports interface** | 変更なし（Phase 4a の 18 関数のまま） | — |
| **Voice State stride push** | 変更なし（Phase 3 D41 の 1024 sample stride 維持） | — |
| **WASM ロード** | 変更なし（`WebAssembly.instantiate(bytes, { env: {} })`、Phase 4a `wasm-opt -O3` 適用済バイナリを読む） | — |

**禁止事項**: `process()` 内での新規 alloc は Phase 1〜4a と同じく禁止。`__synthDev` 計測は dev only で `if (DEV_MODE)` の compile-time const ガード（`import.meta.env.DEV` を tree-shake 可能な形で使う）、production build では完全に削除される。

### wasm-audio 層（C ABI 境界）

| 責務 | 配置 | 既存パターン参照 |
|---|---|---|
| **C ABI 関数追加なし** | `crates/wasm-audio/src/lib.rs` は **Phase 4a の 18 関数を完全維持**、新規追加なし | — |
| **`synth_apply_instrument` の値域拡張** | 既存関数のシグネチャ・動作完全維持、内部の `InstrumentKind::from_u32(7)` で `Some(Piano)` を受けるのみ。`from_u32(8 以上)` は `None` で no-op（既存防御的設計） | Phase 4a 既存パターン継承 |
| **ポインタ deref 安全性** | 既存 `unsafe { &mut *handle }` パターンを継承、`#![allow(clippy::not_unsafe_ptr_arg_deref)]` も継続 | — |
| **`SynthHandle` struct** | 変更なし（`Engine` を保持、scratch_l/r も Phase 1 から継承） | — |

### dsp-core 層（純粋 DSP）

| 責務 | 配置 | 既存パターン参照 |
|---|---|---|
| **`Dispersion` module** | `crates/dsp-core/src/dispersion.rs`（新規） | `lfo.rs` / `loss_filter.rs` / `modal_body.rs` と同じく単一責務モジュール、`#[inline(always)] pub fn process_sample()` パターン |
| **Closed-form 係数算出関数** | `dispersion.rs` 内の `pub fn compute_dispersion_a1(m: u32, b: f32, f0: f32, fs: f32) -> (f32, f32)`（a1 + group_delay_per_stage を返す） | Faust `piano_dispersion_filter` 由来の Rust 移植（pre-research §4.2）、純粋関数（state なし） |
| **`KarplusStrong` 拡張** | `crates/dsp-core/src/karplus_strong.rs` に `dispersion_stages: [DispersionStage; 8]` + `dispersion_active: bool` フィールド追加、`note_on` で a1 算出、`process_sample` で cascade 適用、`note_on_internal` で hammer 経路分岐 | Phase 4a で `lfo_pitch_factor` / `lfo_brightness_offset` を追加した経緯と同形式 |
| **Hammer impulse + LPF** | `karplus_strong.rs::note_on_internal` の buffer 初期化分岐内 | Phase 1 D34 の pluck excitation を Phase 4b で「楽器に応じて切り替える」拡張 |
| **`InstrumentKind::Piano = 7`** | `crates/dsp-core/src/params.rs`（生成ファイル、`gen-params.mjs` 拡張で出力） | Phase 4a 既存 7 値（Default〜Sitar）の延長 |
| **Piano Modal 係数 + Piano 専用フィールド** | `params.rs` に `BODY_MODES_PIANO_L/R` / `STEREO_SPREAD_PIANO` / `INHARMONICITY_B_PIANO` / `HAMMER_CUTOFF_LOW_PIANO` / `HAMMER_CUTOFF_HIGH_PIANO` を追加（gen-params.mjs 拡張で出力） | Phase 4a の楽器ごとの const 出力パターン |
| **`Engine::apply_instrument` の `set_dispersion_active` 呼出** | `crates/dsp-core/src/engine.rs::apply_instrument` 末尾に `let active = matches!(kind, Piano); self.pool.set_dispersion_active(active);` を追加（Phase 4a の即時 release を継承、fade-out なし。当初提案した 5 ms fade-out は SmoothedValue の同期 set_target で実現できないため Phase 4c 送り、指摘事項 #3 反映） | Phase 4a 既存 `apply_instrument` の末尾 1 行追加のみ |
| **Voice / VoicePool への dispersion_active fan-out** | `voice_pool.rs` / `voice.rs` / `traits.rs` に `set_dispersion_active(bool)` を委譲（VoicePool 経由で全 voice に伝播、`apply_instrument` 時に呼ばれる） | Phase 4a の `set_lfo_pitch_factor` / `set_lfo_brightness_offset` と同パターン |

**禁止事項**: `process` ホットパス中のヒープ確保ゼロ（Phase 1 D4 維持）。`dispersion_stages` は `[DispersionStage; 8]` の inline 配列で固定 size、`apply_instrument` 時の `dispersion_active` 切替も bool flag のみで heap 操作なし。Hammer LPF 計算は `note_on` 時の buffer 初期化のみで `process_sample` には影響しない（`buffer` 自体は Phase 1 から `prepare` で確保した固定領域）。

## ビルドパイプラインの差分

### Phase 4a までのパイプライン（Phase 4b でも継続）

```
params.json (単一ソース、Phase 4a で 7 楽器 + LFO ParamId)
   │
   │ scripts/gen-params.mjs (BODY_MODES_<INSTRUMENT>_L/R / InstrumentKind enum 出力)
   ▼
crates/dsp-core/src/params.rs (生成)
web/src/lib/audio/generated/params.ts (生成)
   │
   │ scripts/check-params-sync.mjs (CI 検証)
   ▼
PR diff で drift 検知

cargo build --target wasm32-unknown-unknown --release
   │
   │ scripts/copy-wasm.mjs (wasm-opt -O3 --strip-debug)
   ▼
web/static/wasm-audio.wasm (Phase 4a 実測 18.42 KB gzip)
   │
   │ scripts/check-wasm-exports.mjs (REQUIRED 配列 18 C ABI 関数 + memory export = 19 entry)
   ▼
PR で export 名 drift 検知
```

### Phase 4b でのパイプライン拡張（最小限）

```
params.json (Phase 4b で Piano エントリ 1 件追加 + Piano 専用フィールド 3 件)
   │
   │ scripts/gen-params.mjs (拡張: Piano 専用フィールドを optional として処理、
   │                          BODY_MODES_PIANO_L/R / STEREO_SPREAD_PIANO /
   │                          INHARMONICITY_B_PIANO / HAMMER_CUTOFF_LOW_PIANO /
   │                          HAMMER_CUTOFF_HIGH_PIANO を出力、
   │                          InstrumentKind::Piano = 7 を enum に追加)
   ▼
crates/dsp-core/src/params.rs (Phase 4b 拡張)
web/src/lib/audio/generated/params.ts (Phase 4b で 'piano' を InstrumentKindKey に追加)
   │
   │ scripts/check-params-sync.mjs (Phase 4a 既存、Piano エントリ追加で自動同期)
   ▼
PR diff で drift 検知

cargo build --target wasm32-unknown-unknown --release
   │
   │ scripts/copy-wasm.mjs (wasm-opt -O3、Phase 4a 既存)
   ▼
web/static/wasm-audio.wasm (~19 KB gzip 想定)
   │
   │ scripts/check-wasm-exports.mjs (REQUIRED 配列、Phase 4a と同じ 19 entry、Phase 4b で変更なし)
   ▼
PR で export 名 drift 検知
```

### `gen-params.mjs` の Phase 4b 拡張（D62）

`params.json` の `instruments` 配列が Piano 楽器を 8 番目として持つ。Piano 専用フィールド (`inharmonicity_b` / `hammer_cutoff_low_hz` / `hammer_cutoff_high_hz`) は **`gen-params.mjs` で楽器ごとに optional 扱い** とし、Default 〜 Sitar (0-6) では `null` or 省略、Piano (7) のみ値を持つ:

```javascript
// scripts/gen-params.mjs (Phase 4b 拡張)
function validateInstruments(instruments) {
  // ...Phase 4a 既存検証...
  for (const ins of instruments) {
    // Phase 4b 追加: Piano 専用フィールドは optional だが、kind === 'Piano' のときは必須
    if (ins.kind === 'Piano') {
      if (typeof ins.inharmonicity_b !== 'number' || ins.inharmonicity_b <= 0) {
        throw new Error(`Piano kind must have inharmonicity_b > 0`);
      }
      if (typeof ins.hammer_cutoff_low_hz !== 'number' || ins.hammer_cutoff_low_hz <= 0) {
        throw new Error(`Piano kind must have hammer_cutoff_low_hz > 0`);
      }
      if (typeof ins.hammer_cutoff_high_hz !== 'number' ||
          ins.hammer_cutoff_high_hz <= ins.hammer_cutoff_low_hz) {
        throw new Error(`Piano kind must have hammer_cutoff_high_hz > hammer_cutoff_low_hz`);
      }
    }
  }
}

function generateRustSource(paramsJson) {
  // ...Phase 4a 既存生成...

  // Phase 4b 追加: Piano 専用 const
  const piano = paramsJson.instruments.find(i => i.kind === 'Piano');
  if (piano) {
    lines.push(`pub const INHARMONICITY_B_PIANO: f32 = ${formatF32(piano.inharmonicity_b)};`);
    lines.push(`pub const HAMMER_CUTOFF_LOW_PIANO: f32 = ${formatF32(piano.hammer_cutoff_low_hz)};`);
    lines.push(`pub const HAMMER_CUTOFF_HIGH_PIANO: f32 = ${formatF32(piano.hammer_cutoff_high_hz)};`);
  }

  // 既存パターンと同形式で BODY_MODES_PIANO_L/R 生成
}
```

**重要**: Piano 専用フィールドは Phase 4b では Piano kind の 1 楽器のみが持つ。将来 Phase 4c で Grand / Upright を追加する際は、各楽器が独自値を持つ設計に変更（Phase 4c で扱う）。

## メッセージプロトコル拡張

### `ToWorkletMessage`（main → Worklet）

Phase 4a 既存:
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
  | { type: 'dispose' }
  | { type: 'lfoSetRate'; hz: number }
  | { type: 'lfoSetWaveform'; kind: LfoWaveformKey }
  | { type: 'lfoSetDepth'; dest: LfoDestinationKey; depth: number }
  | { type: 'applyInstrument'; kind: InstrumentKindKey };
```

Phase 4b で追加:
```typescript
type ToWorkletMessage =
  | ...既存...
  // Phase 4b D66: F38b 計測自動化スクリプト用 (dev only)
  | { type: 'startTimingCapture' }
  | { type: 'stopTimingCapture' };
```

`InstrumentKindKey` の値域を拡張:
```typescript
export type InstrumentKindKey =
  | 'default'
  | 'guitar_classical'
  | 'ukulele'
  | 'mandolin'
  | 'bass'
  | 'guitar_steel'
  | 'sitar'
  | 'piano';        // ← Phase 4b 追加
```

### `FromWorkletMessage`（Worklet → main）

Phase 4a 既存:
```typescript
type FromWorkletMessage =
  | { type: 'ready' }
  | { type: 'error'; message: string }
  | { type: 'debug'; message: string }
  | { type: 'voiceState'; mask: number; amplitudes: number[] };
```

Phase 4b で追加（D66）:
```typescript
type FromWorkletMessage =
  | ...既存...
  // Phase 4b D66: F38b 計測値の集約 (dev only)
  | { type: 'timing'; samples: number[]; bufferOverflow: boolean };
```

`samples` は `process` 1 回ごとの self time（ms）の配列、`bufferOverflow` はリングバッファが満杯で古いデータが上書きされたかのフラグ。

## プリセット適用フロー（Phase 4a から不変、Piano エントリが追加されるだけ）

```
ユーザー操作: PresetSelector で "Piano" を選択
   ▼
PresetSelector.svelte: presetStore.apply('Piano', engine)
   ▼
preset-store.svelte.ts: apply(name, engine):
   const preset = this.findByName('Piano');  // factory_presets.ts の Piano エントリ
   engine.applyPreset(preset);
   ▼
SynthEngine.applyPreset(preset):
   1. engine.applyInstrument('piano')
        → MessagePort: { type: 'applyInstrument', kind: 'piano' }
        → Worklet: synth_apply_instrument(handle, 7)  // Piano = 7
        → dsp-core::Engine::apply_instrument(InstrumentKind::Piano):
             - pool.all_notes_off()                      ← 既存 Phase 4a D53 (即時 release)
             - hold_stack.clear()
             - sustain_state.reset()
             - current_instrument = Piano
             - stereo_spread = STEREO_SPREAD_PIANO
             - modal_body.set_instrument(Piano, sample_rate)
             - pool.set_dispersion_active(true)         ← Phase 4b 追加 (D67)
   2-11. Phase 4a 既存（params / lfo の applyPreset 経路）
```

**重要（D63 改訂後）**: 当初提案した 5 ms fade-out は **`SmoothedValue::set_target` が target 代入のみで current は `next_sample()` でしか進まず、同期メソッド内で `set_target(0.0)` → `set_target(prev_value)` を実行しても fade-out は発生しない**ため撤回（指摘事項 #3）。Phase 4b では Phase 4a D53「即時 `pool.all_notes_off()`」を完全継承し、`apply_instrument` 末尾に `pool.set_dispersion_active(piano)` の 1 行を追加するのみ。pop noise 軽減（fade-out / cross-fade）は Phase 4c 以降で `PendingInstrumentChange` 状態機械を用いた本実装として再評価する。

## 楽器切替フロー（プリセット選択の subset、Phase 4b 拡張版）

```
ユーザー操作: PresetSelector または InstrumentPicker で楽器変更
   ▼
engine.applyInstrument('piano')
   ▼
MessagePort: { type: 'applyInstrument', kind: 'piano' }
   ▼
Worklet: synth_apply_instrument(handle, 7)  // 7 = Piano
   ▼
dsp-core::Engine::apply_instrument(InstrumentKind::Piano):
   1. self.pool.all_notes_off()                               ← 既存 Phase 4a
   2. self.hold_stack.clear()
   3. self.sustain_state.reset()
   4. self.current_instrument = Piano
   5. self.stereo_spread = STEREO_SPREAD_PIANO
   6. self.modal_body.set_instrument(Piano, sample_rate)
   7. self.pool.set_dispersion_active(true)                   ← Phase 4b 追加 (D67)
```

**`pool.all_notes_off()` の理由**: Modal 係数差し替えと voice 内の Karplus-Strong は独立だが、楽器が変わった瞬間に音色が急変するため、UI 体験として全 voice release が自然。Phase 4b では Phase 4a D53 を継承（即時 release）。pop noise 軽減（fade-out / cross-fade）は Phase 4c 送り。

**`set_dispersion_active(true/false)` の役割**: Piano kind では dispersion cascade を `process_sample` で適用（`true`）、他の 7 楽器では `false` で skip。VoicePool を経由して全 8 voice に fan-out。`matches!(kind, InstrumentKind::Piano)` で楽器に応じて自動切替、漏れなし。

## Dispersion + Hammer 配置フロー（per sample / per note_on）

### `KarplusStrong::note_on`（per note）

```
note_on(midi_note, freq_hz, velocity):
  ┌─ Phase 1〜4a 既存 ──────────────────────────────────┐
  │ raw_len = sample_rate / freq_hz                      │
  │ brightness_tau_g = (1 - b) / b                       │
  └─────────────────────────────────────────────────────┘
  ┌─ Phase 4b D60 拡張 ──────────────────────────────────┐
  │ if dispersion_active:                                  │
  │   (a1, gd_per_stage) = compute_dispersion_a1(          │
  │       M=8, B=INHARMONICITY_B_PIANO, freq_hz, sr)        │
  │   dispersion_tau_g = M * gd_per_stage                  │
  │   for stage in dispersion_stages:                      │
  │       stage.a1 = a1                                    │
  │       stage.z1_in = 0.0                                │
  │       stage.z1_out = 0.0                               │
  │ else:                                                   │
  │   dispersion_tau_g = 0.0                               │
  │                                                         │
  │ adjusted = raw_len - brightness_tau_g - dispersion_tau_g│
  │ length_int = adjusted.floor() ...                      │
  └─────────────────────────────────────────────────────┘
  ┌─ Phase 4b D61 拡張 ──────────────────────────────────┐
  │ if dispersion_active:                                  │
  │   buffer[0] = velocity              # 単位 impulse     │
  │   for i in 1..len_int: buffer[i] = 0.0                 │
  │   cutoff = lerp(800Hz, 4000Hz, velocity)               │
  │   alpha = 1 - exp(-2π * cutoff / sample_rate)          │
  │   z = 0.0                                              │
  │   for i in 0..len_int:                                 │
  │     z = alpha * buffer[i] + (1 - alpha) * z            │
  │     buffer[i] = z                                       │
  │ else:                                                   │
  │   # Phase 1〜4a 既存 pluck excitation                   │
  │   for i in 0..len_int: buffer[i] = rng.next_unit_bipolar() * velocity│
  │   k = (pick_position * len_int).round()                │
  │   for i in (k..len_int).rev():                         │
  │     buffer[i] -= buffer[i - k]                         │
  └─────────────────────────────────────────────────────┘
  ┌─ Phase 1〜4a 既存 ──────────────────────────────────┐
  │ thiran.set_fractional(len_frac)                        │
  │ thiran.reset()                                          │
  │ loss_filter.set_for_frequency(freq_hz)                 │
  │ active = true; energy = velocity^2; ...                │
  └─────────────────────────────────────────────────────┘
```

### `KarplusStrong::process_sample`（per sample）

```
process_sample():
  ┌─ Phase 1〜4a 既存 ──────────────────────────────────┐
  │ if !active: return 0.0                                 │
  │ effective_length = length_target.next_sample()        │
  │                    * lfo_pitch_factor                  │
  │ if diff > 1e-5: re-clamp length_int / fractional      │
  │ read_z = (write_index + buf_len - length_int) % buf_len│
  └─────────────────────────────────────────────────────┘
  ┌─ Phase 4b D60 拡張 ──────────────────────────────────┐
  │ x = buffer[read_z]                                     │
  │ if dispersion_active:                                  │
  │   for stage in dispersion_stages:                      │
  │     y = stage.a1 * x + stage.z1_in - stage.a1 * stage.z1_out│
  │     stage.z1_in = x                                     │
  │     stage.z1_out = y                                    │
  │     x = y                                                │
  │ read_value = thiran.process(x)                         │
  └─────────────────────────────────────────────────────┘
  ┌─ Phase 1〜4a 既存（変更なし）──────────────────────┐
  │ b = (brightness.next_sample() + lfo_brightness_offset).clamp(0, 1)│
  │ filtered = b * read_value + (1 - b) * last_filter_out  │
  │ last_filter_out = filtered                              │
  │ loss_out = loss_filter.process_sample(filtered)        │
  │ d = damping.next_sample()                               │
  │ damped = d * loss_out + 1e-25 - 1e-25                  │
  │ buffer[write_index] = damped                            │
  │ write_index = (write_index + 1) % buf_len              │
  │ energy update; age_samples += 1                         │
  │ return read_value                                       │
  └─────────────────────────────────────────────────────┘
```

**重要（D60 順序）**: dispersion cascade は **`buffer[read_z]` の値を 8 段に通してから Thiran allpass に渡す**。これにより Thiran の出力が分散済の信号を反映、後段の Brightness LPF / LossFilter / damping は既存の Phase 4a 経路と同一。**Phase 4a と Phase 4b の違いは、dispersion_active = true のときに read 値が 8 段の allpass を経由することのみ**。

## ファイル変更リスト

### 新規作成

#### dsp-core
- `crates/dsp-core/src/dispersion.rs` — Stretching all-pass cascade（D57 / D59）

#### web
- `web/src/lib/audio/__synthDev.ts` — F38b 計測自動化スクリプト（D66、dev only）

#### リポジトリ root
- `.gitattributes` — 改行 LF 統一（D65）

### 主要変更

#### dsp-core
- `crates/dsp-core/src/lib.rs` — `pub mod dispersion;` + `pub use dispersion::{...};` 追加
- `crates/dsp-core/src/karplus_strong.rs` — `dispersion_stages: [DispersionStage; 8]` + `dispersion_active: bool` フィールド追加、`note_on_internal` で hammer 経路分岐 + dispersion 係数算出、`process_sample` で cascade 適用、`adjusted_length` 補正に `M·polydel(a1)` 追加
- `crates/dsp-core/src/engine.rs` — `apply_instrument` 末尾に `pool.set_dispersion_active(piano)` の 1 行追加（D67）、`current_instrument` の Default reset 経路でも `set_dispersion_active(false)` を呼ぶ。当初の D63 で 5 ms fade-out を提案していたが、SmoothedValue 同期 set_target の実現不能性により撤回し Phase 4a の即時 release を継承（指摘事項 #3）
- `crates/dsp-core/src/voice_pool.rs` — `set_dispersion_active(active)` 追加（全 voice fan-out）
- `crates/dsp-core/src/voice.rs` — `set_dispersion_active` 委譲追加
- `crates/dsp-core/src/traits.rs` — `Voice` trait に `set_dispersion_active(bool)` 追加
- `crates/dsp-core/src/params.rs`（生成ファイル）— `InstrumentKind::Piano = 7` 追加、`BODY_MODES_PIANO_L/R` / `STEREO_SPREAD_PIANO` / `INHARMONICITY_B_PIANO` / `HAMMER_CUTOFF_LOW_PIANO` / `HAMMER_CUTOFF_HIGH_PIANO` 追加、`body_modes_for_instrument` / `stereo_spread_for_instrument` の Piano 分岐追加

#### params.json + scripts
- `params.json` — `instruments` 配列に Piano エントリを 8 番目として追加（`inharmonicity_b` / `hammer_cutoff_low_hz` / `hammer_cutoff_high_hz` 専用フィールド）
- `scripts/gen-params.mjs` — Piano 専用フィールド処理（optional 扱い、Piano kind のみ必須）+ `INHARMONICITY_B_PIANO` 等の const 出力 + `InstrumentKind::Piano = 7` enum 出力
- `scripts/check-params-sync.mjs` — Phase 4a 既存、Piano エントリ追加で自動同期

#### web
- `web/src/lib/audio/messages.ts` — `InstrumentKindKey` に `'piano'` 追加、`startTimingCapture` / `stopTimingCapture` variant 追加（dev only）
- `web/src/lib/audio/synth-processor.ts` — `INSTRUMENT_KIND_MAP['piano'] = 7` 追加、`startTimingCapture` / `stopTimingCapture` ハンドラ追加（`if (DEV_MODE)` ガード、`DEV_MODE` は `declare const DEV_MODE: boolean;` + `--define:DEV_MODE=true/false` で置換、指摘事項 #2 反映）、`process` 開始/終了の **`performance.now()` 差分**で self time 記録（指摘事項 #1 反映、`currentFrame` は callback 内で進まないため使わない）+ リングバッファ `Float32Array(4096)` (約 10.92 秒分) 集約 + `port.postMessage({type: 'timing'})` （dev only）
- `web/src/lib/state/preset-schema.ts` — `InstrumentKindKey` に `'piano'` 追加、`VALID_INSTRUMENTS` に `'piano'` 追加
- `web/src/lib/state/factory-presets.ts` — Piano エントリ 1 件追加（合計 8 種）
- `web/src/lib/audio/engine.ts` — Phase 4a 既存、変更最小（`InstrumentKindKey` 拡張で型エラー解消のみ）
- `web/src/lib/state/synth.svelte.ts` — `__synthDev` の dev export に `measureProcessTime` 経路追加

### 軽微な更新

- `README.md` — Phase 4b 機能追記、Piano プリセット使用方法、F38b 自動計測の使い方
- `CLAUDE.md` — 「現在のイテレーション」を Phase 4b へ更新、Phase 4c 予告追記
- `docs/retrospective/2026-05-08-004-phase4a.md` §5 — `wasm-opt --print-stats` 内訳記録の追記

## レイヤ間の依存方向（Phase 4a から不変）

```
Svelte UI ──depends on──▶ Worklet messages.ts (型のみ)
Worklet ──depends on──▶ wasm-audio (FFI 経由、TS インポートなし)
wasm-audio ──depends on──▶ dsp-core (rlib 依存)
dsp-core ──depends on──▶ なし（依存ゼロ、Phase 1-4a 制約継承）
```

Phase 4b でも依存方向を逆転させない。`__synthDev.ts` は `synth.svelte.ts` から呼ばれるが、`synth.svelte.ts` は `__synthDev.ts` を `if (import.meta.env.DEV)` ガードで dynamic import して逆参照しない。

## 設計判断の章間相互参照

- D56 (ピアノ音色 = Phase 4b 主目的) → 03 章 §dispersion / §karplus_strong / §engine、05 章 §factory-presets
- D57 (M = 8 段) → 03 章 §dispersion::DispersionStage / §KarplusStrong::dispersion_stages
- D58 (Faust 方式 B 表現) → 03 章 §compute_dispersion_a1 の `Ikey(f0)` 項
- D59 (Closed-form 係数式) → 03 章 §compute_dispersion_a1 全体、`#[allow(clippy::approx_constant)]` 適用
- D60 (dispersion → Thiran 順序 + 群遅延補正) → 03 章 §process_sample / §note_on の `adjusted_length`
- D61 (Hammer = Commuted impulse + LPF) → 03 章 §note_on_internal の buffer 初期化分岐
- D62 (Piano Modal 係数) → 03 章 §params.rs の `BODY_MODES_PIANO_L/R`、05 章 §preset-schema.ts
- D63 (改訂後: Phase 4a D53 を継承、即時 release) → 03 章 §Engine::apply_instrument。当初提案の 5 ms fade-out は撤回（指摘事項 #3）
- D64 (新規 ParamId / C ABI なし) → 04 章 §C ABI、05 章 §messages.ts
- D65 (`.gitattributes` LF 統一) → 06 章 §F49 / §トラブルシューティング
- D66 (F38b 自動計測) → 05 章 §__synthDev.ts、06 章 §F48
- D67 (Phase 4a 互換性バイト一致) → 03 章 §テスト方針、06 章 §F47b 拡張

## 性能目標の整理（03〜06 章への伝達）

| レイヤ | 目標 | 達成方式 | 検証 |
|---|---|---|---|
| dsp-core | Piano 演奏時 dispersion cascade で `process` per sample +32 演算（M=8 × 4 演算） / 8 voice | `[DispersionStage; 8]` inline 配列 + `#[inline(always)]` setter、Phase 4a の他楽器では skip | 03 章 §process_sample 計測、06 章 F50 release timing test |
| dsp-core | Hammer LPF は note_on 時のみ計算（per process 影響ゼロ） | `note_on_internal` の buffer 初期化分岐内で完結 | 03 章 §note_on_internal、F50 |
| wasm-audio | C ABI 関数追加なし、raw WASM +0 KB | `synth_apply_instrument` の値域拡張のみ（既存関数の内部分岐） | 04 章 §関数定義、F48 |
| Worklet | dev-only timing 集約で `process` 内 alloc ゼロ維持（リングバッファは init 時 1 度確保） | `if (DEV_MODE)` ガードで production 削除、リングバッファ `Float32Array(4096)`（48kHz/128 frames で 375 quanta/sec、約 10.92 秒分）を constructor 内 1 回確保 | 06 章 F49（dev_mode のみ） |
| UI | Piano プリセット追加で初期描画 +5 ms 程度 | runes ベース、`PresetSelector.svelte` のループ反復に Piano エントリが自然に取り込まれる | 05 章 §factory-presets |
| ビルド | WASM gzip 目標 < 22 KB / 警戒 25 KB / 撤退 30 KB（Phase 4a 18.42 KB + Phase 4b 純増 0.6 KB = 19 KB 想定） | Piano 楽器係数 + dispersion cascade コード + Hammer LPF コードの増加分 | 06 章 §性能目標、F49 |
