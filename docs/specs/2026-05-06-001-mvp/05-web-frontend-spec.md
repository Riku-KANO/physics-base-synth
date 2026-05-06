# 05. Web フロントエンド仕様（SvelteKit）

## 目的

ブラウザ側の UI、AudioWorklet 統合、3経路の演奏入力（画面鍵盤・PCキーボード・Web MIDI）の実装方針を定義する。WASM のロード経路、メッセージング規約、パラメータ送信のスロットリングを確定する。

## 他文書との関係

- 上流: [`02-architecture.md`](./02-architecture.md)（モノレポ構成）、[`04-wasm-audio-spec.md`](./04-wasm-audio-spec.md)（WASM API）
- 下流: [`06-build-and-verify.md`](./06-build-and-verify.md)（実行手順）
- 参考: pre-research 5.1（推奨構成）、5.2（render quantum）、8.1（責務分離）、8.2（パラメータ更新）、9.3（Audio Worklet samples）

## SvelteKit セットアップ

### Svelte バージョン方針

- **Svelte 5** を採用（`sv create` で生成される最新版）
- **Runes ベース**で実装する: `$state` / `$derived` / `$effect` / `$props` / `$bindable`
- **新しいイベントハンドラ記法** `onclick`、`oninput`、`onpointerdown` 等を使う（旧 `on:click`、修飾子 `|preventDefault` は使わない）
- **共有ステートは `.svelte.ts` 拡張子のモジュール**で `$state` を露出（旧 `writable` ストアは MVP では使わない）
- **副作用の attachment は Svelte action**（`use:action`）でカプセル化し、`src/lib/actions/` に置く

### プロジェクト作成

```powershell
# プロジェクトルートで実行
npx sv create web
```

対話プロンプトで以下を選択:

| 項目 | 選択 |
|---|---|
| Template | `SvelteKit minimal` |
| Type checking | `Yes, using TypeScript syntax` |
| Add features | `prettier`、`eslint`、`vitest`（任意） |
| Package manager | `pnpm` |

> `sv create` 最新版は Svelte 5 がデフォルト。生成された `package.json` で `"svelte": "^5.x"` を確認する。

### アダプタ変更

生成直後は `@sveltejs/adapter-auto` が入っているため、`@sveltejs/adapter-static` に差し替える:

```powershell
cd web
pnpm remove @sveltejs/adapter-auto
pnpm add -D @sveltejs/adapter-static
```

### `web/svelte.config.js`

```javascript
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      pages: 'build',
      assets: 'build',
      fallback: 'index.html',
      precompress: false,
      strict: true,
    }),
    prerender: { entries: ['*'] }
  }
};

export default config;
```

### `web/vite.config.ts`

Worklet スクリプトを単独バンドルとして出力するため、別エントリを追加する:

```typescript
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
  plugins: [sveltekit()],

  // WASM ファイルを Vite に認識させる
  assetsInclude: ['**/*.wasm'],

  // Worklet ビルドのため、別途 build スクリプトを呼ぶ運用にする（後述）
  optimizeDeps: {
    exclude: ['$lib/wasm']  // cargo build から copy-wasm.mjs で配置した .wasm を Vite の依存最適化から除外
  },

  server: {
    fs: {
      allow: ['..']  // src/lib/wasm/ への参照を許可
    }
  }
});
```

> **Worklet スクリプトのビルド方針**: Vite の `build.rollupOptions.input` で複数エントリを定義しても SvelteKit のページビルドと衝突するため、MVPでは **手動で esbuild を1度だけ呼ぶ** スクリプトを `package.json` に追加する。詳細は[06章のビルド手順](./06-build-and-verify.md)。

### `web/package.json` に追加するスクリプト

```json
{
  "scripts": {
    "dev": "pnpm build:worklet:dev && vite dev",
    "build": "pnpm build:worklet && vite build",
    "preview": "vite preview",
    "build:worklet": "esbuild src/lib/audio/synth-processor.ts --bundle --format=iife --outfile=static/worklet/synth-processor.js --target=es2020",
    "build:worklet:dev": "esbuild src/lib/audio/synth-processor.ts --bundle --format=iife --outfile=static/worklet/synth-processor.js --target=es2020 --sourcemap=inline",
    "build:worklet:watch": "esbuild src/lib/audio/synth-processor.ts --bundle --format=iife --outfile=static/worklet/synth-processor.js --target=es2020 --sourcemap=inline --watch"
  },
  "devDependencies": {
    "esbuild": "^0.24"
  }
}
```

- `build:worklet:dev` は `--sourcemap=inline` を付け、DevTools の Sources タブで Worklet 内の TypeScript をデバッグ可能にする
- `build:worklet:watch` は別ターミナルで起動して、Worklet コードを変更するたびに再ビルド
- 本番 `build:worklet` は sourcemap なしでサイズ最小化

## ファイルレイアウト（再掲）

```
web\src\
├── app.html
├── lib\
│   ├── audio\
│   │   ├── engine.ts                # メインスレッド側 SynthEngine
│   │   ├── synth-processor.ts       # Worklet 本体（esbuildで単独バンドル）
│   │   ├── messages.ts              # MessagePort で送るメッセージ型定義
│   │   └── wasm-loader.ts           # メインスレッドでの WASM bytes 取得
│   ├── input\
│   │   ├── midi.ts                  # Web MIDI
│   │   └── note-utils.ts            # MIDI ↔ frequency
│   ├── actions\
│   │   └── pc-keyboard.svelte.ts    # Svelte 5 $effectベース action: ASDF行 keydown/keyup
│   ├── components\
│   │   ├── StartButton.svelte
│   │   ├── Keyboard.svelte
│   │   ├── ParamSlider.svelte
│   │   └── MidiSelect.svelte
│   ├── state\
│   │   └── synth.svelte.ts          # Svelte 5: $state ベースの共有ステート
│   └── wasm\                        # cargo build 出力のコピー先（gitignore）
└── routes\
    ├── +layout.svelte
    └── +page.svelte
```

