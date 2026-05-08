# 05. Web フロントエンド仕様（Phase 4a）

## 目的

Phase 4a で追加・変更する Svelte コンポーネント、TypeScript モジュール、AudioWorklet 拡張を定義する。Phase 1 / 2 / 3 で確立した既存コンポーネント（`StartButton` / `Keyboard` / `MidiSelect` / `ParamSlider` / `VoiceMeter` / `PolyphonyToggle`）と既存ステート（`synth.svelte.ts` / `ui.svelte.ts` / `voice-state.svelte.ts`）はすべて維持し、本書では **Phase 4a で追加・変更する箇所のみ** 記述する。

## 他文書との関係

- 上流: [`pre-research.md`](./pre-research.md)（§3 LFO / §4 Mod Wheel / §5 プリセット / §6 localStorage / §7 多楽器）、[`01-overview.md`](./01-overview.md)（D44-D55）、[`02-architecture.md`](./02-architecture.md)（UI 層責務）、[`03-dsp-core-spec.md`](./03-dsp-core-spec.md)（dsp-core API / InstrumentKind / LfoWaveform / LfoDestination）、[`04-wasm-audio-spec.md`](./04-wasm-audio-spec.md)（C ABI 4 関数）
- 下流: [`07-implementation-checklist.md`](./07-implementation-checklist.md)
- 並行: Phase 3 [`05-web-frontend-spec.md`](../2026-05-07-003-phase3/05-web-frontend-spec.md) — 既存パターンの参照

## ファイル構成（Phase 4a 後）

```
web/src/lib/
├── audio/
│   ├── engine.ts                       (Phase 4a で applyInstrument / lfoSet* / applyPreset 追加)
│   ├── generated/
│   │   └── params.ts                   (gen-params.mjs 出力、Phase 4a で InstrumentKind / BODY_MODES_<INSTRUMENT> 等を出力)
│   ├── messages.ts                     (Phase 4a で 4 variant 追加)
│   ├── synth-processor.ts              (Phase 4a で WasmExports 4 関数 + dispatch 4 ケース追加)
│   └── voice-state.svelte.ts           (Phase 3 同等、変更なし)
├── components/
│   ├── Keyboard.svelte                 (Phase 1 同等、変更なし)
│   ├── LfoSection.svelte               (Phase 4a 新規 — LFO controls UI)
│   ├── MidiSelect.svelte               (Phase 3 同等、CC#1 dispatch は midi-cc.ts 経由で既存パターン)
│   ├── ModWheel.svelte                 (Phase 4a 新規 — Mod Wheel スライダー)
│   ├── ParamSlider.svelte              (Phase 2 同等、変更なし)
│   ├── PolyphonyToggle.svelte          (Phase 3 同等、変更なし)
│   ├── PresetSelector.svelte           (Phase 4a 新規 — Factory + User プリセット選択)
│   ├── StartButton.svelte              (Phase 1 同等、変更なし)
│   └── VoiceMeter.svelte               (Phase 3 同等、変更なし)
├── input/
│   ├── midi.ts                         (Phase 3 同等、rawListener API 利用)
│   └── midi-cc.ts                      (Phase 4a で CC#1 受信時に synth.modWheel = value/127 を更新する経路追加、F41 用)
├── state/
│   ├── factory-presets.ts              (Phase 4a 新規 — Factory Preset 7 種の const テーブル)
│   ├── preset-schema.ts                (Phase 4a 新規 — PresetV1 interface + InstrumentKind type)
│   ├── preset-store.svelte.ts          (Phase 4a 新規 — localStorage 操作レイヤ)
│   ├── synth.svelte.ts                 (Phase 4a で modWheel / lfoRate / lfoWaveform / lfoPitchDepth / lfoBrightnessDepth / lfoVolumeDepth / instrument $state 追加)
│   └── ui.svelte.ts                    (Phase 3 同等、変更なし)
├── actions/
│   └── pc-keyboard.svelte.ts           (Phase 1 同等、変更なし)
└── routes/
    └── +page.svelte                    (Phase 4a で <ModWheel> / <LfoSection> / <PresetSelector> 配置)
```

## messages.ts の Phase 4a 変更点

```typescript
export type ToWorkletMessage =
  | { type: 'init'; wasmBytes: ArrayBuffer; sampleRate: number }
  | { type: 'noteOn'; midi: number; velocity: number }
  | { type: 'noteOff'; midi: number }
  | { type: 'setParam'; id: number; value: number }
  | { type: 'setMode'; mode: 'poly' | 'mono' }
  | { type: 'midiCC'; cc: number; value: number }
  | { type: 'pitchBend'; semitones: number }
  | { type: 'reset' }
  | { type: 'dispose' }
  // Phase 4a D46-D49 (LFO / Mod Wheel)
  | { type: 'lfoSetRate'; hz: number }
  | { type: 'lfoSetWaveform'; kind: LfoWaveformKey }
  | { type: 'lfoSetDepth'; dest: LfoDestinationKey; depth: number }
  // Phase 4a D52 (楽器切替)
  | { type: 'applyInstrument'; kind: InstrumentKindKey };

export type LfoWaveformKey = 'sine' | 'triangle';
export type LfoDestinationKey = 'pitch' | 'brightness' | 'volume';
export type InstrumentKindKey =
  | 'default'
  | 'guitar_classical'
  | 'ukulele'
  | 'mandolin'
  | 'bass'
  | 'guitar_steel'
  | 'sitar';
```

`FromWorkletMessage` は **変更なし**（Phase 3 の `ready` / `error` / `debug` / `voiceState` を維持）。

### 文字列キー → u32 マッピング（synth-processor.ts 内）

```typescript
const LFO_WAVEFORM_MAP: Record<LfoWaveformKey, number> = {
  sine: 0,
  triangle: 1,
};

const LFO_DESTINATION_MAP: Record<LfoDestinationKey, number> = {
  pitch: 0,
  brightness: 1,
  volume: 2,
};

const INSTRUMENT_KIND_MAP: Record<InstrumentKindKey, number> = {
  default: 0,
  guitar_classical: 1,
  ukulele: 2,
  mandolin: 3,
  bass: 4,
  guitar_steel: 5,
  sitar: 6,
};
```

## synth-processor.ts の Phase 4a 変更点

### `WasmExports` interface 拡張

