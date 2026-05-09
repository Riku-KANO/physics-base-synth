# 04. wasm-audio 仕様（Phase 4b）

## 目的

`crates/wasm-audio/src/lib.rs` の C ABI 境界に Phase 4b で **新規追加する関数はない**。Phase 4a 確定の **18 C ABI 関数 + memory export = 19 required exports** のシグネチャ・export 名・動作はすべて完全互換を維持する（D18 / D38 / D39 / D41 / D44-D52 継承）。Phase 4b で唯一変わるのは `synth_apply_instrument` の `kind` 値域が 0-6 から **0-7 に拡張**される点のみで、これは内部の `InstrumentKind::from_u32(7) = Some(Piano)` で対応する（C ABI シグネチャ不変、export 名も同じ）。

## 他文書との関係

- 上流: [`01-overview.md`](./01-overview.md)（C ABI 既存 18 関数 + memory export = 19 required exports の互換性チェックリスト + Phase 4b で追加する関数なし）、[`03-dsp-core-spec.md`](./03-dsp-core-spec.md)（`Engine::apply_instrument` 末尾の `pool.set_dispersion_active` 呼出。当初 D63 で 5 ms fade-out を提案していたが指摘事項 #3 反映で撤回し、Phase 4a D53 即時 release を継承）
- 下流: [`05-web-frontend-spec.md`](./05-web-frontend-spec.md)（Worklet `WasmExports` interface 不変、`InstrumentKindKey` に 'piano' 追加のみ）、[`07-implementation-checklist.md`](./07-implementation-checklist.md)
- 並行: Phase 4a [`04-wasm-audio-spec.md`](../2026-05-08-004-phase4a/04-wasm-audio-spec.md) — 同形式の C ABI 維持パターン参照

## C ABI 既存 18 関数 + memory export = 19 required exports（Phase 4b で完全維持）

| 関数名 | シグネチャ | Phase 4b 状況 |
|---|---|---|
| `synth_new` | `(f32, u32) -> *mut SynthHandle` | 維持。内部で dispersion stages も一括確保（heap 確保ゼロ維持） |
| `synth_free` | `(*mut SynthHandle)` | 維持 |
| `synth_note_on` | `(*mut SynthHandle, u8, f32)` | 維持。内部で `dispersion_active` に応じて pluck or hammer 経路で buffer 初期化（dsp-core 側分岐） |
| `synth_note_off` | `(*mut SynthHandle, u8)` | 維持 |
| `synth_set_param` | `(*mut SynthHandle, u32, f32)` | 維持 |
| `synth_set_polyphony_mode` | `(*mut SynthHandle, u32)` | 維持 |
| `synth_reset` | `(*mut SynthHandle)` | 維持。LFO / Mod Wheel / 楽器選択 (Default に reset) / dispersion stages も reset（dsp-core 側で連動） |
| `synth_out_l_ptr` | `(*const SynthHandle) -> *const f32` | 維持 |
| `synth_out_r_ptr` | `(*const SynthHandle) -> *const f32` | 維持 |
| `synth_capacity` | `(*const SynthHandle) -> u32` | 維持 |
| `synth_process_block` | `(*mut SynthHandle, u32)` | 維持。内部で dispersion cascade も走るが外部仕様は不変 |
| `synth_midi_cc` | `(*mut SynthHandle, u8, f32)` | 維持 |
| `synth_pitch_bend` | `(*mut SynthHandle, f32)` | 維持 |
| `synth_voice_state_ptr` | `(*const SynthHandle) -> *const u8` | 維持 |
| `synth_apply_instrument` | `(*mut SynthHandle, kind: u32)` | **拡張**: kind の値域を 0-6 から 0-7 へ。`InstrumentKind::from_u32(7) = Some(Piano)`、不正値 (8 以上) は黙って無視（既存防御的設計を継承） |
| `synth_lfo_set_rate` | `(*mut SynthHandle, hz: f32)` | 維持 |
| `synth_lfo_set_waveform` | `(*mut SynthHandle, kind: u32)` | 維持 |
| `synth_lfo_set_depth` | `(*mut SynthHandle, dest: u32, depth: f32)` | 維持 |
| (memory export) | WebAssembly.Memory | 維持。byteLength 不変 |

## Phase 4b で追加する C ABI 関数

**なし**。Inharmonicity B / HammerHardness は Piano プリセット内のフィールドで完結し、Phase 4b では UI 露出も Phase 4c 送り。`InstrumentKind::Piano = 7` の追加は `synth_apply_instrument` の値域拡張で吸収される（既存関数の内部分岐のみ）。

