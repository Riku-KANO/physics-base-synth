# 05. Web フロントエンド仕様（Phase 4b）

## 目的

Phase 4b で追加・変更する Svelte コンポーネント、TypeScript モジュール、AudioWorklet 拡張を定義する。Phase 1 / 2 / 3 / 4a で確立した既存コンポーネント（`StartButton` / `Keyboard` / `MidiSelect` / `ParamSlider` / `VoiceMeter` / `PolyphonyToggle` / `ModWheel` / `LfoSection` / `PresetSelector`）と既存ステート（`synth.svelte.ts` / `ui.svelte.ts` / `voice-state.svelte.ts` / `preset-store.svelte.ts`）はすべて維持し、本書では **Phase 4b で追加・変更する箇所のみ** 記述する。

Phase 4b は UI レベルでは「Piano プリセット 1 件追加」+「dev-only `__synthDev.measureProcessTime` 追加」+「型定義拡張 (`InstrumentKindKey` に `'piano'` 追加)」の 3 点のみで、新規コンポーネント追加なし。

## 他文書との関係

- 上流: [`pre-research.md`](./pre-research.md)（§7 Piano Modal Body / §8 新パラメータ判断 / §9 F38b 自動計測）、[`01-overview.md`](./01-overview.md)（D62-D67）、[`02-architecture.md`](./02-architecture.md)（UI 層責務）、[`03-dsp-core-spec.md`](./03-dsp-core-spec.md)（dsp-core API / InstrumentKind::Piano）、[`04-wasm-audio-spec.md`](./04-wasm-audio-spec.md)（C ABI 既存維持）
- 下流: [`07-implementation-checklist.md`](./07-implementation-checklist.md)
- 並行: Phase 4a [`05-web-frontend-spec.md`](../2026-05-08-004-phase4a/05-web-frontend-spec.md) — 既存パターンの参照

## ファイル構成（Phase 4b 後）

```
web/src/lib/
├── audio/
│   ├── __synthDev.ts                  (Phase 4b 新規 — F38b 計測自動化、dev only)
│   ├── engine.ts                       (Phase 4b 軽微 — InstrumentKindKey 'piano' 拡張のみ、メソッド追加なし)
│   ├── generated/
│   │   └── params.ts                   (gen-params.mjs 出力、Phase 4b で InstrumentKind 'piano' / Piano BodyMode を出力)
│   ├── messages.ts                     (Phase 4b 軽微 — InstrumentKindKey に 'piano' 追加 + startTimingCapture/stopTimingCapture 追加)
│   ├── synth-processor.ts              (Phase 4b で INSTRUMENT_KIND_MAP['piano']=7 + dev-only timing 集約コード追加)
│   └── voice-state.svelte.ts           (Phase 3 同等、変更なし)
├── components/
│   ├── Keyboard.svelte                 (Phase 1 同等、変更なし)
│   ├── LfoSection.svelte               (Phase 4a 同等、変更なし)
│   ├── MidiSelect.svelte               (Phase 3 同等、変更なし)
│   ├── ModWheel.svelte                 (Phase 4a 同等、変更なし)
│   ├── ParamSlider.svelte              (Phase 2 同等、変更なし)
│   ├── PolyphonyToggle.svelte          (Phase 3 同等、変更なし)
│   ├── PresetSelector.svelte           (Phase 4a 同等、Piano エントリは factory-presets.ts 経由で自動表示)
│   ├── StartButton.svelte              (Phase 1 同等、変更なし)
│   └── VoiceMeter.svelte               (Phase 3 同等、変更なし)
├── input/
│   ├── midi.ts                         (Phase 3 同等、変更なし)
│   └── midi-cc.ts                      (Phase 4a 同等、変更なし)
├── state/
│   ├── factory-presets.ts              (Phase 4b で Piano エントリ 1 件追加、合計 8 種)
│   ├── preset-schema.ts                (Phase 4b で InstrumentKindKey に 'piano' 追加 + VALID_INSTRUMENTS 配列に 'piano' 追加)
│   ├── preset-store.svelte.ts          (Phase 4a 同等、変更なし)
│   ├── synth.svelte.ts                 (Phase 4b で __synthDev export に measureProcessTime 経路追加)
│   └── ui.svelte.ts                    (Phase 3 同等、変更なし)
├── actions/
│   └── pc-keyboard.svelte.ts           (Phase 1 同等、変更なし)
└── routes/
    └── +page.svelte                    (Phase 4a 同等、変更なし)
```