```typescript
interface WasmExports {
  memory: WebAssembly.Memory;
  // Phase 1-3 既存 14 関数
  synth_new: (sr: number, maxBlock: number) => number;
  synth_free: (ptr: number) => void;
  synth_note_on: (ptr: number, midi: number, vel: number) => void;
  synth_note_off: (ptr: number, midi: number) => void;
  synth_set_param: (ptr: number, id: number, value: number) => void;
  synth_reset: (ptr: number) => void;
  synth_out_l_ptr: (ptr: number) => number;
  synth_out_r_ptr: (ptr: number) => number;
  synth_capacity: (ptr: number) => number;
  synth_process_block: (ptr: number, frames: number) => void;
  synth_set_polyphony_mode: (ptr: number, mode: number) => void;
  synth_midi_cc: (ptr: number, cc: number, value: number) => void;
  synth_pitch_bend: (ptr: number, semitones: number) => void;
  synth_voice_state_ptr: (ptr: number) => number;
  // Phase 4a 追加（D45-D52）
  synth_apply_instrument: (ptr: number, kind: number) => void;
  synth_lfo_set_rate: (ptr: number, hz: number) => void;
  synth_lfo_set_waveform: (ptr: number, kind: number) => void;
  synth_lfo_set_depth: (ptr: number, dest: number, depth: number) => void;
}
```

### `onMessage` switch case 追加

```typescript
private async onMessage(msg: ToWorkletMessage): Promise<void> {
  switch (msg.type) {
    // ...Phase 3 既存ケース...

    case 'lfoSetRate':
      this.exports?.synth_lfo_set_rate(this.handlePtr, msg.hz);
      break;
    case 'lfoSetWaveform':
      this.exports?.synth_lfo_set_waveform(this.handlePtr, LFO_WAVEFORM_MAP[msg.kind]);
      break;
    case 'lfoSetDepth':
      this.exports?.synth_lfo_set_depth(
        this.handlePtr,
        LFO_DESTINATION_MAP[msg.dest],
        msg.depth
      );
      break;
    case 'applyInstrument':
      this.exports?.synth_apply_instrument(
        this.handlePtr,
        INSTRUMENT_KIND_MAP[msg.kind]
      );
      break;
  }
}
```

### `process()` 内のヒープ確保ゼロ維持

`process()` 自体は Phase 3 と同じ実装、Voice State stride push (`maybePushVoiceState`) も不変。LFO / 楽器切替は event-driven のため `process` 内に新規 alloc は出ない。Phase 3 D4 維持。

### Float32Array view の扱い

Voice State view は Phase 3 同等。LFO / 楽器切替で WASM memory が grow することはない（Engine 内のフィールド追加のみで `vec` の new alloc なし）、`memory.buffer` 不変前提を維持。

## engine.ts の Phase 4a 変更点

### 新規メソッド

```typescript
export class SynthEngine {
  // ...Phase 3 既存...

  // Phase 4a 追加: LFO / instrument の現在値を保持 (Worklet 再起動時の再送用)
  // 既存 Phase 1-3 の `currentParams: Map<number, number>` と同形式の永続化方針。
  private currentLfo: { rate: number; waveform: LfoWaveformKey;
                        pitchDepth: number; brightnessDepth: number; volumeDepth: number } = {
    rate: 5.0, waveform: 'sine',
    pitchDepth: 0, brightnessDepth: 0, volumeDepth: 0,
  };
  private currentInstrument: InstrumentKindKey = 'default';

  // Phase 4a (D46-D49)
  lfoSetRate(hz: number): void {
    this.currentLfo.rate = hz;        // 状態保持 (start 成功後に再送)
    if (!this.ready) return;
    this.post({ type: 'lfoSetRate', hz });
  }

  lfoSetWaveform(kind: LfoWaveformKey): void {
    this.currentLfo.waveform = kind;
    if (!this.ready) return;
    this.post({ type: 'lfoSetWaveform', kind });
  }

  lfoSetDepth(dest: LfoDestinationKey, depth: number): void {
    if (dest === 'pitch') this.currentLfo.pitchDepth = depth;
    else if (dest === 'brightness') this.currentLfo.brightnessDepth = depth;
    else if (dest === 'volume') this.currentLfo.volumeDepth = depth;
    if (!this.ready) return;
    this.post({ type: 'lfoSetDepth', dest, depth });
  }

  // Phase 4a (D52)
  applyInstrument(kind: InstrumentKindKey): void {
    this.currentInstrument = kind;
    if (!this.ready) return;
    this.post({ type: 'applyInstrument', kind });
  }

  // Phase 4a (D50): プリセット一括適用
  // ready 前でも各 setter は currentParams / currentLfo / currentInstrument を更新する設計
  // （個別 setter 側で state 保持後に ready チェック）。よって applyPreset 自体は early return しない。
  // ready 後に start() が再送 (currentParams + resendPhase4aState) するため、起動前に
  // applyPreset を呼ぶ経路 (onMount での last preset 復元等) も整合する。
  applyPreset(preset: PresetV1): void {
    // 1. 楽器切替（全 voice release を伴うため最初）
    this.applyInstrument(preset.instrument);

    // 2. パラメータ適用（順序は問わないが、既存の setParam 経路を流用）
    this.setParam(PARAM_IDS.Damping, preset.params.damping);
    this.setParam(PARAM_IDS.Brightness, preset.params.brightness);
    this.setParam(PARAM_IDS.OutputGain, preset.params.outputGain);
    this.setParam(PARAM_IDS.PickPosition, preset.params.pickPosition);
    this.setParam(PARAM_IDS.BodyWet, preset.params.bodyWet);

    // 3. LFO 適用
    this.lfoSetRate(preset.lfo.rate);
    this.lfoSetWaveform(preset.lfo.waveform);
    this.lfoSetDepth('pitch', preset.lfo.pitchDepth);
    this.lfoSetDepth('brightness', preset.lfo.brightnessDepth);
    this.lfoSetDepth('volume', preset.lfo.volumeDepth);
  }

  // Phase 4a: start() 成功時の再送 (既存 currentParams の再送と同位置で実装)
  // start() の最後で呼び出される（Worklet 再初期化 / retry でも整合性保つ）
  private resendPhase4aState(): void {
    // 楽器を最初に送る (内部で all_notes_off + Modal 再構築)
    this.post({ type: 'applyInstrument', kind: this.currentInstrument });
    this.post({ type: 'lfoSetRate', hz: this.currentLfo.rate });
    this.post({ type: 'lfoSetWaveform', kind: this.currentLfo.waveform });
    this.post({ type: 'lfoSetDepth', dest: 'pitch', depth: this.currentLfo.pitchDepth });
    this.post({ type: 'lfoSetDepth', dest: 'brightness', depth: this.currentLfo.brightnessDepth });
    this.post({ type: 'lfoSetDepth', dest: 'volume', depth: this.currentLfo.volumeDepth });
  }
}
```