## `synth_apply_instrument` の値域拡張（D62）

Phase 4a 既存:
```rust
/// Phase 4a D52 / D53: 楽器プリセット切替。
/// `kind`: 0=Default, 1=GuitarClassical, 2=Ukulele, 3=Mandolin, 4=Bass, 5=GuitarSteel, 6=Sitar
/// 不正値（7 以上）は黙って無視（Phase 3 `synth_set_polyphony_mode` と同じ防御的設計）。
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

Phase 4b 拡張（doc コメント更新 + 値域拡張、コード本体の変更は **doc コメントのみ**）:

```rust
/// Phase 4a D52 / D53 + Phase 4b D62 / D63: 楽器プリセット切替。
/// `kind`: 0=Default, 1=GuitarClassical, 2=Ukulele, 3=Mandolin, 4=Bass, 5=GuitarSteel,
///         6=Sitar, 7=Piano (Phase 4b 追加)
/// 不正値（8 以上）は黙って無視（Phase 3 `synth_set_polyphony_mode` と同じ防御的設計）。
/// 内部で `pool.all_notes_off()` + Modal 係数差し替え + reset +
/// `pool.set_dispersion_active(piano)` を実行（dsp-core 側で連動）。
/// 当初 D63 で 5 ms fade-out を提案していたが、SmoothedValue 同期 set_target の
/// 実現不能性により撤回し、Phase 4a D53 の即時 release を継承（指摘事項 #3）。
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

**コード差分は実質ゼロ**: `InstrumentKind::from_u32` が Phase 4b で `0..=7` を受けるよう拡張済（`gen-params.mjs` 拡張で生成）。`Engine::apply_instrument(InstrumentKind::Piano)` の動作も dsp-core 側で完結する（Phase 4a D53 即時 release 継承 + D67 の `pool.set_dispersion_active(true)`、当初 D63 の 5 ms fade-out 提案は指摘事項 #3 反映で撤回）。

## 設計ポリシーの確認（Phase 4a から不変）

### `wasm-bindgen` 不使用（D8 維持）

すべての公開関数は `#[unsafe(no_mangle)] pub extern "C" fn`。Phase 1 から継続のシグネチャ規約。

### 外部 crate 追加禁止

`Cargo.toml` の `[dependencies]` は引き続き:
```toml
[dependencies]
dsp-core = { path = "../dsp-core" }
```
のみ。Phase 4b で **追加なし**（Phase 4a の `binaryen` は npm devDependency で別経路）。

### `clippy::not_unsafe_ptr_arg_deref` allow 維持

```rust
#![allow(clippy::not_unsafe_ptr_arg_deref)]
```

Phase 4b でも継続。

### null チェックパターン継続

```rust
if handle.is_null() {
    return;
}
let h = unsafe { &mut *handle };
```

Phase 4b でも全関数で同パターン（Phase 4b 自体は新規関数追加なし）。

### `from_u32` でのバリデーション

`InstrumentKind::from_u32` は **`Option<Self>` を返す**設計（`#[non_exhaustive]` enum と相性良）。Phase 4b で値域が 0-6 → 0-7 に拡張されるが、`if let Some(x) = ... .from_u32(...)` で「不正値は黙って無視」の防御的設計はそのまま継承。

`from_u32(8)` → `None` → `apply_instrument` 呼出をスキップ。`from_u32(7)` → `Some(Piano)` → `Engine::apply_instrument(Piano)` で D62 + D63 + D67 の連鎖処理が dsp-core 側で実行される。

## `SynthHandle` struct の Phase 4b 状況

```rust
#[repr(C)]
pub struct SynthHandle {
    engine: Engine,
    scratch_l: Vec<f32>,
    scratch_r: Vec<f32>,
}
```

**変更なし**。Phase 4b で `Engine` 内のフィールドが増える（`KarplusStrong::dispersion_stages` / `dispersion_active`）が、`SynthHandle` は `Engine` を保有するだけのため境界は不変。`scratch_l/r` も Phase 1 から不変（`synth_new` で max_block_size 確保、以降 length 変更なし）。

## ビルドと export 検証

### `scripts/check-wasm-exports.mjs` の `REQUIRED` 配列

