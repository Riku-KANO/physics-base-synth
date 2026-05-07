# 05. Phase 3 Web フロントエンド仕様

## 目的

Phase 1 [05 章](../2026-05-06-001-mvp/05-web-frontend-spec.md) と Phase 2 [05 章](../2026-05-07-002-phase2/05-web-frontend-spec.md) を起点に、Phase 3 で発生する **フロントエンド側の差分**（VoiceMeter / PolyphonyToggle コンポーネント新規追加、ParamSlider に Pick Position / Body Wet 対応、`WasmExports` interface に 3 関数追加、`messages.ts` の `midiCC` / `pitchBend` / `voiceState` variant 追加、SynthEngine の MIDI CC / Pitch Bend / Voice State メソッド追加、WebMIDI CC handler 新規追加）を確定する。Svelte 5 runes / SvelteKit 静的ビルド / esbuild Worklet バンドルの構成は Phase 1 / 2 から完全継承する。

## 他文書との関係

- 上流: [`02-architecture.md`](./02-architecture.md)（ParamDescriptor codegen 拡張、モノレポ構成変化）、[`04-wasm-audio-spec.md`](./04-wasm-audio-spec.md)（C ABI 既存 12 + 新 3 関数）
- 下流: [`06-build-and-verify.md`](./06-build-and-verify.md)（実行手順、F26〜F38 検証）
- 参考: Phase 1 / 2 [05 章]（SvelteKit セットアップ、SynthEngine、AudioWorkletProcessor、コンポーネント仕様、共有ステート、PCキーボード/Web MIDI 入力、パラメータスロットリング、本番ビルド）— **本書で明示的に変更しない部分はすべて Phase 1 / 2 の記述を継承**

## SvelteKit セットアップ

