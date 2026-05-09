# physics-base-synth

物理ベース・シンセサイザー（Karplus–Strong）の Phase 4b (Phase 4a + ピアノ音色 / Stretching all-pass + Hammer model) 対応版。Rust + WebAssembly + Svelte 5 (SvelteKit) で実装。

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

## アーキテクチャ概要 (Phase 4a)

```
Svelte UI (main thread) ── MessagePort ─→ AudioWorkletProcessor
   PresetSelector / ModWheel                │ FFI (C ABI、wasm-bindgen 不使用)
   LfoSection (rate/waveform/3 depth)        ▼
   VoiceMeter / PolyphonyToggle          wasm-audio (cdylib)
   PickPosition / BodyWet スライダー       18 関数 + memory export
   WebMIDI CC handler (CC#1/7/64/123)        + synth_apply_instrument
   Pitch Bend                                + synth_lfo_set_rate / waveform / depth
                                             ▼
                                        dsp-core (rlib)
                                        Engine + Lfo + mod_wheel + lfo_*_depth
                                        VoicePool<8> / KarplusStrong (Thiran allpass)
                                        ModalBodyResonator (M=8、楽器 7 種切替)
                                        LossFilter / SoftClip / SustainState / HoldStack
                                        FractionalDelay (Thiran) / NoteAllocator
                                        SmoothedValue / XorShift32 / ParamDescriptor (生成)
                                        InstrumentKind enum + body_modes_for_instrument
```

詳細は仕様書 (`docs/specs/`) を参照:
- Phase 1 (MVP): `docs/specs/2026-05-06-001-mvp/`
- Phase 2 (polyphony / fractional delay / ParamDescriptor): `docs/specs/2026-05-07-002-phase2/`
- Phase 3 (Body Resonator / Extended KS / MIDI CC / Voice Meter): `docs/specs/2026-05-07-003-phase3/`
- Phase 4a (LFO / Mod Wheel / Preset / 多楽器 6 種 / wasm-opt -O3): `docs/specs/2026-05-08-004-phase4a/`
- Phase 4b (ピアノ音色 / Stretching all-pass + Hammer model + Modal Body Piano + `__synthDev.measureProcessTime` + `.gitattributes` LF 統一): `docs/specs/2026-05-09-005-phase4b/`

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

`cargo test -p dsp-core` で **120 件パス + 1 件 ignored** (Phase 3 既存 93 + Phase 4a 新規 27):

