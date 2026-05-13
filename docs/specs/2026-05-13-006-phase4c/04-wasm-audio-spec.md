# 04. wasm-audio 仕様（Phase 4c）

## 目的

`crates/wasm-audio/src/lib.rs` の C ABI 境界に Phase 4c で **新規追加する関数はない**。Phase 4a / 4b 確定の **18 C ABI 関数 + memory export = 19 required exports** のシグネチャ・export 名・動作はすべて完全互換を維持する（D18 / D38 / D39 / D41 / D44-D52 / D64 継承）。Phase 4b で確定した `synth_apply_instrument(handle, 7)` で Piano kind 切替も Phase 4c で同じ動作（内部の `Engine::apply_instrument` 実装が拡張されるのみ、C ABI シグネチャは不変）。

## 他文書との関係

- 上流: [`01-overview.md`](./01-overview.md)（C ABI 既存 18 関数 + memory export = 19 required exports の互換性チェックリスト + Phase 4c で追加する関数なし、D81）、[`03-dsp-core-spec.md`](./03-dsp-core-spec.md)（`Engine::apply_instrument` の Phase 4c 拡張 / `Engine::handle_midi_cc(CC_SUSTAIN_PEDAL)` の Phase 4c 拡張 / `Engine::note_on` の Phase 4c 拡張 / `Engine::process` ブロック関数 per-sample loop の Phase 4c 拡張、いずれも内部実装の変更のみで C ABI 引数は不変）
- 下流: [`05-web-frontend-spec.md`](./05-web-frontend-spec.md)（Worklet `WasmExports` interface 不変）、[`07-implementation-checklist.md`](./07-implementation-checklist.md)
- 並行: Phase 4a [`04-wasm-audio-spec.md`](../2026-05-08-004-phase4a/04-wasm-audio-spec.md) / Phase 4b [`04-wasm-audio-spec.md`](../2026-05-09-005-phase4b/04-wasm-audio-spec.md) — 同形式の C ABI 維持パターン参照

## C ABI 既存 18 関数 + memory export = 19 required exports（Phase 4c で完全維持）

| 関数名 | シグネチャ | Phase 4c 状況 |
|---|---|---|
| `synth_new` | `(f32, u32) -> *mut SynthHandle` | 維持。内部で Phase 4c の `string_buffers` × 3 + `resonance_bus.buffer` も一括確保（heap 確保ゼロ維持） |
| `synth_free` | `(*mut SynthHandle)` | 維持 |
| `synth_note_on` | `(*mut SynthHandle, u8, f32)` | 維持。内部で `Engine::note_on` を呼び、その中で Multi-string detune + B(note) LUT lookup + Hertz hammer raised cosine 初期化が走る（C ABI 引数は変わらず、dsp-core 側で吸収） |
| `synth_note_off` | `(*mut SynthHandle, u8)` | 維持 |
| `synth_set_param` | `(*mut SynthHandle, u32, f32)` | 維持。Phase 4c で新規 ParamId 追加なし（D81、`unison_detune_cents` / `sympathetic_amount` は Piano プリセット内パラメータで、ParamId 経由では設定しない） |
| `synth_set_polyphony_mode` | `(*mut SynthHandle, u32)` | 維持 |
| `synth_reset` | `(*mut SynthHandle)` | 維持。内部で `Engine::reset` → `KarplusStrong::reset` (Multi-string 全弦 + dispersion stages reset) + `ResonanceBus::reset` を実行 |
| `synth_out_l_ptr` | `(*const SynthHandle) -> *const f32` | 維持 |
| `synth_out_r_ptr` | `(*const SynthHandle) -> *const f32` | 維持 |
| `synth_capacity` | `(*const SynthHandle) -> u32` | 維持 |
| `synth_process_block` | `(*mut SynthHandle, u32)` | 維持。内部で Multi-string + ResonanceBus も走るが外部仕様は不変 |
| `synth_midi_cc` | `(*mut SynthHandle, u8, f32)` | 維持。CC#64 (sustain) で内部の `Engine::handle_midi_cc(CC_SUSTAIN_PEDAL, v)` 拡張経路（`sustain_state.set_active` + `release_pending` + `resonance_bus.set_feedback_gain_target`）が走る（C ABI 経由では透過、03 章 §4.5） |
| `synth_pitch_bend` | `(*mut SynthHandle, f32)` | 維持 |
| `synth_voice_state_ptr` | `(*const SynthHandle) -> *const u8` | 維持 |
| `synth_apply_instrument` | `(*mut SynthHandle, u32)` | 維持。Phase 4b で `kind=7` (Piano) 追加済、**Phase 4c でも値域 0-7 のまま変更なし**。内部の `Engine::apply_instrument` が Piano プリセット内パラメータ（`unison_detune_cents` / `sympathetic_amount` / B-curve 関数ポインタ）を切替 |
| `synth_set_lfo_rate` / `synth_set_lfo_depth_pitch` / `synth_set_lfo_depth_brightness` / `synth_set_lfo_depth_volume` | (Phase 4a 関数) | 維持 |
| `memory` (export) | (WebAssembly.Memory) | 維持。Phase 4c で初期確保 +169 KB の影響あるが、`memory.grow()` 自動拡張で吸収 |