[Phase 1 05 章 §SvelteKit セットアップ](../2026-05-06-001-mvp/05-web-frontend-spec.md#sveltekit-セットアップ) を **完全維持**。Phase 3 でも `svelte.config.js` / `vite.config.ts` / `tsconfig.json` / `web/package.json` の変更なし。

## ファイルレイアウト（Phase 3 差分）

Phase 2 のレイアウトに **以下を追加・変更**:

```
web\src\lib\
├── audio\
│   ├── engine.ts                       # 既存、Phase 3 で sendMidiCc / sendPitchBend / receiveVoiceState 追加
│   ├── synth-processor.ts              # WasmExports に 3 関数追加、Voice State stride push
│   ├── messages.ts                     # ToWorkletMessage / FromWorkletMessage に variant 追加
│   ├── voice-state.svelte.ts           # 新規: $state で active mask + 振幅を保持
│   ├── wasm-loader.ts                  # 既存、Phase 1 / 2 維持
│   └── generated\
│       └── params.ts                   # 既存、Phase 3 で BODY_MODES_L/R / STEREO_SPREAD 追加
│
├── components\
│   ├── StartButton.svelte              # 既存、変更なし
│   ├── ParamSlider.svelte              # 既存、Pick Position / Body Wet 対応（既存ロジック流用）
│   ├── KeyboardView.svelte             # 既存、変更なし
│   ├── MidiSelect.svelte               # 既存、Phase 3 で WebMIDI CC handler を追加（あるいは別ファイルに切り出し）
│   ├── VoiceMeter.svelte               ← Phase 3 新規
│   └── PolyphonyToggle.svelte          ← Phase 3 新規
│
├── input\
│   ├── pc-keyboard.svelte.ts           # 既存、変更なし
│   └── midi-cc.ts                      ← Phase 3 新規 (WebMIDI CC parser)
│
└── state\
    └── ui.svelte.ts                    # 既存、Phase 3 で polyphonyMode を追加（D42）
```

`generated/` ディレクトリは Phase 2 から継続、Phase 3 で出力内容のみ拡張。

## ParamDescriptor 生成物の Phase 3 拡張

### `web/src/lib/audio/generated/params.ts` の Phase 3 出力例

Phase 2 の出力に **`PARAM_DESCRIPTORS` 配列の 5 件化** + **Body Mode 関連** + **Stereo Spread** を追加:

```typescript
// AUTO-GENERATED FROM params.json — DO NOT EDIT
// Run `pnpm gen:params` to regenerate.

export interface ParamDescriptor {
  readonly id: number;
  readonly name: string;
  readonly min: number;
  readonly max: number;
  readonly default: number;
  readonly smoothingTau: number;
}

export const PARAM_IDS = {
  Damping: 0,
  Brightness: 1,
  OutputGain: 2,
  PickPosition: 3,
  BodyWet: 4,
} as const;

export type ParamIdValue = (typeof PARAM_IDS)[keyof typeof PARAM_IDS];

export const PARAM_DESCRIPTORS: readonly ParamDescriptor[] = [
  { id: 0, name: 'Damping',      min: 0.90, max: 0.9999, default: 0.996, smoothingTau: 0.02 },
  { id: 1, name: 'Brightness',   min: 0.0,  max: 1.0,    default: 0.5,   smoothingTau: 0.02 },
  { id: 2, name: 'OutputGain',   min: 0.0,  max: 1.5,    default: 0.8,   smoothingTau: 0.01 },
  { id: 3, name: 'PickPosition', min: 0.05, max: 0.5,    default: 0.125, smoothingTau: 0.05 },
  { id: 4, name: 'BodyWet',      min: 0.0,  max: 1.0,    default: 0.5,   smoothingTau: 0.02 },
] as const;

export function getDescriptor(id: ParamIdValue): ParamDescriptor {
  return PARAM_DESCRIPTORS[id];
}

export function clampValue(id: ParamIdValue, value: number): number {
  const d = PARAM_DESCRIPTORS[id];
  return value < d.min ? d.min : value > d.max ? d.max : value;
}

// Phase 3 追加: Body Mode 構造（UI 直接利用は限定的、デバッグ / 将来のプリセット展開準備）
export interface BodyMode {
  readonly freq: number;
  readonly q: number;
  readonly gain: number;
}

export const STEREO_SPREAD = 0.05;

export const BODY_MODES_L: readonly BodyMode[] = [
  { freq: 105.0,  q: 30.0, gain: 1.0  },
  // ... 8 モード ...
];

export const BODY_MODES_R: readonly BodyMode[] = [
  // 各モード ±5% 揺らし
];
```

## `messages.ts` の Phase 3 変更点

```typescript
// web/src/lib/audio/messages.ts (Phase 3)

// PARAM_IDS / PARAM_DESCRIPTORS / 型は generated から re-export（Phase 2 と同じ）
export {
  PARAM_IDS,
  PARAM_DESCRIPTORS,
  getDescriptor,
  clampValue,
  STEREO_SPREAD,
  BODY_MODES_L,
  BODY_MODES_R,
  type ParamIdValue,
  type ParamDescriptor,
  type BodyMode,
} from './generated/params';

// ToWorkletMessage に Phase 3 variant を追加
export type ToWorkletMessage =
  | { type: 'init'; wasmBytes: ArrayBuffer; sampleRate: number }
  | { type: 'noteOn'; midi: number; velocity: number }
  | { type: 'noteOff'; midi: number }
  | { type: 'setParam'; id: number; value: number }
  | { type: 'setMode'; mode: 'poly' | 'mono' }
  | { type: 'midiCC'; cc: number; value: number }       // Phase 3 (D38)
  | { type: 'pitchBend'; semitones: number }            // Phase 3 (D38)
  | { type: 'reset' }
  | { type: 'dispose' };

// FromWorkletMessage に voiceState を追加
export type FromWorkletMessage =
  | { type: 'ready' }
  | { type: 'error'; message: string }
  | { type: 'voiceState'; activeMask: number; amplitudes: Float32Array };  // Phase 3 (D41)
```

> **互換性**: Phase 1 / 2 の既存 variant は完全維持。新規 variant 追加のみ。Worklet 側の switch dispatch も既存 case を変えず追加 case を加えるだけ。

## SynthEngine（main thread）の Phase 3 拡張

### 既存メソッド（Phase 1 / 2、不変）

```typescript
class SynthEngine {
  start(): Promise<void>;
  noteOn(midi: number, velocity: number): void;
  noteOff(midi: number): void;
  setParam(id: number, value: number): void;
  setMode(mode: 'poly' | 'mono'): void;
  reset(): void;
  dispose(): void;
}
```

### Phase 3 追加メソッド

```typescript
class SynthEngine {
  // 既存…

  /// MIDI CC を Worklet へ送信
  /// cc: MIDI CC番号、Phase 3 では (7, 64, 123) のみ受け付ける
  ///     CC#1 (Mod Wheel) は midi-cc.ts でフィルタ済、Phase 4 送り
  /// value: 0..127 (内部で /127 normalize)
  sendMidiCc(cc: number, value: number): void {
    if (!this.port) return;
    const msg: ToWorkletMessage = { type: 'midiCC', cc, value: value / 127 };
    this.port.postMessage(msg);
  }

  /// Pitch Bend を Worklet へ送信
  /// semitones: -2.0..+2.0
  sendPitchBend(semitones: number): void {
    if (!this.port) return;
    const clamped = Math.max(-2, Math.min(2, semitones));
    if (this._lastPitchBend === clamped) return;  // 連続値の重複送信回避
    this._lastPitchBend = clamped;
    const msg: ToWorkletMessage = { type: 'pitchBend', semitones: clamped };
    this.port.postMessage(msg);
  }

  /// Voice State 受信ハンドラ（init 時に登録）
  /// 受信した active mask と振幅配列を `voice-state.svelte.ts` の $state に反映
  private onVoiceState(msg: { activeMask: number; amplitudes: Float32Array }): void {
    voiceState.activeMask = msg.activeMask;
    voiceState.amplitudes = msg.amplitudes;
  }

  // private 変数
  private _lastPitchBend: number = 0;
}
```

### rAF スロットルとの関係

- Pitch Bend は連続値で頻度高い → **rAF スロットル適用**（既存 setParam と同じ仕組みで送信頻度を 60 Hz に制限）
- Channel Volume (CC#7) も連続値だが頻度は中程度 → スロットル適用検討（実装簡易性のため Phase 3 では即時送信、必要なら Phase 4 でスロットル）
- Sustain (CC#64) / All Notes Off (CC#123) は離散イベント → **スロットル適用せず即時送信**
- 連続値の前値一致時は送信スキップ（実装上のオプティマイゼーション）
- Mod Wheel (CC#1) は Phase 3 で受信せず（midi-cc.ts でフィルタ）、Phase 4 で再評価

## AudioWorkletProcessor (`synth-processor.ts`) の Phase 3 拡張

### `WasmExports` interface の Phase 3 拡張

```typescript
interface WasmExports {
  // Phase 1 既存（10 関数）
  memory: WebAssembly.Memory;
  synth_new(sample_rate: number, max_block_size: number): number;
  synth_free(handle: number): void;
  synth_note_on(handle: number, midi: number, velocity: number): void;
  synth_note_off(handle: number, midi: number): void;
  synth_set_param(handle: number, id: number, value: number): void;
  synth_reset(handle: number): void;
  synth_out_l_ptr(handle: number): number;
  synth_out_r_ptr(handle: number): number;
  synth_capacity(handle: number): number;
  synth_process_block(handle: number, frames: number): void;

  // Phase 2 追加
  synth_set_polyphony_mode(handle: number, mode: number): void;

  // Phase 3 追加（D38 / D41）
  synth_midi_cc(handle: number, cc: number, value_normalized: number): void;
  synth_pitch_bend(handle: number, semitones: number): void;
  synth_voice_state_ptr(handle: number): number;
}
```

### Voice State stride push（D41、リアルタイム制約遵守）

**リアルタイム境界の明示**: AudioWorkletProcessor の `process()` は audio render thread 上で動作するためリアルタイム制約が厳格。`new Float32Array(8)` などの動的確保は **constructor / `init` で事前確保**する。`postMessage` は 21 ms 周期に制限し、JS 側 alloc ゼロを維持する。

**ただし**: `postMessage` の structured clone コストは Chrome 実装次第で render thread 上に出る可能性があり、**Rust の F37 cargo timing test では計測できない**。したがって本仕様では:

1. **F37**（`cargo test --release` の `test_engine_process_block_timing`）= **Rust DSP 内部の timing のみ**を保証
2. **F38b**（Chrome DevTools Performance タブで `pnpm preview` 本番ビルドの "Audio Worklet" レーン実機計測）= **Worklet 全体の process 時間（postMessage 込み）**を保証

Phase 3 完成判定には **両方が必須**。F37 単独では Voice State push のコストを担保できない（[06 章 §F38b](./06-build-and-verify.md#f38bworklet-process-時間の実機計測phase-3-完成後必須) / [リスク R30](./06-build-and-verify.md#リスクと対策表)）。

```typescript
class SynthProcessor extends AudioWorkletProcessor {
  private frameCounter: number = 0;
  private static readonly VOICE_STATE_STRIDE = 1024;  // ≈ 21 ms @ 48 kHz
  private voiceStatePtr: number = 0;
  private voiceStateView: Uint8Array | null = null;
  private voiceStateDataView: DataView | null = null;
  // 事前確保: process() 内では new しない
  private readonly amplitudesScratch: Float32Array = new Float32Array(8);

  // ... constructor / init で synth_new / view 確保（Phase 1/2 既存）...

  init() {
    // ... Phase 1/2 既存処理: WASM ロード、synth_new、scratch_l/r view 確保 ...
    this.voiceStatePtr = this.exports.synth_voice_state_ptr(this.handle);
    this.refreshVoiceStateView();
  }

  private refreshVoiceStateView(): void {
    // 33 bytes = active mask 1 + 8 振幅 × 4
    this.voiceStateView = new Uint8Array(
      this.exports.memory.buffer,
      this.voiceStatePtr,
      33,
    );
    this.voiceStateDataView = new DataView(
      this.exports.memory.buffer,
      this.voiceStatePtr,
      33,
    );
  }

  process(inputs: Float32Array[][], outputs: Float32Array[][]): boolean {
    // Phase 1/2 既存処理: synth_process_block 呼び出し、scratch をコピー
    this.exports.synth_process_block(this.handle, 128);
    // ... output へコピー ...

    // Phase 3: Voice State stride push (事前確保スクラッチ使用、process 中 alloc ゼロ)
    this.frameCounter += 128;
    if (this.frameCounter >= SynthProcessor.VOICE_STATE_STRIDE) {
      this.frameCounter = 0;
      this.pushVoiceState();
    }

    return true;
  }

  private pushVoiceState(): void {
    if (!this.voiceStateView || !this.voiceStateDataView) return;
    // memory.buffer が grow したら view が detach される（D9 対策、Phase 3 では grow しない設計だが防御的）
    if (this.voiceStateView.byteLength === 0) {
      this.refreshVoiceStateView();
      if (!this.voiceStateView || !this.voiceStateDataView) return;
    }

    const activeMask = this.voiceStateView[0];
    // 事前確保した amplitudesScratch に書き込む（new しない）
    for (let i = 0; i < 8; i++) {
      this.amplitudesScratch[i] = this.voiceStateDataView.getFloat32(1 + i * 4, true);
    }

    // postMessage は structured clone でコピーが受信側で生成される（送信側は転送のみ）
    // 21 ms 周期、F37 の process 時間計測対象に含める
    this.port.postMessage({
      type: 'voiceState',
      activeMask,
      amplitudes: this.amplitudesScratch,
    } satisfies FromWorkletMessage);
  }
}
```

> **リアルタイム性の保証（限界明示）**:
> - `process()` 内で **JS 側 alloc ゼロ**: `amplitudesScratch` は constructor で確保、ループは `for` で in-place 書き込み（コードレビューで確認、F38b で実機検証必須）
> - WASM 側も `synth_new` で全バッファ確保済み（D4）
> - **`postMessage` は 21 ms 周期で 1 回のみ呼ぶ**（Float32Array 32 bytes + active mask 1 byte）。**ただし render thread 上で structured clone のコストが発生する可能性があり、F37 の Rust cargo timing test では計測できない**。コストは Chrome 実装次第（V8 の場合 typed array は内部 ArrayBuffer detach せず参照渡し的に振る舞うが保証なし）
> - **F37 (Rust DSP 内部 timing) は Worklet 全体の process 時間を測れない**: Step 13 の cargo timing test では JS 側 / postMessage コストは含まれない。本仕様では **F38b（Phase 3 完成後の Chrome DevTools Performance タブ実機計測）を必須化**（[06 章 §F37 補強と F38b](./06-build-and-verify.md#f38bworklet-process-時間の実機計測phase-3-完成後必須)）
> - **撤退判断**: 実機計測で Worklet `process()` 平均が 1.5 ms を超え、postMessage が主因と判明した場合は (a) Voice State push 頻度を 4096 サンプル毎（85 ms）に下げる、(b) Voice Meter を Phase 4 送り（UI から削除）、(c) `SharedArrayBuffer` 化（COOP/COEP ヘッダが必要、GitHub Pages 静的ホストでは不可なので別ホスティングへ移行が前提）のいずれかで対応
> - Phase 4 では SharedArrayBuffer + Atomics への移行を本格検討（Voice Meter polling 経路、postMessage 不要化）

### Message dispatch の Phase 3 拡張

```typescript
private onMessage(event: MessageEvent<ToWorkletMessage>): void {
  const msg = event.data;
  switch (msg.type) {
    // Phase 1 / 2 既存 case...

    case 'midiCC':
      this.exports.synth_midi_cc(this.handle, msg.cc, msg.value);
      break;

    case 'pitchBend':
      this.exports.synth_pitch_bend(this.handle, msg.semitones);
      break;

    // dispose / reset / etc.
  }
}
```

## Voice State の共有ステート (`voice-state.svelte.ts`)

```typescript
// web/src/lib/audio/voice-state.svelte.ts

class VoiceState {
  activeMask = $state(0);
  amplitudes = $state<Float32Array>(new Float32Array(8));

  isActive(voiceIndex: number): boolean {
    return (this.activeMask & (1 << voiceIndex)) !== 0;
  }

  amplitudeOf(voiceIndex: number): number {
    return this.amplitudes[voiceIndex] ?? 0;
  }
}

export const voiceState = new VoiceState();
```

`SynthEngine.onVoiceState` が `voiceState.activeMask` / `voiceState.amplitudes` を更新、Svelte の reactivity で `VoiceMeter.svelte` が自動再描画。

## VoiceMeter コンポーネント (`VoiceMeter.svelte`)

### 役割（D41）

8 セルの voice 表示。active なら不透明、inactive なら半透明。振幅で輝度を変調。

```svelte
<script lang="ts">
  import { voiceState } from '$lib/audio/voice-state.svelte';

  const voiceIndices = [0, 1, 2, 3, 4, 5, 6, 7];
</script>

<div class="voice-meter">
  <span class="label">Voices</span>
  <div class="cells">
    {#each voiceIndices as i}
      {@const active = voiceState.isActive(i)}
      {@const amp = voiceState.amplitudeOf(i)}
      <div
        class="cell"
        class:active
        style:--brightness="{Math.min(1, amp * 4)}"
      ></div>
    {/each}
  </div>
  <span class="count">{voiceState.activeMask.toString(2).split('').filter(c => c === '1').length} / 8</span>
</div>

<style>
  .voice-meter {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .cells {
    display: grid;
    grid-template-columns: repeat(8, 12px);
    gap: 2px;
  }
  .cell {
    width: 12px;
    height: 16px;
    background: hsl(120deg 30% calc(20% + var(--brightness, 0) * 50%));
    opacity: 0.3;
    transition: opacity 0.05s ease, background 0.05s ease;
  }
  .cell.active {
    opacity: calc(0.5 + var(--brightness, 0) * 0.5);
  }
  .label, .count {
    font-size: 0.85rem;
    color: var(--text-secondary);
  }
</style>
```

## PolyphonyToggle コンポーネント (`PolyphonyToggle.svelte`)

### 役割（D42）

mono / poly のラジオボタン。デフォルト poly。

```svelte
<script lang="ts">
  import { uiState } from '$lib/state/ui.svelte';
  import type { SynthEngine } from '$lib/audio/engine';

  let { engine }: { engine: SynthEngine } = $props();

  function setMode(mode: 'poly' | 'mono'): void {
    uiState.polyphonyMode = mode;
    engine.setMode(mode);
  }
</script>

<fieldset class="polyphony-toggle">
  <legend>Mode</legend>
  <label>
    <input
      type="radio"
      name="polyphony-mode"
      value="poly"
      checked={uiState.polyphonyMode === 'poly'}
      onchange={() => setMode('poly')}
    />
    Poly
  </label>
  <label>
    <input
      type="radio"
      name="polyphony-mode"
      value="mono"
      checked={uiState.polyphonyMode === 'mono'}
      onchange={() => setMode('mono')}
    />
    Mono
  </label>
</fieldset>

<style>
  .polyphony-toggle {
    display: flex;
    gap: 0.5rem;
    border: none;
    padding: 0;
  }
  .polyphony-toggle legend {
    font-size: 0.85rem;
    color: var(--text-secondary);
  }
</style>
```

`uiState.polyphonyMode` を `web/src/lib/state/ui.svelte.ts` に追加（Phase 1 既存ファイルに `polyphonyMode = $state<'poly' | 'mono'>('poly')` 1 行追加）。

## ParamSlider の Phase 3 拡張

ParamSlider 自体は **既存ロジックを完全流用**（Phase 2 で descriptor 駆動化済）。Phase 3 では `+page.svelte` で `<ParamSlider id={PARAM_IDS.PickPosition} />` / `<ParamSlider id={PARAM_IDS.BodyWet} />` を追加するだけ。

## WebMIDI CC handler (`midi-cc.ts`)

### 役割（D38）

Web MIDI API から CC バイトを parse して `SynthEngine.sendMidiCc` / `sendPitchBend` を呼ぶ。

```typescript
// web/src/lib/input/midi-cc.ts

import type { SynthEngine } from '$lib/audio/engine';

export function handleMidiMessage(message: MIDIMessageEvent, engine: SynthEngine): void {
  const data = message.data;
  if (!data || data.length < 3) return;

  const status = data[0] & 0xf0;
  const channel = data[0] & 0x0f;

  switch (status) {
    case 0x90: { // Note On
      const midi = data[1];
      const velocity = data[2] / 127;
      if (velocity > 0) {
        engine.noteOn(midi, velocity);
      } else {
        engine.noteOff(midi);  // velocity 0 = note off
      }
      break;
    }
    case 0x80: { // Note Off
      engine.noteOff(data[1]);
      break;
    }
    case 0xb0: { // Control Change
      const cc = data[1];
      const value = data[2];
      // CC#1 (Mod Wheel) は Phase 4 送り、Phase 3 では CC#7 / #64 / #123 のみ受け付ける
      if ([7, 64, 123].includes(cc)) {
        engine.sendMidiCc(cc, value);
      }
      break;
    }
    case 0xe0: { // Pitch Bend
      const lsb = data[1];
      const msb = data[2];
      const value14bit = (msb << 7) | lsb;          // 0..16383
      const normalized = (value14bit - 8192) / 8192; // -1..+1
      const semitones = normalized * 2;              // ±2 半音
      engine.sendPitchBend(semitones);
      break;
    }
  }
}
```

### `MidiSelect.svelte` での組み込み

```svelte
<script lang="ts">
  import { handleMidiMessage } from '$lib/input/midi-cc';
  // ...

  $effect(() => {
    if (!selectedInput) return;
    const handler = (e: MIDIMessageEvent) => handleMidiMessage(e, engine);
    selectedInput.onmidimessage = handler;
    return () => { selectedInput.onmidimessage = null; };
  });
</script>
```

> 既存の note on/off 処理を `handleMidiMessage` に統合する形（Phase 1 / 2 の MidiSelect.svelte 内ロジックを `midi-cc.ts` に切り出し）。

## `+page.svelte` のレイアウト変更

Phase 2 のレイアウトに **VoiceMeter / PolyphonyToggle を Header 直下に追加** + **ParamSlider に Pick Position / Body Wet 追加**:

```svelte
<header>
  <h1>Physics-Based Synth</h1>
  <div class="header-controls">
    <VoiceMeter />
    <PolyphonyToggle {engine} />
  </div>
</header>

<main>
  <StartButton {engine} />
  <MidiSelect {engine} />
  <KeyboardView {engine} />

  <section class="params">
    <ParamSlider id={PARAM_IDS.Damping} />
    <ParamSlider id={PARAM_IDS.Brightness} />
    <ParamSlider id={PARAM_IDS.OutputGain} />
    <ParamSlider id={PARAM_IDS.PickPosition} />   <!-- Phase 3 -->
    <ParamSlider id={PARAM_IDS.BodyWet} />        <!-- Phase 3 -->
  </section>
</main>
```

## dev-only `__synthDev`

Phase 2 の `import.meta.env.DEV` ガード経路は **継続維持**。`__synthDev.setMode()` は QA 用に残し、UI からの `setMode` 呼び出しと併用可能。本番ビルドでは tree-shake で完全除去（Phase 2 retrospective F22 と同等の検証を Phase 3 でも実施）。

## アクセシビリティ

Phase 3 では VoiceMeter / PolyphonyToggle が新規追加。最低限:

- VoiceMeter: 装飾要素、`aria-hidden="true"` または役割を `role="status" aria-live="polite"` で active 数のみ
- PolyphonyToggle: `<fieldset>` + `<legend>` + radio で標準 a11y 確保
- 詳細な ARIA ラベル / キーボードナビゲーションは Phase 4 以降

## 本番ビルド検証（Phase 3 版）

Phase 1 / 2 同様、本番ビルドで以下を確認:

1. `pnpm build` → `web/build/_app/immutable/` に WASM ハッシュ付きファイルが配置される
2. `voice-state.svelte.ts` / `VoiceMeter.svelte` / `PolyphonyToggle.svelte` / `midi-cc.ts` がすべてバンドルに含まれる（chunked にならない）
3. `__synthDev` が production bundle に 0 hits（grep で機械的検証、F22 拡張）
4. WASM gzip 12.9 KB 程度（target 30 KB の 43%、F36）
5. Worklet バンドルが < 10 KB 維持（Phase 2 5.04 KB + voice state push ロジック ≈ 5.5 KB 想定）

詳細は [`06-build-and-verify.md`](./06-build-and-verify.md)。