`applyPreset` は **MessagePort で個別送信** する設計。Phase 3 既存の rAF スロットル (`flushParams`) は `setParam` 経路のみに作用し、`lfoSetRate` 等は即時送信。プリセット切替は数 ms 内で完了するため UX 影響なし。

**`start()` の Phase 4a 拡張**: 既存 Phase 1-3 では `start()` の最後で `currentParams` を Worklet に再送していた:
```typescript
for (const [id, value] of this.currentParams) {
  this.post({ type: 'setParam', id, value });
}
```
Phase 4a でこの直後に **`this.resendPhase4aState()` を呼出**、LFO / 楽器の状態も再送する。これで Worklet 再初期化 / retry 時に Param だけ復元され LFO/instrument は default のまま、という非対称を防ぐ。`engine.ts` の `start()` メソッド末尾に 1 行追加。

## preset-schema.ts (Phase 4a 新規)

```typescript
// web/src/lib/state/preset-schema.ts

export type InstrumentKindKey =
  | 'default'
  | 'guitar_classical'
  | 'ukulele'
  | 'mandolin'
  | 'bass'
  | 'guitar_steel'
  | 'sitar';

export type LfoWaveformKey = 'sine' | 'triangle';

export interface PresetV1 {
  version: 1;
  name: string;
  createdAt: string;  // ISO 8601, "2026-05-08T12:34:56.789Z"
  instrument: InstrumentKindKey;
  params: {
    damping: number;
    brightness: number;
    outputGain: number;
    pickPosition: number;
    bodyWet: number;
  };
  lfo: {
    rate: number;
    waveform: LfoWaveformKey;
    pitchDepth: number;
    brightnessDepth: number;
    volumeDepth: number;
  };
}

const VALID_INSTRUMENTS: InstrumentKindKey[] = [
  'default', 'guitar_classical', 'ukulele', 'mandolin', 'bass', 'guitar_steel', 'sitar'
];
const VALID_WAVEFORMS: LfoWaveformKey[] = ['sine', 'triangle'];

// 各 Param の値域 (params.json と同期、ParamDescriptor を import して使う場合あり)
const PARAM_RANGES = {
  damping: { min: 0.9, max: 0.9999 },
  brightness: { min: 0.0, max: 1.0 },
  outputGain: { min: 0.0, max: 1.5 },
  pickPosition: { min: 0.05, max: 0.5 },
  bodyWet: { min: 0.0, max: 1.0 },
} as const;

const LFO_RANGES = {
  rate: { min: 0.1, max: 8.0 },
  pitchDepth: { min: 0.0, max: 1.0 },
  brightnessDepth: { min: 0.0, max: 1.0 },
  volumeDepth: { min: 0.0, max: 1.0 },
} as const;

function isFiniteInRange(v: unknown, min: number, max: number): boolean {
  return typeof v === 'number' && Number.isFinite(v) && v >= min && v <= max;
}

/** 受信した unknown オブジェクトが PresetV1 として valid か検証 (型 + 有限性 + 値域) */
export function isValidPresetV1(obj: unknown): obj is PresetV1 {
  if (!obj || typeof obj !== 'object') return false;
  const p = obj as Record<string, unknown>;
  if (p.version !== 1) return false;
  if (typeof p.name !== 'string' || p.name.length === 0 || p.name.length > 64) return false;
  if (typeof p.createdAt !== 'string') return false;
  if (typeof p.instrument !== 'string') return false;
  if (!VALID_INSTRUMENTS.includes(p.instrument as InstrumentKindKey)) return false;

  // params の内部構造 + 値域検証
  if (!p.params || typeof p.params !== 'object') return false;
  const pp = p.params as Record<string, unknown>;
  if (!isFiniteInRange(pp.damping, PARAM_RANGES.damping.min, PARAM_RANGES.damping.max)) return false;
  if (!isFiniteInRange(pp.brightness, PARAM_RANGES.brightness.min, PARAM_RANGES.brightness.max)) return false;
  if (!isFiniteInRange(pp.outputGain, PARAM_RANGES.outputGain.min, PARAM_RANGES.outputGain.max)) return false;
  if (!isFiniteInRange(pp.pickPosition, PARAM_RANGES.pickPosition.min, PARAM_RANGES.pickPosition.max)) return false;
  if (!isFiniteInRange(pp.bodyWet, PARAM_RANGES.bodyWet.min, PARAM_RANGES.bodyWet.max)) return false;

  // lfo の内部構造 + 値域検証
  if (!p.lfo || typeof p.lfo !== 'object') return false;
  const pl = p.lfo as Record<string, unknown>;
  if (!isFiniteInRange(pl.rate, LFO_RANGES.rate.min, LFO_RANGES.rate.max)) return false;
  if (typeof pl.waveform !== 'string' || !VALID_WAVEFORMS.includes(pl.waveform as LfoWaveformKey)) return false;
  if (!isFiniteInRange(pl.pitchDepth, LFO_RANGES.pitchDepth.min, LFO_RANGES.pitchDepth.max)) return false;
  if (!isFiniteInRange(pl.brightnessDepth, LFO_RANGES.brightnessDepth.min, LFO_RANGES.brightnessDepth.max)) return false;
  if (!isFiniteInRange(pl.volumeDepth, LFO_RANGES.volumeDepth.min, LFO_RANGES.volumeDepth.max)) return false;

  return true;
}

export function getDefaultPreset(): PresetV1 {
  return {
    version: 1,
    name: 'Default',
    createdAt: new Date().toISOString(),
    instrument: 'default',
    params: {
      damping: 0.996,
      brightness: 0.5,
      outputGain: 0.8,
      pickPosition: 0.125,
      bodyWet: 0.5,
    },
    lfo: {
      rate: 5.0,
      waveform: 'sine',
      pitchDepth: 0.0,
      brightnessDepth: 0.0,
      volumeDepth: 0.0,
    },
  };
}
```

## factory-presets.ts (Phase 4a 新規)

