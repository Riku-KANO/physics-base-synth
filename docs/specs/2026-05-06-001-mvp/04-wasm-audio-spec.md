# 04. wasm-audio クレート仕様

## 目的

JavaScript（AudioWorklet）と `dsp-core` の橋渡しを行う `wasm-audio` クレートの構造、公開API、メモリ管理戦略を定義する。WASM linear memory を効率的に使い、`process_block` 呼び出しごとのオーバーヘッドを最小化する。

## 他文書との関係

- 上流: [`03-dsp-core-spec.md`](./03-dsp-core-spec.md)（呼び出される側）
- 下流: [`05-web-frontend-spec.md`](./05-web-frontend-spec.md)（呼び出し側 = AudioWorkletProcessor）
- 参考: pre-research 5.1（推奨構成）、5.2（128 sample render quantum）

## クレート設定

### `crates/wasm-audio/Cargo.toml`

```toml
[package]
name = "wasm-audio"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
dsp-core = { path = "../dsp-core" }
# wasm-bindgen は使わない（C ABI で安定したFFIを提供する。後述「設計判断」参照）

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
```

### 設計判断: wasm-bindgen を使わず C ABI で公開する

| アプローチ | 採用 | 理由 |
|---|---|---|
| **`#[no_mangle] extern "C"` の薄いC ABI**（採用） | ✓ | 生成されるWASM exportが安定。AudioWorklet内で生 export を直接呼び出す方式に最適。wasm-bindgenのバージョン依存・自動生成JSラッパー問題を完全に回避 |
| `#[wasm_bindgen]` を使い、生成JSラッパーをWorkletへinline | × | 生成JSが `import.meta.url`/`fetch` に依存しWorkletで動かないことが多い。バージョン整合の維持コストが高い |
| `#[wasm_bindgen]` を使い、生 export を直接呼ぶ | × | export 名が wasm-bindgen のバージョンや内部 mangling に依存し脆い |

C ABI 化することで、Worklet 側は `instance.exports.synth_new(...)` のように **公開した関数名そのまま** で呼べる。`__wbindgen_*` import も不要となり、`WebAssembly.instantiate` の `imports` オブジェクトを最小化できる。

## ビルド方法

`wasm-pack` は wasm-bindgen 前提のため使用しない。代わりに `cargo build` の生 WASM 出力を `wasm-opt` で最適化（任意）し、`web/src/lib/wasm/` へコピーする。

ルート `package.json` のスクリプト（[02章を更新](./02-architecture.md)）:

```json
{
  "scripts": {
    "build:wasm": "cargo build -p wasm-audio --target wasm32-unknown-unknown --release && node scripts/copy-wasm.mjs release && node scripts/check-wasm-exports.mjs",
    "build:wasm:dev": "cargo build -p wasm-audio --target wasm32-unknown-unknown && node scripts/copy-wasm.mjs debug && node scripts/check-wasm-exports.mjs"
  }
}
```