Phase 4a 後の REQUIRED:
```javascript
const REQUIRED = [
  'memory',
  'synth_new', 'synth_free',
  'synth_note_on', 'synth_note_off',
  'synth_set_param', 'synth_reset',
  'synth_out_l_ptr', 'synth_out_r_ptr', 'synth_capacity',
  'synth_process_block', 'synth_set_polyphony_mode',
  'synth_midi_cc', 'synth_pitch_bend', 'synth_voice_state_ptr',
  // Phase 4a (D44-D55)
  'synth_apply_instrument',
  'synth_lfo_set_rate',
  'synth_lfo_set_waveform',
  'synth_lfo_set_depth',
];
```

Phase 4b で **変更なし**。`pnpm build:wasm` で REQUIRED 全 19 entry の export 検証は Phase 4a と同じ手順、Phase 4b でも exit 0 が期待される（C ABI 関数追加なしのため）。

### `scripts/copy-wasm.mjs` の `wasm-opt -O3` 統合（Phase 4a D45 既存）

Phase 4b で **変更なし**。Phase 4a 既存の `wasm-opt -O3 --strip-debug` を継続使用。Phase 4b では Step 3 で **`wasm-opt --print-stats` を一時的に追加**して各 pass の効果を計測（Phase 4a 18.42 KB の内訳調査）するが、これは scripts 本体ではなく開発者が手動で実行する一時手順。最終的なビルドパイプラインは Phase 4a と同じ。

### `package.json` 追加

**変更なし**。Phase 4a で追加した `binaryen` を継続使用。

## バイナリサイズの想定（Phase 4b 後）

| ビルド種別 | Phase 4a 後実測 | Phase 4b 想定 |
|---|---|---|
| `wasm-audio.wasm` raw | 40.44 KB | ~42.5 KB（Piano 楽器係数 + dispersion cascade コード + Hammer LPF コードで +2 KB raw） |
| `wasm-audio.wasm` gzip | 18.42 KB | ~19 KB（Phase 4b 純増 ~0.6 KB gzip） |
| Worklet バンドル (synth-processor.\*.js) | 8.17 KB | ~9 KB（dev-only timing 集約コードで +0.8 KB、production では tree-shake で削除） |

WASM gzip < 22 KB target（警戒）/ < 30 KB（撤退、Phase 1〜3 継承）は Phase 4b で維持される見込み。実測値が 22 KB を超える場合は Step 14 / 17 で再評価（pre-research §10.2 早期検証ポイント）。

## テスト方針

C ABI レベルのテストは Phase 1〜4a と同じく **Rust 側の `cargo test` ではなく、JS 側の Worklet 動作確認 + `check-wasm-exports.mjs` の export 名検証** で担保する（C ABI は数行の wrapper のみ、internal 関数の動作テストは dsp-core 側で網羅）。

### 検証項目

| 項目 | 方法 |
|---|---|
| 既存 18 関数すべての export 名が WASM バイナリに含まれる | `pnpm build:wasm` で `check-wasm-exports.mjs` exit 0（Phase 4a と同じ 19 entry） |
| `synth_apply_instrument(handle, 7)` が Piano 切替動作 | dev ビルドでブラウザ起動 + Console から `synth.engine.applyInstrument('piano')` 呼出、音が変化 |
| `synth_apply_instrument(handle, 8)` 等の不正値で no-op | `from_u32` の None 経路、`if let Some` で skip。dev ビルドでクラッシュしない |
| 既存 18 関数の動作完全互換 | Phase 4a 既存 cargo test 全件パス + 実機での Phase 4a 機能動作確認 |

## 依存方向の確認

```
wasm-audio (cdylib)
  └─ depends on dsp-core (rlib)
       └─ depends on nothing (依存ゼロ、Phase 1-4a 制約継承)
```

`dsp_core::params::InstrumentKind` が Phase 4b で Piano 値を持つため、`wasm-audio` 内で `use` するパスは Phase 4a と同じ（gen-params.mjs 経由で dsp-core::params::InstrumentKind が拡張、`crates/dsp-core/src/lib.rs` の `pub use lfo::{...};` パターンと同様、Phase 4b では `pub mod dispersion;` + `pub use dispersion::{...};` を追加するのみ）。

## まとめ

Phase 4b で wasm-audio 層に追加されるのは **0 関数**、変更は `synth_apply_instrument` の doc コメント更新のみ（コード本体は Phase 4a と完全一致）。複雑な分岐（Piano 楽器の dispersion cascade、Hammer model、Modal Body 差し替え、`set_dispersion_active` fan-out）はすべて dsp-core::Engine / KarplusStrong 内に閉じ込め、wasm-audio 層は C ABI 境界としての責務（null チェック / unsafe deref / enum 変換）のみ担う。Phase 4a と同じ薄さを維持。