```typescript
// web/src/lib/state/factory-presets.ts

import type { PresetV1 } from './preset-schema';

/**
 * Factory Preset 7 種（Default + 楽器 6 種）。
 * 編集不可、コードで管理（再ビルドで更新）。
 * createdAt は const 値（実機で時刻取得しない、再現性のため）。
 */
export const FACTORY_PRESETS: PresetV1[] = [
  {
    version: 1,
    name: 'Default',
    createdAt: '2026-05-08T00:00:00.000Z',
    instrument: 'default',
    params: { damping: 0.996, brightness: 0.5, outputGain: 0.8, pickPosition: 0.125, bodyWet: 0.5 },
    lfo: { rate: 5.0, waveform: 'sine', pitchDepth: 0.0, brightnessDepth: 0.0, volumeDepth: 0.0 },
  },
  {
    version: 1,
    name: 'Classical Guitar',
    createdAt: '2026-05-08T00:00:00.000Z',
    instrument: 'guitar_classical',
    params: { damping: 0.997, brightness: 0.45, outputGain: 0.8, pickPosition: 0.12, bodyWet: 0.6 },
    lfo: { rate: 5.0, waveform: 'sine', pitchDepth: 0.0, brightnessDepth: 0.0, volumeDepth: 0.0 },
  },
  {
    version: 1,
    name: 'Ukulele',
    createdAt: '2026-05-08T00:00:00.000Z',
    instrument: 'ukulele',
    params: { damping: 0.992, brightness: 0.65, outputGain: 0.85, pickPosition: 0.18, bodyWet: 0.55 },
    lfo: { rate: 5.0, waveform: 'sine', pitchDepth: 0.0, brightnessDepth: 0.0, volumeDepth: 0.0 },
  },
  {
    version: 1,
    name: 'Mandolin',
    createdAt: '2026-05-08T00:00:00.000Z',
    instrument: 'mandolin',
    params: { damping: 0.994, brightness: 0.7, outputGain: 0.85, pickPosition: 0.1, bodyWet: 0.6 },
    lfo: { rate: 6.5, waveform: 'sine', pitchDepth: 0.3, brightnessDepth: 0.0, volumeDepth: 0.0 },
  },
  {
    version: 1,
    name: 'Acoustic Bass',
    createdAt: '2026-05-08T00:00:00.000Z',
    instrument: 'bass',
    params: { damping: 0.998, brightness: 0.3, outputGain: 0.9, pickPosition: 0.15, bodyWet: 0.5 },
    lfo: { rate: 4.0, waveform: 'sine', pitchDepth: 0.0, brightnessDepth: 0.0, volumeDepth: 0.0 },
  },
  {
    version: 1,
    name: 'Steel Guitar',
    createdAt: '2026-05-08T00:00:00.000Z',
    instrument: 'guitar_steel',
    params: { damping: 0.996, brightness: 0.6, outputGain: 0.8, pickPosition: 0.13, bodyWet: 0.55 },
    lfo: { rate: 5.0, waveform: 'sine', pitchDepth: 0.0, brightnessDepth: 0.0, volumeDepth: 0.0 },
  },
  {
    version: 1,
    name: 'Sitar',
    createdAt: '2026-05-08T00:00:00.000Z',
    instrument: 'sitar',
    params: { damping: 0.997, brightness: 0.55, outputGain: 0.85, pickPosition: 0.08, bodyWet: 0.7 },
    lfo: { rate: 5.5, waveform: 'sine', pitchDepth: 0.4, brightnessDepth: 0.0, volumeDepth: 0.0 },
  },
];
```

## preset-store.svelte.ts (Phase 4a 新規)

```typescript
// web/src/lib/state/preset-store.svelte.ts

import { isValidPresetV1, type PresetV1 } from './preset-schema';
import { FACTORY_PRESETS } from './factory-presets';
import type { SynthEngine } from '../audio/engine';

const STORAGE_KEY_LIST = 'physbase.preset.v1.list';
const STORAGE_KEY_PREFIX = 'physbase.preset.v1.';
const STORAGE_KEY_LAST = 'physbase.preset.v1.last';
export const MAX_USER_PRESETS = 32;

class PresetStore {
  readonly factoryPresets: ReadonlyArray<PresetV1> = FACTORY_PRESETS;
  userPresets = $state<PresetV1[]>([]);
  currentPresetName = $state<string>('Default');
  errorMessage = $state<string | null>(null);

  /** localStorage から User Preset を読み込み。bad data は skip。
   *  STORAGE_KEY_LIST 不在時 (User preset 未保存) でも STORAGE_KEY_LAST は読む
   *  (Factory preset だけを選択して保存しているケースに対応)。
   *  stale な lastName (削除済み User preset / 存在しない名前) は findByName で検証し、
   *  なければ 'Default' に fallback。
   *  load() が再実行されても古い userPresets が残らないよう、`loaded` を常に新規配列で
   *  作り、最後に必ず `this.userPresets = loaded` で上書きする。 */
  load(): void {
    // loaded を最初に作り、エラー経路でも空配列のまま userPresets を上書きできるようにする。
    const loaded: PresetV1[] = [];
    try {
      // 1. User preset リストの読み込み。
      //    LIST 不在 / JSON parse 失敗 / 配列でない場合は loaded を空のまま継続
      //    (Factory のみ利用シナリオや手動破壊への耐性)。
      const listJson = localStorage.getItem(STORAGE_KEY_LIST);
      if (listJson) {
        let names: unknown = null;
        try {
          names = JSON.parse(listJson);
        } catch (e) {
          console.warn('[PresetStore] STORAGE_KEY_LIST JSON parse failed:', e);
        }
        if (Array.isArray(names)) {
          for (const name of names) {
            if (typeof name !== 'string') continue;
            const presetJson = localStorage.getItem(STORAGE_KEY_PREFIX + name);
            if (!presetJson) continue;
            try {
              const obj = JSON.parse(presetJson);
              if (isValidPresetV1(obj)) {
                loaded.push(obj);
              } else {
                console.warn(`[PresetStore] Invalid preset ${name}, skipping`);
              }
            } catch (e) {
              console.warn(`[PresetStore] Failed to parse preset ${name}:`, e);
            }
          }
        } else {
          console.warn('[PresetStore] STORAGE_KEY_LIST is not an array, ignoring');
        }
      }
      // 必ず実行: 再 load() 時の stale state 防止 (LIST 不在 / 不正でも空で上書き)
      this.userPresets = loaded;

      // 2. last preset 名の読み込み (LIST の有無に関係なく実行)。
      //    findByName で Factory + User の両方を確認、存在しなければ 'Default' に fallback。
      //    手動破壊 / 古いデータ / 削除済み User preset で <select> の value が
      //    option と不一致になるのを防ぐ。
      const lastName = localStorage.getItem(STORAGE_KEY_LAST);
      this.currentPresetName = (lastName && this.findByName(lastName)) ? lastName : 'Default';
    } catch (e) {
      console.error('[PresetStore] load failed:', e);
      this.errorMessage = 'Failed to load presets from storage';
      // エラー時も userPresets を空にし、currentPresetName を Default に戻す。
      this.userPresets = loaded; // ← loaded はこれまでに追加された分（または空）
      this.currentPresetName = 'Default';
    }
  }

  /** Factory + User すべてのプリセット名を返す */
  allPresetNames(): { factory: string[]; user: string[] } {
    return {
      factory: this.factoryPresets.map(p => p.name),
      user: this.userPresets.map(p => p.name),
    };
  }

  /** 名前から PresetV1 を取得（Factory 優先） */
  findByName(name: string): PresetV1 | undefined {
    return this.factoryPresets.find(p => p.name === name)
        ?? this.userPresets.find(p => p.name === name);
  }

  /** 現在の synth state からプリセット作成 (UI 操作値を参照) */
  capturePreset(name: string, snapshot: Omit<PresetV1, 'version' | 'name' | 'createdAt'>): PresetV1 {
    return {
      version: 1,
      name,
      createdAt: new Date().toISOString(),
      ...snapshot,
    };
  }

  /** User Preset を保存 */
  save(preset: PresetV1): void {
    if (preset.name.length === 0) {
      this.errorMessage = 'Preset name cannot be empty';
      return;
    }
    // Factory プリセット名との重複を拒否（findByName が Factory 優先のため、
    // 同名 User を保存しても選択時に Factory が勝ち、削除も Factory 判定でブロックされる）
    if (this.factoryPresets.some(p => p.name === preset.name)) {
      this.errorMessage = `Cannot use factory preset name: ${preset.name}`;
      return;
    }
    const existingIdx = this.userPresets.findIndex(p => p.name === preset.name);
    if (existingIdx === -1 && this.userPresets.length >= MAX_USER_PRESETS) {
      this.errorMessage = `Preset slot full (max ${MAX_USER_PRESETS})`;
      return;
    }
    try {
      localStorage.setItem(STORAGE_KEY_PREFIX + preset.name, JSON.stringify(preset));
      if (existingIdx === -1) {
        this.userPresets = [...this.userPresets, preset];
      } else {
        this.userPresets = this.userPresets.map((p, i) => (i === existingIdx ? preset : p));
      }
      const names = this.userPresets.map(p => p.name);
      localStorage.setItem(STORAGE_KEY_LIST, JSON.stringify(names));
      this.errorMessage = null;
    } catch (e) {
      console.error('[PresetStore] save failed:', e);
      this.errorMessage = 'Failed to save preset (storage quota exceeded?)';
    }
  }

  /** User Preset を削除 (Factory は削除不可) */
  delete(name: string): void {
    if (this.factoryPresets.some(p => p.name === name)) {
      this.errorMessage = 'Cannot delete factory preset';
      return;
    }
    try {
      localStorage.removeItem(STORAGE_KEY_PREFIX + name);
      this.userPresets = this.userPresets.filter(p => p.name !== name);
      const names = this.userPresets.map(p => p.name);
      localStorage.setItem(STORAGE_KEY_LIST, JSON.stringify(names));
      this.errorMessage = null;
    } catch (e) {
      console.error('[PresetStore] delete failed:', e);
    }
  }

  /** プリセットを engine に適用 */
  apply(name: string, engine: SynthEngine): void {
    const preset = this.findByName(name);
    if (!preset) {
      this.errorMessage = `Preset not found: ${name}`;
      return;
    }
    engine.applyPreset(preset);
    this.currentPresetName = name;
    try {
      localStorage.setItem(STORAGE_KEY_LAST, name);
    } catch {
      /* localStorage failure は無視（apply 自体は成功） */
    }
  }
}

export const presetStore = new PresetStore();
```