## messages.ts の Phase 4b 変更点

### `InstrumentKindKey` に `'piano'` 追加（D62）

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

### `INSTRUMENT_KIND_MAP` の拡張（synth-processor.ts 内）

```typescript
const INSTRUMENT_KIND_MAP: Record<InstrumentKindKey, number> = {
  default: 0,
  guitar_classical: 1,
  ukulele: 2,
  mandolin: 3,
  bass: 4,
  guitar_steel: 5,
  sitar: 6,
  piano: 7,         // ← Phase 4b 追加
};
```

### dev-only timing capture variant 追加（D66）

```typescript
export type ToWorkletMessage =
  | ...Phase 4a 既存...
  // Phase 4b D66: F38b 計測自動化スクリプト用 (dev only)
  | { type: 'startTimingCapture' }
  | { type: 'stopTimingCapture' };

export type FromWorkletMessage =
  | ...Phase 4a 既存...
  // Phase 4b D66: F38b 計測値の集約 (dev only)
  | { type: 'timing'; samples: number[]; bufferOverflow: boolean };
```

`samples` は `process` 1 回ごとの self time（ms）の配列、`bufferOverflow` はリングバッファが満杯で古いデータが上書きされたかのフラグ。

## synth-processor.ts の Phase 4b 変更点

### `WasmExports` interface

**変更なし**（Phase 4a の 18 関数のまま、Phase 4b で C ABI 関数追加なし）。

### `INSTRUMENT_KIND_MAP` 拡張（D62）

上記 messages.ts セクション参照、`piano: 7` を追加するのみ。

### dev-only timing 集約コード（D66）

#### 計測方式: AudioWorkletGlobalScope の `performance.now()`

**`currentFrame` は callback 内で進まない**ため、`endFrame - startFrame` は常に 0 で self time 計測には使えない。AudioWorkletGlobalScope は `performance.now()` を持つ（Chrome / Firefox / Safari でサポート、精度はブラウザ依存だが ~5μs 程度）。**`process` 開始/終了で `performance.now()` の差分を取る**ことで、Worklet thread の `process` メソッドの純粋な実行時間（self time）を ms 単位で取得する。

#### `DEV_MODE` の置換方式

仕様変更: **`const DEV_MODE = false;` のローカル定義は esbuild define 置換対象にならない**ため、TypeScript の **`declare const DEV_MODE: boolean;`** （type-only declaration、ランタイムには存在しない識別子）にして、worklet build script (`web/package.json` の `build:worklet:dev` / `build:worklet`) 側で **`--define:DEV_MODE=true` / `--define:DEV_MODE=false`** を渡す。これにより esbuild が `DEV_MODE` 識別子を `true` / `false` に直接置換し、`if (false) { ... }` ブロックが tree-shake で完全削除される。