> `check-wasm-exports.mjs` の実装は [06章 export 名の自動検証スクリプト](./06-build-and-verify.md#export-名の自動検証スクリプト) 参照。`#[no_mangle]` 忘れや関数追加忘れをビルド時に検知する。

`scripts/copy-wasm.mjs`（最小実装の方針）:

```javascript
// プロジェクトルートに置く小さなコピースクリプト
import { copyFileSync, mkdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const profile = process.argv[2] === 'release' ? 'release' : 'debug';
const src = resolve(__dirname, `../target/wasm32-unknown-unknown/${profile}/wasm_audio.wasm`);
const dst = resolve(__dirname, '../web/src/lib/wasm/wasm_audio.wasm');
mkdirSync(dirname(dst), { recursive: true });
copyFileSync(src, dst);
console.log(`copied ${src} -> ${dst}`);
```

> 任意で `wasm-opt -O3` を release ビルド側で挟むとサイズが半分以下になる。MVP の必須ではない。

## 公開API（`crates/wasm-audio/src/lib.rs`）

```rust
use dsp_core::engine::Engine;

#[repr(C)]
pub struct SynthHandle {
    engine: Engine,
    scratch_l: Vec<f32>,
    scratch_r: Vec<f32>,
}

#[no_mangle]
pub extern "C" fn synth_new(sample_rate: f32, max_block_size: u32) -> *mut SynthHandle {
    let max = max_block_size as usize;
    let mut engine = Engine::new();
    engine.prepare(sample_rate, max);
    let handle = Box::new(SynthHandle {
        engine,
        scratch_l: vec![0.0; max],
        scratch_r: vec![0.0; max],
    });
    Box::into_raw(handle)
}

#[no_mangle]
pub extern "C" fn synth_free(handle: *mut SynthHandle) {
    if handle.is_null() { return; }
    unsafe { drop(Box::from_raw(handle)); }
}

#[no_mangle]
pub extern "C" fn synth_note_on(handle: *mut SynthHandle, midi_note: u8, velocity: f32) {
    if handle.is_null() { return; }
    let h = unsafe { &mut *handle };
    h.engine.note_on(midi_note, velocity);
}

#[no_mangle]
pub extern "C" fn synth_note_off(handle: *mut SynthHandle, midi_note: u8) {
    if handle.is_null() { return; }
    let h = unsafe { &mut *handle };
    h.engine.note_off(midi_note);
}

#[no_mangle]
pub extern "C" fn synth_set_param(handle: *mut SynthHandle, id: u32, value: f32) {
    if handle.is_null() { return; }
    let h = unsafe { &mut *handle };
    h.engine.set_param(id, value);
}

#[no_mangle]
pub extern "C" fn synth_reset(handle: *mut SynthHandle) {
    if handle.is_null() { return; }
    let h = unsafe { &mut *handle };
    h.engine.reset();
}

#[no_mangle]
pub extern "C" fn synth_out_l_ptr(handle: *const SynthHandle) -> *const f32 {
    if handle.is_null() { return core::ptr::null(); }
    let h = unsafe { &*handle };
    h.scratch_l.as_ptr()
}

#[no_mangle]
pub extern "C" fn synth_out_r_ptr(handle: *const SynthHandle) -> *const f32 {
    if handle.is_null() { return core::ptr::null(); }
    let h = unsafe { &*handle };
    h.scratch_r.as_ptr()
}

#[no_mangle]
pub extern "C" fn synth_capacity(handle: *const SynthHandle) -> u32 {
    if handle.is_null() { return 0; }
    let h = unsafe { &*handle };
    h.scratch_l.len() as u32
}

#[no_mangle]
pub extern "C" fn synth_process_block(handle: *mut SynthHandle, frames: u32) {
    if handle.is_null() { return; }
    let h = unsafe { &mut *handle };
    let n = (frames as usize).min(h.scratch_l.len());
    // scratch_l / scratch_r は別フィールドなので分割借用が成立する
    h.engine.process(&mut h.scratch_l[..n], &mut h.scratch_r[..n]);
}
```

### 公開関数一覧

| 関数 | シグネチャ | 役割 |
|---|---|---|
| `synth_new` | `(f32, u32) -> *mut SynthHandle` | ハンドル生成と prepare。最大ブロックサイズ分のスクラッチを確保 |
| `synth_free` | `(*mut SynthHandle)` | ハンドル破棄。`SynthEngine.dispose()`（[05章](./05-web-frontend-spec.md#メインスレッド側-synthengineenginets)）から `dispose` メッセージ経由で呼ばれる。HMR / 画面遷移 / start 失敗時の復旧で WASM handle を確実に解放する |
| `synth_note_on` | `(*mut SynthHandle, u8, f32)` | MIDI ノートON |
| `synth_note_off` | `(*mut SynthHandle, u8)` | MIDI ノートOFF |
| `synth_set_param` | `(*mut SynthHandle, u32, f32)` | パラメータ更新 |
| `synth_reset` | `(*mut SynthHandle)` | エンジン状態をリセット（MessagePort `reset` の対応） |
| `synth_out_l_ptr` | `(*const SynthHandle) -> *const f32` | 左ch出力スクラッチへのポインタ取得 |
| `synth_out_r_ptr` | `(*const SynthHandle) -> *const f32` | 右ch出力スクラッチへのポインタ取得 |
| `synth_capacity` | `(*const SynthHandle) -> u32` | スクラッチ容量（u32） |
| `synth_process_block` | `(*mut SynthHandle, u32)` | frames 個のサンプルを内部スクラッチに書き込む |

## バッファレイアウトと FFI 戦略

### 戦略: 内部スクラッチ + ポインタ公開

| アプローチ | 採用 | 理由 |
|---|---|---|
| **内部スクラッチに書き込み、ポインタを公開**（採用） | ✓ | ポインタ取得は初期化時の1回のみ。`process_block` 呼び出しごとに引数でポインタを渡す必要がない |
| 引数でポインタを毎回渡す | × | 関数呼び出し引数が増え、unsafe スパンが増える |
| インターリーブ・フラットバッファ | × | 1本のVec で持つとインデックス計算が必要。MVPでは可読性優先 |

### JS側からのアクセス手順（[05章で詳細](./05-web-frontend-spec.md)）

```typescript
// 初期化時（Worklet内）に1度だけ実行
const handlePtr = exports.synth_new(sampleRate, 128);
const lPtr = exports.synth_out_l_ptr(handlePtr);
const rPtr = exports.synth_out_r_ptr(handlePtr);
let memBuf = exports.memory.buffer;
let leftView  = new Float32Array(memBuf, lPtr, 128);   // ← ここでキャッシュ
let rightView = new Float32Array(memBuf, rPtr, 128);

// process 呼び出しごと
exports.synth_process_block(handlePtr, 128);
if (memBuf !== exports.memory.buffer) {
  // memory.grow が起きた場合のみ view を作り直す（通常は発生しない）
  memBuf = exports.memory.buffer;
  leftView  = new Float32Array(memBuf, lPtr, 128);
  rightView = new Float32Array(memBuf, rPtr, 128);
}
output[0].set(leftView);
output[1].set(rightView);
```

> **重要**: `Float32Array` を `process` 内で毎フレーム生成すると Worklet スレッドで JS オブジェクト生成が連発し GC リスクとなる。**初期化時にキャッシュし、`memory.buffer` が変化したときのみ再作成** する（[05章の synth-processor.ts](./05-web-frontend-spec.md#audioworkletprocessorsynth-processorts) と整合）。

### memory.grow 対策

WASM の `memory.grow` が起きると `memory.buffer` の参照が無効化され、既存の `Float32Array` ビューが使えなくなる。

- **第一防衛線**: `synth_new` 以降は **追加のヒープ確保を一切行わない**。`dsp-core::Engine::prepare` で確保される `KarplusStrong::buffer` と、`SynthHandle` 自身の `scratch_l/r` のみが allocate されるが、これらは `synth_new` 内で完了する。
- **検証手順**: `cargo expand -p wasm-audio` または `wasm-objdump -x` で `process_block` 経路に `Vec::push`/`Vec::with_capacity` 由来の関数呼び出しがないことを確認（[06章 F8](./06-build-and-verify.md)）。
- **第二防衛線（フォールバック）**: 万が一 grow が起きてもクラッシュしないよう、JS側は `process` ごとに `memBuf === exports.memory.buffer` をチェックし不一致時のみ view を作り直す。

## ブロックサイズ設計

| サイズ | 採用 | 理由 |
|---|---|---|
| **128 frames（render quantum と同一）** | ✓ MVP の唯一の選択 | 現行ブラウザでは AudioWorklet の `process()` が事実上 128 で呼ばれている。仕様上は将来可変となりうるが、Worklet 側で長さガードを入れる前提で MVP は 128 固定処理に倒す。同サイズなら ring buffer が不要 |
| 256/512 frames（pre-research 5.2 で言及） | × | ring buffer 設計が必要となり MVP のスコープ外 |

`synth_new(sample_rate, 128)` を初期化時に呼び、`scratch_l/r` を 128 要素で確保する。Worklet 側では `outputs[0][0].length !== 128` のときに無音フォールバックする実装ガードを入れる（[05章 注意点 5](./05-web-frontend-spec.md#重要な注意点) 参照）。`01章 用語集の render quantum` 項とも整合。

## エラーハンドリング方針

- C ABI のため戻り値で `Result` を返さない。
- 不正な引数（例: ParamId未知、note 範囲外）は **黙って無視** する（dsp-core 側で `clamp` 済み、`from_u32` で `None` チェック済み）。
- ハンドル null チェックを各関数の冒頭で行う（早期 return）。
- `panic` は `panic = "abort"` でWASMモジュール全体停止。dsp-core 側で `clamp` と `debug_assert!` のみ使用しリリースでは消える。

> 将来エラー伝播が必要になったら、戻り値 `i32`（0 = 成功、非ゼロ = エラーコード）の関数群を追加する。MVP では不要。

## ビルド出力

`pnpm build:wasm`（または `:dev`）を実行すると:

| ファイル | 役割 |
|---|---|
| `target/wasm32-unknown-unknown/release/wasm_audio.wasm` | cargo の生成物 |
| `web/src/lib/wasm/wasm_audio.wasm` | `copy-wasm.mjs` でコピーされたファイナル成果物 |

`wasm_audio.js`、`wasm_audio.d.ts` のような JS ラッパー / 型定義は生成されない（C ABI のため不要）。型補完が必要なら、Worklet 側で手書きの interface を [05章](./05-web-frontend-spec.md) に従い定義する。

## サイズ最適化（MVPでは深追いしない）

- `cargo build --release` + `wasm-opt -O3` で十分。MVP では `wasm-opt` を必須にしない
- `getrandom` は **追加しない**（自前 `XorShift32` を採用済み）
- `console_error_panic_hook` も追加しない（panic = abort のため）

## サンプル呼び出しシーケンス（リファレンス）

```
[main thread]
  // ?url 形式で fetch（dev/build 両対応）
  import wasmUrl from '$lib/wasm/wasm_audio.wasm?url'
  bytes = await fetch(wasmUrl).then(r => r.arrayBuffer())
  worklet.port.postMessage({ type: 'init', wasmBytes: bytes }, [bytes])

[worklet thread]
  port.onmessage で init 受信
  WebAssembly.instantiate(wasmBytes, { /* env imports */ })
  exports.synth_new(sampleRate, 128) → handlePtr
  lPtr = exports.synth_out_l_ptr(handlePtr)
  rPtr = exports.synth_out_r_ptr(handlePtr)
  memBuf = exports.memory.buffer
  leftView = new Float32Array(memBuf, lPtr, 128)   // ← キャッシュ
  rightView = new Float32Array(memBuf, rPtr, 128)  // ← キャッシュ
  port.postMessage({ type: 'ready' })

[main thread, ユーザーがキー押下]
  worklet.port.postMessage({ type: 'noteOn', midi: 60, velocity: 0.8 })

[worklet thread]
  port.onmessage で noteOn 受信
  exports.synth_note_on(handlePtr, 60, 0.8)

[worklet thread, process()コールバック]
  exports.synth_process_block(handlePtr, 128)
  if (memBuf !== exports.memory.buffer) {
    // grow 検出時のみ再作成
    memBuf = exports.memory.buffer
    leftView = new Float32Array(memBuf, lPtr, 128)
    rightView = new Float32Array(memBuf, rPtr, 128)
  }
  outputs[0][0].set(leftView)
  outputs[0][1].set(rightView)
  return true
```