## synth.svelte.ts の Phase 4a 拡張

```typescript
import { SynthEngine } from '$lib/audio/engine';
import { PARAM_DESCRIPTORS, PARAM_IDS } from '$lib/audio/messages';
import type { LfoWaveformKey, InstrumentKindKey } from '$lib/state/preset-schema';

class SynthState {
  readonly engine = new SynthEngine();
  ready = $state(false);
  // Phase 1-3 既存
  damping = $state(PARAM_DESCRIPTORS[PARAM_IDS.Damping].default);
  brightness = $state(PARAM_DESCRIPTORS[PARAM_IDS.Brightness].default);
  outputGain = $state(PARAM_DESCRIPTORS[PARAM_IDS.OutputGain].default);
  pickPosition = $state(PARAM_DESCRIPTORS[PARAM_IDS.PickPosition].default);
  bodyWet = $state(PARAM_DESCRIPTORS[PARAM_IDS.BodyWet].default);

  // Phase 4a 追加 (D46-D49)
  modWheel = $state(0.0);                     // 0..1, slider 値 (UI 上は 0..127 表示)
  lfoRate = $state(5.0);                       // 0.1..8.0 Hz
  lfoWaveform = $state<LfoWaveformKey>('sine');
  lfoPitchDepth = $state(0.0);                 // 0..1
  lfoBrightnessDepth = $state(0.0);            // 0..1
  lfoVolumeDepth = $state(0.0);                // 0..1

  // Phase 4a 追加 (D52)
  instrument = $state<InstrumentKindKey>('default');
}

export const synth = new SynthState();

// __synthDev は Phase 3 同等
if (import.meta.env.DEV) {
  // ...既存...
}
```

## ModWheel.svelte (Phase 4a 新規)

```svelte
<!-- web/src/lib/components/ModWheel.svelte -->
<script lang="ts">
  import { synth } from '$lib/state/synth.svelte';

  // 0..127 の MIDI CC value で UI 表示。内部的には 0..1 で SmoothedValue
  let cc127 = $derived(Math.round(synth.modWheel * 127));

  function handleInput(e: Event) {
    const v = Number((e.target as HTMLInputElement).value);
    synth.modWheel = Math.max(0, Math.min(127, v)) / 127;
    if (synth.engine.isReady()) {
      synth.engine.sendMidiCc(1, v);  // CC#1 = Mod Wheel
    }
  }
</script>

<label class="mod-wheel">
  <span>Mod Wheel</span>
  <input
    type="range"
    min="0"
    max="127"
    step="1"
    value={cc127}
    oninput={handleInput}
    disabled={!synth.ready}
  />
  <span class="value">{cc127}</span>
</label>

<style>
  .mod-wheel {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .value {
    min-width: 2rem;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
```

## LfoSection.svelte (Phase 4a 新規)

