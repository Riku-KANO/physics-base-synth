# 02. Phase 4c アーキテクチャ

## 目的

Phase 1 / 2 / 3 / 4a / 4b で確立した 4 レイヤ構成（Svelte UI → AudioWorkletProcessor → wasm-audio → dsp-core）に対し、Phase 4c で追加する責務（Multi-string per voice / Hertz law raised cosine hammer / Global sympathetic resonance bus / B(note) LUT / Piano プリセット聴感チューニング / F38b 実機計測再取得）の配置を明確化する。Phase 4b までの構成は崩さず、新規責務をどのレイヤに置くかと既存コンポーネントの拡張点を定義する。

## 他文書との関係

- 上流: [`01-overview.md`](./01-overview.md)（Phase 4c の確定事項 / D68-D85 / 完成像）
- 下流: 本書のレイヤ責務に従って 03〜05 章で具体的な API / モジュール / コンポーネントを定義
- 並行: Phase 1 / 2 / 3 / 4a / 4b [`02-architecture.md`] — 同じ章番号 / 構造、Phase 4c は差分のみ記述

## 4 レイヤ構成（Phase 4b から不変）

```
Svelte UI (main thread) ──MessagePort──▶ AudioWorkletProcessor
                                              │ FFI (C ABI, raw exports)
                                              ▼
                                         wasm-audio (cdylib)
                                              │
                                              ▼
                                         dsp-core (rlib)
```

各レイヤの責務と禁止事項は CLAUDE.md と Phase 1 / 2 / 3 / 4a / 4b の 02 章で定義済み。Phase 4c でも変更しない。

## Phase 4c で追加される責務の配置

### Svelte UI 層（main thread）

| 責務 | ファイル | 内容 |
|---|---|---|
| Piano プリセットの `unison_detune_cents` / `sympathetic_amount` フィールド表示 | `web/src/lib/state/factory-presets.ts` | Piano エントリに新フィールドを追加。**UI 露出なし**（Phase 4d 候補、D81）。プリセット内部値として localStorage v1 に保存 |
| Piano プリセット聴感チューニング (Step 17-19) | `web/src/lib/state/factory-presets.ts` | Step 19 で `damping` / `brightness` / `bodyWet` / `unison_detune_cents` / `sympathetic_amount` / `hammer_cutoff_*` の値を聴感調整で反復更新 |
| F38b 実機計測 (Step 1 / Step 20) | （Phase 4b で完成済の `__synthDev.measureProcessTime` API を使う） | `pnpm dev` + Console から `window.__synthDev.measureProcessTime(5000)` 呼び出し、avg/max/min/samples を取得。実装変更なし |

新規コンポーネント / 新規ストア / 新規 action は **追加なし**。Phase 4c は UI レベルでは「Piano プリセットの値が変わる」のみ（D81）。

### AudioWorkletProcessor 層

| 責務 | ファイル | 内容 |
|---|---|---|
| Piano プリセットの新フィールド受け渡し | `web/src/lib/audio/synth-processor.ts` | 既存パターン継承、Piano kind 切替時に Engine 内部で WASM 側へ値を渡す（C ABI 関数追加なし、D81） |
| `InstrumentKindKey` 拡張 | `web/src/lib/audio/messages.ts` | Phase 4b で既に `'piano'` 追加済、Phase 4c での変更なし |
| F38b 計測 API | `web/src/lib/audio/__synthDev.ts` | Phase 4b 完成済、Phase 4c での変更なし |

### wasm-audio 層

| 責務 | ファイル | 内容 |
|---|---|---|
| C ABI 既存 18 関数 + memory export = 19 required exports の維持 | `crates/wasm-audio/src/lib.rs` | **変更なし**（D81、Phase 4b と完全同形式）。`synth_apply_instrument(kind=7)` で Piano 切替動作も同じ |

### dsp-core 層（Phase 4c で最大の変更を受ける層）