```typescript
// synth-processor.ts 冒頭
declare const DEV_MODE: boolean;
//                       ^^^^^^^ esbuild define で `true` / `false` に置換される識別子
//                       ローカル `const DEV_MODE = ...` は禁止 (define 対象外、置換されない)

class SynthProcessor extends AudioWorkletProcessor {
  // ...Phase 4a 既存 fields...

  // Phase 4b D66: dev-only timing 集約 (production では tree-shake で削除)
  private timingBuffer: Float32Array | null = null;
  private timingBufferCapacity = 0;
  private timingBufferWriteIndex = 0;
  private timingBufferCount = 0;          // 蓄積された entry 数 (≤ capacity)
  private timingBufferWrapped = false;     // リングバッファが 1 周以上 wrap したか
  private timingCaptureActive = false;

  constructor() {
    super();
    // ...Phase 4a 既存...

    if (DEV_MODE) {
      // capacity = 4096 entry × f32 = 16 KB
      // 48kHz / 128 frames = 375 quanta/sec、4096 entry で約 10.92 秒分を保持。
      // durationMs > 10000 (= 10 秒) では wrap が発生し、リングバッファとして
      // 「最新 capacity 個」を保持する（古いデータが上書きされる）。
      // measureProcessTime(durationMs) 側で durationMs を 10000 ms 程度に制限する想定。
      this.timingBufferCapacity = 4096;
      this.timingBuffer = new Float32Array(this.timingBufferCapacity);
    }
  }

  private async onMessage(msg: ToWorkletMessage): Promise<void> {
    switch (msg.type) {
      // ...Phase 4a 既存ケース...

      // Phase 4b D66: dev only
      case 'startTimingCapture':
        if (DEV_MODE) {
          this.timingBufferWriteIndex = 0;
          this.timingBufferCount = 0;
          this.timingBufferWrapped = false;
          this.timingCaptureActive = true;
        }
        break;

      case 'stopTimingCapture':
        if (DEV_MODE && this.timingBuffer) {
          // リングバッファから時系列順の有効サンプルを取り出す:
          // wrap していなければ [0..count) が有効
          // wrap していれば [writeIndex..capacity) ++ [0..writeIndex) が時系列順
          const samples: number[] = [];
          if (this.timingBufferWrapped) {
            for (let i = this.timingBufferWriteIndex; i < this.timingBufferCapacity; i++) {
              samples.push(this.timingBuffer[i]);
            }
            for (let i = 0; i < this.timingBufferWriteIndex; i++) {
              samples.push(this.timingBuffer[i]);
            }
          } else {
            for (let i = 0; i < this.timingBufferCount; i++) {
              samples.push(this.timingBuffer[i]);
            }
          }
          this.port.postMessage({
            type: 'timing',
            samples,
            bufferOverflow: this.timingBufferWrapped,
          } as FromWorkletMessage);
          this.timingCaptureActive = false;
        }
        break;
    }
  }

  process(_inputs: Float32Array[][], outputs: Float32Array[][]): boolean {
    let startMs = 0;
    if (DEV_MODE && this.timingCaptureActive) {
      startMs = performance.now();   // AudioWorkletGlobalScope の performance.now()
    }

    // ...Phase 4a 既存 process 経路 (KS + Body + LFO + soft clip + Voice State 等)...

    if (DEV_MODE && this.timingCaptureActive && this.timingBuffer) {
      const elapsedMs = performance.now() - startMs;
      // リングバッファに書き込み
      this.timingBuffer[this.timingBufferWriteIndex] = elapsedMs;
      this.timingBufferWriteIndex++;
      if (this.timingBufferWriteIndex >= this.timingBufferCapacity) {
        this.timingBufferWriteIndex = 0;
        this.timingBufferWrapped = true;
      }
      if (this.timingBufferCount < this.timingBufferCapacity) {
        this.timingBufferCount++;
      }
    }

    return true;
  }
}
```

**重要（D66）**:
- `DEV_MODE` は **`declare const DEV_MODE: boolean;`**（type-only declaration）。`web/package.json` の `build:worklet*` script に **`--define:DEV_MODE=true`**（dev）/ **`--define:DEV_MODE=false`**（production）を esbuild に渡し、build 時に識別子を `true` / `false` リテラルに置換。production の `if (false) { ... }` ブロックは tree-shake で完全削除
- **`performance.now()` を AudioWorkletGlobalScope で使用**: 仕様上 Chrome / Firefox / Safari の AudioWorkletGlobalScope は `performance` API を持つ（精度は ~5μs、ブラウザ依存）。`process` の純粋な実行時間（self time）を ms 単位で取得可能
- `currentFrame` / `sampleRate` も AudioWorkletGlobalScope のグローバルだが、**`currentFrame` は callback 内で進まない**ため self time 計測には使えない（音声時間 128/sampleRate ≈ 2.67ms を返すだけ）。Phase 4b では使わない
- リングバッファ `Float32Array(4096)` は constructor で 1 度確保、`process` 内 alloc ゼロ維持（Phase 1 D4 / D9 継承）
- リングバッファとして wrap 時には **古いサンプルが上書きされ、最新 capacity 個（最新 ~10.92 秒分）が保持される**。stop 時に時系列順に並べ直して main へ送る
- 計測オーバーヘッド（performance.now() × 2 + buffer 書込）は dev only で問題なし、production では tree-shake で完全削除されるため影響ゼロ

