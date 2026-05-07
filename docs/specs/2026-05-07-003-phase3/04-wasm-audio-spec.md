# 04. Phase 3 wasm-audio クレート仕様

## 目的

Phase 1 [04 章](../2026-05-06-001-mvp/04-wasm-audio-spec.md) と Phase 2 [04 章](../2026-05-07-002-phase2/04-wasm-audio-spec.md) を起点に、Phase 3 で発生する **C ABI の差分**（既存 12 関数の動作拡張、`synth_midi_cc` / `synth_pitch_bend` / `synth_voice_state_ptr` の 3 関数追加）と SynthHandle 内部の Engine 構造変化を確定する。設計判断 D8（C ABI 統一、wasm-bindgen 不使用）を維持し、Phase 1 / 2 の export 名と signature は完全互換とする。

## 他文書との関係

- 上流: [`03-dsp-core-spec.md`](./03-dsp-core-spec.md)（Engine の Modal Body / Sustain / Voice State / MIDI CC dispatch 統合）
- 下流: [`05-web-frontend-spec.md`](./05-web-frontend-spec.md)（Worklet 側の `WasmExports` interface 拡張）
- 参考: Phase 2 [04 章](../2026-05-07-002-phase2/04-wasm-audio-spec.md)（C ABI 既存 12 関数のシグネチャ、SynthHandle、scratch buffer、memory.grow 対策、ビルド方法）— **本書で明示的に変更しない部分はすべて Phase 2 の記述を継承**

## クレート設定