合計: **19 required exports**（Phase 4a / 4b と完全同値、新規追加なし）。

## SynthHandle / Engine ラッパの構造（Phase 4c で内部のみ拡張）

```rust
// Phase 4b と同型、変更なし
pub struct SynthHandle {
    engine: Engine,
    scratch_l: Vec<f32>,
    scratch_r: Vec<f32>,
}

impl SynthHandle {
    pub fn new(sample_rate: f32, max_block: u32) -> Self {
        let mut handle = Self {
            engine: Engine::new(),
            scratch_l: Vec::with_capacity(max_block as usize),
            scratch_r: Vec::with_capacity(max_block as usize),
        };
        handle.engine.prepare(sample_rate);  // ← Phase 4c で内部に resonance_bus.prepare 等が増えるが外部 API 不変
        handle.scratch_l.resize(max_block as usize, 0.0);
        handle.scratch_r.resize(max_block as usize, 0.0);
        handle
    }
}
```

`SynthHandle` 自体の構造変化なし。`Engine` 内部に Phase 4c で `resonance_bus` / `unison_detune_cents` / `sympathetic_amount` / `inharmonicity_b_for_note` / `hammer_cutoff_low_hz` / `hammer_cutoff_high_hz` / `bus_out_prev` が追加されるが、これらは C ABI からは見えない。

## `synth_apply_instrument(handle, 7)` の Phase 4c での動作

```rust
#[unsafe(no_mangle)]
pub extern "C" fn synth_apply_instrument(handle: *mut SynthHandle, kind: u32) {
    if handle.is_null() {
        return;
    }
    let handle = unsafe { &mut *handle };
    if let Some(instrument_kind) = InstrumentKind::from_u32(kind) {
        handle.engine.apply_instrument(instrument_kind);
        // ↑ Phase 4c で内部実装が拡張される:
        //   - Phase 4a/4b: pool.all_notes_off + modal_body.set_instrument + (Phase 4b) pool.set_dispersion_active
        //   - Phase 4c: 上記 + unison_detune_cents / sympathetic_amount / b_curve 関数ポインタ / hammer_cutoff の切替
        //               + resonance_bus.feedback_gain の再設定（Sustain ON 状態を考慮）
    }
}
```

外部 (Worklet) からは Phase 4b と同じ呼び方で動作（`synth_apply_instrument(handle, 7)` で Piano、`6` で Sitar、`0` で Default）。

## `InstrumentKind::from_u32` の値域（Phase 4b と同型）

```rust
impl InstrumentKind {
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(InstrumentKind::Default),
            1 => Some(InstrumentKind::GuitarClassical),
            2 => Some(InstrumentKind::Ukulele),
            3 => Some(InstrumentKind::Mandolin),
            4 => Some(InstrumentKind::Bass),
            5 => Some(InstrumentKind::GuitarSteel),
            6 => Some(InstrumentKind::Sitar),
            7 => Some(InstrumentKind::Piano),
            _ => None,
        }
    }
}
```

Phase 4c で `InstrumentKind` enum の variant 追加なし。`INSTRUMENT_KIND_COUNT = 8` 維持。

## `scripts/check-wasm-exports.mjs` への影響

Phase 4b と同じ 19 required exports が出力されること、Phase 4c で追加なし、これを CI で自動検証（Phase 4a 既存スクリプト）。