### `process()` 内のヒープ確保ゼロ維持

`process()` 自体は Phase 4a と同じ実装。dev-only timing 集約は **`Float32Array(4096)` を constructor 内 1 回確保**（48kHz/128 frames で 375 quanta/sec、約 10.92 秒分を保持）、`process` 内では `timingBuffer[writeIndex++] = elapsedMs` の代入のみで alloc ゼロ。Phase 1 D4 / D9 維持。

### Float32Array view の扱い

Voice State view は Phase 4a 同等。Phase 4b で WASM memory grow なし（`KarplusStrong::dispersion_stages` は inline 配列、heap 操作なし）、`memory.buffer` 不変前提を維持。

## __synthDev.ts (Phase 4b 新規、D66)

```typescript
// web/src/lib/audio/__synthDev.ts
//
// Phase 4b D66: F38b 計測自動化スクリプト (dev only)。
//
// 使い方:
//   await window.__synthDev.measureProcessTime(10000);
//   // → { avg: 0.045, max: 0.087, samples: [...], bufferOverflow: false }
//
// production build では Step 14 で `--define:DEV_MODE=false` を渡すことで
// Worklet 側の timing 集約コードが完全に tree-shake される。Web 側 (本ファイル)
// も `if (import.meta.env.DEV)` ガードで synth.svelte.ts から動的 import する。
//
// 計測方式: Worklet 側で performance.now() の差分を取り、リングバッファ (4096 entry)
// に self time (ms) を蓄積。stop メッセージで時系列順に main へ送る。
// 4096 entry は 48kHz/128 frames で約 10.92 秒分、durationMs > 10000 では古い
// サンプルが上書きされ最新 ~10.92 秒分が保持される (bufferOverflow=true で報告)。

import type { ToWorkletMessage, FromWorkletMessage } from './messages';

export interface TimingResult {
  avg: number;
  max: number;
  min: number;
  samples: number[];
  bufferOverflow: boolean;
  durationMs: number;
}

/**
 * AudioWorklet `process` の self time を `durationMs` ms 間計測し、
 * avg / max / min を返す。
 *
 * Worklet 側でリングバッファ (4096 entry × f32 = 16 KB) に self time を蓄積、
 * stop メッセージで時系列順の有効サンプルを main へ集約 postMessage。
 * 4096 entry は約 10.92 秒分 (48kHz / 128 frames quanta) を保持、durationMs > 10000
 * では bufferOverflow=true で報告される (最新 ~10.92 秒分のみ有効)。
 *
 * @param port - SynthEngine の AudioWorkletNode.port
 * @param durationMs - 計測時間 (ms、推奨 5000-10000、上限 10000 で overflow なし)
 */
export async function measureProcessTime(
  port: MessagePort,
  durationMs: number,
): Promise<TimingResult> {
  return new Promise((resolve, reject) => {
    const timeoutId = setTimeout(() => {
      port.removeEventListener('message', onMessage);
      reject(new Error(`measureProcessTime timeout (${durationMs}ms)`));
    }, durationMs + 5000);  // 5 秒余裕

    function onMessage(e: MessageEvent<FromWorkletMessage>) {
      if (e.data.type !== 'timing') return;
      port.removeEventListener('message', onMessage);
      clearTimeout(timeoutId);

      const samples = e.data.samples;
      if (samples.length === 0) {
        resolve({
          avg: 0,
          max: 0,
          min: 0,
          samples: [],
          bufferOverflow: e.data.bufferOverflow,
          durationMs,
        });
        return;
      }

      let sum = 0;
      let max = -Infinity;
      let min = Infinity;
      for (const s of samples) {
        sum += s;
        if (s > max) max = s;
        if (s < min) min = s;
      }
      resolve({
        avg: sum / samples.length,
        max,
        min,
        samples,
        bufferOverflow: e.data.bufferOverflow,
        durationMs,
      });
    }

    port.addEventListener('message', onMessage);

    // 計測開始
    port.postMessage({ type: 'startTimingCapture' } as ToWorkletMessage);

    // durationMs 後に停止 + 集約
    setTimeout(() => {
      port.postMessage({ type: 'stopTimingCapture' } as ToWorkletMessage);
    }, durationMs);
  });
}
```