## メッセージ仕様（`messages.ts`）

```typescript
// メインスレッド → Worklet
export type ToWorkletMessage =
  | { type: 'init'; wasmBytes: ArrayBuffer; sampleRate: number }
  | { type: 'noteOn'; midi: number; velocity: number }
  | { type: 'noteOff'; midi: number }
  | { type: 'setParam'; id: number; value: number }
  | { type: 'reset' }
  | { type: 'dispose' };  // synth_free を呼び、Worklet 内のハンドルと view を解放

// Worklet → メインスレッド
export type FromWorkletMessage =
  | { type: 'ready' }
  | { type: 'error'; message: string }
  | { type: 'debug'; message: string };  // 開発時の状態通知（任意）

export const PARAM_IDS = {
  Damping: 0,
  Brightness: 1,
  OutputGain: 2,
} as const;
```

## メインスレッド側 `SynthEngine`（`engine.ts`）

```typescript
import { base } from '$app/paths';
import type { ToWorkletMessage, FromWorkletMessage } from './messages';
// Vite の ?url インポート（dev/build 両対応）。
// wasm はソース graph 上の asset なので ?url が正しい。
import wasmUrl from '$lib/wasm/wasm_audio.wasm?url';

export class SynthEngine {
  private ctx: AudioContext | null = null;
  private node: AudioWorkletNode | null = null;
  private ready = false;

  // currentParams: ユーザーが指定した最新値の真実（起動前の操作も保持する）
  // pendingParams: ready 後の rAF 間引き用（送信予約）
  private currentParams = new Map<number, number>();
  private pendingParams = new Map<number, number>();
  private rafHandle: number | null = null;

  /**
   * 必ずユーザージェスチャ（StartButton の onclick 内など）から呼ぶこと。
   * Worklet の `ready` メッセージを受信するまで resolve しない。
   * AudioContext 作成から ready 受信までの全工程を単一 try/catch で囲み、
   * どの段階で失敗しても dispose() で全リソースを片付けて再試行可能な状態に戻す。
   */
  async start(): Promise<void> {
    if (this.ready) return;

    // secure context チェックは ctx を作る前に行うので try の外側
    if (typeof window !== 'undefined' && !window.isSecureContext) {
      throw new Error('AudioWorklet requires a secure context. Use HTTPS or localhost.');
    }

    const READY_TIMEOUT_MS = 5000;  // WASM instantiate にかかる現実的な上限
    let timer: ReturnType<typeof setTimeout> | null = null;

    try {
      // AudioContext 作成、API 存在チェック、resume を含めて全部 try に入れる。
      // resume 失敗や audioWorklet 非対応で throw した場合も catch → dispose で ctx を閉じる。
      if (!this.ctx) {
        this.ctx = new AudioContext({ latencyHint: 'interactive' });
      }
      if (!this.ctx.audioWorklet) {
        throw new Error('AudioWorklet is not supported in this browser.');
      }
      // ユーザージェスチャ内で resume() を明示的に呼ぶ（iOS Safari 等の suspend 対策）
      if (this.ctx.state === 'suspended') {
        await this.ctx.resume();
      }

      if (!this.node) {
        // Worklet スクリプトは web/static/ 配下に置かれた純粋な静的 asset。
        // Vite の ?url graph には乗せず、root absolute path で解決する。
        // SvelteKit のサブパス配信を考慮し $app/paths の base を前置する。
        await this.ctx.audioWorklet.addModule(`${base}/worklet/synth-processor.js`);
        this.node = new AudioWorkletNode(this.ctx, 'synth-processor', {
          numberOfInputs: 0,
          numberOfOutputs: 1,
          outputChannelCount: [2],
        });
        this.node.connect(this.ctx.destination);
      }

      // wasm を fetch（HTTPステータスを確認してから arrayBuffer 化）
      const res = await fetch(wasmUrl);
      if (!res.ok) {
        throw new Error(`Failed to fetch ${wasmUrl}: HTTP ${res.status}`);
      }
      const wasmBytes = await res.arrayBuffer();

      // ここまで来たら post(init) → readyPromise を構築する。
      // timer は post 直前に開始する。fetch / addModule が遅延しても tタイムアウトに巻き込まれない。
      let settled = false;
      const readyPromise = new Promise<void>((resolve, reject) => {
        this._readyHandlers = {
          resolve, reject,
          get settled() { return settled; },
          markSettled: () => { settled = true; },
        };
      });

      // node の onmessage は readyPromise の resolver が用意できた今のタイミングで装着
      this.node.port.onmessage = (e: MessageEvent<FromWorkletMessage>) => {
        const h = this._readyHandlers;
        if (!h) {
          if (e.data.type === 'debug') console.warn('[Worklet]', e.data.message);
          return;
        }
        if (e.data.type === 'ready' && !h.settled) {
          h.markSettled();
          this.ready = true;
          h.resolve();
        } else if (e.data.type === 'error' && !h.settled) {
          h.markSettled();
          console.error('[Worklet]', e.data.message);
          h.reject(new Error(e.data.message));
        } else if (e.data.type === 'debug') {
          console.warn('[Worklet]', e.data.message);
        }
      };

      // post(init) の直前で timer 開始。これ以降のみが計測対象
      timer = setTimeout(() => {
        const h = this._readyHandlers;
        if (h && !h.settled) {
          h.markSettled();
          h.reject(new Error(`Worklet did not become ready within ${READY_TIMEOUT_MS}ms`));
        }
      }, READY_TIMEOUT_MS);

      this.post({ type: 'init', wasmBytes, sampleRate: this.ctx.sampleRate }, [wasmBytes]);

      // 初期化完了（WASM instantiate / synth_new / view キャッシュ）まで resolve しない
      await readyPromise;
    } catch (err) {
      // どの段階で落ちても timer を解除し、確実に状態をリセットする
      if (timer !== null) clearTimeout(timer);
      this._readyHandlers = null;
      await this.resetForRetry();
      throw err;
    }

    // 成功時も timer は不要（解除済みだが念のためクリーンアップ）
    if (timer !== null) clearTimeout(timer);
    this._readyHandlers = null;

    // 起動前にユーザーが動かしたスライダー値を Worklet に再送する
    // currentParams が真実、Worklet は今この瞬間に空の状態
    for (const [id, value] of this.currentParams) {
      this.post({ type: 'setParam', id, value });
    }
  }

  // 実装メモ: readyPromise の resolver を保持する内部ハンドル。
  // try/catch ブロックの外側で onmessage を共有するために使う。
  private _readyHandlers: {
    resolve: () => void;
    reject: (e: Error) => void;
    readonly settled: boolean;
    markSettled: () => void;
  } | null = null;

  noteOn(midi: number, velocity: number): void {
    if (!this.ready) return;  // 起動前のキー入力は静かに無視
    this.post({ type: 'noteOn', midi, velocity });
  }

  noteOff(midi: number): void {
    if (!this.ready) return;
    this.post({ type: 'noteOff', midi });
  }

  /**
   * パラメータ更新。currentParams に常に最新値を保持し、
   * ready 後は rAF で間引いて Worklet へ送信する。
   * 起動前の操作は currentParams にだけ反映され、start() 完了時にまとめて送信される。
   */
  setParam(id: number, value: number): void {
    this.currentParams.set(id, value);  // 真実は常に保持
    if (!this.ready) return;             // Worklet 未起動時は送信しない
    this.pendingParams.set(id, value);
    if (this.rafHandle === null) {
      this.rafHandle = requestAnimationFrame(() => this.flushParams());
    }
  }

  private flushParams(): void {
    this.rafHandle = null;
    for (const [id, value] of this.pendingParams) {
      this.post({ type: 'setParam', id, value });
    }
    this.pendingParams.clear();
  }

  private post(msg: ToWorkletMessage, transfer: Transferable[] = []): void {
    if (!this.node) return;
    this.node.port.postMessage(msg, transfer);
  }

  isReady(): boolean { return this.ready; }

  /**
   * エンジン全体を完全停止し、リソースを解放する。
   * HMR / 画面遷移 / start() 失敗時の復旧などで呼ぶ。
   * 呼び出し後は再度 start() できる（currentParams は保持される）。
   */
  async dispose(): Promise<void> {
    if (this.rafHandle !== null) {
      cancelAnimationFrame(this.rafHandle);
      this.rafHandle = null;
    }
    this.pendingParams.clear();

    if (this.node) {
      this.node.port.postMessage({ type: 'dispose' } as ToWorkletMessage);
      this.node.port.onmessage = null;
      this.node.disconnect();
      this.node = null;
    }
    if (this.ctx && this.ctx.state !== 'closed') {
      try {
        await this.ctx.close();
      } catch {
        // close 失敗は致命ではない
      }
    }
    this.ctx = null;
    this.ready = false;
  }

  /** start() 失敗時の内部リセット。dispose と同等だが currentParams を維持し再試行可能 */
  private async resetForRetry(): Promise<void> {
    await this.dispose();
  }
}
```

