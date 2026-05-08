# 04. wasm-audio 仕様（Phase 4a）

## 目的

`crates/wasm-audio/src/lib.rs` の C ABI 境界に Phase 4a で追加する 4 関数（`synth_apply_instrument` / `synth_lfo_set_rate` / `synth_lfo_set_waveform` / `synth_lfo_set_depth`）を定義し、`scripts/check-wasm-exports.mjs` の `REQUIRED` 配列を更新する。Phase 1 / 2 / 3 の C ABI 既存 **14 関数 + memory export = 15 required exports** のシグネチャ・export 名・動作はすべて完全互換を維持する（D18 / D38 / D39 / D41 継承）。Phase 4a 後は **18 C ABI 関数 + memory export = 19 required exports**。

## 他文書との関係

- 上流: [`01-overview.md`](./01-overview.md)（C ABI 既存 14 C ABI 関数 + memory export = 15 required exports の互換性チェックリスト + Phase 4a で追加する 4 関数）、[`03-dsp-core-spec.md`](./03-dsp-core-spec.md)（`Engine::apply_instrument` / `Engine::lfo_set_*` の inherent methods 仕様）
- 下流: [`05-web-frontend-spec.md`](./05-web-frontend-spec.md)（Worklet `WasmExports` interface 拡張）、[`07-implementation-checklist.md`](./07-implementation-checklist.md)
- 並行: Phase 3 [`04-wasm-audio-spec.md`](../2026-05-07-003-phase3/04-wasm-audio-spec.md) — 同形式の C ABI 拡張パターン参照

## C ABI 既存 14 関数 + memory export = 15 required exports（Phase 4a で完全維持）

| 関数名 | シグネチャ | Phase 4a 状況 |
|---|---|---|
| `synth_new` | `(f32, u32) -> *mut SynthHandle` | 維持。内部で LFO 状態 / 楽器選択 / 楽器ごとの Modal 係数も初期化 |
| `synth_free` | `(*mut SynthHandle)` | 維持 |
| `synth_note_on` | `(*mut SynthHandle, u8, f32)` | 維持 |
| `synth_note_off` | `(*mut SynthHandle, u8)` | 維持 |
| `synth_set_param` | `(*mut SynthHandle, u32, f32)` | 維持 |
| `synth_set_polyphony_mode` | `(*mut SynthHandle, u32)` | 維持 |
| `synth_reset` | `(*mut SynthHandle)` | 維持。LFO / Mod Wheel / 楽器選択も Default に reset |
| `synth_out_l_ptr` | `(*const SynthHandle) -> *const f32` | 維持 |
| `synth_out_r_ptr` | `(*const SynthHandle) -> *const f32` | 維持 |
| `synth_capacity` | `(*const SynthHandle) -> u32` | 維持 |
| `synth_process_block` | `(*mut SynthHandle, u32)` | 維持。内部で LFO process も走るが外部仕様は不変 |
| `synth_midi_cc` | `(*mut SynthHandle, u8, f32)` | 維持。**CC#1 (Mod Wheel) 分岐の実装は dsp-core 側、本層は不変**（D49） |
| `synth_pitch_bend` | `(*mut SynthHandle, f32)` | 維持 |
| `synth_voice_state_ptr` | `(*const SynthHandle) -> *const u8` | 維持 |
| (memory export) | WebAssembly.Memory | 維持。byteLength 不変 |

## Phase 4a で追加する C ABI 関数（4 件）

### `synth_apply_instrument`（D52 / D53）

```rust
/// Phase 4a D52 / D53: 楽器プリセット切替。
/// `kind`: 0=Default, 1=GuitarClassical, 2=Ukulele, 3=Mandolin, 4=Bass, 5=GuitarSteel, 6=Sitar
/// 不正値（7 以上）は黙って無視（Phase 3 `synth_set_polyphony_mode` と同じ防御的設計）。
/// 内部で `pool.all_notes_off()` + Modal 係数差し替え + reset を実行。
#[unsafe(no_mangle)]
pub extern "C" fn synth_apply_instrument(handle: *mut SynthHandle, kind: u32) {
    if handle.is_null() {
        return;
    }
    let h = unsafe { &mut *handle };
    if let Some(instrument_kind) = dsp_core::params::InstrumentKind::from_u32(kind) {
        h.engine.apply_instrument(instrument_kind);
    }
}
```

### `synth_lfo_set_rate`（D46）

```rust
/// Phase 4a D46: LFO レート設定 (0.1〜8.0 Hz、SmoothedValue tau=0.05s で平滑化)。
/// 範囲外の値は dsp-core 側で clamp。
#[unsafe(no_mangle)]
pub extern "C" fn synth_lfo_set_rate(handle: *mut SynthHandle, hz: f32) {
    if handle.is_null() {
        return;
    }
    let h = unsafe { &mut *handle };
    h.engine.lfo_set_rate(hz);
}
```

### `synth_lfo_set_waveform`（D47）