[Phase 1 04 章 §クレート設定](../2026-05-06-001-mvp/04-wasm-audio-spec.md#クレート設定) を **完全維持**:

- `crates/wasm-audio/Cargo.toml` の `crate-type = ["cdylib"]`、`dependencies = { dsp-core }` のみ、wasm-bindgen 不使用
- `[profile.release]` のワークスペース定義依存
- 設計判断 D8（`#[unsafe(no_mangle)] extern "C"` 統一）

Phase 3 でも依存追加なし。

## C ABI の互換性維持（D18 継承）

Phase 1 で公開した 10 関数 + Phase 2 で追加した 2 関数 = 既存 12 関数のシグネチャ・export 名・動作はすべて維持する。

### 互換維持関数一覧

| 関数 | Phase 2 シグネチャ | Phase 3 動作変化 |
|---|---|---|
| `synth_new` | `(f32, u32) -> *mut SynthHandle` | 内部で Engine::prepare に Modal Body 係数初期化 / Voice State buffer 確保 / Sustain state reset / Pitch Bend SmoothedValue 初期化を追加（外部仕様不変） |
| `synth_free` | `(*mut SynthHandle)` | 不変 |
| `synth_note_on` | `(*mut SynthHandle, u8, f32)` | 内部で `Engine::note_on` が Loss filter / Pick position 励振 shaping / Brightness 補正を計算 + 冒頭で `sustain_state.clear_pending(midi)` 呼ぶ（同一ノート再打鍵対策、D40）。外部仕様不変 |
| `synth_note_off` | `(*mut SynthHandle, u8)` | 内部で **Poly mode のみ** Sustain 状態を判定し、active 中なら pending bitmap に記録して voice の release を defer。**Mono mode では Sustain を無視**し Phase 2 既存の hold_stack 復帰判定 + `pool.note_off` 経路を完全継承（D40 の Mono+Sustain 仕様、Mono+Sustain は Phase 4 で再評価）。外部仕様不変 |
| `synth_set_param` | `(*mut SynthHandle, u32, f32)` | 内部で `PickPosition` / `BodyWet` の新規パラメータも fan-out。既存 Damping / Brightness / OutputGain も維持 |
| `synth_set_polyphony_mode` | `(*mut SynthHandle, u32)` | UI トグルからも呼ばれる。**内部で Sustain pending を即時 release してから sustain_state.reset()**（mode 切替の境界仕様、D40）+ Phase 2 既存の `hold_stack.clear()`。C ABI 自体は不変 |
| `synth_reset` | `(*mut SynthHandle)` | 内部で Modal Body / Sustain / Pitch Bend SmoothedValue / Voice State buffer も reset |
| `synth_out_l_ptr` | `(*const SynthHandle) -> *const f32` | 不変 |
| `synth_out_r_ptr` | `(*const SynthHandle) -> *const f32` | 不変 |
| `synth_capacity` | `(*const SynthHandle) -> u32` | 不変。返値は 128 |
| `synth_process_block` | `(*mut SynthHandle, u32)` | 内部で Modal Body / Soft clip / Voice State buffer 書き込みを実行（外部仕様不変） |
| (memory export) | WebAssembly.Memory | 不変 |

### Worklet 側からの呼び出し互換性

Phase 2 の Worklet `WasmExports` interface（[Phase 2 05 章](../2026-05-07-002-phase2/05-web-frontend-spec.md#wasmexports-interface-の拡張)）は Phase 3 でも **既存 12 関数の宣言部分は変更不要**。新 export 3 件を追加するのみ（[`05-web-frontend-spec.md`](./05-web-frontend-spec.md#wasmexports-interface-の-phase-3-拡張)）。

## Phase 3 で追加する C ABI 関数

### `synth_midi_cc`（D38）

```rust
/// MIDI CC dispatch を 1 関数で集約。
///   cc = 7   : Channel Volume → Engine の channel_volume target（D38b、UI OutputGain と直交）
///   cc = 64  : Sustain Pedal → Engine の sustain_state
///   cc = 123 : All Notes Off → 全 voice 即時 deactivate + hold_stack.clear() + sustain_state.reset()
///   その他   : 無視（CC#1 Mod Wheel は LFO 仕様確定が Phase 3 スコープ外、Phase 4 で対応）
///
/// value_normalized は [0.0, 1.0] 範囲。Worklet 側で MIDI 7-bit 値を /127 して渡す
#[unsafe(no_mangle)]
pub extern "C" fn synth_midi_cc(
    handle: *mut SynthHandle,
    cc: u8,
    value_normalized: f32,
) {
    if handle.is_null() {
        return;
    }
    let h = unsafe { &mut *handle };
    let v = value_normalized.clamp(0.0, 1.0);
    h.engine.handle_midi_cc(cc, v);
}
```

### `synth_pitch_bend`（D38）

```rust
/// Pitch Bend を全 active voice に fan-out。
/// semitones は [-2.0, +2.0] 範囲（±2 半音）、それ以外はクランプ
/// SmoothedValue 5ms tau で滑らかに遷移
#[unsafe(no_mangle)]
pub extern "C" fn synth_pitch_bend(
    handle: *mut SynthHandle,
    semitones: f32,
) {
    if handle.is_null() {
        return;
    }
    let h = unsafe { &mut *handle };
    h.engine.handle_pitch_bend(semitones);
}
```

### `synth_voice_state_ptr`（D41）

```rust
/// Voice State 共有メモリへのポインタを返す。
/// 33 bytes: active mask (u8) + 8 voice 振幅 (f32 little-endian × 8 = 32 bytes)
///
/// バッファは Engine が `process_block` 終端で 1 度だけ書き込む。
/// Worklet が `process` 終端で読んで postMessage で main へ転送する想定。
/// 共有メモリだが atomic 不要（write/read のスレッド境界が明確）。
#[unsafe(no_mangle)]
pub extern "C" fn synth_voice_state_ptr(
    handle: *const SynthHandle,
) -> *const u8 {
    if handle.is_null() {
        return core::ptr::null();
    }
    let h = unsafe { &*handle };
    h.engine.voice_state_ptr()
}
```

### 追加関数一覧

| 関数 | シグネチャ | 役割 |
|---|---|---|
| `synth_midi_cc` | `(*mut SynthHandle, u8, f32)` | MIDI CC#7 / #64 / #123 の汎用 dispatch（D38、Mod Wheel CC#1 は Phase 4 送り、未対応 CC は無視）。値は normalize 済 |
| `synth_pitch_bend` | `(*mut SynthHandle, f32)` | Pitch Bend ±2 半音、全 active voice fan-out（D38 / D39） |
| `synth_voice_state_ptr` | `(*const SynthHandle) -> *const u8` | Voice State 共有メモリ 33 bytes へのポインタ（D41） |

### Worklet 側 dispatch のテンプレ

Worklet で `synth_midi_cc` を呼ぶときの想定:

```typescript
// MessagePort で受信
{ type: 'midiCC', cc: 64, value: 1.0 }    // value = 127 / 127

// Worklet 側 dispatch
case 'midiCC':
  this.exports.synth_midi_cc(this.handle, msg.cc, msg.value);
  break;

case 'pitchBend':
  this.exports.synth_pitch_bend(this.handle, msg.semitones);
  break;
```

## SynthHandle 構造体の Phase 3 版

### 構造体定義（不変）

```rust
#[repr(C)]
pub struct SynthHandle {
    engine: Engine,           // Phase 3: Modal Body / Sustain / Voice State buffer / Pitch Bend SmoothedValue を含む
    scratch_l: Vec<f32>,      // Phase 1 / 2 と同じ、128 サンプル分
    scratch_r: Vec<f32>,      // 同上
}
```

外部から見たフィールド構成は Phase 1 / 2 と完全に同じ。`engine` の中身が拡張されただけで、wasm-audio 側のコードは Phase 1 / 2 と同じ構造のまま。

### Engine の内部構造（参考、03 章から再掲）

```text
Engine (Phase 3)
├─ pool: VoicePool<8>
│   ├─ voices: [KarplusStrong; 8]
│   │   ├─ buffer / lagrange (or thiran) / damping / brightness（既存）
│   │   ├─ loss_filter: LossFilter         ← Phase 3
│   │   ├─ pick_position: f32              ← Phase 3 (励振 shaping、SmoothedValue 不要)
│   │   ├─ pitch_bend_semitones: f32       ← Phase 3
│   │   └─ length_target: SmoothedValue    ← Phase 3 (Pitch Bend で動的)
│   └─ voice stealing 戦略（Phase 2 既存）
├─ output_gain: SmoothedValue (既存、UI master)
├─ hold_stack: HoldStack (Phase 2 既存)
├─ mode: SynthMode (Phase 2 既存)
├─ modal_body: ModalBodyResonator           ← Phase 3
├─ body_wet: SmoothedValue                  ← Phase 3
├─ pick_position: f32                       ← Phase 3 (励振 shaping、SmoothedValue 不要)
├─ channel_volume: SmoothedValue            ← Phase 3 (CC#7、output_gain と直交、D38b)
├─ sustain_state: SustainState              ← Phase 3 (active + pending_release u128)
└─ voice_state_buffer: [u8; 33]             ← Phase 3
```

## メモリ・パフォーマンスへの影響

### Phase 2 のメモリレイアウト（再掲）

`Engine::prepare` で `VoicePool::voices[i].buffer` × 8 + `scratch_l/r` を一括確保 ≈ 57 KB。

### Phase 3 でのメモリ追加

[02 章 §メモリレイアウトの変更](./02-architecture.md#phase-3-のメモリレイアウト) より、Phase 3 追加分 ≈ 1.5 KB:

| 追加領域 | サイズ |
|---|---|
| Modal Body 状態 + 係数 (stereo、bandpass biquad) | 320 B |
| Sustain pending bitmap (u128 + bool) | 17 B |
| Voice State buffer | 33 B |
| Pitch Bend SmoothedValue × 8 voice | 96 B |
| Body Wet SmoothedValue | 12 B |
| Channel Volume SmoothedValue (CC#7、D38b、UI OutputGain と直交) | 12 B |
| 各 Voice の Loss filter 状態 (× 8) | 32 B（z1 のみ × 8 voices） |
| Engine.pick_position: f32 (D34、SmoothedValue 不要) | 4 B |
| Pick position delay buffer | 0 B（励振 shaping は既存 KS buffer 使い回し） |

> **注**: 旧版仕様（PickPosition delay バッファ 256 × f32 × 8 voices = 8 KB）は、励振 shaping（D34 設計変更版）への移行に伴い **不要**。Pick position は KarplusStrong の既存 buffer を `note_on` 時に使い回すため追加メモリゼロ。Phase 2 の 57 KB に対する Phase 3 追加分は 02 章 §メモリレイアウトの変更で +1.5 KB 程度（Modal Body 状態 + Voice State + Pitch Bend SmoothedValue 等）に縮小。

### `synth_new` 内部での確保フロー（Phase 3 版、03 章から再掲）

```
synth_new(sample_rate, max_block_size)
  └─ Box::new(SynthHandle { engine, scratch_l, scratch_r })
       └─ Engine::new() / engine.prepare(sr, mb)
            ├─ pool.prepare(sr, mb)
            │    ├─ voices[0..7].prepare(sr, mb)
            │    │   ├─ buffer = vec![0; 1749]
            │    │   ├─ loss_filter.reset()
            │    │   └─ length_target / pitch_bend SmoothedValue 初期化
            │    │   // pick_position は f32 フィールドのみ、励振 shaping は note_on 時に既存 buffer を使い回す（D34）
            ├─ output_gain / body_wet / channel_volume SmoothedValue 初期化
            │    // channel_volume はデフォルト 1.0 (CC#7 未送信時に音量変化なし)
            ├─ hold_stack.clear()
            ├─ modal_body.prepare(sr)
            │    ├─ coeffs_l[0..7].calc_coeffs(BODY_MODES_L[i], sr)  // bandpass biquad
            │    └─ coeffs_r[0..7].calc_coeffs(BODY_MODES_R[i], sr)
            ├─ sustain_state.reset()
            └─ voice_state_buffer = [0; 33]
            // engine.pick_position: f32 フィールドは default で 0.125 (D34 デフォルト)
```

`synth_new` 完了後、`process_block` / `note_on` / `note_off` / `set_param` / `set_polyphony_mode` / `synth_midi_cc` / `synth_pitch_bend` / `synth_voice_state_ptr` のいずれを呼んでも byteLength 不変（D4 / D9 継承、F37 で検証）。

## C ABI 名前空間

### Phase 3 完了時点の export 一覧

```rust
// Phase 1（10 関数）
synth_new, synth_free,
synth_note_on, synth_note_off,
synth_set_param, synth_reset,
synth_out_l_ptr, synth_out_r_ptr,
synth_capacity, synth_process_block

// Phase 2（+2 = 12 関数）
synth_set_polyphony_mode

// Phase 3（+3 = 15 関数）
synth_midi_cc
synth_pitch_bend
synth_voice_state_ptr

// メモリ export（不変）
memory  // WebAssembly.Memory
```

### `scripts/check-wasm-exports.mjs` REQUIRED 配列

Phase 3 で 3 関数を追加するため、以下を追記:

```javascript
const REQUIRED = [
  'memory',
  'synth_new',
  'synth_free',
  'synth_note_on',
  'synth_note_off',
  'synth_set_param',
  'synth_reset',
  'synth_out_l_ptr',
  'synth_out_r_ptr',
  'synth_capacity',
  'synth_process_block',
  'synth_set_polyphony_mode',
  // Phase 3 追加
  'synth_midi_cc',
  'synth_pitch_bend',
  'synth_voice_state_ptr',
];
```

`pnpm build:wasm` で WASM が生成された直後に検証され、3 関数のいずれかが export されていなければ exit 1。

## ビルド方法（Phase 1 / 2 から不変）

[Phase 1 04 章 §ビルド方法](../2026-05-06-001-mvp/04-wasm-audio-spec.md#ビルド方法) を完全継承。

```bash
pnpm build:wasm        # release: gen-params → cargo build --release → copy-wasm release → check-wasm-exports
pnpm build:wasm:dev    # dev:     gen-params → cargo build (dev)   → copy-wasm debug   → check-wasm-exports
```

`check-wasm-exports.mjs` の REQUIRED 配列に Phase 3 で追加した 3 関数が含まれることを Step 13（仕様書 07 章）で確認。

## サイズ予算

[01 章 §性能予算](./01-overview.md#ゴール) と [pre-research §9.1](./pre-research.md#91-wasm-サイズ予算gzip) より、Phase 3 完了時の WASM gzip 想定 12.9 KB（target 30 KB の 43%）。

wasm-audio 自体の Phase 3 増分は 3 関数追加で +0.5 KB raw / +0.25 KB gzip 程度。残りは dsp-core 側（Modal Body / Loss filter / Pick position / Sustain / Soft clip / Thiran allpass）。

## 拡張性

Phase 1 / 2 の拡張性確保を継承。Phase 3 で新たに考慮する点:

- **`synth_midi_cc` 1 関数集約の利点**: Phase 4 で CC#11 (Expression) / CC#10 (Pan) / CC#71 (Resonance) 等を追加する場合、C ABI 関数追加なし、`Engine::handle_midi_cc` の switch 拡張のみで対応可能（D38 の根拠）
- **`synth_voice_state_ptr` の 33 byte レイアウト**: Phase 4 で voice 数を増やす（N=16）場合、レイアウトを `1 + 16×4 = 65 bytes` に拡張するだけで対応。共有メモリポインタ方式は柔軟
- **Pitch Bend の SmoothedValue 化**: Phase 4 で polyphonic aftertouch / per-key Pitch Bend を入れる場合、`synth_pitch_bend_per_voice(midi_note, semitones)` を追加して voice ごとに分岐可能