```svelte
<!-- web/src/lib/components/LfoSection.svelte -->
<script lang="ts">
  import { synth } from '$lib/state/synth.svelte';
  import type { LfoWaveformKey } from '$lib/state/preset-schema';

  function setRate(e: Event) {
    const v = Number((e.target as HTMLInputElement).value);
    synth.lfoRate = v;
    if (synth.engine.isReady()) synth.engine.lfoSetRate(v);
  }

  function setWaveform(kind: LfoWaveformKey) {
    synth.lfoWaveform = kind;
    if (synth.engine.isReady()) synth.engine.lfoSetWaveform(kind);
  }

  function setDepth(dest: 'pitch' | 'brightness' | 'volume', e: Event) {
    const v = Number((e.target as HTMLInputElement).value);
    if (dest === 'pitch') synth.lfoPitchDepth = v;
    if (dest === 'brightness') synth.lfoBrightnessDepth = v;
    if (dest === 'volume') synth.lfoVolumeDepth = v;
    if (synth.engine.isReady()) synth.engine.lfoSetDepth(dest, v);
  }
</script>

<section class="lfo">
  <h2>LFO</h2>
  <label>
    <span>Rate</span>
    <input
      type="range"
      min="0.1"
      max="8.0"
      step="0.1"
      value={synth.lfoRate}
      oninput={setRate}
      disabled={!synth.ready}
    />
    <span class="value">{synth.lfoRate.toFixed(1)} Hz</span>
  </label>

  <fieldset class="waveform">
    <legend>Waveform</legend>
    <label>
      <input
        type="radio"
        name="lfo-waveform"
        value="sine"
        checked={synth.lfoWaveform === 'sine'}
        onchange={() => setWaveform('sine')}
        disabled={!synth.ready}
      />
      Sine
    </label>
    <label>
      <input
        type="radio"
        name="lfo-waveform"
        value="triangle"
        checked={synth.lfoWaveform === 'triangle'}
        onchange={() => setWaveform('triangle')}
        disabled={!synth.ready}
      />
      Triangle
    </label>
  </fieldset>

  <label>
    <span>Pitch Depth</span>
    <input type="range" min="0" max="1" step="0.01" value={synth.lfoPitchDepth} oninput={(e) => setDepth('pitch', e)} disabled={!synth.ready} />
    <span class="value">{synth.lfoPitchDepth.toFixed(2)}</span>
  </label>
  <label>
    <span>Brightness Depth</span>
    <input type="range" min="0" max="1" step="0.01" value={synth.lfoBrightnessDepth} oninput={(e) => setDepth('brightness', e)} disabled={!synth.ready} />
    <span class="value">{synth.lfoBrightnessDepth.toFixed(2)}</span>
  </label>
  <label>
    <span>Volume Depth</span>
    <input type="range" min="0" max="1" step="0.01" value={synth.lfoVolumeDepth} oninput={(e) => setDepth('volume', e)} disabled={!synth.ready} />
    <span class="value">{synth.lfoVolumeDepth.toFixed(2)}</span>
  </label>
</section>

<style>
  .lfo {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    border: 1px solid #ccc;
    border-radius: 4px;
    padding: 0.75rem;
  }
  .lfo h2 {
    margin: 0 0 0.5rem 0;
    font-size: 1rem;
  }
  .lfo label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .lfo .value {
    min-width: 3rem;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .waveform {
    border: none;
    padding: 0;
    margin: 0;
    display: flex;
    gap: 1rem;
  }
  .waveform legend {
    margin-right: 0.5rem;
  }
</style>
```

## PresetSelector.svelte (Phase 4a 新規)

```svelte
<!-- web/src/lib/components/PresetSelector.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { synth } from '$lib/state/synth.svelte';
  import { presetStore } from '$lib/state/preset-store.svelte';
  import type { PresetV1 } from '$lib/state/preset-schema';

  let saveName = $state('');

  onMount(() => {
    presetStore.load();
    // 起動時に最後に選択された Preset を復元 (UI state を無条件で同期)。
    // onMount は通常 Start Audio 前に走り、engine.isReady() は false。
    // engine.applyPreset は ready 前でも currentParams / currentLfo / currentInstrument
    // を更新する設計のため、起動前から呼んで OK。Start Audio 後の start() 末尾で
    // currentParams 再送 + resendPhase4aState() が走り、Worklet にも反映される。
    const lastPreset = presetStore.findByName(presetStore.currentPresetName);
    if (lastPreset) {
      synth.engine.applyPreset(lastPreset);
      applyPresetToUiState(lastPreset);
    }
  });

  /**
   * Preset 適用時の UI state 同期 helper。handleSelect / handleDelete / 起動時の
   * last preset 復元など、すべての preset 切替経路で同じ処理を使う（DRY）。
   * Engine への適用は `presetStore.apply` 経由の `engine.applyPreset` で済んでいる前提。
   */
  function applyPresetToUiState(preset: PresetV1): void {
    synth.damping = preset.params.damping;
    synth.brightness = preset.params.brightness;
    synth.outputGain = preset.params.outputGain;
    synth.pickPosition = preset.params.pickPosition;
    synth.bodyWet = preset.params.bodyWet;
    synth.lfoRate = preset.lfo.rate;
    synth.lfoWaveform = preset.lfo.waveform;
    synth.lfoPitchDepth = preset.lfo.pitchDepth;
    synth.lfoBrightnessDepth = preset.lfo.brightnessDepth;
    synth.lfoVolumeDepth = preset.lfo.volumeDepth;
    synth.instrument = preset.instrument;
  }

  function handleSelect(e: Event) {
    const name = (e.target as HTMLSelectElement).value;
    if (name) {
      presetStore.apply(name, synth.engine);
      const preset = presetStore.findByName(name);
      if (preset) applyPresetToUiState(preset);
    }
  }

  function handleSave() {
    if (saveName.trim().length === 0) return;
    const preset = presetStore.capturePreset(saveName.trim(), {
      instrument: synth.instrument,
      params: {
        damping: synth.damping,
        brightness: synth.brightness,
        outputGain: synth.outputGain,
        pickPosition: synth.pickPosition,
        bodyWet: synth.bodyWet,
      },
      lfo: {
        rate: synth.lfoRate,
        waveform: synth.lfoWaveform,
        pitchDepth: synth.lfoPitchDepth,
        brightnessDepth: synth.lfoBrightnessDepth,
        volumeDepth: synth.lfoVolumeDepth,
      },
    });
    presetStore.save(preset);
    saveName = '';
  }

  function handleDelete() {
    if (confirm(`Delete preset "${presetStore.currentPresetName}"?`)) {
      presetStore.delete(presetStore.currentPresetName);
      // 削除後はデフォルトに戻す。engine と UI state を `applyPresetToUiState` 経由で揃える。
      presetStore.apply('Default', synth.engine);
      const defaultPreset = presetStore.findByName('Default');
      if (defaultPreset) applyPresetToUiState(defaultPreset);
    }
  }
</script>

<section class="preset">
  <label>
    <span>Preset</span>
    <select value={presetStore.currentPresetName} onchange={handleSelect} disabled={!synth.ready}>
      <optgroup label="Factory">
        {#each presetStore.factoryPresets as p}
          <option value={p.name}>{p.name}</option>
        {/each}
      </optgroup>
      {#if presetStore.userPresets.length > 0}
        <optgroup label="User">
          {#each presetStore.userPresets as p}
            <option value={p.name}>{p.name}</option>
          {/each}
        </optgroup>
      {/if}
    </select>
  </label>

  <div class="actions">
    <input
      type="text"
      placeholder="New preset name"
      bind:value={saveName}
      maxlength="32"
      disabled={!synth.ready}
    />
    <button onclick={handleSave} disabled={!synth.ready || saveName.trim().length === 0}>
      Save
    </button>
    <button
      onclick={handleDelete}
      disabled={!synth.ready
        || presetStore.factoryPresets.some(p => p.name === presetStore.currentPresetName)}
    >
      Delete
    </button>
  </div>

  {#if presetStore.errorMessage}
    <p class="error">{presetStore.errorMessage}</p>
  {/if}
</section>

<style>
  .preset {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    border: 1px solid #ccc;
    border-radius: 4px;
    padding: 0.75rem;
  }
  .preset label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .actions {
    display: flex;
    gap: 0.5rem;
  }
  .actions input {
    flex: 1;
  }
  .error {
    color: #c00;
    font-size: 0.85rem;
    margin: 0;
  }
</style>
```

