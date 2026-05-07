# physics-base-synth

物理ベース・シンセサイザー（Karplus–Strong）の Phase 3 (Body Resonator + Extended KS + MIDI CC + Voice Meter UI + Soft clip + Thiran allpass) 対応版。Rust + WebAssembly + Svelte 5 (SvelteKit) で実装。

## 動作環境

- **推奨ブラウザ**: Chrome / Edge 最新版（Chromium 系）
  - Web MIDI と AudioWorklet は **secure context (HTTPS / localhost) 必須**
  - Firefox 126+ で Web MIDI 対応
  - iOS Safari は HTTPS 配信下でのみ動作（StartButton のユーザージェスチャ必須）
- Rust stable 1.83+ (target: `wasm32-unknown-unknown`)
- Node.js 24+ (GitHub Pages Workflow も Node 24 で実行)
- pnpm 9+ (corepack 経由)

## セットアップ

```powershell
rustup target add wasm32-unknown-unknown
corepack enable
corepack prepare pnpm@latest --activate
pnpm install
```

## 開発

```powershell
pnpm dev
```

`http://localhost:5173/` を開いて「▶ Start Audio」をクリック → A〜L キーで発音。

## 主なスクリプト

| コマンド | 内容 |
|---|---|
| `pnpm gen:params` | `params.json` から Rust / TS の `params` モジュールを生成 |
| `pnpm check:params-sync` | 生成物 drift を CI で検証 (drift で exit 1) |
| `pnpm build:wasm:dev` | `gen:params` → dev WASM ビルド → コピー → export 検証 |
| `pnpm build:wasm` | release WASM ビルド (同上の release 版) |
| `pnpm dev` | WASM(dev) ビルド後、Vite dev server 起動 (5173) |
| `pnpm build` | 本番ビルド（静的サイト → `web/build/`） |
| `pnpm preview` | 本番プレビュー (http://localhost:4173) |
| `pnpm check` | `cargo check --workspace` + `svelte-check` + `check:params-sync` |
| `pnpm lint` | `cargo clippy --workspace --all-targets -- -D warnings` |
| `pnpm fmt` | `cargo fmt` + prettier |

## アーキテクチャ概要 (Phase 3)

```
Svelte UI (main thread) ── MessagePort ─→ AudioWorkletProcessor
   VoiceMeter / PolyphonyToggle             │ FFI (C ABI、wasm-bindgen 不使用)
   PickPosition / BodyWet スライダー         ▼
   WebMIDI CC handler (CC#7/#64/#123)   wasm-audio (cdylib)
   Pitch Bend                                │ + synth_midi_cc / synth_pitch_bend
                                             │ + synth_voice_state_ptr
                                             ▼
                                        dsp-core (rlib)
                                        Engine / VoicePool<8> / KarplusStrong (Thiran allpass)
                                        ModalBodyResonator (M=8 並列 bandpass、stereo)
                                        LossFilter (one-zero) / SoftClip (区間関数型)
                                        SustainState / Pitch Bend / Voice State buffer
                                        FractionalDelay (Thiran) / NoteAllocator / HoldStack
                                        SmoothedValue / XorShift32 / ParamDescriptor (生成)
```

詳細は仕様書 (`docs/specs/`) を参照:
- Phase 1 (MVP): `docs/specs/2026-05-06-001-mvp/`
- Phase 2 (polyphony / fractional delay / ParamDescriptor): `docs/specs/2026-05-07-002-phase2/`
- Phase 3 (Body Resonator / Extended KS / MIDI CC / Voice Meter): `docs/specs/2026-05-07-003-phase3/`

## Phase 3 で追加された機能

- **Modal Body Resonator (D30/D31/D32)**: M=8 並列 bandpass biquad で楽器ボディ共鳴。stereo は左右係数 ±5%。
- **Extended Karplus–Strong**:
  - One-zero loss filter (D33): `(1+ρ·z⁻¹)/(1+ρ)`、`note_on` 時に周波数依存式で算出
  - Pick position 励振 shaping (D34): `note_on` 内で `noise[n] − noise[n−K]` を in-place 適用
- **Thiran allpass (D36 案 D 採用)**: `KarplusStrong` の補間を `LagrangeCoeffs` から `ThiranCoeffs` 単一型に解消。A4 で 0.0002% 級のピッチ精度（Lagrange 0.89% → Thiran 0.0002% = ~4000× 改善）。C8 は damping=0.9999 では loop gain<1 で物理的に自己発振せず、ignore 継続。
- **Brightness 群遅延補正 (D37)**: Thiran 採用後も brightness LPF (1-b)/b 群遅延は残るため `note_on` 時に `adjusted_length = raw_len - τ_g` で補正。
- **Soft clip (D43)**: 区間関数型 `|x| ≤ 0.95` で完全 linear、`|x| > 0.95` で rational mapping、`|x| → ∞` で ±1.0 に厳密漸近。
- **MIDI CC (D38/D38b/D40)**:
  - CC#7 Channel Volume: OutputGain と直交 (final = output_gain × channel_volume)
  - CC#64 Sustain Pedal: Poly mode のみ defer、Mono は Phase 2 既存挙動継承
  - CC#123 All Notes Off: 全 voice + hold_stack + sustain_state を reset
  - Pitch Bend (±2 半音): SmoothedValue 5ms tau で滑らか遷移
- **Voice State (D41)**: 33 byte 共有メモリ (active mask + 8 振幅 LE f32) を 1024 sample stride で UI に push、VoiceMeter で表示。
- **mono / poly トグル (D42)**: ヘッダー直下の正式 UI（dev-only `__synthDev.setMode` も維持）。

## 自己検証手順

### Phase 1 (F1〜F9): 単音動作

[Phase 1 README](docs/specs/2026-05-06-001-mvp/06-build-and-verify.md#検証項目f1f9) を参照。実機検証は持ち越し継続。

### Phase 2 (F10〜F25): polyphony / pitch / size

[Phase 2 06 章](docs/specs/2026-05-07-002-phase2/06-build-and-verify.md) を参照。実機検証は持ち越し継続。

### Phase 3 (F26〜F38b): Body / Extended KS / MIDI CC / Voice Meter / サイズ

| ID | 手順 | 期待結果 |
|---|---|---|
| **F26** | `cargo test -p dsp-core test_modal_body_` / `test_single_biquad_` | 単体: DC<0.001、ピーク `mode.gain` ± 5%、aggregate: ピーク 0.5〜1.5 倍 |
| **F27** | `cargo test -p dsp-core test_loss_filter_` | DC ゲイン 1.0、Nyquist 減衰 (1-ρ)/(1+ρ)、A2<A4 で ρ 増 |
| **F28** | `cargo test -p dsp-core test_pick_position_` | β=0.5 で K=L/2 lag に強い anti-correlation、buffer.len() 不変 |
| **F29** | (Step 1 試作評価) Thiran 採用判定 | 案 D 採用済み (D36)、A1〜C6 で 0.02% 級、C8 のみ ignore 継続 |
| **F30** | `cargo test -p dsp-core test_engine_brightness_pitch_correction` | brightness=0.5 で A4 誤差 < 0.5% |
| **F31** | `cargo test -p dsp-core test_engine_midi_cc_` | CC#7 直交 / CC#64 defer / CC#123 reset、Mono+Sustain は no-op |
| **F32** | `cargo test -p dsp-core test_pitch_bend_` | ±2 clamp、ring buffer 不変、bend→0 で base_length 復帰 |
| **F33** | `cargo test -p dsp-core test_sustain_` | active/pending bitmap、retrigger で clear、reset で active=false |
| **F34** | 実機: `pnpm dev` → 8 鍵同時押下 | VoiceMeter 8 セルすべて active 表示、振幅で輝度変化 |
| **F35** | `cargo test -p dsp-core test_soft_clip_` | `\|x\|≤0.95` で完全 linear、`\|x\|→∞` で `\|y\|<1.0` 漸近 |
| **F36** | `pnpm build` 後 `gzip -c web/build/_app/immutable/assets/wasm_audio.*.wasm \| wc -c` | gzip < 30 KB (実測 28 KB) |
| **F37** | `cargo test --release -p dsp-core test_engine_process_block_timing -- --nocapture` | 平均 < 1.5 ms (実測 0.012 ms) |
| **F38** | `cargo test -p dsp-core test_no_allocation_with_modal_body_and_midi_cc` | 8 voice 全 active + CC + Pitch Bend で buffer.len() 不変 |
| **F38b** | (Phase 3 完成判定) `pnpm preview` → Chrome DevTools Performance タブで Worklet `process` Self time avg/max を計測 | avg < 1.5 ms / max < 2.5 ms。実機操作のため手動検証 |

### dsp-core ユニットテスト一覧

`cargo test -p dsp-core` で **94 件パス + 1 件 ignored**:

- Phase 1 既存 + Phase 2 拡張 (40 件): silence_when_inactive / energy_rises_after_note_on / decay_with_low_damping / length_matches_freq / no_allocation_in_process / paramid_roundtrip / damping_preserved_across_note_on / engine_processes_block_without_panic / midi_to_freq_a4 / poly_mode_independent_voices / setparam_clamps_out_of_range / note_on_first_block_nonzero / hold_stack 系 / voice_pool 系 / note_allocator 系
- fractional_delay (10 件): Lagrange 4 件 + set_fractional + Thiran 関連 6 件
- modal_body_biquad / modal_body (9 件): 単体 / aggregate
- loss_filter (4 件)
- karplus_strong_pick (5 件)
- soft_clip (6 件)
- pitch_bend (4 件)
- sustain (6 件)
- midi_cc (9 件)
- pitch_accuracy (5 PASS, 1 IGNORED): A1/A2/A4/C6 + long_term_stability + (`#[ignore]` C8、damping<1 で物理限界)

## クレート構成

| クレート | 種類 | 役割 |
|---|---|---|
| `crates/dsp-core` | rlib（純粋 Rust、依存ゼロ） | Engine / VoicePool / KarplusStrong (Thiran 単一型) / ModalBodyResonator / LossFilter / SoftClip / SustainState / NoteAllocator / HoldStack / SmoothedValue / XorShift32 / ParamDescriptor (生成) |
| `crates/wasm-audio` | cdylib（C ABI、wasm-bindgen 不使用） | `synth_*` 15 関数を `#[unsafe(no_mangle)] extern "C"` で公開（Phase 2 の 12 関数 + Phase 3 の midi_cc / pitch_bend / voice_state_ptr） |
| `web` | SvelteKit + adapter-static | UI / AudioWorklet / Web MIDI / VoiceMeter / PolyphonyToggle |

## Phase 3 で解消された Phase 2 の課題

- ✅ **音色のリアリティ**: Modal Body Resonator (M=8 ボディ共鳴) + Extended KS で「弦音だけでは安っぽい」から脱却
- ✅ **A4 ピッチ精度 0.89% → 0.0002%**: Thiran allpass (案 D) で約 4000× 改善
- ✅ **MIDI 表現力**: Pitch Bend / Channel Volume / Sustain Pedal 対応
- ✅ **mono/poly トグル UI 正式化** (Phase 2 では dev-only)
- ✅ **Voice Meter UI** (Phase 2 では internal API のみ)

## Phase 4 への申し送り

- C8 ピッチ自己発振: damping=1.0 自己発振モード or FFT-based estimator で再評価
- Mod Wheel (CC#1) + LFO の仕様確定 (rate / 波形 / 配分 / 深さ)
- プリセット保存・ロード (Modal 係数 + 全パラメータの localStorage / IndexedDB)
- 多楽器プリセット (クラシックギター / ウクレレ / マンドリン / ベース)
- Stretching all-pass + impact model でピアノ音色
- Pick position の fractional 化 + 連続変更
- Look-ahead limiter (5 ms 遅延型、soft clip より透明)
- WASM SIMD (`target-feature=+simd128`)
- F38b 実機計測 (Chrome DevTools Performance タブ): Worklet process 時間の検証

## ライセンス

未定（開発段階）。