## synth.svelte.ts の Phase 4b 拡張（D66）

`__synthDev` の dev-only export に `measureProcessTime` 経路を追加:

```typescript
import { SynthEngine } from '$lib/audio/engine';
// ...Phase 4a 既存 imports...

class SynthState {
  readonly engine = new SynthEngine();
  // ...Phase 4a 既存 $state...
}

export const synth = new SynthState();

// __synthDev は Phase 1〜4a 同等の dev-only export パターン
if (import.meta.env.DEV) {
  // 既存 Phase 1〜4a の __synthDev に加えて Phase 4b D66 で measureProcessTime を追加
  type SynthDevApi = {
    setMode?: (mode: 'mono' | 'poly') => void;
    measureProcessTime?: (durationMs: number) => Promise<unknown>;
    // ...他既存 API...
  };

  const w = window as Window & { __synthDev?: SynthDevApi };
  w.__synthDev = w.__synthDev ?? {};

  // ...Phase 4a 既存 setter 群...

  // Phase 4b D66: F38b 計測自動化
  w.__synthDev.measureProcessTime = async (durationMs: number) => {
    const port = synth.engine.workletPort();  // SynthEngine から AudioWorkletNode.port を取得
    if (!port) {
      throw new Error('Worklet not initialized, call StartButton first');
    }
    const { measureProcessTime } = await import('$lib/audio/__synthDev');
    return measureProcessTime(port, durationMs);
  };
}
```

`SynthEngine` に `workletPort()` メソッドを追加（既存の Worklet 参照を返す薄い getter、Phase 4a で `engine.ts` 内に持っている `MessagePort` を public に公開）:

```typescript
// engine.ts
export class SynthEngine {
  // ...Phase 4a 既存 fields...
  private node: AudioWorkletNode | null = null;

  // Phase 4b D66: dev-only timing 計測用、AudioWorkletNode.port を返す
  workletPort(): MessagePort | null {
    return this.node?.port ?? null;
  }
}
```

## engine.ts の Phase 4b 拡張（D62 + D66）

Phase 4a 既存:
```typescript
private currentInstrument: InstrumentKindKey = 'default';
```

Phase 4b で **型のみ拡張**: `InstrumentKindKey` に `'piano'` が追加されるため、コードの値域が自動的に拡張される（`engine.ts` 内のメソッド・分岐に変更なし）。

`workletPort()` メソッドを Phase 4b で追加（D66、上記 §synth.svelte.ts セクション参照）。

## preset-schema.ts の Phase 4b 拡張（D62）

Phase 4a 既存:
```typescript
export type InstrumentKindKey =
  | 'default'
  | 'guitar_classical'
  | 'ukulele'
  | 'mandolin'
  | 'bass'
  | 'guitar_steel'
  | 'sitar';

const VALID_INSTRUMENTS: InstrumentKindKey[] = [
  'default', 'guitar_classical', 'ukulele', 'mandolin', 'bass', 'guitar_steel', 'sitar'
];
```

Phase 4b 拡張:
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

