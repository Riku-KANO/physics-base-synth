# 02. Phase 3 アーキテクチャ差分

## 目的

Phase 1 [02 章](../2026-05-06-001-mvp/02-architecture.md) と Phase 2 [02 章](../2026-05-07-002-phase2/02-architecture.md) を起点に、Phase 3 で発生する **構成差分**（Modal Body Resonator の Engine 内配置、Extended KS 拡張モジュール、MIDI CC dispatch 経路、Voice State 共有メモリ通信、Voice Meter / mono–poly トグル UI、Soft clip 配置、Thiran allpass 並列追加）を確定する。Phase 1 / 2 で確定した 4 レイヤ構成・モノレポレイアウト・ParamDescriptor codegen パイプライン・既存スクリプトはすべて維持する。

## 他文書との関係

- 上流: [`01-overview.md`](./01-overview.md)（Phase 3 スコープと決定事項 D30〜D43）、[`pre-research.md`](./pre-research.md)（§2 Modal Body / §6 MIDI CC アーキ / §7 UI / §9 性能予算）
- 並列: [`03-dsp-core-spec.md`](./03-dsp-core-spec.md)、[`04-wasm-audio-spec.md`](./04-wasm-audio-spec.md)、[`05-web-frontend-spec.md`](./05-web-frontend-spec.md)
- 下流: [`06-build-and-verify.md`](./06-build-and-verify.md)、[`07-implementation-checklist.md`](./07-implementation-checklist.md)
- Phase 1 / 2 参照: 本書で **明示的に変更しない部分はすべて Phase 1 / 2 の記述を継承**

## 信号フローと責務分担

### Phase 1 / 2 構成は維持

