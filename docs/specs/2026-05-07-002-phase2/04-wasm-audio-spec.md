# 04. Phase 2 wasm-audio クレート仕様

## 目的

Phase 1 [04 章 wasm-audio クレート仕様](../2026-05-06-001-mvp/04-wasm-audio-spec.md) を起点に、Phase 2 で発生する **C ABI の差分**（互換 10 関数の動作拡張、`synth_set_polyphony_mode` の追加）と SynthHandle 内部の Engine 構造変化を確定する。設計判断 D8（C ABI 統一、wasm-bindgen 不使用）を維持し、Phase 1 の export 名と signature は完全互換とする。

## 他文書との関係

- 上流: [`03-dsp-core-spec.md`](./03-dsp-core-spec.md)（Engine の VoicePool 化、SynthMode の導入）
- 下流: [`05-web-frontend-spec.md`](./05-web-frontend-spec.md)（Worklet 側の `WasmExports` interface 拡張）
- 参考: [Phase 1 04 章](../2026-05-06-001-mvp/04-wasm-audio-spec.md)（C ABI 既存 10 関数のシグネチャ、SynthHandle、scratch buffer、memory.grow 対策、ビルド方法）— **本書で明示的に変更しない部分はすべて Phase 1 の記述を継承**

## クレート設定