## +page.svelte の Phase 4a 変更点

```svelte
<script lang="ts">
  import { onDestroy } from 'svelte';
  import StartButton from '$lib/components/StartButton.svelte';
  import Keyboard from '$lib/components/Keyboard.svelte';
  import MidiSelect from '$lib/components/MidiSelect.svelte';
  import ParamSlider from '$lib/components/ParamSlider.svelte';
  import VoiceMeter from '$lib/components/VoiceMeter.svelte';
  import PolyphonyToggle from '$lib/components/PolyphonyToggle.svelte';
  // Phase 4a 追加
  import ModWheel from '$lib/components/ModWheel.svelte';
  import LfoSection from '$lib/components/LfoSection.svelte';
  import PresetSelector from '$lib/components/PresetSelector.svelte';
  import { pcKeyboard } from '$lib/actions/pc-keyboard.svelte';
  import { PARAM_IDS } from '$lib/audio/messages';
  import { synth } from '$lib/state/synth.svelte';

  // ...Phase 3 既存ロジック (testNoteC4 / onDestroy) は維持...
</script>

<main use:pcKeyboard={...}>
  <header class="header">
    <h1>Physics-Base Synth</h1>
    <div class="header-controls">
      <VoiceMeter />
      <PolyphonyToggle />
    </div>
  </header>
  <StartButton />
  <MidiSelect />
  <button onclick={testNoteC4} disabled={!synth.ready}>Play C4 (test)</button>

  <!-- Phase 4a: Preset セレクター -->
  <PresetSelector />

  <section class="params">
    <ParamSlider label="Damping" paramId={PARAM_IDS.Damping} step={0.0001} bind:value={synth.damping} />
    <ParamSlider label="Brightness" paramId={PARAM_IDS.Brightness} step={0.01} bind:value={synth.brightness} />
    <ParamSlider label="Output Gain" paramId={PARAM_IDS.OutputGain} step={0.01} bind:value={synth.outputGain} />
    <ParamSlider label="Pick Position" paramId={PARAM_IDS.PickPosition} step={0.01} bind:value={synth.pickPosition} />
    <ParamSlider label="Body Wet" paramId={PARAM_IDS.BodyWet} step={0.01} bind:value={synth.bodyWet} />
    <!-- Phase 4a: Mod Wheel -->
    <ModWheel />
  </section>

  <!-- Phase 4a: LFO Section -->
  <LfoSection />

  <Keyboard />
  <small class="hint">PC keyboard: A S D F G H J K (white) / W E T Y U O (black)</small>
</main>
```

## midi-cc.ts の Phase 4a 変更点

Phase 3 で実装済みの CC#1 → `engine.sendMidiCc(1, value)` の経路は維持しつつ、**WebMIDI 物理 Mod Wheel と UI スライダーの同期** を実現するため、CC#1 受信時に `synth.modWheel` を更新する追加経路を入れる（F41 「物理 wheel を動かすと UI スライダーが追従」を満たすため）。

**変更後のスケッチ**:
```typescript
// web/src/lib/input/midi-cc.ts (Phase 4a 拡張)
import type { SynthEngine } from '$lib/audio/engine';
import { synth } from '$lib/state/synth.svelte';

const STATUS_MASK = 0xf0;
const STATUS_CONTROL_CHANGE = 0xb0;
const STATUS_PITCH_BEND = 0xe0;
const PITCH_BEND_CENTER = 8192;
const PITCH_BEND_RANGE_SEMITONES = 2;

const CC_MOD_WHEEL = 1;  // Phase 4a 追加

let lastPitchBend14: number | null = null;

export function handleMidiMessage(data: Uint8Array, engine: SynthEngine): boolean {
  if (data.length === 0) return false;
  const cmd = data[0] & STATUS_MASK;
  if (cmd === STATUS_CONTROL_CHANGE && data.length >= 3) {
    const ccNum = data[1];
    const ccValue = data[2];
    engine.sendMidiCc(ccNum, ccValue);
    // Phase 4a: 物理 Mod Wheel を UI スライダーと同期 (D49 / F41)
    if (ccNum === CC_MOD_WHEEL) {
      synth.modWheel = ccValue / 127;
    }
    return true;
  }
  // Phase 3 既存の Pitch Bend 経路は不変
  if (cmd === STATUS_PITCH_BEND && data.length >= 3) {
    const lsb = data[1] & 0x7f;
    const msb = data[2] & 0x7f;
    const combined14 = (msb << 7) | lsb;
    if (combined14 === lastPitchBend14) return true;
    lastPitchBend14 = combined14;
    const normalized = (combined14 - PITCH_BEND_CENTER) / PITCH_BEND_CENTER;
    engine.sendPitchBend(normalized * PITCH_BEND_RANGE_SEMITONES);
    return true;
  }
  return false;
}
```