```rust
/// Phase 4a D47: LFO 波形設定。
/// `kind`: 0=Sine, 1=Triangle。不正値は無視。
#[unsafe(no_mangle)]
pub extern "C" fn synth_lfo_set_waveform(handle: *mut SynthHandle, kind: u32) {
    if handle.is_null() {
        return;
    }
    let h = unsafe { &mut *handle };
    if let Some(waveform) = dsp_core::lfo::LfoWaveform::from_u32(kind) {
        h.engine.lfo_set_waveform(waveform);
    }
}
```

### `synth_lfo_set_depth`（D48）

```rust
/// Phase 4a D48: LFO destination depth 設定。
/// `dest`: 0=Pitch, 1=Brightness, 2=Volume
/// `depth`: 0.0〜1.0 (dsp-core 側で clamp)
/// 不正な dest は無視。
#[unsafe(no_mangle)]
pub extern "C" fn synth_lfo_set_depth(handle: *mut SynthHandle, dest: u32, depth: f32) {
    if handle.is_null() {
        return;
    }
    let h = unsafe { &mut *handle };
    if let Some(destination) = dsp_core::lfo::LfoDestination::from_u32(dest) {
        h.engine.lfo_set_depth(destination, depth);
    }
}
```

## 設計ポリシーの確認

### `wasm-bindgen` 不使用（D8 維持）

すべての公開関数は `#[unsafe(no_mangle)] pub extern "C" fn`。Phase 1 から継続のシグネチャ規約。

### 外部 crate 追加禁止

`Cargo.toml` の `[dependencies]` は引き続き:
```toml
[dependencies]
dsp-core = { path = "../dsp-core" }
```
のみ。Phase 4a で **`binaryen` 等の WASM 最適化ツールは npm devDependency**（build-time、Cargo 依存ではない）として扱うため抵触しない。

### `clippy::not_unsafe_ptr_arg_deref` allow 維持

```rust
#![allow(clippy::not_unsafe_ptr_arg_deref)]
```

C ABI 設計上、関数シグネチャの `*mut SynthHandle` は呼び元（JS）が `unsafe` 文脈を担う設計。Phase 4a 追加 4 関数も同パターン。

### null チェックパターン継続

```rust
if handle.is_null() {
    return;
}
let h = unsafe { &mut *handle };
```

すべての追加関数で同パターン。

### `from_u32` でのバリデーション

`InstrumentKind::from_u32` / `LfoWaveform::from_u32` / `LfoDestination::from_u32` は **`Option<Self>` を返す**設計（`#[non_exhaustive]` enum と相性良）。`if let Some(x) = ... .from_u32(...)` で「不正値は黙って無視」。

`synth_set_polyphony_mode` の Phase 3 既存パターン（`match` で 0/1 以外を `return`）と同じ防御的設計。

## `SynthHandle` struct の Phase 4a 状況

```rust
#[repr(C)]
pub struct SynthHandle {
    engine: Engine,
    scratch_l: Vec<f32>,
    scratch_r: Vec<f32>,
}
```

**変更なし**。Phase 4a で `Engine` 内のフィールドが増えるが、`SynthHandle` は `Engine` を保有するだけのため境界は不変。`scratch_l/r` も Phase 1 から不変（`synth_new` で max_block_size 確保、以降 length 変更なし）。

## ビルドと export 検証

### `scripts/check-wasm-exports.mjs` の `REQUIRED` 配列拡張

Phase 3 後の REQUIRED:
```javascript
const REQUIRED = [
  'memory',
  'synth_new', 'synth_free',
  'synth_note_on', 'synth_note_off',
  'synth_set_param', 'synth_reset',
  'synth_out_l_ptr', 'synth_out_r_ptr', 'synth_capacity',
  'synth_process_block', 'synth_set_polyphony_mode',
  'synth_midi_cc', 'synth_pitch_bend', 'synth_voice_state_ptr',
];
```

Phase 4a で追加:
```javascript
const REQUIRED = [
  // ...Phase 3 既存...
  // Phase 4a (D44-D55)
  'synth_apply_instrument',
  'synth_lfo_set_rate',
  'synth_lfo_set_waveform',
  'synth_lfo_set_depth',
];
```

`pnpm build:wasm` が `REQUIRED` 全件の export を検証、欠落で exit 1。

### `scripts/copy-wasm.mjs` の `wasm-opt` 統合（D45）

Phase 3 までは素のコピーのみ:
```javascript
// scripts/copy-wasm.mjs (Phase 3)
import { copyFileSync } from 'node:fs';
copyFileSync(srcPath, dstPath);
```

