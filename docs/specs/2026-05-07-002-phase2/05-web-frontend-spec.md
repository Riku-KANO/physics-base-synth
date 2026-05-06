# 05. Phase 2 Web フロントエンド仕様

## 目的

Phase 1 [05 章 Web フロントエンド仕様](../2026-05-06-001-mvp/05-web-frontend-spec.md) を起点に、Phase 2 で発生する **フロントエンド側の差分**（ParamDescriptor 生成物の import 経路、ParamSlider の descriptor 駆動化、`WasmExports` interface の `synth_set_polyphony_mode` 追加、`messages.ts` 改修）を確定する。Svelte 5 runes / SvelteKit 静的ビルド / esbuild Worklet バンドルの構成は Phase 1 から完全継承する。

## 他文書との関係

- 上流: [`02-architecture.md`](./02-architecture.md)（ParamDescriptor codegen、モノレポ構成変化）、[`04-wasm-audio-spec.md`](./04-wasm-audio-spec.md)（C ABI 互換 + 新 export）
- 下流: [`06-build-and-verify.md`](./06-build-and-verify.md)（実行手順、F10〜F25 検証）
- 参考: [Phase 1 05 章](../2026-05-06-001-mvp/05-web-frontend-spec.md)（SvelteKit セットアップ、SynthEngine、AudioWorkletProcessor、コンポーネント仕様、共有ステート、PCキーボード/Web MIDI 入力、パラメータスロットリング、本番ビルド）— **本書で明示的に変更しない部分はすべて Phase 1 の記述を継承**

## SvelteKit セットアップ