| 責務 | ファイル | 内容 |
|---|---|---|
| Multi-string per voice (1/2/3 弦並列 KS) | `crates/dsp-core/src/karplus_strong.rs` | **拡張**: `[StringState; 3]` + `n_strings_active: usize` + 弦個別 buffer 配列（案 1）を追加（D70 / D71）。`note_on_internal` で `n_strings(midi)` 関数で弦数確定 + 各弦に detune 適用 + 弦個別 `adjusted_length` 算出。`process_sample` で N 弦の KS ループを並列実行 + 加算で 1 値を返す |
| Hertz law raised cosine hammer | `crates/dsp-core/src/karplus_strong.rs` | **拡張**: `note_on_internal` の dispersion_active 経路で buffer 初期化を `velocity → (t_c_ms, f_c_hz, amp)` パラメータ式で再構築（D74 / D75）。Phase 4b の Commuted impulse 経路を置換 |
| Global sympathetic resonance bus | `crates/dsp-core/src/resonance_bus.rs` | **新規**: `ResonanceBus { buffer, lpf, feedback_gain, write_idx }` 構造体 + `process()` メソッド（D76） |
| Engine への ResonanceBus 統合 | `crates/dsp-core/src/engine.rs` | **拡張**: `Engine` 構造体に `resonance_bus: ResonanceBus` を追加。**既存 `Engine::process(output_l, output_r)` ブロック関数の per-sample loop 内** (`engine.rs:445-492`) に「全 voice 出力を bus に sum → bus lossy feedback → 次 sample で各 voice ループに inject」の経路を 3 行追加（dry / bus_out / modal_body 入力の置換、03 章 §4.4）。`handle_midi_cc(CC_SUSTAIN_PEDAL, v)` の既存経路を拡張して `feedback_gain` ターゲットも SmoothedValue 経由で切替（D76 / D77、03 章 §4.5） |
| 88 鍵 Inharmonicity B(note) LUT | `crates/dsp-core/src/params.rs` (生成) | **拡張**: `gen-params.mjs` で `pub const INHARMONICITY_B_CURVE_PIANO: [f32; 88]` を出力（D78 / D79） |
| Dispersion a1 計算の B 引数化 | `crates/dsp-core/src/dispersion.rs` | **拡張**: `compute_dispersion_a1(M, b, f_0, fs)` のシグネチャを Phase 4b から維持しつつ、呼び出し側で `b` を LUT から渡すよう変更（既存 const `INHARMONICITY_B_PIANO = 7.5e-4` は Phase 4b 互換性のため残置、新規呼出は LUT 値を使用） |
| VoicePool への Phase 4c メソッド追加 | `crates/dsp-core/src/voice_pool.rs` | **拡張**: `set_piano_params(unison_detune, b, cutoff_low, cutoff_high)` で全 voice に楽器パラメータを fan-out、`note_on_with_piano_params(midi, freq, vel, ...)` で割当先 voice にだけ `set_instrument_params + note_on_with_id` を順次呼ぶ、`process_sample_with_feedback(bus_out_prev, feedback_gain) -> f32` で Sympathetic 注入経路を内包し既存 `poly_scale = 1/√N` を維持。`voices` は **private のまま**（Phase 4b 同等、`voice_pool.rs:8`） |
| Phase 4a 互換性 fixture 継承 | `crates/dsp-core/tests/fixtures/phase4a_default_c4_v08.rs` | **維持**: Phase 4b の 527 行 fixture をそのまま使用、`test_default_n_strings_1_matches_phase4a` で `n_strings = 1` 経路の byte 一致を機械保証（D83） |
| Multi-string 統合テスト | `crates/dsp-core/tests/multi_string_tests.rs` | **新規**: `n_strings(midi)` 関数 / 弦別 detune / 弦別 dispersion / `n_strings=1` で Phase 4b 同等 |
| Sympathetic resonance 統合テスト | `crates/dsp-core/tests/sympathetic_tests.rs` | **新規**: ペダル ON で bus feedback 動作 / Piano 以外で sympathetic_amount=0 / ループ安定性 |
| Hertz hammer 統合テスト | `crates/dsp-core/tests/hammer_hertz_tests.rs` | **新規**: velocity 別 t_c_ms / f_c_hz / amplitude 検証 / raised cosine 形状の波形検証 |