[Phase 1 02 章 §信号フローと責務分担](../2026-05-06-001-mvp/02-architecture.md#信号フローと責務分担) と [Phase 2 02 章 §信号フローと責務分担](../2026-05-07-002-phase2/02-architecture.md#信号フローと責務分担) の 4 レイヤ構成（Svelte UI / SynthEngine / SynthProcessor (Worklet) / wasm-audio / dsp-core）は **完全維持**。各レイヤの責務分離原則も継続。

### Phase 3 で発生するレイヤ内変更

| レイヤ | Phase 2 | Phase 3 差分 |
|---|---|---|
| Svelte UI | ParamSlider が ParamDescriptor 駆動、ミニマル UI（mode / activeVoices 非表示）| **VoiceMeter コンポーネント追加**（D41）、**PolyphonyToggle コンポーネント追加**（D42）、**ParamSlider に Pick Position / Body Wet を追加**、**WebMIDI handler に Pitch Bend / CC dispatch 追加**（D38） |
| SynthEngine（main） | AudioContext 起動、MessagePort 仲介、rAF パラメータスロットル | **MIDI CC / Pitch Bend 用メソッド追加**（`sendMidiCc(cc, value)` / `sendPitchBend(semitones)`、D38）、**Voice State 受信ハンドラ追加**（Worklet → main message を `$state` に反映、D41） |
| SynthProcessor（Worklet） | WASM ロード、`process` 委譲、Float32Array view キャッシュ、`synth_set_polyphony_mode` dispatch | **WasmExports interface に 3 関数追加**（`synth_midi_cc` / `synth_pitch_bend` / `synth_voice_state_ptr`）、**Voice State stride カウンタ追加**（1024 サンプル毎に main へ push、D41）、**ToWorkletMessage union 拡張**（midiCC / pitchBend variant、D38） |
| wasm-audio | C ABI 12 関数、SynthHandle 保持 | **3 関数追加**（`synth_midi_cc` / `synth_pitch_bend` / `synth_voice_state_ptr`、D38 / D41）。SynthHandle 内部で Engine が拡張されるが既存 12 関数の外部 API は不変 |
| dsp-core | Engine / VoicePool / KarplusStrong / SmoothedValue / XorShift32 / Voice / FractionalDelay / NoteAllocator / HoldStack / SynthMode / ParamDescriptor | **新規モジュール 4 件**: `modal_body.rs` / `loss_filter.rs` / `sustain_state.rs` / `soft_clip.rs`（Pick position は専用モジュールなし、`KarplusStrong::note_on` 内の励振 shaping、D34）。**既存モジュール拡張**: `fractional_delay.rs` に `ThiranCoeffs` 追加（D36）、`voice.rs` に `Voice` trait method 1 件追加（`set_pitch_bend`、D39、Mod Wheel `set_mod_depth` は Phase 4 送り）、`engine.rs` に MIDI CC dispatch (CC#7/#64/#123) / Voice State export / Sustain 連携 / Modal Body / Soft clip 統合、`karplus_strong.rs` に Loss filter / Brightness 補正 / Pitch Bend / Pick position 励振 shaping 統合 |

## モノレポレイアウト

[Phase 1 02 章 §モノレポレイアウト](../2026-05-06-001-mvp/02-architecture.md#モノレポレイアウト) と [Phase 2 02 章 §モノレポレイアウト](../2026-05-07-002-phase2/02-architecture.md#モノレポレイアウト) を継承。Phase 3 で追加・変更されるファイルのみ列挙する。

### Phase 3 で追加されるファイル

```
C:\Users\81903\projects\physics-base-synth\
│
├── crates\
│   ├── dsp-core\
│   │   └── src\
│   │       ├── modal_body.rs                       # 新規: ModalBodyResonator (M=8 並列 bandpass biquad、stereo)
│   │       ├── loss_filter.rs                      # 新規: One-zero loss filter (D33)
│   │       ├── sustain_state.rs                    # 新規: Sustain Pedal 状態管理 (D40)
│   │       └── soft_clip.rs                        # 新規: 区間関数型 soft clip (D43)
│   │   # 注: pick_position は専用モジュールではなく KarplusStrong::note_on 内の励振 shaping (D34)
│   └── （wasm-audio は src/lib.rs に 3 関数追加のみ）
│
├── web\src\lib\
│   ├── components\
│   │   ├── VoiceMeter.svelte                       # 新規: 8 セル active + 振幅 (D41)
│   │   └── PolyphonyToggle.svelte                  # 新規: mono/poly ラジオ (D42)
│   ├── audio\
│   │   └── voice-state.svelte.ts                   # 新規: $state で Voice State を保持
│   └── input\
│       └── midi-cc.ts                              # 新規: WebMIDI CC parser (D38)
│
└── docs\specs\2026-05-07-003-phase3\
    └── （本仕様書群、pre-research + 01〜07 の 8 ファイル）
```

### Phase 3 で変更されるファイル

| ファイル | 変更内容 |
|---|---|
| `params.json` | Body Mode 8 個 × 3 係数 = 24 値 + `pick_position` + `body_wet` を追加（合計 +26 パラメータ） |
| `crates/dsp-core/src/lib.rs` | 新規モジュール 4 件を `mod` 宣言 + `pub use` 追加 |
| `crates/dsp-core/src/engine.rs` | Modal Body 統合、Soft clip 配置、Sustain 状態保持、Voice State buffer、MIDI CC dispatch (CC#7/#64/#123)、Pitch Bend fan-out、`channel_volume` フィールド追加（D38b）、**`Engine::new_with_thiran()` test-only constructor 追加**（Step 1 試作で各 voice に Thiran 注入、D36） |
| `crates/dsp-core/src/karplus_strong.rs` | Loss filter / Brightness 補正 / Pitch Bend / Pick position 励振 shaping 統合、**`fractional_delay: FractionalDelay` field 追加**（旧 `lagrange: LagrangeCoeffs` を置換）、`note_on_internal(note_id, freq, vel)` 共通ヘルパへの集約、`new_with_fractional_delay(fd)` test-only constructor 追加 |
| `crates/dsp-core/src/fractional_delay.rs` | **`ThiranCoeffs` 構造体を追加**（並列、D36 試作用、`d ∈ [1e-4, 0.999]` clamp）+ **`LagrangeCoeffs::set_fractional(&mut self, d: f32)` を追加**（中身は `*self = Self::new(d)`、enum 経由再計算用）+ **`FractionalDelay` enum で Lagrange/Thiran を統合**（`set_fractional` / `apply` / `reset` / `new_lagrange` / `new_thiran`） |
| `crates/dsp-core/src/smoothing.rs` | **変更なし**（Pitch Bend SmoothedValue の note_on 時初期化は既存 `set_immediate(value)` を流用、新メソッド追加せず） |
| `crates/dsp-core/src/voice.rs` | `Voice` trait に `set_pitch_bend` 追加、`KarplusStrong` の inherent `set_pick_position` 委譲を追記（D39） |
| `crates/dsp-core/src/voice_pool.rs` | Pitch Bend / Pick position fan-out、`voice_state(&self)` 公開 API 化（D41）、**`new_with_fractional_delay_factory<F>(factory)` test-only constructor 追加**（Step 1 で各 voice に FractionalDelay 注入） |
| `crates/wasm-audio/src/lib.rs` | `synth_midi_cc` / `synth_pitch_bend` / `synth_voice_state_ptr` の 3 関数追加（D38 / D41） |
| `web/src/lib/audio/messages.ts` | `ToWorkletMessage` に `midiCC` / `pitchBend` variant 追加、`FromWorkletMessage` に `voiceState` variant 追加 |
| `web/src/lib/audio/synth-processor.ts` | WasmExports 拡張、Voice State stride push（事前確保スクラッチ Float32Array、process 中 alloc ゼロ維持）、message dispatch 拡張 |
| `web/src/lib/audio/engine.ts` | `sendMidiCc` / `sendPitchBend` メソッド追加、Voice State 受信ハンドラ |
| `web/src/routes/+page.svelte` | VoiceMeter / PolyphonyToggle を Header 直下に配置 |
| `web/src/lib/components/ParamSlider.svelte` | Pick Position / Body Wet 用に表示拡張（既存ロジックを流用） |
| `scripts/gen-params.mjs` | 純粋関数 `generateRustSource` / `generateTsSource` を `body_modes` / `stereo_spread` 対応に拡張、`applyStereoSpread(modes, spread)` 純粋関数を追加（左 ch から右 ch を ±spread% で生成） |
| `scripts/check-wasm-exports.mjs` | REQUIRED 配列に 3 関数追加（`synth_midi_cc` / `synth_pitch_bend` / `synth_voice_state_ptr`） |

### Phase 1 / 2 との変更を伴わないファイル

- `Cargo.toml`（ワークスペース定義）、`rust-toolchain.toml`、`pnpm-workspace.yaml`
- `web/svelte.config.js`、`web/vite.config.ts`、`web/tsconfig.json`
- `crates/dsp-core/Cargo.toml`（依存ゼロを維持、Phase 3 でも `microfft` 等を追加しない）
- `crates/wasm-audio/Cargo.toml`（dsp-core path 依存のみ）
- `scripts/check-params-sync.mjs` / `scripts/copy-wasm.mjs`（Phase 2 から動作不変）
- 注: `scripts/gen-params.mjs` は **拡張対象**（上記「Phase 3 で変更されるファイル」一覧）。`body_modes` / `stereo_spread` セクションを処理する純粋関数 `applyStereoSpread` を追加

## ワークスペース設定

### `Cargo.toml`（ワークスペースルート）

[Phase 1 02 章 §Cargo.toml](../2026-05-06-001-mvp/02-architecture.md#cargotomlワークスペースルート) を **完全維持**。`[profile.release]` の `lto = "fat"` / `codegen-units = 1` / `panic = "abort"` も継続。

### `crates/dsp-core/Cargo.toml`

依存ゼロを Phase 3 でも維持（D23 継承）。新規モジュール 4 件（`modal_body.rs` / `loss_filter.rs` / `sustain_state.rs` / `soft_clip.rs`、Pick position は専用モジュールなし）は内部実装のみで、外部 crate を一切追加しない。Modal Body の bandpass biquad / Soft clip の区間関数（`f32::tanh` 不使用、Padé 近似不要） / 全モジュールは標準ライブラリ（`core::f32`）のみで実装する。

### `crates/wasm-audio/Cargo.toml`

dsp-core path 依存のみ。3 関数追加でも依存追加なし。

### ルート `package.json`

Phase 2 のスクリプト構成を **完全維持**。`gen:params` / `check:params-sync` / `build:wasm` / `build:wasm:dev` / `dev` / `build` / `preview` / `check` / `fmt` / `lint` のいずれも変更なし。

ただし `gen-params.mjs` 自体は **Step 2 で拡張改修される**（上記「Phase 3 で変更されるファイル」一覧）。`body_modes` / `stereo_spread` セクションを `params.json` から読み、Rust の `BODY_MODES_L/R` / `STEREO_SPREAD` const と TS の `BODY_MODES_L/R` / `STEREO_SPREAD` export を出力する処理を追加する。`applyStereoSpread(modes, spread)` 純粋関数を新設し左 ch → 右 ch の係数生成を行う。

### `.gitignore`

Phase 2 と同じ。生成物 `params.rs` / `generated/params.ts` を git commit する方針（D25 継承）。

## ParamDescriptor コード生成パイプライン（Phase 3 拡張）

### `params.json` の拡張

Phase 2 の 3 パラメータ（Damping / Brightness / OutputGain）に加え、Phase 3 では以下を追加。詳細スキーマと値は [`03-dsp-core-spec.md` §params.json 拡張](./03-dsp-core-spec.md#paramsjson-phase-3-拡張)。

| id | name | min | max | default | smoothing_tau | 用途 |
|---|---|---|---|---|---|---|
| 0 | Damping | 0.90 | 0.9999 | 0.996 | 0.02 | （既存）|
| 1 | Brightness | 0.0 | 1.0 | 0.5 | 0.02 | （既存）|
| 2 | OutputGain | 0.0 | 1.5 | 0.8 | 0.01 | （既存）|
| 3 | PickPosition | 0.05 | 0.5 | 0.125 | 0.05 | Pick position β（D34） |
| 4 | BodyWet | 0.0 | 1.0 | 0.5 | 0.02 | Modal Body の dry/wet ミックス |

> Body Mode 24 係数（M=8 × 3）は **`params.json` の `params` 配列に入れず、別セクション `body_modes`** として持つ。これは「ユーザーがリアルタイムに変えるパラメータ」と「ビルド時定数」を分離するため。`gen-params.mjs` は `body_modes` を読んで Rust の `pub const BODY_MODES_L: [BodyMode; 8]` / `BODY_MODES_R: [BodyMode; 8]` を生成（D32）。

```json
{
  "params": [ ... 5 entries ... ],
  "body_modes": [
    { "freq": 105.0,  "q": 30.0, "gain": 1.0  },
    { "freq": 200.0,  "q": 25.0, "gain": 0.8  },
    { "freq": 280.0,  "q": 20.0, "gain": 0.5  },
    { "freq": 420.0,  "q": 35.0, "gain": 0.4  },
    { "freq": 580.0,  "q": 40.0, "gain": 0.35 },
    { "freq": 850.0,  "q": 45.0, "gain": 0.25 },
    { "freq": 1400.0, "q": 50.0, "gain": 0.2  },
    { "freq": 2300.0, "q": 60.0, "gain": 0.15 }
  ],
  "stereo_spread": 0.05
}
```

### 生成ターゲット 1: `crates/dsp-core/src/params.rs`

Phase 2 の出力に加え、`gen-params.mjs` が以下を追加生成:

- `pub struct BodyMode { pub freq: f32, pub q: f32, pub gain: f32 }`
- `pub const BODY_MODES_L: [BodyMode; 8] = [...]`（左 ch 係数、初期値そのまま）
- `pub const BODY_MODES_R: [BodyMode; 8] = [...]`（右 ch 係数、`freq` / `q` / `gain` 各 ±5% 揺らし）
- `pub const STEREO_SPREAD: f32 = 0.05`（gen-params.mjs から取得）

冒頭の `// AUTO-GENERATED FROM params.json — DO NOT EDIT` コメントは継続。

### 生成ターゲット 2: `web/src/lib/audio/generated/params.ts`

Phase 2 の出力に加え、TS 側でも Body Mode 構造を出力（UI が直接参照しないが、デバッグ / プリセット展開準備のため）:

- `export interface BodyMode { freq: number; q: number; gain: number }`
- `export const BODY_MODES_L: readonly BodyMode[] = [...]`
- `export const BODY_MODES_R: readonly BodyMode[] = [...]`

冒頭の AUTO-GENERATED コメントは継続。

### 既存 `web/src/lib/audio/messages.ts` との関係

Phase 2 と同じく `generated/params.ts` から re-export する形を維持。Phase 3 で `ToWorkletMessage` / `FromWorkletMessage` の手書き union を拡張（[`05-web-frontend-spec.md` §messages.ts 変更](./05-web-frontend-spec.md#messagets-phase-3-変更点)）。

### `scripts/gen-params.mjs` の責務（Phase 3 拡張）

純粋関数 / CLI entrypoint の分離は Phase 2 と同じ。Phase 3 で純粋関数を 1 件拡張:

| 関数 | シグネチャ | 責務 |
|---|---|---|
| `generateRustSource(paramsJson)` | `(object) => string` | params + body_modes + stereo_spread をすべて Rust ソース文字列化 |
| `generateTsSource(paramsJson)` | `(object) => string` | 同 TS |
| `applyStereoSpread(modes, spread)` | `(BodyMode[], number) => BodyMode[]` | 左 ch から右 ch を生成（freq / q / gain 各 ±spread%）。**純粋関数、副作用なし** |

CLI entrypoint は Phase 2 と同じく `import.meta.url === pathToFileURL(process.argv[1]).href` ガードで実行（D15、Phase 2 D15 継承の Windows 対応パターン）。

### `scripts/check-params-sync.mjs` の責務

Phase 2 と同じ。Body Mode 追加部分も `generateRustSource` / `generateTsSource` の出力に含まれるため、文字列一致判定だけで drift 検知が成立する。

## メモリレイアウトの変更

### Phase 2 のメモリレイアウト（再掲）

`Engine::prepare` で `VoicePool::voices[i].buffer` × 8 + `SynthHandle::scratch_l/r` を一括確保。合計約 57 KB。`process` 中ヒープ確保ゼロ（D4）。

### Phase 3 のメモリレイアウト

`Engine::prepare` で **Phase 2 の 57 KB に加え、以下を一括確保**:

| 領域 | サイズ計算 | 値（48 kHz、N=8） |
|---|---|---|
| `ModalBody::states_l[i]` × 2 状態 × 8 モード | 2 × 8 × 4 bytes | 64 B |
| `ModalBody::states_r[i]` × 2 状態 × 8 モード | 同上 | 64 B |
| `ModalBody::coeffs_l[i]` × 3 係数 × 8 モード | 3 × 8 × 4 bytes | 96 B |
| `ModalBody::coeffs_r[i]` × 3 係数 × 8 モード | 同上 | 96 B |
| `SustainState::pending_release` (u128 bitmap = 16 bytes) | 16 B | 16 B |
| `SustainState::sustain_active` | 1 B | 1 B |
| `Engine::voice_state_buffer` (active mask 1 + 8 振幅 × 4) | 33 B | 33 B |
| `Engine::pitch_bend_smoothed` (per voice × 8) × SmoothedValue (3 f32) | 8 × 12 = 96 B | 96 B |
| ~~`Engine::mod_depth` (1 f32)~~ | 0 B | 0 B（Mod Wheel は Phase 4 送り） |
| `Engine::body_wet_smoothed` (SmoothedValue) | 12 B | 12 B |
| `Engine::pick_position` (f32 のみ、SmoothedValue 不要、D34 励振 shaping) | 4 B | 4 B |
| `Engine::channel_volume` (SmoothedValue、CC#7 用、D38b) | 12 B | 12 B |
| その他 const-size 構造体 | < 1 KB | 〜 1 KB |
| **dsp-core / wasm-audio が新規確保するメモリ（Phase 3 追加分）** | | **約 1.5 KB** |

> **重要**: Phase 2 の `synth_new` 完了後 byteLength 不変条件は **Phase 3 でも維持**。Modal Body 状態 / Voice State buffer / Sustain pending bitmap / Pitch Bend SmoothedValue 配列は **すべて `synth_new` 内で一括確保**、`process_block` / `note_on` / `note_off` / `set_param` / `set_polyphony_mode` / `synth_midi_cc` / `synth_pitch_bend` / `synth_voice_state_ptr` のいずれを呼んでも byteLength が一切変化しないこと。F17 の検証手順は Phase 2 から継承（baseline 記録 → 操作 → 一致確認）。

### `synth_new` 内部での確保フロー（Phase 3 版）

```
synth_new(sample_rate=48000, max_block_size=128)
  └─ Box::new(SynthHandle { engine, scratch_l: vec![0; 128], scratch_r: vec![0; 128] })
       └─ Engine::new()
       └─ engine.prepare(sample_rate=48000, max_block_size=128)
            ├─ pool.prepare(sample_rate, max_block_size)
            │    ├─ voices[0..7].prepare(sr, mb)  → buffer = vec![0; 1749] each
            │    │  + LossFilter::new() (state = 0)
            │    │  + 各 voice の length_target / pitch_bend SmoothedValue 初期化
            │    │  // pick_position は f32 フィールドのみ、追加確保なし（励振 shaping、D34）
            ├─ output_gain.set_time_constant(sample_rate, OUTPUT_GAIN_TAU)
            ├─ hold_stack.clear()
            ├─ modal_body.prepare(sample_rate)
            │    ├─ coeffs_l[0..7].calculate(BODY_MODES_L[i], sr)  // bandpass biquad
            │    └─ coeffs_r[0..7].calculate(BODY_MODES_R[i], sr)
            ├─ sustain_state.reset()
            ├─ body_wet_smoothed.set_time_constant(sample_rate, 0.02)
            ├─ channel_volume.set_time_constant(sample_rate, 0.02)  // CC#7 用、デフォルト 1.0
            └─ voice_state_buffer = [0; 33]
            // pick_position は Engine.pick_position: f32 フィールドのみ（D34、SmoothedValue 不要）
```

`synth_new` 完了後、`process_block` 中の追加確保はゼロ。F17（`test_no_allocation_in_polyphonic_process`）と新規 F37（`test_no_allocation_with_modal_body_and_midi_cc`）で保証する（本仕様 06 章）。

## ビルドツールチェーン

[Phase 1 02 章 §ビルドツールチェーン](../2026-05-06-001-mvp/02-architecture.md#ビルドツールチェーン) を **完全継承**。

### Phase 3 で wasm-opt の扱い

Phase 2 では「VoicePool 追加で必須化候補」と書かれたが結果として未必須化（gzip 10.56 KB で予算大幅余裕）。Phase 3 でも **任意のまま**で進める。Phase 3 想定 12.9 KB / target 30 KB の 43% 余裕があるため、`wasm-opt -O3` は将来検討（CI に組み込む場合は Phase 4 候補）。

### 開発スクリプトの呼び出し関係（Phase 3 版）

Phase 2 から **不変**。`gen:params` が `params.json`（拡張済み）を読み、Body 係数を含む生成物を出すだけ。

```
pnpm dev
  └─ pnpm build:wasm:dev
  │    ├─ pnpm gen:params
  │    │    └─ node scripts/gen-params.mjs
  │    │         （params.json から Rust + TS を生成、body_modes も含む）
  │    ├─ cargo build -p wasm-audio --target wasm32-unknown-unknown
  │    │    （内部で生成済み params.rs を使用、BODY_MODES_L/R も含む）
  │    ├─ node scripts/copy-wasm.mjs debug
  │    └─ node scripts/check-wasm-exports.mjs
  │         （synth_midi_cc / synth_pitch_bend / synth_voice_state_ptr を含めて検証）
  └─ pnpm --filter web dev
       ├─ pnpm --filter web build:worklet:dev
       │    └─ esbuild ... → web/static/worklet/synth-processor.js
       │         （内部で generated/params.ts を import、midiCC dispatch を含む）
       └─ vite dev
```

## ビルド成果物の流れ

[Phase 2 02 章 §ビルド成果物の流れ](../2026-05-07-002-phase2/02-architecture.md#ビルド成果物の流れ) を **完全継承**。Phase 3 で生成物の構造は同じ、内容のみ拡張。

| 段階 | Phase 3 差分 |
|---|---|
| ParamDescriptor 生成 | `params.json` に `body_modes` / `stereo_spread` セクション追加、生成 Rust / TS に `BODY_MODES_L/R` / `STEREO_SPREAD` も含む |
| Rust ビルド | dsp-core が新規 4 モジュール（`modal_body` / `loss_filter` / `sustain_state` / `soft_clip`、Pick position は専用モジュールなし）を含む、wasm-audio が 3 関数追加 |
| WASM コピー | 維持 |
| Worklet ビルド | `synth-processor.ts` が 3 関数 dispatch を含む |
| Vite ビルド | VoiceMeter / PolyphonyToggle / midi-cc.ts も含む |

## アンチパターン回避（Phase 3 版）

Phase 1 / 2 のアンチパターン回避表に Phase 3 固有項目を追加。

| アンチパターン | Phase 1/2 防止箇所 | Phase 3 追加防止箇所 |
|---|---|---|
| `process` 中のヒープ確保 | Engine::prepare で全バッファ事前確保 | Modal Body 状態 / Voice State buffer / Sustain pending / Pitch Bend SmoothedValue を `Engine::prepare` で一括確保 |
| Mutex によるロック | Worklet 単独完結 | Modal Body も const-size、Voice State buffer もロックフリー（Worklet が単独書き、main は read-only） |
| WASM memory.grow | `synth_new` のみ alloc | Phase 3 追加バッファも `synth_new` で完結 |
| 細かい JS↔WASM 往復 | 1 ブロック単位 | MIDI CC / Pitch Bend は CC 受信時のみ dispatch、Voice State は 1024 サンプル毎（21 ms）に押し出し、毎サンプル往復しない |
| AudioWorklet 内 `console.log` 連発 | 開発時のみ条件分岐 | 維持 |
| AudioParam 多用 | MessagePort 経由 | MIDI CC / Pitch Bend も MessagePort 経由（D38） |
| ParamId / PARAM_IDS 二重管理 | params.json + codegen | 維持。Body Mode 係数も同じパイプラインで生成 |
| Lagrange 係数の毎サンプル再計算 | note_on のみ | **Pitch Bend で length 変動時の再計算戦略**: Phase 3 では `length_target` を SmoothedValue 化、process 中に整数部 / 小数部の再分解は **5ms tau の遷移中のみ**発生。Step 1 の Thiran 試作評価でこの再計算コストを実測（D26 拡張、03 章で詳細） |
| **`synth_voice_state_ptr` 経由の race condition** | （Phase 3 新規） | Worklet が voice_state_buffer に書き終えてから main が読む保証。Phase 3 では「Worklet が write して直後に postMessage、main が message 受信時に読む」シーケンス（共有メモリだが atomic 不要、tear はスレッド境界の自然な ordering で許容） |
| **Modal Body の biquad denormal** | （Phase 3 新規） | 各 biquad の出力に `+1e-25 -1e-25` トリック適用（D6 拡張）、または ARM/x86 共通の zero-flush 不要化 |
| **MIDI CC で連続値が来たときの flooding** | （Phase 3 新規） | rAF スロットルは Pitch Bend に適用（CC#7 / #64 / #123 はスロットルしない、頻度低）、SynthEngine 側で前値と一致なら送信しない |

## 拡張性の確保

Phase 1 / 2 の拡張性確保を継承。Phase 3 で新たに考慮する点:

- **Modal Body Resonator は `prepare` で係数初期化、`process` で状態更新のみ**: Phase 4 でプリセット切替を入れる場合、`Engine::set_body_preset(preset_id: u32)` で係数テーブルを差し替える経路を追加可能（既存 `params.rs` 生成出力を複数プリセット対応に拡張）
- **`Voice` trait に追加した `set_pitch_bend`** は Phase 4 で他楽器（ピアノ / ウクレレ）にも有効。Pitch Bend は MIDI 標準なので楽器独立。Phase 4 で `set_mod_depth` を追加して LFO 仕様と併せて確定する
- **MIDI CC dispatch を 1 関数 (`synth_midi_cc`) に集約した利点**: Phase 4 で CC#11 (Expression) / CC#10 (Pan) / CC#71 (Resonance) 等を追加する場合、C ABI 関数追加なし、内部 switch 拡張のみで対応可能
- **Voice State 共有メモリ方式の利点**: 33 bytes 固定で stride push なので、Phase 4 で voice 数を増やしても `synth_voice_state_ptr` のサイズ exposure を変えるだけで対応
- **Soft clip 配置を `output_gain` 後に固定**: Phase 4 で Look-ahead limiter を追加する場合、Soft clip と直列配置（Soft clip → Look-ahead）かどちらか片方かを選べる
- **Thiran allpass 並列実装**: Step 1 で `LagrangeCoeffs` と `ThiranCoeffs` を併設するため、Phase 4 で楽器別に補間方式を選ぶ拡張余地あり（KS = Thiran、ピアノ = Lagrange など）
- **dsp-core は Phase 3 でも依存ゼロ維持**（D23 継承）。VST/CLAP / CLI / ネイティブアプリへの転用余地を Phase 1 と同じ条件で残す