**注意**:
- `synth.modWheel` の更新は Engine 経由 (`engine.sendMidiCc`) と同時に行う。`engine.sendMidiCc` は MessagePort で Worklet に送るのみ、UI 状態は別更新が必要（Phase 3 までの ParamSlider / Pitch Bend は別経路）
- 他 CC（CC#7 / CC#64 / CC#123）は UI 反映先がないため CC#1 のみ追加
- ModWheel.svelte 内の `oninput` ハンドラから来る `engine.sendMidiCc(1, v)` も同経路で `synth.modWheel` を更新するが、`oninput` 内で直接 `synth.modWheel = v / 127` も更新するため二重更新になる。これは UI → Engine と MIDI → Engine の双方向経路を許容する設計上の意図的なもので、最終値が一致すれば問題なし

## ParamSlider.svelte の Phase 4a 状況

**変更なし**。既存の ParamDescriptor 駆動パターンが LFO controls には不要（ParamId に組み込まず Engine 直接 setter 経由のため）。Pick Position / Body Wet と同じく、Phase 4a の LFO は params.json `params` 配列に追加せず、専用 setter (`lfoSetRate` など) を使う。

> 設計判断: LFO 5 値（rate / waveform / 3 depths）を `ParamId` に追加すべきか検討したが、(a) `synth_set_param(id, f32)` の f32 単一値 API では波形 enum を渡せない、(b) destinations の dest 番号と depth を 1 つの ParamId にマップしにくい、(c) 既存 Pick Position / Body Wet と異なり LFO は構造的に複数のサブ値を持つ、の 3 点から **専用 C ABI 関数 (`synth_lfo_set_*`) で扱う**設計を採用した。

## テスト方針

### svelte-check

`pnpm --filter ./web check` で以下を検証:
- 新規コンポーネント `ModWheel.svelte` / `LfoSection.svelte` / `PresetSelector.svelte` の Svelte 5 runes 構文
- `preset-schema.ts` / `factory-presets.ts` / `preset-store.svelte.ts` の型整合
- `messages.ts` の Phase 4a variant (LfoWaveformKey / LfoDestinationKey / InstrumentKindKey) の型エクスポート

### eslint

`pnpm --filter ./web lint` で `svelte/prefer-svelte-reactivity` 等の Svelte 5 ルールが新規コンポーネントでも通る。`new Set` / `new Map` を `$state` 内で使う場合は局所抑止。

### prettier

`pnpm fmt` で新規ファイルを整形。

### 単体動作確認（Step 15 で実施）

`pnpm dev` でブラウザ起動、以下を手動確認:

| 項目 | 確認方法 |
|---|---|
| プリセット選択 | Factory 7 種を順次選択、楽器名・パラメータ・LFO 値が UI に反映 |
| プリセット保存 | 名前入力 → Save → リロード → User Preset として表示される |
| プリセット削除 | User Preset 選択 → Delete → 確認ダイアログ → 削除される、Factory は Delete ボタン disabled |
| 32 件超過 | 32 件保存後 33 件目で `errorMessage` 表示 |
| Mod Wheel | スライダー操作 → LFO 効果が音に反映（depth が 0 でなければ） |
| LFO Rate | 0.1〜8.0 Hz で vibrato / tremolo の速度変化を体感 |
| LFO Waveform | sine / triangle 切替で音色変化 |
| LFO Pitch Depth | 0 → 1 で pitch 揺れの増加 |
| LFO Brightness Depth | 0 → 1 で brightness 揺れの増加 |
| LFO Volume Depth | 0 → 1 で tremolo（音量揺れ）の増加 |
| 楽器切替時の音切れ | 音を出しながら他楽器選択 → 即時 release で音が止まる |
| Mod Wheel = 0 | LFO depth が 1.0 でも音が変調されない（Phase 3 互換挙動） |

## 視覚的レイアウト想定

```
┌─────────────────────────────────────────────────────────────┐
│ Physics-Base Synth         [VoiceMeter]  [Mono / Poly]      │
├─────────────────────────────────────────────────────────────┤
│ [Start Audio]                                                │
│ MIDI: [Select Device ▾]                                      │
│ [Play C4 (test)]                                             │
├─────────────────────────────────────────────────────────────┤
│ Preset                                                       │
│ Preset: [Default ▾]                                          │
│ [New preset name________] [Save] [Delete]                    │
├─────────────────────────────────────────────────────────────┤
│ Damping       [====●====] 0.996                              │
│ Brightness    [===●=====] 0.50                               │
│ Output Gain   [====●====] 0.80                               │
│ Pick Position [==●======] 0.13                               │
│ Body Wet      [====●====] 0.50                               │
│ Mod Wheel     [●========] 0                                  │
├─────────────────────────────────────────────────────────────┤
│ LFO                                                          │
│ Rate          [====●====] 5.0 Hz                             │
│ Waveform      ⦿ Sine  ◯ Triangle                             │
│ Pitch Depth      [●========] 0.00                            │
│ Brightness Depth [●========] 0.00                            │
│ Volume Depth     [●========] 0.00                            │
├─────────────────────────────────────────────────────────────┤
│ [Piano Keyboard widget]                                      │
│ PC keyboard: A S D F G H J K (white) / W E T Y U O (black)   │
└─────────────────────────────────────────────────────────────┘
```

## 設計判断の章間相互参照

- D44 (F38b §0) → 06 章 §F38b 計測手順
- D45 (wasm-opt) → 02 章 §ビルドパイプライン拡張、04 章 §wasm-opt 統合、06 章 §性能目標
- D46 (LFO 配置) → 03 章 §Lfo、05 章 §LfoSection / §synth.svelte.ts
- D47 (LFO 波形) → 03 章 §Lfo の `process_sample`、05 章 §LfoSection radio
- D48 (LFO destinations) → 03 章 §Engine の destination 適用、05 章 §LfoSection 3 sliders
- D49 (Mod Wheel) → 03 章 §Engine::handle_midi_cc、04 章 §既存 synth_midi_cc 不変、05 章 §ModWheel
- D50 (Preset 形式) → 05 章 §preset-schema.ts、§preset-store.svelte.ts
- D51 (User 上限 32) → 05 章 §preset-store.svelte.ts MAX_USER_PRESETS
- D52 (楽器 6 種) → 03 章 §multi-instrument modal、05 章 §factory-presets.ts
- D53 (楽器切替挙動) → 03 章 §Engine::apply_instrument、05 章 §PresetSelector の即時反映
- D54 (stereo_spread 楽器別) → 03 章 §params.rs、05 章 §factory-presets.ts には spread 直接記述せず楽器 enum で持つ
- D55 (Mono+Sustain 現状維持) → Phase 3 既存実装、本仕様書では UI 変更なし