[Phase 1 05 章 §SvelteKit セットアップ](../2026-05-06-001-mvp/05-web-frontend-spec.md#sveltekit-セットアップ) を **完全維持**:

- Svelte 5、Runes ベース（D11）
- `@sveltejs/adapter-static`、`prerender: { entries: ['*'] }`
- `svelte.config.js` / `vite.config.ts` / `tsconfig.json` 変更なし
- `web/package.json` の esbuild Worklet ビルドスクリプトも変更なし

## ファイルレイアウト（Phase 2 差分）

[Phase 1 05 章 §ファイルレイアウト](../2026-05-06-001-mvp/05-web-frontend-spec.md#ファイルレイアウト再掲) に **以下を追加**:

```
web\src\lib\audio\
├── engine.ts                          # 既存、Phase 1 維持（変更なし）
├── synth-processor.ts                 # WasmExports に synth_set_polyphony_mode 追加
├── messages.ts                        # PARAM_IDS / PARAM_DESCRIPTORS を generated/ から re-export
├── wasm-loader.ts                     # 既存、Phase 1 維持
└── generated\                         ← Phase 2 新規ディレクトリ
    └── params.ts                      ← scripts/gen-params.mjs 出力（git commit、D25）
```

`generated/` ディレクトリ全体が `params.json` 駆動の生成物。手動編集は禁止。

## ParamDescriptor 生成物の import 経路

### `web/src/lib/audio/generated/params.ts` の出力例

`scripts/gen-params.mjs` が以下のような TypeScript ソースを出力（[02 章](./02-architecture.md#paramdescriptor-コード生成パイプライン)）:

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
  OutputGain: 2
} as const;

export type ParamIdValue = (typeof PARAM_IDS)[keyof typeof PARAM_IDS];

export const PARAM_DESCRIPTORS: readonly ParamDescriptor[] = [
  { id: 0, name: 'Damping',    min: 0.90, max: 0.9999, default: 0.996, smoothingTau: 0.02 },
  { id: 1, name: 'Brightness', min: 0.0,  max: 1.0,    default: 0.5,   smoothingTau: 0.02 },
  { id: 2, name: 'OutputGain', min: 0.0,  max: 1.5,    default: 0.8,   smoothingTau: 0.01 }
] as const;

export function getDescriptor(id: ParamIdValue): ParamDescriptor {
  return PARAM_DESCRIPTORS[id];
}

export function clampValue(id: ParamIdValue, value: number): number {
  const d = PARAM_DESCRIPTORS[id];
  return value < d.min ? d.min : value > d.max ? d.max : value;
}
```

### `messages.ts` の変更点

```typescript
// web/src/lib/audio/messages.ts (Phase 2)

// PARAM_IDS / PARAM_DESCRIPTORS / 型は generated から re-export
export {
  PARAM_IDS,
  PARAM_DESCRIPTORS,
  getDescriptor,
  clampValue,
  type ParamIdValue,
  type ParamDescriptor
} from './generated/params';

// メッセージ型は手書きを維持（生成しない）
export type ToWorkletMessage =
  | { type: 'init'; wasmBytes: ArrayBuffer; sampleRate: number }
  | { type: 'noteOn'; midi: number; velocity: number }
  | { type: 'noteOff'; midi: number }
  | { type: 'setParam'; id: number; value: number }
  | { type: 'setMode'; mode: 'poly' | 'mono' }   // Phase 2 追加（D17）
  | { type: 'reset' }
  | { type: 'dispose' };

export type FromWorkletMessage =
  | { type: 'ready' }
  | { type: 'error'; message: string }
  | { type: 'debug'; message: string };
```

### Phase 1 との差分

| 項目 | Phase 1 | Phase 2 |
|---|---|---|
| `PARAM_IDS` の定義場所 | `messages.ts` に手書き | `generated/params.ts` に自動生成、`messages.ts` で re-export |
| `PARAM_DESCRIPTORS` | （存在しない） | **新規**（generated/）|
| `ToWorkletMessage` | 6 variants | **+1 variant** (`setMode`)、Phase 2 では UI から呼ばないが内部 API として存在 |
| `getDescriptor` / `clampValue` | （存在しない） | **新規**（generated/、UI とエンジンの両方で使用可）|

### 既存コードへの影響

`web/src/lib/audio/messages.ts` から `PARAM_IDS` を import している箇所（`+page.svelte` の `<ParamSlider paramId={PARAM_IDS.Damping} ... />` 等）は **import パスを変更不要**。re-export で透過的に利用できる。

## ParamSlider の descriptor 駆動化

### Phase 1 ParamSlider

```svelte
<!-- Phase 1: min / max / step / paramId をすべて props で個別指定 -->
<ParamSlider
  label="Damping"
  paramId={PARAM_IDS.Damping}
  min={0.90} max={0.9999} step={0.0001}
  bind:value={synth.damping}
/>
```

### Phase 2 ParamSlider（descriptor 駆動）

`getDescriptor(paramId)` から min / max を取得することで、`min` / `max` の二重管理を解消する。`step` は Phase 2 では UI 側にハードコード（descriptor の `step` フィールドを Phase 2 では追加せず、Phase 3 で必要時に拡張、[`02-architecture.md` §params.json](./02-architecture.md#単一ソース-paramsjson) 参照）。

```svelte
<!-- web/src/lib/components/ParamSlider.svelte (Phase 2 改修後) -->
<script lang="ts">
  import { synth } from '$lib/state/synth.svelte';
  import { getDescriptor, type ParamIdValue } from '$lib/audio/messages';

  type Props = {
    label: string;
    paramId: ParamIdValue;
    step: number;
    value: number;
  };

  let { label, paramId, step, value = $bindable() }: Props = $props();

  // descriptor から min / max を取得
  const descriptor = getDescriptor(paramId);

  function onInput(e: Event) {
    const v = parseFloat((e.target as HTMLInputElement).value);
    value = v;
    synth.engine.setParam(paramId, v);
  }
</script>

<label>
  {label}: <span>{value.toFixed(3)}</span>
  <input type="range" min={descriptor.min} max={descriptor.max} {step} {value} oninput={onInput} />
</label>
```

### 利用側の変更（`+page.svelte`）

```svelte
<!-- Phase 2: min / max props は不要に -->
<ParamSlider label="Damping"     paramId={PARAM_IDS.Damping}    step={0.0001} bind:value={synth.damping} />
<ParamSlider label="Brightness"  paramId={PARAM_IDS.Brightness} step={0.01}   bind:value={synth.brightness} />
<ParamSlider label="Output Gain" paramId={PARAM_IDS.OutputGain} step={0.01}   bind:value={synth.outputGain} />
```

これで `min` / `max` を変更したいときは `params.json` を 1 箇所修正して `pnpm gen:params` を実行すれば、Rust と TS 双方が更新され UI も追従する（D14 / D15）。

### 実装上の注意

- `descriptor` は const なので reactive ではない。`paramId` が変わらない前提で OK（Phase 2 では各 ParamSlider インスタンスで `paramId` は固定）
- `step` を `params.json` に追加するかは Phase 2 では見送り。Damping は 0.0001、Brightness/OutputGain は 0.01 で UI 側ハードコード継続

## AudioWorkletProcessor（synth-processor.ts）の差分

### `WasmExports` interface の拡張

[Phase 1 05 章](../2026-05-06-001-mvp/05-web-frontend-spec.md#audioworkletprocessorsynth-processorts) の interface に `synth_set_polyphony_mode` を追加:

```typescript
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
  // Phase 2 追加（D17）
  synth_set_polyphony_mode: (ptr: number, mode: number) => void;
}
```

### `onMessage` の `setMode` ケース追加

```typescript
private async onMessage(msg: ToWorkletMessage): Promise<void> {
  switch (msg.type) {
    case 'init':
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
    case 'setMode':  // Phase 2 追加
      this.exports?.synth_set_polyphony_mode(this.handlePtr, msg.mode === 'mono' ? 1 : 0);
      break;
    case 'reset':
      this.exports?.synth_reset(this.handlePtr);
      break;
    case 'dispose':
      this.disposeWasm();
      break;
  }
}
```

### Phase 1 との互換性

- `process()` 本体は変更なし（128 frames 出力スクラッチへの書き込みと set）
- Float32Array view キャッシュは Phase 1 と同じ
- generation race 対策（dispose / init の非同期競合）は Phase 1 と同じ
- memory.grow 検出時の view 再作成も Phase 1 と同じ
- 128 frames 固定ガードも Phase 1 と同じ

## メインスレッド側 `SynthEngine`（engine.ts）

### Phase 1 から完全維持

[Phase 1 05 章 §メインスレッド側 SynthEngine](../2026-05-06-001-mvp/05-web-frontend-spec.md#メインスレッド側-synthengineenginets) の以下はすべて維持:

- `start()` の起動シーケンス（secure context チェック、AudioContext 作成、resume、addModule、fetch wasm bytes、init post、ready 待ち、5 秒タイムアウト、失敗時 dispose）
- `noteOn` / `noteOff` / `setParam` の API
- rAF スロットル（currentParams + pendingParams）
- `dispose()` のリソース解放
- `_readyHandlers` の generation 管理

### Phase 2 で追加するメソッド（D21 / D22 / 検証用 dev-only API）

Phase 2 では UI トグルを出さないが、検証（F18 / F20）と将来の拡張のために以下の API を提供する。

#### 本番＋dev 両方で利用可能（プロダクションビルドにも含まれる）

```typescript
class SynthEngine {
  // ... Phase 1 既存実装

  /**
   * モノ / ポリ切替。Phase 2 では UI からは呼ばないが Worklet → wasm-audio に setMode を送る
   * 内部 API として提供（D17 / D21）。Phase 3 で UI トグルを追加するか、E2E テストから呼ぶ用途。
   */
  setMode(mode: 'poly' | 'mono'): void {
    if (!this.ready) return;
    this.post({ type: 'setMode', mode });
  }
}
```

#### dev ビルドのみ（`import.meta.env.DEV` ガード、本番では tree-shake で除去）

検証手順（F18 / F20）で必要となる「mono モード切替」を **dev ビルド時のみグローバル公開** する。Vite の `import.meta.env.DEV` は production build で `false` 定数評価され、tree-shaking で本番バンドルから完全除去されるため、本番に診断 API が漏れない。

```typescript
// web/src/lib/state/synth.svelte.ts (Phase 2 改修後の末尾に追加)
if (import.meta.env.DEV) {
  // dev ビルド時のみ window に診断 API を生やす。本番ビルドでは tree-shake で消える
  type DevDiagnostics = {
    setMode: (mode: 'poly' | 'mono') => void;
  };
  (window as unknown as { __synthDev?: DevDiagnostics }).__synthDev = {
    setMode: (mode) => synth.engine.setMode(mode),
  };
}
```

DevTools Console から `__synthDev.setMode('mono')` で mono 切替、検証完了後は console から離れるだけで OK（コードに残っても本番に出ない）。

> **active voice count の実機観測について**: Phase 2 では active voice count の Worklet → main の round-trip API は提供しない（D17 で C ABI への voice count export を追加せず、メッセージ型も簡潔に保つため）。F10 / F11 の検証は (1) `cargo test test_voice_pool_allocates_distinct_voices` / `test_voice_pool_steals_quietest` で内部状態を確認、(2) ブラウザでの聴感確認の 2 段で行う。Phase 3 で UI に voice meter を追加する時点で `synth_active_voice_count` C ABI と `queryActiveVoiceCount` メッセージを正式追加検討。

呼び出されない場合、デフォルトは `Engine::new()` の `SynthMode::Poly`（[`03-dsp-core-spec.md`](./03-dsp-core-spec.md#engineenginers-の-phase-2-版)）。

## 共有ステート（state/synth.svelte.ts）

### Phase 1 から維持

[Phase 1 05 章 §共有ステート](../2026-05-06-001-mvp/05-web-frontend-spec.md#共有ステートstatesynthsveltets) の `SynthState` クラスは Phase 2 でも以下を維持:

```typescript
class SynthState {
  readonly engine = new SynthEngine();
  ready = $state(false);
  damping = $state(PARAM_DESCRIPTORS[0].default);    // Phase 2: descriptor から default を取得
  brightness = $state(PARAM_DESCRIPTORS[1].default);
  outputGain = $state(PARAM_DESCRIPTORS[2].default);
}

export const synth = new SynthState();
```

### Phase 2 差分

- 初期値を `PARAM_DESCRIPTORS[i].default` から取得することで、`params.json` の変更が UI 初期値にも反映される
- `mode` / `activeVoices` フィールドは **追加しない**（D21 / D22、UI 出さない）

### Phase 1 既存値からの変化

| フィールド | Phase 1 値 | Phase 2 値 | 取得元 |
|---|---|---|---|
| `damping` | `$state(0.996)` | `$state(PARAM_DESCRIPTORS[0].default)` = 0.996 | generated |
| `brightness` | `$state(0.5)` | `$state(PARAM_DESCRIPTORS[1].default)` = 0.5 | generated |
| `outputGain` | `$state(0.8)` | `$state(PARAM_DESCRIPTORS[2].default)` = 0.8 | generated |

数値自体は同じだが、`params.json` 単一ソース化により UI 側のハードコードが消える。

## ページ構成（+page.svelte）

[Phase 1 05 章 §ページ構成](../2026-05-06-001-mvp/05-web-frontend-spec.md#ページ構成pagesvelte) を **完全維持**。レイアウト・コンポーネント配置は Phase 1 と同じ。

### Phase 2 差分

`<ParamSlider>` 呼び出しから `min` / `max` 属性が消える（descriptor 駆動化、上記 §ParamSlider の descriptor 駆動化）。それ以外（`StartButton` / `MidiSelect` / `Keyboard` / `pcKeyboard` action / `onDestroy` の dispose）は変更なし。

## コンポーネント仕様（Phase 2 差分のみ）

### `StartButton.svelte`

[Phase 1 05 章 §StartButton](../2026-05-06-001-mvp/05-web-frontend-spec.md#startbuttonsvelte) を **完全維持**。Phase 2 で追加機能なし。

### `Keyboard.svelte`

[Phase 1 05 章 §Keyboard](../2026-05-06-001-mvp/05-web-frontend-spec.md#keyboardsvelte) を **完全維持**。

### `ParamSlider.svelte`

上記 §ParamSlider の descriptor 駆動化に従い、`min` / `max` props を削除し `descriptor` 参照に変更。

### `MidiSelect.svelte`

[Phase 1 05 章 §MidiSelect](../2026-05-06-001-mvp/05-web-frontend-spec.md#midiselectsvelte) を **完全維持**。Phase 2 で追加機能なし。

## 演奏入力詳細

### PCキーボード Svelte action

[Phase 1 05 章 §PCキーボード Svelte action](../2026-05-06-001-mvp/05-web-frontend-spec.md#pcキーボード-svelte-actionactionspc-keyboardsveltets) を **完全維持**。Phase 2 でも 15 鍵マッピング（KeyA〜KeyL + KeyW〜KeyO）はそのまま。

### Phase 2 でのポリフォニー対応

PC キーボードの 15 鍵を **同時押下しても Phase 2 では正常に 8 ボイスまで重畳**する。Phase 1 では last-note simple が動作していたため、複数キー押下しても 1 音しか鳴らなかった（最後のキーのみ）。Phase 2 のポリモード（デフォルト）では、各キーが独立した VoicePool ボイスに割り当てられる。

| 状況 | Phase 1 挙動 | Phase 2 挙動（poly モード）|
|---|---|---|
| A 押下中に S を追加押下 | A は即破棄、S のみ発音 | A と S の両方が同時発音（2 ボイス）|
| 8 鍵同時押下 | 最後のキーのみ発音 | 8 ボイス全部が発音 |
| 9 鍵以降を追加 | 同上 | 9 鍵目以降は voice stealing で既存ボイスを置換 |

### Web MIDI

[Phase 1 05 章 §Web MIDI](../2026-05-06-001-mvp/05-web-frontend-spec.md#web-midimidits) を **完全維持**。Phase 2 でも `handleMidi` 内の note_on / note_off は単純に `synth.engine.noteOn` / `noteOff` を呼ぶだけで、ポリフォニー対応は wasm-audio / dsp-core 側で透過的に処理される。

### `note-utils.ts`

[Phase 1 05 章 §note-utils.ts](../2026-05-06-001-mvp/05-web-frontend-spec.md#note-utilsts) を **完全維持**。Phase 2 で追加機能なし。

## パラメータ送信のスロットリング方針

[Phase 1 05 章 §パラメータ送信のスロットリング方針](../2026-05-06-001-mvp/05-web-frontend-spec.md#パラメータ送信のスロットリング方針) を **完全維持**:

- スロットルの場所: `SynthEngine.setParam` 内
- 周期: rAF（約 60Hz）
- 値の合算: `Map<paramId, value>` で同一 ID は最後の値で上書き
- Worklet 側のクリック対策: dsp-core `SmoothedValue`

Phase 2 で setMode（`synth_set_polyphony_mode`）はスロットルしない（離散的なモード切替のため即時送信が望ましい）。

## 本番ビルドでの調整事項

[Phase 1 05 章 §本番ビルドでの調整事項](../2026-05-06-001-mvp/05-web-frontend-spec.md#本番ビルドでの調整事項) を継承し、Phase 2 で追加:

1. **WASMパス**: `import wasmUrl from '$lib/wasm/wasm_audio.wasm?url'` で dev/build 統一（Phase 1 同）
2. **Worklet パス**: `${base}/worklet/synth-processor.js`（Phase 1 同）
3. **adapter-static の prerender**: SSR 時にブラウザ API を参照しない（Phase 1 同）
4. **`pnpm build` 後の検証**: `pnpm --filter web preview` で本番バンドル動作確認（Phase 1 同）
5. **Phase 2 追加: 生成物 import パス**: `import { PARAM_IDS, PARAM_DESCRIPTORS } from '$lib/audio/messages'` のように既存パスを維持（messages.ts が generated から re-export するため）

## 詰まりやすい箇所まとめ

[Phase 1 05 章 §詰まりやすい箇所まとめ](../2026-05-06-001-mvp/05-web-frontend-spec.md#詰まりやすい箇所まとめ) の 10 件を継承し、Phase 2 で追加:

1. **C ABI export 名の確認**（Phase 1 R6 同、`synth_set_polyphony_mode` 追加検証）
2. **AudioContext.sampleRate と Rust 側 prepare の整合**（Phase 1 同）
3. **secure context 要件**（Phase 1 同）
4. **iOS Safari の AudioContext 作成タイミング**（Phase 1 同、D5 / D10）
5. **Vite dev server で worklet 更新されない**（Phase 1 同）
6. **adapter-static の fallback**（Phase 1 同）
7. **Float32Array view を毎フレーム作らない**（Phase 1 同、D9）
8. **Svelte 5 の `.svelte.ts` 拡張子**（Phase 1 同、D11）
9. **Svelte 5 のイベント修飾子廃止**（Phase 1 同、D11）
10. **`$props()` と `$bindable()` の組み合わせ**（Phase 1 同、D11）
11. **Phase 2 追加: ParamDescriptor 生成漏れ**: `params.json` を編集後に `pnpm gen:params` を実行し忘れると Rust と TS が drift する。`pnpm build:wasm` のチェーンに `gen:params` が組み込まれているため、`pnpm dev` 経由なら自動再生成されるが、手動で `cargo build` だけ実行すると古い `params.rs` のまま。`scripts/check-params-sync.mjs` を `pnpm check` で必ず走らせる
12. **Phase 2 追加: `params.json` 編集忘れ**: 新パラメータ追加時に Rust 側のみ手書きで追加すると、TS 側に存在しない ID を post してエラー。`params.json` を必ず単一ソースとして編集する
13. **Phase 2 追加: ポリフォニー時の音量バランス**: 8 音同時発音すると `1/sqrt(N)` スケールでも体感的に大きく感じる場合あり。`OutputGain` 初期値 0.8 が適切か実機検証（F24）
14. **Phase 2 追加: PC キーボード 8 鍵超え**: 9 鍵目以降は voice stealing で既存音が置き換わる。期待挙動と異なる場合は dsp-core の note_allocator を確認（F11）