## Phase 4c で変更なし / Phase 4b 同等のレイヤ

- **AudioContext / AudioWorkletNode 起動**: Phase 1 D1〜D8（`StartButton.onclick` 内で `new AudioContext()` + `resume()` + `audioWorklet.addModule()` + `AudioWorkletNode`）を完全継承
- **MessagePort + SmoothedValue でクリック対策**: Phase 1 D9 を完全継承
- **Polyphony 8 / NoteAllocator (LIFO 古い → 静か)**: Phase 2 D12 / D13 を完全継承
- **Lagrange 3 次補間 + Thiran allpass + LFO + Mod Wheel + Preset v1 + 多楽器 8 種**: Phase 2 / 3 / 4a / 4b を完全継承
- **`wasm-opt -O3` + `excitation_snapshot` cfg(test) + `.gitattributes` LF**: Phase 4a / 4b を完全継承

## データフロー（Phase 4c で追加される経路）

### Multi-string + Sympathetic Resonance フロー (Piano kind)

```
Engine::note_on(midi=60, velocity=0.8)            ← 既存 Mono / Sustain ロジックは維持
  ├─ sustain_state.clear_pending(60)              ← Phase 3 既存
  ├─ (Mono mode) hold_stack.push_unique(60) + 前 top release  ← Phase 2 既存
  └─▶ Engine::trigger_voice(60, 0.8)              ← Phase 4c で内部だけ差し替え
        ├─ B = b_curve_piano(60) = LUT[clamp(60,21,108) - 21]    ← D78 / MIDI clamp
        ├─ freq = midi_to_freq(60)
        └─▶ VoicePool::note_on_with_piano_params(60, freq, 0.8, detune, B, cutoff_low, cutoff_high)
              ├─ assigned = allocate_voice(60)            ← 既存 3 段フォールバック (same-note / free / steal)
              ├─ voices[assigned].set_instrument_params(detune, B, cutoff_low, cutoff_high)
              └─▶ voices[assigned].note_on_with_id(60, freq, 0.8)
                    └─▶ KarplusStrong::note_on_internal()
                          ├─ n_strings_active = if dispersion_active { n_strings(60)=3 } else { 1 }
                          ├─ for string_idx in 0..3:
                          │    ├─ detune_cents = [0, -1.5, +1.5][string_idx]  ← D72
                          │    ├─ f_0_string = f_0_base × 2^(detune/1200)
                          │    ├─ (a1, gd_per_stage) = compute_dispersion_a1(M=8, B, f_0_string, fs)  ← tuple 戻り値、D78
                          │    ├─ adjusted_length = raw_len - brightness_tau_g - M·gd_per_stage
                          │    └─ init_excitation_for_string(string_idx, velocity):
                          │         ├─ t_c_ms = 4.0 - 2.8·v             ← D75
                          │         ├─ f_c_hz = 800 + 4700·v
                          │         ├─ amp = √v
                          │         ├─ buffer[i] = amp · sin²(πi/t_c_samples)   ← raised cosine
                          │         └─ velocity LPF (alpha = compute_lpf_alpha(f_c_hz))
                          └─ note_id = Some(60); active = true

Engine::apply_instrument(Piano)                                              ← 楽器切替時
  ├─ pool.all_notes_off()                                                    ← Phase 4a D53
  ├─ hold_stack.clear()
  ├─ sustain_state.reset()                                                   ← engine.rs:317 既存挙動
  ├─ pool.set_dispersion_active(true)                                        ← Phase 4b D67
  ├─ pool.set_piano_params(detune, 0.0, cutoff_low, cutoff_high)             ← Phase 4c (D72/D75)
  ├─ resonance_bus.set_feedback_gain_target(0.0)                             ← sustain リセット済なので 0
  ├─ resonance_bus.reset()                                                   ← Phase 4c: bus 残留を切る
  └─ bus_out_prev = 0.0                                                      ← Phase 4c: 前 sample もクリア

process_sample() (per sample, in Engine loop)
  ├─ feedback_gain = resonance_bus.next_feedback_gain()        ← SmoothedValue 進行
  └─▶ sum_voices = pool.process_sample_with_feedback(bus_out_prev, feedback_gain)
       │   (VoicePool::voices は private のまま、Engine から voice 配列に触らない)
       │   ├─ inject = bus_out_prev × feedback_gain            ← 注入値（Default kind は 0）
       │   ├─ for each voice in voices (private):
       │   │    ├─ voice.inject_feedback(inject)               ← bus_feedback_pending を更新
       │   │    └─▶ voice.process_sample()
       │   │          ├─ sum_strings = 0.0
       │   │          ├─ for string_idx in 0..n_strings_active:
       │   │          │    ├─ x = dispersion_cascade(buffer_string[read_z])  ← 弦個別
       │   │          │    ├─ y = thiran_string.process(x)                    ← 弦個別
       │   │          │    ├─ buffer_string[write] = y·damping + bus_feedback_pending
       │   │          │    └─ sum_strings += y
       │   │          ├─ bus_feedback_pending = 0.0           ← 1 sample で消費
       │   │          └─ out = brightness_lpf(sum_strings) × loss_filter
       │   │
       │   └─ return sum * poly_scale                          ← Phase 2 D20 1/√N
       │
       ├─ bus_out = resonance_bus.process(sum_voices)          ← D76 lossy delay+LPF (常に駆動)
       ├─ bus_out_prev = bus_out                               ← 次 sample 用
       ├─ bus_mix = feedback_gain / FEEDBACK_GAIN_MAX           ← gate (Default kind 等は常に 0)
       └─ main_out = modal_body(sum_voices + bus_out·BUS_DIRECT_MIX_GAIN·bus_mix)
            → soft_clip × output_gain → audio out

(Default kind での byte 一致):
  feedback_gain = 0 → 注入 inject = 0 / 直接 mix bus_mix = 0
  → modal_body 入力は dry のみ、Phase 4a HEAD と byte 一致継承 (D83)
```