[Phase 1 04 章 §クレート設定](../2026-05-06-001-mvp/04-wasm-audio-spec.md#クレート設定) を **完全維持**:

- `crates/wasm-audio/Cargo.toml` の `crate-type = ["cdylib"]`、`dependencies = { dsp-core }` のみ、wasm-bindgen 不使用
- `[profile.release]` のワークスペース定義依存（個別 `[profile.release]` は Phase 1 retrospective で削除済み）
- 設計判断 D8（`#[unsafe(no_mangle)] extern "C"` 統一）

Phase 2 でも依存追加なし。

## C ABI の互換性維持（D18）

Phase 1 で公開した 10 関数のシグネチャ・export 名・動作はすべて維持する。

### 互換維持関数一覧

| 関数 | Phase 1 シグネチャ | Phase 2 動作変化 |
|---|---|---|
| `synth_new` | `(f32, u32) -> *mut SynthHandle` | 内部で N=8 ボイス分の VoicePool を `Engine::prepare` で確保（外部仕様不変） |
| `synth_free` | `(*mut SynthHandle)` | 不変 |
| `synth_note_on` | `(*mut SynthHandle, u8, f32)` | 内部で `Engine::note_on` が VoicePool への allocation または stealing を実行（mono モード時は hold stack 経由） |
| `synth_note_off` | `(*mut SynthHandle, u8)` | 内部で `Engine::note_off` が該当ボイスへ damping 加速、mono モード時は hold stack 復帰判定 |
| `synth_set_param` | `(*mut SynthHandle, u32, f32)` | 内部で全ボイスへ fan-out（VoicePool::set_damping / set_brightness）|
| `synth_reset` | `(*mut SynthHandle)` | 内部で全ボイス reset、hold_stack クリア |
| `synth_out_l_ptr` | `(*const SynthHandle) -> *const f32` | 不変。scratch_l のポインタは Phase 1 と同一 |
| `synth_out_r_ptr` | `(*const SynthHandle) -> *const f32` | 不変 |
| `synth_capacity` | `(*const SynthHandle) -> u32` | 不変。返値は `scratch_l.len() as u32 = 128` |
| `synth_process_block` | `(*mut SynthHandle, u32)` | 内部で全アクティブボイスを VoicePool::process_sample でミックスして scratch に書き込む |

### Worklet 側からの呼び出し互換性

Phase 1 の Worklet `WasmExports` interface（[Phase 1 05 章](../2026-05-06-001-mvp/05-web-frontend-spec.md#audioworkletprocessorsynth-processorts)）は Phase 2 でも **既存 10 関数の宣言部分は変更不要**。新 export 1 件を追加するのみ（[`05-web-frontend-spec.md`](./05-web-frontend-spec.md#wasmexports-interface-の拡張)）。

## Phase 2 で追加する C ABI 関数

### `synth_set_polyphony_mode`（D17）

```rust
/// mode: 0 = poly, 1 = mono
/// 不正な値は無視（黙って返る、Phase 1 既存パターン）
#[unsafe(no_mangle)]
pub extern "C" fn synth_set_polyphony_mode(handle: *mut SynthHandle, mode: u32) {
    if handle.is_null() {
        return;
    }
    let h = unsafe { &mut *handle };
    let synth_mode = match mode {
        0 => dsp_core::engine::SynthMode::Poly,
        1 => dsp_core::engine::SynthMode::Mono,
        _ => return,  // 不正な値は無視
    };
    h.engine.set_mode(synth_mode);
}
```

### 追加関数一覧

| 関数 | シグネチャ | 役割 |
|---|---|---|
| `synth_set_polyphony_mode` | `(*mut SynthHandle, u32)` | mode = 0 (poly) / 1 (mono) を切り替え。不正値は無視。Phase 2 では UI から呼ばないが C ABI として提供（D17 / D21）|

### Phase 2 で UI から呼ばない理由

Phase 1 のミニマル UI 思想を維持するため（D21 / D22）。`synth_set_polyphony_mode` は外部ツール / 将来の拡張 UI / 統合テストから呼ぶことを想定して提供する。

## SynthHandle 構造体の Phase 2 版

### 構造体定義

```rust
#[repr(C)]
pub struct SynthHandle {
    engine: Engine,           // Phase 2: 内部に VoicePool<8> + HoldStack + SynthMode を保持
    scratch_l: Vec<f32>,      // Phase 1 と同じ、128 サンプル分
    scratch_r: Vec<f32>,      // Phase 1 と同じ、128 サンプル分
}
```

### Phase 1 との差分

外部から見た `SynthHandle` のフィールド構成は Phase 1 と同じ。`engine` の中身が VoicePool 化された Engine になっただけで、wasm-audio 側のコードは Phase 1 と同じ構造で書ける。

### Engine の内部構造（参考）

```text
Engine
├─ pool: VoicePool<8>
│   ├─ voices: [KarplusStrong; 8]   ← 各ボイスが内部で buffer / lagrange / damping / brightness 等を保持
│   └─ sample_rate: f32
├─ output_gain: SmoothedValue
├─ hold_stack: HoldStack            ← LinearStack<u8, 16>
├─ mode: SynthMode                   ← Poly / Mono
├─ current_damping: f32
└─ sample_rate: f32
```

詳細は [`03-dsp-core-spec.md` §Engine（Phase 2 改修）](./03-dsp-core-spec.md#enginephase-2-改修)。

## バッファレイアウトと FFI 戦略

[Phase 1 04 章 §バッファレイアウトと FFI 戦略](../2026-05-06-001-mvp/04-wasm-audio-spec.md#バッファレイアウトと-ffi-戦略) を **完全維持**。

- 内部スクラッチ（scratch_l / scratch_r、128 frames 固定）にミックス済み出力を書き込み、ポインタを公開
- JS 側は init 時に `Float32Array(memory.buffer, ptr, 128)` をキャッシュ
- `memory.grow` 検出時のみ view 再作成（通常発火しない）

### ポリフォニー時の合成

VoicePool::process_sample が 8 ボイスを内部で累積して 1 サンプルを返す。1/sqrt(N) スケール（D20）も VoicePool 内で適用済み。wasm-audio 側は Phase 1 と同じく `engine.process(&mut h.scratch_l[..n], &mut h.scratch_r[..n])` を呼ぶだけで、追加処理は不要。

## メモリ確保の見積もり

### Phase 1 のメモリ使用量

`Engine::prepare(48000, 128)` 時:

- `KarplusStrong::buffer`: 48000 / 27.5 = 1746 サンプル × 4 bytes = **6.8 KB**
- `SynthHandle::scratch_l/r`: 各 128 サンプル × 4 bytes = **1.0 KB**
- 合計: **約 8 KB**（実測 7.98 KB gzip と整合）

### Phase 2 のメモリ使用量（N=8 ボイス）

`Engine::prepare(48000, 128)` 時に **dsp-core / wasm-audio が新規確保するメモリ**:

- `VoicePool::voices[i].buffer` × 8: (1746 + 3 [Lagrange margin]) × 4 bytes × 8 = **約 56 KB**
- `SynthHandle::scratch_l/r`: 1.0 KB（Phase 1 同等）
- HoldStack: `[Option<u8>; 16]` + `len: usize` = const-size、約 100 bytes
- VoicePool meta（sample_rate）、Engine meta（output_gain / mode 等）: 数百 bytes
- 合計: **約 57-58 KB**（ユーザーコード起源の確保分）

> **注意**: 上記は dsp-core / wasm-audio 由来の確保サイズ。WASM linear memory 全体は Rust runtime の stack / allocator metadata / static data も含むため数百 KB 規模になり得る。Phase 2 の保証は「**`synth_new` 完了直後の `memory.buffer.byteLength` を baseline として、以後 `process_block` / `note_on` / `note_off` / `set_param` / `set_polyphony_mode` のいずれを呼んでも byteLength が変化しない**」こと（D4 維持、`memory.grow` 発生ゼロ）。F17 の検証手順は baseline 比較で行う（[`06-build-and-verify.md` §F17](./06-build-and-verify.md#f17ポリフォニー時のメモリ確保ゼロの詳細手順)）。これは Worklet の Float32Array view キャッシュ（D9）の前提条件。

### サイズ目標との整合性

[Phase 2 06 章 §性能目標](./06-build-and-verify.md#性能目標) で `WASM gzip < 30 KB` を設定（Phase 1 実績 7.98 KB から +20-30 KB の見積）。VoicePool 追加コードと FractionalDelay / NoteAllocator / HoldStack の合計が `wasm-opt -O3` 圧縮後で +15-20 KB に収まれば達成可能。06 章リスク表 R19 で「サイズ膨張時は wasm-opt -O3 必須化」を提示する。

## ビルド方法

[Phase 1 04 章 §ビルド方法](../2026-05-06-001-mvp/04-wasm-audio-spec.md#ビルド方法) を継承し、Phase 2 で **前段に `pnpm gen:params` を追加**（[02 章](./02-architecture.md#ルート-packagejson)）。

```json
{
  "scripts": {
    "build:wasm": "pnpm gen:params && cargo build -p wasm-audio --target wasm32-unknown-unknown --release && node scripts/copy-wasm.mjs release && node scripts/check-wasm-exports.mjs",
    "build:wasm:dev": "pnpm gen:params && cargo build -p wasm-audio --target wasm32-unknown-unknown && node scripts/copy-wasm.mjs debug && node scripts/check-wasm-exports.mjs"
  }
}
```

`scripts/copy-wasm.mjs` は Phase 1 から変更なし（[Phase 1 04 章](../2026-05-06-001-mvp/04-wasm-audio-spec.md#ビルド方法)）。

## `scripts/check-wasm-exports.mjs` の更新

`REQUIRED` 配列に `synth_set_polyphony_mode` を追加。

```javascript
const REQUIRED = [
  'memory',
  'synth_new', 'synth_free',
  'synth_note_on', 'synth_note_off',
  'synth_set_param', 'synth_reset',
  'synth_out_l_ptr', 'synth_out_r_ptr', 'synth_capacity',
  'synth_process_block',
  'synth_set_polyphony_mode',  // Phase 2 追加（D17）
];
```

その他の検証ロジック（exit code、ログ出力）は Phase 1 から変更なし。

## メモリ確保の防衛線

### 第一防衛線: synth_new 以降の追加確保ゼロ

[Phase 1 04 章 §memory.grow 対策](../2026-05-06-001-mvp/04-wasm-audio-spec.md#memorygrow-対策) を継承。Phase 2 でも以下を守る:

- `synth_new` 内で `Box::new(SynthHandle { ... })` と `Engine::prepare`（その内部で VoicePool::prepare → 各ボイスの KarplusStrong::prepare → buffer の vec! 確保）が完結
- `synth_note_on` / `synth_note_off` / `synth_set_param` / `synth_reset` / `synth_set_polyphony_mode` / `synth_process_block` のいずれも、内部で `Vec::push` / `Vec::with_capacity` / `Box::new` を呼ばない
- VoicePool / NoteAllocator / HoldStack の Phase 2 新規モジュールも const-size 配列・固定容量スタックで実装され、`process` 中の動的確保ゼロ

### 検証手順

[Phase 1 04 章](../2026-05-06-001-mvp/04-wasm-audio-spec.md#memorygrow-対策) と同様、以下で検証:

1. `cargo test -p dsp-core` で `test_no_allocation_in_polyphonic_process` を含むテストがパス（[03 章](./03-dsp-core-spec.md#phase-2-で追加するテスト11-件)）
2. `cargo expand -p wasm-audio` で `synth_process_block` 経路に `Vec::push` / `Vec::with_capacity` が現れない
3. ブラウザでの実機検証: `synth-processor.ts` に一時挿入する `memory.buffer.byteLength` 不変チェック（[Phase 1 06 章 F8](../2026-05-06-001-mvp/06-build-and-verify.md#f8メモリ確保チェックの詳細手順)）を Phase 2 では `process_block` 8 ボイス全力連打中で確認（F17）

### 第二防衛線

`memory.grow` が万一発生してもクラッシュしないよう、JS 側の view 再作成ロジックは Phase 1 のまま維持（[Phase 1 04 章](../2026-05-06-001-mvp/04-wasm-audio-spec.md#memorygrow-対策)）。

## ブロックサイズ設計

[Phase 1 04 章 §ブロックサイズ設計](../2026-05-06-001-mvp/04-wasm-audio-spec.md#ブロックサイズ設計) を **完全維持**。

- `synth_new(sample_rate, 128)` で scratch_l / scratch_r を 128 frames で確保
- Worklet 側で `outputs[0][0].length !== 128` のとき無音フォールバック（[Phase 2 05 章](./05-web-frontend-spec.md)）
- ring buffer は Phase 2 でも導入しない

## エラーハンドリング方針

[Phase 1 04 章 §エラーハンドリング方針](../2026-05-06-001-mvp/04-wasm-audio-spec.md#エラーハンドリング方針) を **完全維持**:

- C ABI のため戻り値で `Result` を返さない
- 不正引数は黙って無視
- `synth_set_polyphony_mode` の不正な mode 値（0 / 1 以外）も黙って無視
- panic は `panic = "abort"` で WASM モジュール全体停止
- ハンドル null チェックを各関数の冒頭で行う

## ビルド出力

[Phase 1 04 章 §ビルド出力](../2026-05-06-001-mvp/04-wasm-audio-spec.md#ビルド出力) を維持。Phase 2 では追加で `params.json` の生成物（`crates/dsp-core/src/params.rs`）が cargo build の入力として読み込まれるが、出力ファイルは Phase 1 と同じ:

| ファイル | 役割 |
|---|---|
| `target/wasm32-unknown-unknown/release/wasm_audio.wasm` | cargo の生成物 |
| `web/src/lib/wasm/wasm_audio.wasm` | `copy-wasm.mjs` でコピーされたファイナル成果物 |

## サイズ最適化

[Phase 1 04 章 §サイズ最適化](../2026-05-06-001-mvp/04-wasm-audio-spec.md#サイズ最適化mvpでは深追いしない) を継承。Phase 2 では VoicePool / FractionalDelay 追加でサイズが増えるため、`wasm-opt -O3` を **release ビルドで実質必須化** する方針を [`06-build-and-verify.md` リスク表 R19](./06-build-and-verify.md#リスクと対策表) で提示する。Phase 1 と同じく `getrandom` / `console_error_panic_hook` は追加しない。

## サンプル呼び出しシーケンス（Phase 2 リファレンス）

```
[main thread]
  // ?url 形式で fetch（Phase 1 と同じ）
  import wasmUrl from '$lib/wasm/wasm_audio.wasm?url'
  bytes = await fetch(wasmUrl).then(r => r.arrayBuffer())
  worklet.port.postMessage({ type: 'init', wasmBytes: bytes }, [bytes])

[worklet thread]
  port.onmessage で init 受信
  WebAssembly.instantiate(wasmBytes, { env: {} })
  exports.synth_new(sampleRate, 128) → handlePtr
  // Phase 2: 内部で VoicePool / HoldStack / SynthMode が一括確保される
  lPtr = exports.synth_out_l_ptr(handlePtr)
  rPtr = exports.synth_out_r_ptr(handlePtr)
  memBuf = exports.memory.buffer
  leftView = new Float32Array(memBuf, lPtr, 128)   // ← キャッシュ
  rightView = new Float32Array(memBuf, rPtr, 128)
  port.postMessage({ type: 'ready' })

[main thread, ユーザーがキー押下を 8 連打]
  worklet.port.postMessage({ type: 'noteOn', midi: 60, velocity: 0.8 })
  worklet.port.postMessage({ type: 'noteOn', midi: 62, velocity: 0.8 })
  ... // 計 8 件

[worklet thread]
  port.onmessage で noteOn を順次受信
  exports.synth_note_on(handlePtr, 60, 0.8)
  // 内部で VoicePool::note_on が voice 0 に割当
  exports.synth_note_on(handlePtr, 62, 0.8)
  // voice 1 に割当
  ... // 計 8 件、すべて異なるボイスに割当

[main thread, 9 音目]
  worklet.port.postMessage({ type: 'noteOn', midi: 72, velocity: 0.8 })

[worklet thread]
  exports.synth_note_on(handlePtr, 72, 0.8)
  // 内部で VoicePool::note_on
  //   → 同名ノート不在
  //   → 空きボイス不在
  //   → note_allocator::select_voice_for_steal で energy 最小ボイスを選定（D13/D28）
  //   → そのボイスを 72 で再励振

[worklet thread, process()コールバック]
  exports.synth_process_block(handlePtr, 128)
  // 内部で全 8 ボイスを VoicePool::process_sample で累積、1/sqrt(8) スケール、128 frames を scratch に書き込む
  if (memBuf !== exports.memory.buffer) {
    memBuf = exports.memory.buffer
    leftView = new Float32Array(memBuf, lPtr, 128)
    rightView = new Float32Array(memBuf, rPtr, 128)
  }
  outputs[0][0].set(leftView)
  outputs[0][1].set(rightView)
  return true
```

## 詰まりやすい箇所

[Phase 1 05 章 §詰まりやすい箇所まとめ](../2026-05-06-001-mvp/05-web-frontend-spec.md#詰まりやすい箇所まとめ) を継承し、Phase 2 で追加:

1. **`synth_set_polyphony_mode` の export 名忘れ**: `#[unsafe(no_mangle)]` を Phase 2 でも忘れない。`scripts/check-wasm-exports.mjs` の `REQUIRED` 配列に追加し、ビルド時に検知（[06 章 F14](./06-build-and-verify.md)）
2. **C ABI の動作変化に C ABI シグネチャを引きずられない**: `synth_note_on` の動作が「単一ボイスへの発火」から「VoicePool への allocation」に変わるが、シグネチャは `(*mut SynthHandle, u8, f32)` で完全に Phase 1 互換（D18）。Worklet 側のコードは変更不要
3. **`synth_new` の `max_block_size` を VoicePool に渡し忘れ**: `Engine::prepare` 内で `pool.prepare(sample_rate, max_block_size)` を呼ぶ。各 KarplusStrong::prepare で max_block_size を使うわけではないが、API として正しく渡す
4. **`memory.grow` の発生**: Phase 2 では VoicePool / HoldStack で固定配列を使うため発生しないはずだが、cargo expand での確認を Phase 1 同様に実施（F17）