Phase 4a で `wasm-opt -O3 --strip-debug` を統合（既存 `process.argv[2]` の profile 引数渡し規約を維持）:
```javascript
// scripts/copy-wasm.mjs (Phase 4a 拡張)
import { execFileSync } from 'node:child_process';
import { copyFileSync, statSync, existsSync, mkdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const profile = process.argv[2] === 'release' ? 'release' : 'debug';

function resolveWasmOpt() {
  // Windows: .cmd 拡張子も探す
  const candidates = [
    resolve(__dirname, '../node_modules/.bin/wasm-opt'),
    resolve(__dirname, '../node_modules/.bin/wasm-opt.cmd'),  // Windows
  ];
  for (const c of candidates) {
    if (existsSync(c)) return c;
  }
  return null;
}

const wasmOptBin = resolveWasmOpt();

if (profile === 'release' && wasmOptBin) {
  const beforeSize = statSync(srcPath).size;
  execFileSync(wasmOptBin, ['-O3', '--strip-debug', srcPath, '-o', dstPath], {
    stdio: 'inherit',
  });
  const afterSize = statSync(dstPath).size;
  console.log(
    `[copy-wasm] wasm-opt -O3 applied: ${beforeSize} → ${afterSize} bytes ` +
    `(${((1 - afterSize / beforeSize) * 100).toFixed(1)}% reduction)`
  );
} else {
  copyFileSync(srcPath, dstPath);
  if (profile === 'release' && !wasmOptBin) {
    console.warn('[copy-wasm] wasm-opt not found in node_modules/.bin, ' +
                 'install binaryen as devDependency');
  }
}
```

**重要**:
- dev ビルド (`pnpm build:wasm:dev`、`node scripts/copy-wasm.mjs debug`) では profile === 'debug' で wasm-opt をスキップ（ビルド時間短縮 + デバッグ情報保持）
- production ビルド (`pnpm build:wasm`、`node scripts/copy-wasm.mjs release`) で profile === 'release' のとき wasm-opt 適用、サイズログを出力
- wasm-opt 不在時は警告ログ + 素コピーで続行（CI 環境差分を吸収）
- `package.json` の script 定義は不変（既存 `node scripts/copy-wasm.mjs release` / `node scripts/copy-wasm.mjs debug` 規約を継承）

### `package.json` 追加

```json
{
  "devDependencies": {
    "binaryen": "^123.0.0"
  }
}
```

`pnpm install` で `node_modules/.bin/wasm-opt` が配置される。Cargo の依存ではないため Phase 1〜3 の依存ゼロ制約に抵触しない（npm の build-time tooling）。

## バイナリサイズの想定（Phase 4a 後）

| ビルド種別 | Phase 3 後実測 | Phase 4a 想定（wasm-opt -O3 込み） |
|---|---|---|
| `wasm-audio.wasm` raw | ~80 KB | ~25 KB |
| `wasm-audio.wasm` gzip | 27.78 KB | ~13 KB |
| Worklet バンドル (synth-processor.\*.js) | ~3 KB | ~3.5 KB（LFO + applyInstrument message dispatch +500 B） |

WASM gzip < 30 KB target は Phase 3 から維持、wasm-opt -O3 で大幅改善見込み。実測値が想定を超える場合は Phase 4a Step 2 で調査（pre-research §9.2 早期検証ポイント）。

## テスト方針

C ABI レベルのテストは Phase 1〜3 と同じく **Rust 側の `cargo test` ではなく、JS 側の Worklet 動作確認 + `check-wasm-exports.mjs` の export 名検証** で担保する（C ABI は数行の wrapper のみ、internal 関数の動作テストは dsp-core 側で網羅）。

### 検証項目

| 項目 | 方法 |
|---|---|
| 4 関数すべての export 名が WASM バイナリに含まれる | `pnpm build:wasm` で `check-wasm-exports.mjs` exit 0 |
| 各関数が null handle で no-op | dev ビルドで `synth_lfo_set_rate(0, 5.0)` 呼出 → panic / segfault なし |
| `apply_instrument(7)` 等の不正値で no-op | `from_u32` の None 経路、`if let Some` で skip |
| `lfo_set_depth(99, 0.5)` 等の不正 dest で no-op | 同上 |
| 既存 14 関数の動作完全互換 | Phase 3 既存 cargo test 全件パス + 実機での Phase 3 機能動作確認 |

## 依存方向の確認

```
wasm-audio (cdylib)
  └─ depends on dsp-core (rlib)
       └─ depends on nothing (依存ゼロ、Phase 1-3 制約継承)
```

`dsp_core::params::InstrumentKind` / `dsp_core::lfo::{LfoWaveform, LfoDestination}` を `wasm-audio` 内で `use` するため、`dsp-core` の公開 API に enum が含まれる必要がある。`crates/dsp-core/src/lib.rs` で `pub use lfo::{Lfo, LfoWaveform, LfoDestination};` を追加。

## まとめ

Phase 4a で wasm-audio 層に追加されるのは **4 関数のみ**、各 5-7 行の薄い wrapper。複雑な分岐は dsp-core::Engine 内に閉じ込め、wasm-audio 層は C ABI 境界としての責務（null チェック / unsafe deref / enum 変換）のみ担う。Phase 3 と同じ薄さを維持。