### Phase 4b との差分

- **Voice 配列**: Phase 4b は 1 voice = 1 弦、Phase 4c は 1 voice = N 弦 (Piano は 1/2/3、他楽器は常に 1)、`process_sample` で N 弦の和を返す
- **Sympathetic 経路**: Phase 4b は voice → modal_body → soft_clip、Phase 4c は **`pool.process_sample_with_feedback(bus_out_prev, feedback_gain)`** が内部で voice 注入 + 合算 + `poly_scale` を実行し、その後 `resonance_bus.process(sum_voices)` で bus_out を更新、modal_body 入力に bus_out も重ねる
- **note_on の差し替え点**: 既存の `Engine::note_on(midi, velocity)` の構造 (`sustain_state.clear_pending` / Mono 分岐 / `hold_stack.push_unique`) は **完全に維持**、内部の `trigger_voice(midi, velocity)` だけ Phase 4c で `pool.note_on(midi, freq, velocity)` → `pool.note_on_with_piano_params(midi, freq, velocity, detune, B, cutoff_low, cutoff_high)` に差し替え
- **apply_instrument の Sustain 扱い**: 既存実装 (`engine.rs:317`) が `sustain_state.reset()` を呼ぶため、楽器切替後の bus feedback_gain は常に 0 ターゲット。「楽器切替で sustain を引き継ぐ」設計ではない

## ファイル変更リスト

### 新規作成