- Phase 1 既存 + Phase 2 拡張 (40 件): silence_when_inactive / energy_rises_after_note_on / decay_with_low_damping / length_matches_freq / no_allocation_in_process / paramid_roundtrip / damping_preserved_across_note_on / engine_processes_block_without_panic / midi_to_freq_a4 / poly_mode_independent_voices / setparam_clamps_out_of_range / note_on_first_block_nonzero / hold_stack 系 / voice_pool 系 / note_allocator 系
- fractional_delay (10 件): Lagrange 4 件 + set_fractional + Thiran 関連 6 件
- modal_body_biquad / modal_body (9 件): 単体 / aggregate
- loss_filter (4 件)
- karplus_strong_pick (5 件)
- soft_clip (6 件)
- pitch_bend (4 件)
- sustain (6 件)
- midi_cc (11 件: Phase 3 9 件 + Phase 4a Mod Wheel 2 件)
- pitch_accuracy (5 PASS, 1 IGNORED): A1/A2/A4/C6 + long_term_stability + (`#[ignore]` C8、damping<1 で物理限界)
- **Phase 4a 新規**:
  - lfo (8 件): sine/triangle range / zero_at_init / period / rate_smoothing / waveform_switch / no_alloc / phase_wrap
  - lfo_destinations (6 件): test_mod_wheel_zero_disables_lfo (Phase 3 互換最重要) / one_full_lfo / Pitch/Brightness/Volume modulation / no_alloc
  - instrument (7 件): changes_modal_coeffs / releases_voices / clears_sustain / resets_modal_state / no_alloc / stereo_spread_per_instrument / default_matches_phase3
  - modal_body Phase 4a 拡張 (3 件): set_instrument_changes_coeffs / clears_state / default_matches_phase3
  - karplus_strong excitation_tests (4 件): pick_min_beta / node_at_beta_half / attenuates_kth_harmonic / k_zero_branch (Phase 3 既存 4 件を unit test mod へ移動)
  - no_alloc (1 件): test_no_allocation_with_lfo_and_instrument (LFO + 楽器切替時の capacity 不変)
  - dsp_core (release ビルドのみ): test_engine_process_block_timing_phase4a (8 voice + LFO + Mod Wheel + Pitch Bend + CC#7 で < 1.7 ms)

## クレート構成

| クレート | 種類 | 役割 |
|---|---|---|
| `crates/dsp-core` | rlib（純粋 Rust、依存ゼロ） | Engine / VoicePool / KarplusStrong (Thiran 単一型) / ModalBodyResonator / LossFilter / SoftClip / SustainState / NoteAllocator / HoldStack / SmoothedValue / XorShift32 / ParamDescriptor (生成) |
| `crates/wasm-audio` | cdylib（C ABI、wasm-bindgen 不使用） | `synth_*` 18 関数 + memory export = 19 required exports（Phase 2 の 12 関数 + Phase 3 の midi_cc / pitch_bend / voice_state_ptr + Phase 4a の apply_instrument / lfo_set_rate / lfo_set_waveform / lfo_set_depth） |
| `web` | SvelteKit + adapter-static | UI / AudioWorklet / Web MIDI / VoiceMeter / PolyphonyToggle |

## Phase 3 で解消された Phase 2 の課題

- ✅ **音色のリアリティ**: Modal Body Resonator (M=8 ボディ共鳴) + Extended KS で「弦音だけでは安っぽい」から脱却
- ✅ **A4 ピッチ精度 0.89% → 0.0002%**: Thiran allpass (案 D) で約 4000× 改善
- ✅ **MIDI 表現力**: Pitch Bend / Channel Volume / Sustain Pedal 対応
- ✅ **mono/poly トグル UI 正式化** (Phase 2 では dev-only)
- ✅ **Voice Meter UI** (Phase 2 では internal API のみ)

## Phase 4a で追加された機能

- **wasm-opt -O3 統合 (D45)**: `scripts/copy-wasm.mjs` に Binaryen 製 wasm-opt を組み込み。release ビルドで `--strip-debug` + 全最適化 pass。WASM gzip 27.78 → 18.42 KB に圧縮。
- **`excitation_snapshot` cfg(test) 化 (D45)**: production binary から完全除外、関連 test を unit test mod に移動して件数保持。
- **多楽器プリセット 6 種 (D52/D54)**: Default + Guitar Classical / Ukulele / Mandolin / Bass / Guitar Steel / Sitar。各楽器に固有の `BODY_MODES_<INSTRUMENT>_L/R` 8 mode + `STEREO_SPREAD_<INSTRUMENT>`。Default kind の係数は Phase 3 既存値の完全 alias で後方互換を保証。
- **グローバル LFO (D46/D47/D48)**: Engine 内 1 個。波形 Sine / Triangle、レンジ 0.1〜8.0 Hz、SmoothedValue tau=0.05s で rate 平滑化。Pitch / Brightness / Volume の 3 destinations を独立 depth で制御。Engine 側で `pitch_factor = exp2(...)` を 1 回計算して全 voice に fan-out (per voice exp2 を回避、+15 演算/sample)。
- **Mod Wheel (CC#1, D49)**: `Engine::handle_midi_cc` の CC#1 分岐を有効化。`mod_wheel: SmoothedValue (tau=0.05s)` を全 LFO destination depth の master 乗数として保持。**Mod Wheel = 0 で LFO 効果ゼロ → Phase 3 互換動作と完全一致** (`test_mod_wheel_zero_disables_lfo` で機械保証)。
- **`Engine::apply_instrument(kind)` (D52/D53)**: 楽器切替で `pool.all_notes_off()` → `hold_stack.clear()` → `sustain_state.reset()` → `current_instrument` 更新 → `modal_body.set_instrument(kind, sample_rate)`。即時 release (fade-out なし)、UI で「楽器を切り替えると現在の音は止まります」を告知。
- **C ABI 4 関数追加**: `synth_apply_instrument` / `synth_lfo_set_rate` / `synth_lfo_set_waveform` / `synth_lfo_set_depth`。Phase 3 既存 14 関数 + memory export = 15 → Phase 4a で **18 関数 + memory = 19 required exports**。
- **プリセット保存・ロード (D50/D51)**: localStorage v1 schema (`physbase.preset.v1.*`)、Factory Preset 7 種 + User Preset 最大 32 件。`isValidPresetV1` で schema レベル一括バリデーション (型 / 値域 / NaN / Infinity / 空名 / name.length > 64 / 未知 enum)、Factory 名衝突 / User 上限は store-specific で別途チェック。
- **PresetSelector / ModWheel / LfoSection UI**: `optgroup` で Factory / User を分離、Save / Delete ボタン (Factory 削除 disabled)、`onMount` で last preset 復元。

### Phase 4a 検証項目 (F38b + F39〜F47)

| ID | 手順 | 期待結果 |
|---|---|---|
| **F38b** | Phase 3 持ち越し: `pnpm preview` → Chrome DevTools Performance タブで Worklet `process` Self time avg/max を計測 | avg < 1.5 ms / max < 2.5 ms (Phase 3 完成判定)、再計測で avg < 1.7 ms / max < 2.7 ms |
| **F39** | `pnpm build` 後 `gzip -kc web/build/_app/immutable/assets/wasm_audio.*.wasm \| wc -c` | gzip 目標 < 15 KB / 警戒 < 18 KB / 撤退 < 30 KB (実測 18.42 KB、警戒微超過) |
| **F40** | `cargo test -p dsp-core test_lfo_` + 実機操作 | LFO Sine/Triangle 範囲 / 5Hz 周期 / rate 平滑 / phase wrap、実機で vibrato/tremolo/wah 確認 |
| **F41** | `cargo test -p dsp-core test_midi_cc_mod_wheel_` + 実機操作 | CC#1 で mod_wheel.target() 更新 / 0..1 clamp、WebMIDI 物理 wheel と UI スライダー同期 |
| **F42** | `pnpm --filter ./web check` + 実機操作 | isValidPresetV1 が schema 違反を一括 reject、Save / Delete / リロードで User Preset が残る、32 件超過で errorMessage |
| **F43** | `cargo test -p dsp-core test_apply_instrument_` + 実機操作 | apply_instrument で modal coeffs 変化 / 全 voice release / sustain pending=0、6 種で音色切替 |
| **F44** | `cargo build --target wasm32-unknown-unknown --release` で `excitation_snapshot` シンボル除外 | production binary で関数除外、cargo test 件数保持 |
| **F45** | `cargo test -p dsp-core test_no_allocation_with_lfo_and_instrument` | 8 voice + LFO + Mod Wheel + 楽器切替で voice buffer / LFO 状態 capacity 不変 |
| **F46** | `cargo test --release -p dsp-core test_engine_process_block_timing_phase4a -- --nocapture` | 平均 < 1.7 ms (実測 0.023 ms、74× 余裕) |
| **F47** | Phase 3 既存 93 件 + 1 IGNORED 維持 + Default プリセット + Mod Wheel = 0 で Phase 3 と同じ音 | regression なし |

## Phase 4b で追加された機能

- **ピアノ音色 (D56-D62)**: 8 番目の楽器 `InstrumentKind::Piano = 7` を追加。
  - **Stretching all-pass cascade (D57/D58/D59)**: M=8 段の 1 次 allpass を KS ループに直列、Rauhala-Välimäki 2006 closed-form で a1 を算出。Piano `inharmonicity_b = 7.5e-4` (A4 基準) で stiff string の `f_n = n·f_0·√(1+B·n²)` を再現。
  - **Hammer model (D61)**: `note_on` の buffer 初期化を pluck/hammer で分岐。Piano は **Commuted impulse + velocity-dependent 1pole IIR LPF** (cutoff = lerp(800Hz, 4000Hz, velocity)) で felt hammer を近似。
  - **Piano Modal Body 係数 (D62)**: Conklin 1996 の grand piano soundboard 第 1 モード = 55Hz、stereo_spread = 0.05、M=8 (`BODY_MODES_PIANO_L/R`)。
  - **Factory Preset Piano**: damping=0.998 / brightness=0.55 / outputGain=0.7 / pickPosition=0.13 / bodyWet=0.4。LFO depth 全 0 (標準ピアノは vibrato なし)。
- **Phase 4a 互換性のバイト一致保証 (D67)**: `dispersion_active = false` の Phase 4a 既存 7 楽器では `process_sample` で cascade を skip し、`thiran.process(self.buffer[read_z])` の Phase 4a 経路と完全一致。`test_dispersion_disabled_matches_phase4a` で Phase 4a HEAD (commit dfa81c3) との出力 256 frame × 2ch を ε=1e-6 でバイト一致確認。
- **`__synthDev.measureProcessTime` (D66)**: dev-only F38b 計測自動化スクリプト。AudioWorklet 側で `performance.now()` の差分を取り Float32Array(4096) リングバッファ (約 10.92 秒分) に self time を蓄積、`stopTimingCapture` で main へ集約。`if (DEV_MODE)` ガード + `--define:DEV_MODE=true/false` + `--minify-syntax` で production tree-shake。
- **`.gitattributes` LF 統一 (D65)**: `* text=auto eol=lf` + 主要拡張子の eol=lf を明示、Phase 4a で頻発した CRLF/LF 戦争を断つ。

### Phase 4b 検証項目 (F48〜F58)

| ID | 手順 | 期待結果 |
|---|---|---|
| **F48** | `pnpm dev` → ブラウザ DevTools Console: `await window.__synthDev.measureProcessTime(10000)` | avg < 1.7 ms / max < 2.7 ms (Piano kind 最悪ケース) |
| **F49** | `pnpm build` 後 `gzip -kc web/build/_app/immutable/assets/wasm_audio.*.wasm \| wc -c` | gzip 目標 < 22 KB / 警戒 < 25 KB / 撤退 < 30 KB (実測 18.71 KB) |
| **F50** | `cargo test --release -p dsp-core test_engine_process_block_timing_phase4b -- --nocapture` | Piano kind avg < 1.7 ms / 非 Piano avg < 1.0 ms (実測 0.043 / 0.026 ms) |
| **F51** | `cargo test -p dsp-core test_dispersion_` | a1 値域 / B 単調性 / Ikey 補正 / cascade 安定 / 群遅延正、計 8 件 |
| **F52** | `cargo test -p dsp-core test_note_on_with_dispersion_` + `test_hammer_velocity_affects_brightness` | hammer 経路 = 単調減衰、pluck 経路 = noise burst、velocity で高域成分変化 |
| **F53** | `cargo test -p dsp-core test_piano_modal_` + `test_apply_instrument_piano_modal_coeffs` | Piano Modal 係数 (55Hz/0.05) を Engine 経由で取得・適用 |
| **F54** | `cargo test -p dsp-core test_apply_instrument_piano_enables_dispersion` | apply_instrument(Piano) で全 8 voice の dispersion_active=true、Default で false |
| **F55** | `cargo test -p dsp-core test_dispersion_disabled_matches_phase4a` | Default + Mod Wheel=0 + LFO depth=0 で Phase 4a HEAD と ε=1e-6 バイト一致 |
| **F56** | `git ls-files --eol \| grep -v "i/lf" \| grep -v binary` | 出力ゼロ (LF 統一)、`pnpm fmt` 後の CRLF/LF 差分なし |
| **F57** | Phase 4a 既存 120 + 1 IGNORED + Default + Mod Wheel = 0 で Phase 4a と同じ音 | regression なし |
| **F58** | `cargo test -p dsp-core test_no_allocation_with_piano_kind` | 8 voice + Piano + 楽器切替で voice buffer / dispersion_stages capacity 不変 |

## Phase 4c への申し送り (別計画扱い)

- C8 ピッチ自己発振: damping=1.0 自己発振モード or FFT-based estimator
- Pick position の fractional 化 + 連続変更
- Look-ahead limiter (5 ms 遅延型、soft clip より透明)
- WASM SIMD (`target-feature=+simd128`) — Safari/Firefox 対応再評価
- LFO 波形拡張 (S&H / Square / Sawtooth)
- LFO destinations 拡張 (Pick / Damping / BodyWet)
- 楽器切替の fade-out / cross-fade (D63 改訂で Phase 4b 撤回、`PendingInstrumentChange` 状態機械で本実装)
- Cross-tab preset 同期 (`storage` event)
- Preset JSON ファイル import / export
- Mono + Sustain の本実装
- 複数 Piano 機種プリセット (Grand / Upright / Honkytonk)
- Hammer Hardness UI 露出
- Sustain × Sympathetic resonance
- Piano 高次モード (M=16)
- Hertz law hammer (Boutillon)

## ライセンス

未定（開発段階）。