```javascript
// scripts/check-wasm-exports.mjs (Phase 4c で変更なし)
const REQUIRED_EXPORTS = [
  'synth_new', 'synth_free', 'synth_note_on', 'synth_note_off',
  'synth_set_param', 'synth_set_polyphony_mode', 'synth_reset',
  'synth_out_l_ptr', 'synth_out_r_ptr', 'synth_capacity',
  'synth_process_block', 'synth_midi_cc', 'synth_pitch_bend',
  'synth_voice_state_ptr', 'synth_apply_instrument',
  'synth_set_lfo_rate', 'synth_set_lfo_depth_pitch',
  'synth_set_lfo_depth_brightness', 'synth_set_lfo_depth_volume',
  'memory',
];
```

## C ABI の memory 影響（Phase 4c での増分）

| 項目 | Phase 4b | Phase 4c | 増分 |
|---|---|---|---|
| `KarplusStrong` × 8 voice の buffer 合計 | ~56 KB (8 × 1746 × 4 byte) | ~168 KB (8 × 3 × 1746 × 4 byte) | +112 KB |
| `KarplusStrong` の dispersion stages 合計 | ~0.77 KB (8 × 8 × 12 byte) | ~2.3 KB (8 × 3 × 8 × 12 byte) | +1.5 KB |
| `KarplusStrong` の string_states (Thiran 含む) | — | ~0.77 KB | +0.77 KB |
| `ResonanceBus::buffer` | — | ~0.38 KB (96 × 4 byte) | +0.38 KB |
| `Engine` の Phase 4c 追加フィールド | — | <0.1 KB | +0.1 KB |
| **合計（WASM ヒープ）** | ~64 KB | ~233 KB | **+169 KB** |

`WebAssembly.Memory` の初期サイズが 256 KB なら、Phase 4c で `memory.grow()` が `prepare` 時に 1 回発火する可能性。Worklet 側 `refreshViews()` が prepare 時に 1 回発火（Phase 4b までも同じ）、`process` ホットパスでは発火ゼロ（既存条件継承）。

## Phase 4a / 4b 互換性の機械保証

`synth_apply_instrument(handle, 0)` (Default kind) の動作が Phase 4a HEAD と byte 一致継承を保証する経路:

1. `Engine::apply_instrument(Default)` → `pool.set_dispersion_active(false)` (Phase 4b D67 経路)
2. 各 voice の `dispersion_active = false`、`n_strings_active = 1` (D70 / D83)
3. `note_on` で pluck excitation path（Phase 4a と同じ noise burst + pick comb）
4. `process_sample` で dispersion cascade skip + 1 弦のみ処理
5. `ResonanceBus::feedback_gain.target = 0` で bus inject も 0

→ Default kind の出力は Phase 4a HEAD と完全に byte 一致（`tests/fixtures/phase4a_default_c4_v08.rs` を Phase 4c でも使用、D83）。

## 互換性テスト（Phase 4c で追加・更新）

`crates/wasm-audio/` 自体には Phase 4c 追加テストなし。dsp-core 側の `test_default_n_strings_1_matches_phase4a` で機械保証（F61-a、03 章 §7.5 参照）。

## まとめ

Phase 4c の wasm-audio 仕様は **C ABI 完全不変** が中核。19 required exports すべて Phase 4a / 4b と同シグネチャ・同 export 名で、`synth_apply_instrument(handle, 7)` で Piano 切替動作も同じ。Phase 4c の DSP 拡張（Multi-string / Hertz hammer / Sympathetic / B(note) LUT）はすべて dsp-core 内部の `Engine::apply_instrument` / `Engine::note_on` / `Engine::handle_midi_cc(CC_SUSTAIN_PEDAL)` / **`Engine::process(output_l, output_r)` ブロック関数の per-sample loop** の実装拡張で吸収（03 章 §4.2〜§4.5）。`scripts/check-wasm-exports.mjs` で 19 exports の維持を CI で自動検証。memory は +169 KB（`prepare` で一括確保、`process` 中 alloc ゼロ維持）。Phase 4a HEAD byte 一致は Default kind 経路 (`n_strings = 1` + dispersion_active=false + feedback_gain=0) で完全継承（D83）。