| ファイル | 内容 | サイズ目安 |
|---|---|---|
| `crates/dsp-core/src/resonance_bus.rs` | `ResonanceBus` 構造体 + `process()` メソッド | ~150 行 |
| `crates/dsp-core/tests/multi_string_tests.rs` | Multi-string 統合テスト | ~250 行 |
| `crates/dsp-core/tests/sympathetic_tests.rs` | Sympathetic resonance 統合テスト | ~200 行 |
| `crates/dsp-core/tests/hammer_hertz_tests.rs` | Hertz hammer 統合テスト | ~180 行 |
| `crates/dsp-core/src/string_state.rs` | `StringState` 構造体 + 関連定数（`karplus_strong.rs` 内に置く案も検討、§3 で確定） | ~100 行 |

### 拡張

| ファイル | 主な変更 |
|---|---|
| `crates/dsp-core/src/karplus_strong.rs` | `[StringState; 3]` + `n_strings_active` 追加、`note_on_internal` を multi-string + Hertz hammer 対応、`process_sample` を N 弦並列化、`set_instrument_params(...)` / `inject_feedback(value)` を追加、`bus_feedback_pending: f32` を追加（公開 Voice trait は不変） |
| `crates/dsp-core/src/voice_pool.rs` | `note_on_with_piano_params(midi, freq, vel, detune, b, cutoff_low, cutoff_high)` を追加（割当 voice にだけ `set_instrument_params` + `note_on_with_id` を順次呼ぶ）、`set_piano_params(detune, b, cutoff_low, cutoff_high)` で全 voice に楽器パラメータを fan-out、`process_sample_with_feedback(bus_out_prev, feedback_gain) -> f32` で `inject = bus_out_prev × feedback_gain` を内部で各 voice に注入後 `poly_scale = 1/√N` を最後に掛けて返す。`voices` は **private のまま**（`voice_pool.rs:8`）、既存 `process_sample()` / `note_on(midi, freq, vel)` も削除せず互換維持 |
| `crates/dsp-core/src/engine.rs` | `resonance_bus: ResonanceBus` 追加、`bus_out_prev: f32` 追加、`unison_detune_cents` / `sympathetic_amount` / `inharmonicity_b_for_note: fn(u8) -> f32` / `hammer_cutoff_low_hz` / `hammer_cutoff_high_hz` 追加、`apply_instrument` で楽器パラメータ切替 + `pool.set_piano_params` fan-out + `resonance_bus.set_feedback_gain_target(0.0)`（`sustain_state.reset()` 既存挙動と整合）、`trigger_voice` の内部呼出だけ `pool.note_on` → `pool.note_on_with_piano_params` に差し替え（公開 `Engine::note_on` の Mono / Sustain ロジックは現行 `engine.rs:129-147` を完全維持）、**既存 `Engine::process(output_l, output_r)` の per-sample loop 内** に `pool.process_sample_with_feedback + resonance_bus.process + bus_out を modal_body 入力にミックス` の 3 行を追加、`handle_midi_cc(CC_SUSTAIN_PEDAL, v)` 既存経路を拡張して `feedback_gain.target` 切替（**Phase 3 D40 既存の `let released = sustain_state.set_active(on); self.release_pending(released);` ペアは絶対に維持**） |
| `crates/dsp-core/src/dispersion.rs` | `compute_dispersion_a1(m, b, f_0, fs) -> (f32, f32)` の **tuple 戻り値を維持**（`karplus_strong.rs:201` 同型）、`b_curve_piano(midi)` / `b_curve_zero(midi)` を追加（前者は `midi.clamp(21, 108) - 21` を index に LUT 引き） |
| `crates/dsp-core/src/voice_pool.rs` | `inject_feedback` メソッド追加（Sympathetic bus 用） |
| `crates/dsp-core/src/lib.rs` | `pub mod resonance_bus;` + `pub mod string_state;` (※後者は §3 で確定) 追加 |
| `crates/dsp-core/src/params.rs` (生成) | `INHARMONICITY_B_CURVE_PIANO: [f32; 88]` 追加、Piano エントリに `unison_detune_cents` / `sympathetic_amount` を追加 |
| `params.json` | Piano エントリに `unison_detune_cents` / `sympathetic_amount` / `inharmonicity_b_curve` (88 値) フィールド追加 |
| `scripts/gen-params.mjs` | 上記新フィールドを Rust const + TS 定数で出力 |
| `web/src/lib/state/factory-presets.ts` | Piano エントリの聴感調整値を Step 19 で更新 |