const VALID_INSTRUMENTS: InstrumentKindKey[] = [
  'default', 'guitar_classical', 'ukulele', 'mandolin', 'bass', 'guitar_steel', 'sitar',
  'piano',          // ← Phase 4b 追加
];
```

`isValidPresetV1` / `getDefaultPreset` は変更なし（`VALID_INSTRUMENTS.includes` で自動的に Piano が valid 扱いになる）。

## factory-presets.ts の Phase 4b 拡張（D62）

Phase 4a の 7 エントリに Piano を 8 番目として追加:

```typescript
import type { PresetV1 } from './preset-schema';

export const FACTORY_PRESETS: PresetV1[] = [
  // ...Phase 4a 既存 7 エントリ (Default / Classical Guitar / Ukulele / Mandolin / Acoustic Bass / Steel Guitar / Sitar)...

  // Phase 4b D62 新規: Piano プリセット
  {
    version: 1,
    name: 'Piano',
    createdAt: '2026-05-09T00:00:00.000Z',
    instrument: 'piano',
    params: {
      damping: 0.998,           // ピアノは持続音、Phase 4a Bass と同程度
      brightness: 0.55,          // 中庸〜やや明るめ
      outputGain: 0.7,           // 他楽器より控えめ (倍音多いため)
      pickPosition: 0.13,        // hammer 経路では未使用、pluck フォールバック用デフォルト
      bodyWet: 0.4,              // soundboard 寄与中庸
    },
    lfo: {
      rate: 5.0,
      waveform: 'sine',
      pitchDepth: 0.0,           // 標準ピアノは vibrato なし
      brightnessDepth: 0.0,
      volumeDepth: 0.0,
    },
  },
];
```

`PresetSelector.svelte` のドロップダウンは `presetStore.factoryPresets` 配列を反復してオプション生成するため、Piano エントリ追加で **自動的に 8 番目のオプションとして表示される**（コード変更不要、Phase 4b で UI コンポーネント変更なし）。

## PresetSelector.svelte の Phase 4b 状況

**変更なし**。`factory-presets.ts` の Piano エントリ追加で自動的にドロップダウンに Piano が表示される。Phase 4a 既存の以下経路がそのまま動作:

```svelte
<!-- web/src/lib/components/PresetSelector.svelte (Phase 4a 既存) -->
<select bind:value={presetStore.currentPresetName} onchange={onChange}>
  <optgroup label="Factory">
    {#each presetStore.factoryPresets as preset (preset.name)}
      <option value={preset.name}>{preset.name}</option>
    {/each}
  </optgroup>
  <!-- ...User Preset 部分...-->
</select>
```

## generated/params.ts の Phase 4b 拡張（D62、gen-params.mjs 出力）

`gen-params.mjs` 拡張で `web/src/lib/audio/generated/params.ts` に Piano kind を追加:

```typescript
// 生成: web/src/lib/audio/generated/params.ts (Phase 4a + Piano)
export type InstrumentKindKey =
  | 'default'
  | 'guitar_classical'
  | 'ukulele'
  | 'mandolin'
  | 'bass'
  | 'guitar_steel'
  | 'sitar'
  | 'piano';                                                                  // Phase 4b 追加

export const INSTRUMENT_KIND_TO_NUMBER: Record<InstrumentKindKey, number> = {
  default: 0,
  guitar_classical: 1,
  ukulele: 2,
  mandolin: 3,
  bass: 4,
  guitar_steel: 5,
  sitar: 6,
  piano: 7,                                                                   // Phase 4b 追加
};

// Phase 4b D62 新規: Piano 専用フィールド (TS 側に出力、UI で参照する用途)
export const INHARMONICITY_B_PIANO = 7.5e-4;
export const HAMMER_CUTOFF_LOW_PIANO = 800.0;
export const HAMMER_CUTOFF_HIGH_PIANO = 4000.0;
```

これらの定数は **Phase 4b では UI から直接参照しない**（Piano プリセット内に閉じ込めるため）が、将来の Phase 4c で Piano の Inharmonicity / HammerHardness UI スライダー実装時に使う想定で生成しておく（drift 防止）。

## Worklet build script の Phase 4b 拡張（D66）

`web/package.json` の `build:worklet:dev` / `build:worklet`（または equivalent）スクリプトに **`--define:DEV_MODE=true`** / **`--define:DEV_MODE=false`** を esbuild に渡す引数を追加する:

```jsonc
{
  "scripts": {
    // Phase 4a 既存の esbuild 呼出に --define オプションを追加
    "build:worklet:dev":  "esbuild ... --define:DEV_MODE=true  --outfile=...",
    "build:worklet":      "esbuild ... --define:DEV_MODE=false --outfile=..."
  }
}
```

これにより `synth-processor.ts` 内の `declare const DEV_MODE: boolean;` が build 時に `true` / `false` リテラルに置換され、`if (false) { ... }` ブロックが tree-shake で削除される。**ローカル `const DEV_MODE = false;` を宣言してはならない**（define 置換対象は識別子のみ、ローカル変数定義は置換されない）。

実際の esbuild 引数の形は Phase 4a の `package.json` 既存 script を踏襲（例: `--bundle --format=iife --target=es2022` 等のオプションは Phase 4a と同じ）、`--define:DEV_MODE=...` を 1 つ追加するだけ。

## Worklet bundle サイズの想定

| ビルド種別 | Phase 4a 後実測 | Phase 4b 想定 |
|---|---|---|
| Worklet バンドル (synth-processor.\*.js) dev | ~9 KB | ~10 KB（dev-only timing コードで +1 KB） |
| Worklet バンドル production | ~8.17 KB | ~8.5 KB（INSTRUMENT_KIND_MAP の `piano: 7` 追加 + factory-presets.ts の Piano エントリ参照のみ、+0.3 KB） |

target < 12 KB（Phase 4b で再設定、Phase 4a の < 10 KB から余裕を取る）。dev / production の差は esbuild の `--define:DEV_MODE=true/false` 置換 + tree-shake により timing コードが production から完全削除されるため。

## 実機確認（Step 15 想定）

| 項目 | 確認方法 |
|---|---|
| Piano プリセットがドロップダウンに表示 | `pnpm dev` 起動 → `PresetSelector` ドロップダウンを開く、Piano が 8 番目に表示 |
| Piano プリセット選択で楽器切替 | Piano を選択 → 鍵盤押下 → ピアノっぽい音が出る（stretched harmonics + hammer 風 attack） |
| 楽器切替の挙動（D63 改訂後） | 鍵盤押下中に Piano ↔ Default を切替 → 即時 release（音切れ）、pop noise は Phase 4a と同レベル（指摘事項 #3 反映で fade-out 撤回、Phase 4c 送り） |
| Phase 4a 互換性 (Default + Mod Wheel=0) | Default プリセットで鍵盤押下 → Phase 4a と同じ音が出る、`test_dispersion_disabled_matches_phase4a` で機械保証済 |
| `__synthDev.measureProcessTime` (dev only) | DevTools Console で `await window.__synthDev.measureProcessTime(10000)` 実行 → avg/max が表示される |

## まとめ

Phase 4b で Web フロントエンド層に追加されるのは:
- **`__synthDev.ts` 1 ファイル**（dev only）
- **`messages.ts` の `InstrumentKindKey` に 'piano' 追加 + dev-only `startTimingCapture` / `stopTimingCapture` variant**
- **`synth-processor.ts` に `INSTRUMENT_KIND_MAP['piano']=7` + dev-only timing 集約コード**
- **`preset-schema.ts` の `InstrumentKindKey` 型 + `VALID_INSTRUMENTS` 配列に `'piano'` 追加**
- **`factory-presets.ts` に Piano エントリ 1 件追加**
- **`synth.svelte.ts` の dev-only `__synthDev.measureProcessTime` 経路追加**
- **`engine.ts` に `workletPort()` getter 追加**

新規 Svelte コンポーネントなし、UI レイアウト変更なし。Phase 4b の音色実装の主体は dsp-core 層で完結し、Web フロントエンド層は **Piano プリセット 1 件追加 + 計測 dev API 1 つ追加**で対応する設計。