> **起動前のパラメータ操作**: ユーザーが Start Audio を押す前にスライダーを動かしても `currentParams` に値が保持される。`start()` が `ready` 受信後にまとめて Worklet へ送信するため、Rust 側の damping/brightness が UI と整合する。`pendingParams` は ready 後の rAF スロットル専用で、起動前は使わない（`post()` が `node` 未作成時に黙って drop する事故を防ぐ）。
>
> **Worklet URL の base 対応**: SvelteKit の adapter-static でサブパス配信（`/myapp/`）する可能性があるため、`$app/paths` の `base` を前置する。ルート配信（`/`）なら `base` は空文字なので影響なし。

> **wasmBytes のフェッチパス**: 上記の `import wasmUrl from '$lib/wasm/wasm_audio.wasm?url'` で dev/build を統一して扱う。Vite が dev では生のパスを返し、build ではハッシュ付きの最適化済みURLに解決する。生パスの直接 fetch（`/src/lib/wasm/...`）は本番で 404 になるため使わない。

## AudioWorkletProcessor（`synth-processor.ts`）

[04章で定義した C ABI](./04-wasm-audio-spec.md#公開api-cratesーwasm-audiosrclibrs) に対応する。wasm-bindgen の自動生成JSは使わない。

```typescript
import type { ToWorkletMessage, FromWorkletMessage } from './messages';

declare const sampleRate: number;  // AudioWorkletGlobalScope の組み込み変数
declare const registerProcessor: any;

interface WasmExports {
  memory: WebAssembly.Memory;
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
}

const FRAMES = 128;

class SynthProcessor extends AudioWorkletProcessor {
  private exports: WasmExports | null = null;
  private handlePtr = 0;
  private lPtr = 0;
  private rPtr = 0;
  private cachedMemBuf: ArrayBuffer | SharedArrayBuffer | null = null;
  private leftView: Float32Array | null = null;
  private rightView: Float32Array | null = null;

  // dispose と init の非同期競合対策。
  // dispose で generation を進め、init 中の世代不一致は処理を中断する。
  private generation = 0;

  constructor() {
    super();
    this.port.onmessage = (e: MessageEvent<ToWorkletMessage>) => this.onMessage(e.data);
  }

  private async onMessage(msg: ToWorkletMessage): Promise<void> {
    switch (msg.type) {
      case 'init':
        // initWasm 自体は非同期だが、await しなくても generation チェックで安全
        await this.initWasm(msg.wasmBytes, msg.sampleRate);
        break;
      case 'noteOn':
        this.exports?.synth_note_on(this.handlePtr, msg.midi, msg.velocity);
        break;
      case 'noteOff':
        this.exports?.synth_note_off(this.handlePtr, msg.midi);
        break;
      case 'setParam':
        this.exports?.synth_set_param(this.handlePtr, msg.id, msg.value);
        break;
      case 'reset':
        this.exports?.synth_reset(this.handlePtr);
        break;
      case 'dispose':
        this.disposeWasm();
        break;
    }
  }

  /**
   * ハンドルと view を解放。dispose 後の process は無音を返す。
   * generation を進めることで、進行中の initWasm をキャンセルする。
   */
  private disposeWasm(): void {
    this.generation++;
    if (this.exports && this.handlePtr !== 0) {
      this.exports.synth_free(this.handlePtr);
    }
    this.handlePtr = 0;
    this.lPtr = 0;
    this.rPtr = 0;
    this.cachedMemBuf = null;
    this.leftView = null;
    this.rightView = null;
    this.exports = null;
  }

  private async initWasm(bytes: ArrayBuffer, sr: number): Promise<void> {
    // この init の世代を記録。dispose で generation が進んだら中断する
    const myGen = ++this.generation;

    // すべての準備を local 変数で完結させ、最後に一括 commit する。
    // commit 前に throw すれば catch で local handle のみ解放すればよく、
    // commit 後にはこの関数から throw しないため double free が起きない。
    let localExports: WasmExports | null = null;
    let localHandle = 0;

    try {
      // C ABI のため、wasm-bindgen 由来の __wbindgen_* import は不要。
      // panic = abort のため abort も呼ばれないが、念のため env を空で渡す。
      const imports: WebAssembly.Imports = { env: {} };
      const { instance } = await WebAssembly.instantiate(bytes, imports);

      // instantiate 完了時点で dispose されていたら何もしない
      if (myGen !== this.generation) return;

      localExports = instance.exports as unknown as WasmExports;
      localHandle = localExports.synth_new(sr, FRAMES);

      // synth_new 直後にもう一度世代チェック。負けていたら作ったハンドルを即解放
      if (myGen !== this.generation) {
        localExports.synth_free(localHandle);
        return;
      }

      // 続く準備も local で行う（this には触らない）
      const localLPtr = localExports.synth_out_l_ptr(localHandle);
      const localRPtr = localExports.synth_out_r_ptr(localHandle);
      const memBuf = localExports.memory.buffer;
      const localLeftView = new Float32Array(memBuf, localLPtr, FRAMES);
      const localRightView = new Float32Array(memBuf, localRPtr, FRAMES);

      // view 生成完了後の最終世代チェック
      if (myGen !== this.generation) {
        localExports.synth_free(localHandle);
        return;
      }

      // ===== ここから一括 commit。throw する処理を含めない =====
      this.exports = localExports;
      this.handlePtr = localHandle;
      this.lPtr = localLPtr;
      this.rPtr = localRPtr;
      this.cachedMemBuf = memBuf;
      this.leftView = localLeftView;
      this.rightView = localRightView;

      const ready: FromWorkletMessage = { type: 'ready' };
      this.port.postMessage(ready);
    } catch (e: any) {
      // 失敗時は local handle のみ解放（this には commit していないので触らない）
      if (localExports && localHandle !== 0) {
        try { localExports.synth_free(localHandle); } catch { /* noop */ }
      }
      // 世代が進んでいる（dispose 済み）なら error は送らない
      if (myGen !== this.generation) return;
      const err: FromWorkletMessage = { type: 'error', message: String(e) };
      this.port.postMessage(err);
    }
  }

  /** memory.buffer への view を作り直す。init 時と grow 検出時のみ呼ぶ */
  private refreshViews(): void {
    if (!this.exports) return;
    const memBuf = this.exports.memory.buffer;
    this.cachedMemBuf = memBuf;
    this.leftView = new Float32Array(memBuf, this.lPtr, FRAMES);
    this.rightView = new Float32Array(memBuf, this.rPtr, FRAMES);
  }

  private warnedFrameLength = false;

  process(_inputs: Float32Array[][], outputs: Float32Array[][]): boolean {
    if (!this.exports || this.handlePtr === 0) return true;
    const exports = this.exports;
    const out = outputs[0];

    // 128 frames 固定前提のガード。
    // 現行ブラウザは render quantum = 128 だが、MDN は将来可変になりうると警告している。
    // MVPでは 128 以外のブロックが来たら無音を返し、警告を1度だけ送る。
    if (out[0].length !== FRAMES) {
      if (!this.warnedFrameLength) {
        this.warnedFrameLength = true;
        const warn: FromWorkletMessage = {
          type: 'debug',
          message: `Unexpected render quantum: ${out[0].length} (expected ${FRAMES}). Output silenced.`,
        };
        this.port.postMessage(warn);
      }
      out[0].fill(0);
      if (out[1]) out[1].fill(0);
      return true;
    }

    exports.synth_process_block(this.handlePtr, FRAMES);

    // memory.grow が起きた場合のみ view を作り直す（通常は発生しない）
    if (this.cachedMemBuf !== exports.memory.buffer) {
      this.refreshViews();
    }

    if (this.leftView) out[0].set(this.leftView);
    if (out[1] && this.rightView) out[1].set(this.rightView);
    return true;
  }
}

registerProcessor('synth-processor', SynthProcessor);
```

### 重要な注意点

1. **C ABI に統一したことで、Worklet 側のラッパーは `WasmExports` 型を介した直接呼び出し**となる。`__wbindgen_*` import は一切不要で、import object は `{ env: {} }` で済む。
2. **Float32Array view は init 時にキャッシュ**し、`process()` 内では作り直さない。これにより音声スレッドでの JS オブジェクト生成を排除し、GC リスクを下げる。
3. **`memory.grow` の検出は `cachedMemBuf !== memory.buffer` チェックのみ**。grow が起きたときだけ `refreshViews()` を呼ぶ。Rust側で `synth_new` 以降の確保を排除しているため、通常は呼ばれない。
4. **export 名の確認**: `cargo build --release` 後に `wasm-objdump -x web/src/lib/wasm/wasm_audio.wasm | findstr Export` で `synth_new` 等が含まれることを確認する（[06章 Step 9](./06-build-and-verify.md)）。
5. **128 frames 固定の前提**: 現行ブラウザでは render quantum は事実上 128 で運用されているが、ブラウザ仕様上は可変になりうる。`process()` 冒頭で `outputs[0][0].length !== FRAMES` を判定し、想定外のブロック長が来たら無音を返して警告を 1 度だけ送る（永続ログを避けるため `warnedFrameLength` フラグで抑止）。将来可変対応するなら、`max_block_size` を初期化時に確保しておく現在の設計が既に拡張容易。
6. **init / dispose の非同期競合対策**: `port.onmessage` ハンドラは `onMessage` の Promise を await しない。よって `init` 処理中に `dispose` が来ると、`disposeWasm` が先に走った後で `initWasm` 完了側がフィールド代入してしまうレースが起きる。これを防ぐため `generation: number` カウンタを保持し、`disposeWasm` で `generation++`、`initWasm` 内では (a) `WebAssembly.instantiate` 直後、(b) `synth_new` 直後、(c) view 生成後の最終チェックの 3 箇所で `myGen !== this.generation` を判定。世代不一致なら作ったハンドルを `synth_free` で解放してから return する。
7. **double free / freed handle 利用の回避**: `initWasm` の準備（`synth_new` / `synth_out_*_ptr` / `Float32Array` 生成）はすべて **local 変数で完結** させ、最後に一括でフィールドへ commit する。commit 後に throw しうる処理を含めないことで、catch 内では local handle だけを解放すればよい。`this.handlePtr` には commit 完了後の値しか入らないため、後続の `disposeWasm` が解放済みポインタを再 free することがない。

## ページ構成（`+page.svelte`）

```
┌────────────────────────────────────────────────┐
│  Physics-Base Synth                            │
├────────────────────────────────────────────────┤
│  [▶ Start Audio]    (ready/not-ready 表示)      │
│                                                │
│  MIDI Device: [▼ select... ]                   │
│                                                │
│  Damping:    [─────●──────] 0.996              │
│  Brightness: [────●───────] 0.50               │
│  Output:     [──●─────────] 0.80               │
│                                                │
│  ┌──┬──┬──┬──┬──┬──┬──┬──┐                    │
│  │A │S │D │F │G │H │J │K │  PCキーボード        │
│  └──┴──┴──┴──┴──┴──┴──┴──┘                    │
│                                                │
│  [Keyboard Component]                          │
│   ▼▼▼▼▼▼▼▼▼▼▼▼▼▼▼▼                            │
│  [white][white][white][white]...               │
│   [black][black]...(黒鍵)                       │
└────────────────────────────────────────────────┘
```

レイアウトは縦1列で十分。CSSは最小限（プレーンなフレックスレイアウト、Tailwind等は導入しない）。

`+page.svelte` の構造例:

```svelte
<script lang="ts">
  import { onDestroy } from 'svelte';
  import StartButton from '$lib/components/StartButton.svelte';
  import MidiSelect from '$lib/components/MidiSelect.svelte';
  import ParamSlider from '$lib/components/ParamSlider.svelte';
  import Keyboard from '$lib/components/Keyboard.svelte';
  import { pcKeyboard } from '$lib/actions/pc-keyboard.svelte';
  import { synth } from '$lib/state/synth.svelte';
  import { PARAM_IDS } from '$lib/audio/messages';

  // HMR や画面遷移で残骸 (rAF / AudioContext / WASM handle) を残さない
  onDestroy(() => {
    void synth.engine.dispose();
    synth.ready = false;
  });
</script>

<main use:pcKeyboard={{
  onNote: (m) => {
    if (m.type === 'on') synth.engine.noteOn(m.midi, m.velocity);
    else synth.engine.noteOff(m.midi);
  }
}}>
  <h1>Physics-Base Synth</h1>
  <StartButton />
  <MidiSelect />

  <ParamSlider
    label="Damping"
    paramId={PARAM_IDS.Damping}
    min={0.90} max={0.9999} step={0.0001}
    bind:value={synth.damping}
  />
  <ParamSlider
    label="Brightness"
    paramId={PARAM_IDS.Brightness}
    min={0} max={1} step={0.01}
    bind:value={synth.brightness}
  />
  <ParamSlider
    label="Output Gain"
    paramId={PARAM_IDS.OutputGain}
    min={0} max={1.5} step={0.01}
    bind:value={synth.outputGain}
  />

  <Keyboard />
</main>
```

> `bind:value={synth.damping}` のように `$state` プロパティに直接バインドできる。`ParamSlider` 側で `$bindable()` を使っているため双方向バインディングが成立する。

## コンポーネント仕様

### `StartButton.svelte`

```svelte
<script lang="ts">
  import { synth } from '$lib/state/synth.svelte';

  let starting = $state(false);
  let error = $state<string | null>(null);

  async function start() {
    if (synth.ready) return;
    starting = true;
    error = null;
    try {
      // SynthEngine.start() は Worklet の ready 受信まで resolve しない（タイムアウト 5s）
      // 失敗時は内部で dispose 済みなので、再度ボタンを押せば再試行できる
      await synth.engine.start();
      synth.ready = true;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      synth.ready = false;
    } finally {
      starting = false;
    }
  }
</script>

<button onclick={start} disabled={starting || synth.ready}>
  {#if starting}
    Starting...
  {:else if synth.ready}
    ✓ Audio Ready
  {:else}
    ▶ Start Audio
  {/if}
</button>
{#if error}
  <small style="color: red">{error}</small>
{/if}
```

iOS Safari 対策（D5）として、初回のユーザージェスチャ（`onclick` ハンドラ）の中で `new AudioContext()` と `await ctx.resume()` の両方を実行する（`SynthEngine.start()` 内に集約）。`await synth.engine.start()` は Worklet 初期化完了（`ready` メッセージ受信）まで resolve しないため、その後で `synth.ready = true` をセットすれば UI は実際に音が鳴る状態と同期する。

### `Keyboard.svelte`

```svelte
<script lang="ts">
  import { synth } from '$lib/state/synth.svelte';

  // C3〜C5 の2オクターブを表示（MIDI 48〜72）
  const startNote = 48;
  const endNote = 72;
  const notes = Array.from({ length: endNote - startNote + 1 }, (_, i) => startNote + i);

  function isBlack(midi: number): boolean {
    return [1, 3, 6, 8, 10].includes(midi % 12);
  }

  function down(e: PointerEvent, midi: number) {
    e.preventDefault();
    synth.engine.noteOn(midi, 0.8);
  }
  function up(e: PointerEvent, midi: number) {
    e.preventDefault();
    synth.engine.noteOff(midi);
  }
</script>

<div class="keyboard">
  {#each notes as midi (midi)}
    <button
      class:black={isBlack(midi)}
      onpointerdown={(e) => down(e, midi)}
      onpointerup={(e) => up(e, midi)}
      onpointerleave={(e) => up(e, midi)}
    >
      {midi}
    </button>
  {/each}
</div>
```

Svelte 5 では `|preventDefault` 修飾子が廃止されているため、ハンドラ内で `e.preventDefault()` を明示的に呼ぶ。`pointerdown`/`pointerup`/`pointerleave` の小文字記法でマウス・タッチ両対応。

### `ParamSlider.svelte`

```svelte
<script lang="ts">
  import { synth } from '$lib/state/synth.svelte';

  type Props = {
    label: string;
    paramId: number;
    min: number;
    max: number;
    step: number;
    value: number;  // bindable
  };

  let { label, paramId, min, max, step, value = $bindable() }: Props = $props();

  function onInput(e: Event) {
    const v = parseFloat((e.target as HTMLInputElement).value);
    value = v;
    synth.engine.setParam(paramId, v);
  }
</script>

<label>
  {label}: <span>{value.toFixed(3)}</span>
  <input type="range" {min} {max} {step} {value} oninput={onInput} />
</label>
```

`$props()` でプロパティを宣言し、`$bindable()` で `value` を呼び出し側から `bind:value` 可能にする。`engine.setParam` 内部で rAF スロットルされるため、`oninput` の頻発でも問題ない。

### `MidiSelect.svelte`

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import {
    initMidi, disposeMidi, listInputs, setActiveInput, type MidiInput,
  } from '$lib/input/midi';
  import { synth } from '$lib/state/synth.svelte';

  let supported = $state(false);
  let inputs: MidiInput[] = $state([]);
  let selectedId: string | null = $state(null);

  // 一度だけ走る非同期初期化は onMount に置く（$effect は再実行ループのリスクがあるため避ける）
  onMount(() => {
    supported = 'requestMIDIAccess' in navigator && (window?.isSecureContext ?? false);
    if (!supported) return;

    // alive flag: アンマウント後の resolve / reject で破棄済み state を更新するのを防ぐ
    let alive = true;

    initMidi((msg) => {
      if (msg.type === 'on') synth.engine.noteOn(msg.midi, msg.velocity);
      else synth.engine.noteOff(msg.midi);
    })
      .then(() => {
        if (!alive) return;
        inputs = listInputs();
      })
      .catch((e) => {
        if (!alive) return;
        // ユーザーが MIDI 権限を拒否した、もしくは API が失敗した場合
        console.warn('[MIDI] init failed:', e);
        supported = false;
      });

    // HMR / 画面遷移時に listener を解除
    return () => {
      alive = false;
      disposeMidi();
    };
  });

  // selectedId の変化のみを追跡する $effect（純粋にリアクティブな反映のみ）
  $effect(() => {
    setActiveInput(selectedId);
  });
</script>

{#if supported}
  <label>
    MIDI Device:
    <select bind:value={selectedId}>
      <option value={null}>(all inputs)</option>
      {#each inputs as i (i.id)}
        <option value={i.id}>{i.name}</option>
      {/each}
    </select>
  </label>
{:else}
  <small>Web MIDI is unavailable. Requires HTTPS/localhost and Chrome/Edge (or Firefox 126+).</small>
{/if}
```

役割を明確に分離:
- **`onMount` + cleanup return**: 1度だけ走らせたい非同期初期化と、HMR/画面遷移時の listener 解除
- **`$effect`**: `selectedId` のリアクティブな反映のみ（状態を読むだけで更新はしない）

> `$effect` 内での状態更新（特に非同期初期化の中で複数の `$state` を書き換える）は再実行ループや読み順依存の不具合を生みやすい。Svelte 5 でも `onMount` は引き続き有効で、初期化系には推奨されるパターン。
>
> Web MIDI API は **secure context 必須**かつ Safari/Firefox では限定対応（Firefox 126+ で対応開始）。MVPでは secure context チェックも `supported` 判定に含める。`(none)` ではなく `(all inputs)` とし、MVPの既定動作（全入力購読）を明示する。

## 演奏入力詳細

### PCキーボード Svelte action（`actions/pc-keyboard.svelte.ts`）

`event.code` を使い、IME や物理レイアウトの影響を受けないようにする。配置は Ableton Live / Logic Pro と類似:

| MIDI Note | 音名 | `event.code` |
|---|---|---|
| 60 | C4 | `KeyA` |
| 61 | C#4 | `KeyW` |
| 62 | D4 | `KeyS` |
| 63 | D#4 | `KeyE` |
| 64 | E4 | `KeyD` |
| 65 | F4 | `KeyF` |
| 66 | F#4 | `KeyT` |
| 67 | G4 | `KeyG` |
| 68 | G#4 | `KeyY` |
| 69 | A4 | `KeyH` |
| 70 | A#4 | `KeyU` |
| 71 | B4 | `KeyJ` |
| 72 | C5 | `KeyK` |
| 73 | C#5 | `KeyO` |
| 74 | D5 | `KeyL` |

副作用（window への listener attach）を Svelte 5 の **`$effect` ベース action** でカプセル化する。`$effect` は runes なので、ファイルは `.svelte.ts` 拡張子にする。

```typescript
// src/lib/actions/pc-keyboard.svelte.ts
import type { Action } from 'svelte/action';

const MAPPING: Record<string, number> = {
  KeyA: 60, KeyW: 61, KeyS: 62, KeyE: 63, KeyD: 64,
  KeyF: 65, KeyT: 66, KeyG: 67, KeyY: 68, KeyH: 69,
  KeyU: 70, KeyJ: 71, KeyK: 72, KeyO: 73, KeyL: 74,
};

export type PcKeyboardNote =
  | { type: 'on'; midi: number; velocity: number }
  | { type: 'off'; midi: number };

export interface PcKeyboardParams {
  onNote: (msg: PcKeyboardNote) => void;
}

/**
 * Svelte 5 action（$effect ベース）。
 * window に keydown/keyup を attach し、MAPPING のキーで MIDI ノートイベントを発行する。
 *
 * 使い方:
 *   <div use:pcKeyboard={{ onNote: (m) => ... }}>...</div>
 */
export const pcKeyboard: Action<HTMLElement, PcKeyboardParams> = (_node, params) => {
  // 注意: Svelte 5 では action 関数自体は要素マウント時に1度だけ呼ばれる。
  // 引数 `params` の変更で再呼び出しはされないため、ハンドラは初回の `params.onNote`
  // を保持したまま動く。MVPでは `params.onNote` 内の `synth.engine` は安定参照なので
  // 問題にならない。動的に差し替える必要が出てきたら、`$state` で包んだ holder を
  // 渡すか attachment（Svelte 5.29+）への移行を検討する。
  $effect(() => {
    const heldKeys = new Set<string>();

    const onDown = (e: KeyboardEvent) => {
      if (e.repeat) return;
      const midi = MAPPING[e.code];
      if (midi === undefined) return;
      if (heldKeys.has(e.code)) return;
      heldKeys.add(e.code);
      params.onNote({ type: 'on', midi, velocity: 0.8 });
    };
    const onUp = (e: KeyboardEvent) => {
      const midi = MAPPING[e.code];
      if (midi === undefined) return;
      heldKeys.delete(e.code);
      params.onNote({ type: 'off', midi });
    };

    window.addEventListener('keydown', onDown);
    window.addEventListener('keyup', onUp);

    // $effect の戻り値はクリーンアップ。要素アンマウント時に自動実行
    return () => {
      window.removeEventListener('keydown', onDown);
      window.removeEventListener('keyup', onUp);
      heldKeys.clear();
    };
  });
};
```

`+page.svelte` 側の使い方:

```svelte
<script lang="ts">
  import { pcKeyboard } from '$lib/actions/pc-keyboard.svelte';
  import { synth } from '$lib/state/synth.svelte';
</script>

<main use:pcKeyboard={{
  onNote: (m) => {
    if (m.type === 'on') synth.engine.noteOn(m.midi, m.velocity);
    else synth.engine.noteOff(m.midi);
  }
}}>
  <!-- ... -->
</main>
```

> Svelte 5.29+ では `@attach` 構文（attachments）も使えるが、MVP では action API で十分。`$effect` ベースの action は旧来の `update`/`destroy` を返す形式より簡潔で、Svelte の現在の docs で推奨されるパターン。
>
> **注意**: ファイルは `.svelte.ts` 拡張子（runes 使用のため）。`.ts` だとコンパイルエラー。

### Web MIDI（`midi.ts`）

```typescript
export type MidiInput = { id: string; name: string };
export type MidiNoteMessage =
  | { type: 'on'; midi: number; velocity: number }
  | { type: 'off'; midi: number };

let access: MIDIAccess | null = null;
let listener: ((msg: MidiNoteMessage) => void) | null = null;
let activeInputId: string | null = null;  // null = 全入力購読

export async function initMidi(onNote: (msg: MidiNoteMessage) => void): Promise<void> {
  if (!('requestMIDIAccess' in navigator)) {
    throw new Error('Web MIDI not supported');
  }
  listener = onNote;
  access = await navigator.requestMIDIAccess({ sysex: false });
  for (const input of access.inputs.values()) {
    input.onmidimessage = handleMidi;
  }
  // 後から接続されたデバイスにも反応
  access.onstatechange = (e) => {
    const port = e.port;
    if (port?.type === 'input' && port.state === 'connected') {
      (port as MIDIInput).onmidimessage = handleMidi;
    }
  };
}

/** すべての onmidimessage / onstatechange を解除する。HMR や画面遷移時に呼ぶ */
export function disposeMidi(): void {
  if (!access) return;
  for (const input of access.inputs.values()) {
    input.onmidimessage = null;
  }
  access.onstatechange = null;
  access = null;
  listener = null;
  activeInputId = null;
}

/** UIで選択されたデバイスを設定。null で全入力購読（MVP既定） */
export function setActiveInput(id: string | null): void {
  activeInputId = id;
}

function handleMidi(e: MIDIMessageEvent) {
  if (!listener) return;
  const port = e.target as MIDIInput | null;
  // selectedId で絞り込み（null は全入力許可）
  if (activeInputId !== null && port?.id !== activeInputId) return;

  const [status, data1, data2] = e.data;
  const cmd = status & 0xf0;
  if (cmd === 0x90 && data2 > 0) {
    listener({ type: 'on', midi: data1, velocity: data2 / 127 });
  } else if (cmd === 0x80 || (cmd === 0x90 && data2 === 0)) {
    listener({ type: 'off', midi: data1 });
  }
  // pitch bend / CC は MVP では無視
}

export function listInputs(): MidiInput[] {
  if (!access) return [];
  return Array.from(access.inputs.values()).map((i) => ({ id: i.id, name: i.name ?? 'Unknown' }));
}
```

> **注**: Web MIDI API は **secure context 必須**で、Limited availability（MDN）。Safari は **macOS Safari 18.4 以降** で部分対応、iOS Safari は未対応のブラウザもある。Firefox は **126以降** で対応開始。MVPでは Chrome/Edge を推奨ブラウザとし、未対応時は `MidiSelect.svelte` のフォールバックメッセージで案内する。MVP既定では全入力を購読し、ドロップダウンで特定デバイスに絞り込めるようにする（`setActiveInput`）。

### `note-utils.ts`

```typescript
export function midiToFreq(midi: number): number {
  return 440 * Math.pow(2, (midi - 69) / 12);
}
```

dsp-core 側でも同じ計算を行うため、JS側で使うのは画面表示用のみ。MVPでは利用箇所を最小化（必要に応じて使う）。

## 共有ステート（`state/synth.svelte.ts`）

Svelte 5 では旧来の `writable` ストアではなく、`.svelte.ts` 拡張子のモジュール内で `$state` を露出するパターンが推奨される。コンポーネント間で共有したいリアクティブな値はクラスインスタンスのプロパティとして定義し、シングルトンを export する。

```typescript
// src/lib/state/synth.svelte.ts
import { SynthEngine } from '$lib/audio/engine';

class SynthState {
  readonly engine = new SynthEngine();
  ready = $state(false);
  damping = $state(0.996);
  brightness = $state(0.5);
  outputGain = $state(0.8);
}

export const synth = new SynthState();
```

> **拡張子に注意**: `.svelte.ts` でないと `$state` などのrunesがコンパイルエラーになる。
>
> `synth.ready` の更新タイミング: `SynthEngine.start()` は Worklet の `ready` メッセージ受信まで resolve しない設計（[High 2 対応済み](#メインスレッド側-synthengineenginets)）。したがって `StartButton.svelte` 内で `await synth.engine.start()` の直後に `synth.ready = true` を代入すれば、UI と実際の Worklet 状態が同期する。

利用側の例（コンポーネント内）:

```svelte
<script lang="ts">
  import { synth } from '$lib/state/synth.svelte';

  // 直接プロパティアクセスで読み書き
  synth.engine.noteOn(60, 0.8);
  console.log(synth.ready);
</script>

<p>Damping: {synth.damping}</p>
```

## パラメータ送信のスロットリング方針

| 項目 | 方針 |
|---|---|
| スロットルの場所 | メインスレッド側 `SynthEngine.setParam` 内 |
| 周期 | `requestAnimationFrame`（約60Hz） |
| 値の合算 | `Map<paramId, value>` で **同一IDは最後の値で上書き**（最新値だけが届く） |
| Worklet 側のクリック対策 | dsp-core `SmoothedValue`（時定数20msなど）で別途吸収 |

これにより、UIスライダーの `oninput` ハンドラが60Hz超で発火しても、Worklet の `process` 中で行うパラメータ更新は1ブロック（128 frames = 約2.7ms@48kHz）あたり最大1回となる。

## 本番ビルドでの調整事項

MVPの dev/build 両対応のため、以下を実装段階で確認する:

1. **WASMパス**: 上記 `engine.ts` の `import wasmUrl from '$lib/wasm/wasm_audio.wasm?url'` で dev/build を統一。生パス直接 fetch は使わない。
2. **Worklet パス**: `web/static/worklet/synth-processor.js` に配置し、コードでは `${base}/worklet/synth-processor.js`（`$app/paths` の `base`）で参照する。`?url` インポートは asset graph 用なので static 配下のファイルには使わない。
3. **`adapter-static` の prerender**: ページ内で SSR 時にブラウザAPI（`AudioContext` 等）を参照しないこと。`onMount` 内のみで参照する。
4. **`pnpm build` 後の検証**: `pnpm --filter web preview` で本番バンドルでも音が鳴ることを確認する（[06章 検証手順](./06-build-and-verify.md)）。

## 詰まりやすい箇所まとめ

1. **C ABI export 名の確認**: `cargo build --target wasm32-unknown-unknown` 後、`wasm-objdump -x web/src/lib/wasm/wasm_audio.wasm | findstr Export` で `synth_new`、`synth_process_block` 等が含まれることを確認。`#[no_mangle]` 忘れで mangling されていないか要チェック。
2. **AudioContext.sampleRate と Rust側 prepare の整合**: Worklet の `init` メッセージで sampleRate を必ず渡す。MVPでは init 後に sampleRate が変わらない前提。
3. **secure context 要件**: `AudioWorklet` と `Web MIDI` は secure context 必須。`localhost` または HTTPS でのみ動作。LAN IP の HTTP では動かない（[06章 F9](./06-build-and-verify.md)）。
4. **iOS Safari の `AudioContext` 作成タイミング**: ボタンクリック等のユーザージェスチャ内で `new AudioContext()` を呼ぶ。`$effect` 内で作るとブロックされる。
5. **Vite dev server で `static/worklet/synth-processor.js` が更新されない**: `pnpm build:worklet` を `dev` の前段に必ず実行する。`watch` モード化は MVP では不要。
6. **`@sveltejs/adapter-static` の `fallback`**: SPAルーティング用に `index.html` を fallback に指定すること。直リンクで404にならないように。
7. **Float32Array view を毎フレーム作らない**: `process()` 内での新規 `Float32Array(memory.buffer, ptr, frames)` は GC圧の原因。init 時にキャッシュし、`memory.buffer` 変化時のみ再作成する。
8. **Svelte 5 の `.svelte.ts` 拡張子**: `state/synth.svelte.ts` のように `.svelte.ts` を付けないと `$state` 等の runes がコンパイルエラーになる。普通の `.ts` では runes は使えない。
9. **Svelte 5 でのイベント修飾子廃止**: `on:click|preventDefault` は使えない。`onclick={(e) => { e.preventDefault(); ... }}` のように手書きする。
10. **`$props()` と `$bindable()` の組み合わせ**: 親から `bind:value` する prop は子側で `$bindable()` を付けないと双方向バインドにならない。