### 不変

| ファイル | 理由 |
|---|---|
| `crates/wasm-audio/src/lib.rs` | C ABI 19 required exports 維持（D81） |
| `web/src/lib/audio/synth-processor.ts` | 既存 INSTRUMENT_KIND_MAP に変更なし |
| `web/src/lib/audio/__synthDev.ts` | Phase 4b 完成済、F38b 計測 API は不変 |
| その他 `.svelte` コンポーネント | UI 露出なし（D81） |

## メモリレイアウト（Phase 4c での増分）

Phase 4b の `Engine::prepare` 一括確保戦略を継承。Phase 4c 追加分:

```
KarplusStrong (8 voice)
├── 既存 fields (Phase 4b)
├── string_states: [StringState; 3]                    ← +96 byte/voice (24 byte × 3)
├── string_buffers: [Vec<f32>; 3]                      ← +3 × 1746 sample × 4 byte = +20952 byte/voice
└── n_strings_active: usize                            ← +8 byte/voice

合計 voice 単体 +21056 byte ≈ +20.6 KB/voice
8 voice 合計 +168.5 KB

Engine
├── 既存 fields (Phase 4b)
└── resonance_bus: ResonanceBus                        ← +416 byte
      ├── buffer: Vec<f32>(96 sample × 4 byte = 384)
      ├── lpf: BrightnessLpf (16 byte)
      └── feedback_gain: SmoothedValue (8 byte)

Phase 4c 純増合計 ≈ +169 KB heap (Engine::prepare で一括確保)
```

これは Phase 4b の +0.78 KB heap から大幅増だが、`WebAssembly.Memory` の linear memory で吸収可能（initial size 256 KB 想定 → Phase 4c で `memory.grow()` 自動拡張）。`process` ホットパスでの growth は発生せず（既存 alloc ゼロ条件継承）。

`memory.buffer.byteLength` 不変条件:
- `prepare` 時に 1 回 growth が発火する可能性 → worklet `refreshViews()` が prepare 時に 1 回発火（Phase 4b までも同じ挙動）
- `process` ホットパスでの growth は発生しない（Phase 1 D4 維持）

## 起動シーケンス（Phase 4c で変化なし）

Phase 4b の起動シーケンス (`StartButton.onclick` → `AudioContext.resume()` → `audioWorklet.addModule()` → `AudioWorkletNode` 作成 → MessagePort 確立 → SynthEngine 初期化) を完全継承。Phase 4c 追加要素は **`Engine::prepare` 内で 1 回呼ばれる初期化**のみで、起動シーケンスの章番号 / 段数に変化なし。

## エラー伝播（Phase 4c で変化なし）

Phase 1〜4b と同じ。WASM 内部の Rust `panic` は `set_panic_hook` 経由で console.error、Worklet 側の MessagePort エラーはメインスレッドで catch。Phase 4c で Sympathetic bus の数値発散リスクはあるが、`feedback_gain` を `[0.0, 0.05]` に clamp + bus LPF で抑制（pre-research §5.4）。

---

## まとめ

Phase 4c は 4 レイヤ構成を保ちつつ、**dsp-core 層で Multi-string per voice / Hertz law raised cosine hammer / Global sympathetic resonance bus / B(note) LUT の追加**、**UI 層で Piano プリセット値の更新（聴感チューニング）**、**wasm-audio 層は完全不変** を行う。データフローは Phase 4b の voice → modal_body → soft_clip に **bus injection 経路** を追加し、`note_on` で **Multi-string detune + Hertz hammer + B(note) lookup** を実行する。メモリは +169 KB heap（`Engine::prepare` 一括確保、`process` 中 alloc ゼロ維持）、C ABI は 19 exports を完全維持（D81）。Phase 4a / 4b 互換性は `n_strings = 1` 経路で機械保証継承（D83）。
