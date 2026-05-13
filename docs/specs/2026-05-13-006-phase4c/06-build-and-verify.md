# 06. ビルドと検証（Phase 4c）

## 目的

Phase 4c の検証項目（F59〜F70）、リスク（R40〜R44）、性能目標を定義する。Phase 1〜4b で確定した F1〜F58 および R1〜R39 はすべて維持し、本書では **Phase 4c で追加・更新する箇所のみ** 記述する。

## 他文書との関係

- 上流: [`pre-research.md`](./pre-research.md)（§11 性能予算 / §3.7 互換性 / §5.4 数値安定性）、[`01-overview.md`](./01-overview.md)（D68-D85 / 受け入れ基準）、[`03-dsp-core-spec.md`](./03-dsp-core-spec.md)（テスト方針）、[`04-wasm-audio-spec.md`](./04-wasm-audio-spec.md)（export 検証）、[`05-web-frontend-spec.md`](./05-web-frontend-spec.md)（実機確認項目）
- 下流: [`07-implementation-checklist.md`](./07-implementation-checklist.md)（Step ごとの検証達成ライン）
- 並行: Phase 4a / 4b [`06-build-and-verify.md`] — F1〜F58 および R1〜R39 の継承

## 性能目標（Phase 4c）

| 指標 | Phase 4b 後実測 | Phase 4c target | 警戒ライン | 撤退ライン |
|---|---|---|---|---|
| WASM gzip サイズ | 18.71 KB | **目標 < 22 KB**（想定 ~20 KB、Phase 4b + Phase 4c 純増 1.2 KB） | **警戒 < 25 KB**（要調査） | **撤退 < 30 KB**（R40: Modal M=8 維持 / B(note) LUT 簡素化） |
| Worklet バンドルサイズ (production) | 8.0 KB | < 12 KB（Phase 4b 同等、Phase 4c で TS 側変更最小） | > 14 KB で esbuild 設定見直し |
| `process` per call (Piano: 8 voice × 3 弦 + Body + LFO + Pitch Bend + CC#7 + **Sympathetic bus**、release cargo timing) | 0.047 ms | **目標 < 0.15 ms**（Phase 4b 0.047 + Phase 4c +0.035 = 0.082 ms 想定、Modal M=16 採用時 0.094 ms） | > 0.25 ms で R41（Multi-string buffer 案 2 / bridge coupling 撤回） |
| `process` per call (Piano 以外、Phase 4b 同条件) | 0.029 ms | < 0.05 ms（Sympathetic は Piano のみ active で他楽器影響ゼロ） | > 0.1 ms で regression 調査 |
| Worklet `process` self time avg (`__synthDev.measureProcessTime`、Piano kind) | 未計測（Phase 4b 持ち越し） | < 1.7 ms | > 2.0 ms で R30 (Phase 4a 継承、stride 4096 化等) |
| Worklet `process` self time max (`__synthDev.measureProcessTime`、Piano kind) | 未計測（Phase 4b 持ち越し） | < 2.7 ms | > 3.0 ms で R30 |
| ヒープ確保 in `process` | 0 | 0（Phase 1 D4 維持、`string_buffers` / `dispersion_stages` / `resonance_bus.buffer` は inline 配列 + Vec resize 済） | > 0 で R29（debug build で alloc 検査） |
| Phase 4a 互換性 (Default + Mod Wheel=0 + LFO depth=0) | Phase 4b で確認済 | Phase 4a HEAD と **ε=1e-6 バイト一致継承**（D83 機械保証、`n_strings = 1` 経路） | バイト不一致は実装誤り、即修正 |
| Phase 4b 互換性 (Piano 以外 7 楽器) | — | Phase 4b 出力と byte 一致継承（dispersion_active=false 経路は Phase 4b と完全同型） | バイト不一致は実装誤り、即修正 |
| Piano 聴感達成 (Step 19 ユーザー判定) | — | 「Phase 4b より本物のピアノに近づいた」とユーザー実機聴感で確認（D82） | 未達なら聴感チューニング反復 or Multi-string detune 値再評価 |
| WASM memory heap | ~64 KB (Phase 4b) | ~233 KB (Phase 4c) | > 512 KB で R42 (Multi-string buffer 案 2) |

## 検証項目（F-tag）

Phase 1〜4b の F1〜F58 に加え、Phase 4c で **F59〜F70 の 12 件**を追加。**本表が Phase 4c F-tag の単一マスタ**であり、03 章 §7 / 07 章のステップ記述はこの採番に厳密に従う。

### Phase 4c F-tag マスタ表（一覧）

| F-tag | テーマ | 関連 D タグ | 配置先テストファイル |
|---|---|---|---|
| F59 | `n_strings(midi)` / `string_detune_cents` 関数 | D69 / D72 | `tests/multi_string_tests.rs` |
| F60 | Multi-string per voice の note_on 動作 | D70 | `tests/multi_string_tests.rs` |
| F61 | Phase 4a HEAD byte 一致 + Phase 4b 7 楽器互換 | D83 | `tests/multi_string_tests.rs` |
| F62 | Multi-string detuning による beating / two-stage decay | D72 | `tests/multi_string_tests.rs` |
| F63 | Multi-string で `process_sample` の alloc ゼロ | Phase 1 D4 継承 | `tests/multi_string_tests.rs` |
| F64 | `ResonanceBus::process` 単体動作（feedback_gain と独立） | D76 | `tests/sympathetic_tests.rs` |
| F65 | Engine 経由の Sympathetic 統合と voice 注入経路 | D77 | `tests/sympathetic_tests.rs` |
| F66 | Hertz law raised cosine hammer のパラメータ式 | D74 / D75 | `tests/hammer_hertz_tests.rs` |
| F67 | B(note) LUT と MIDI clamp | D78 / D79 | `tests/hammer_hertz_tests.rs` または `multi_string_tests.rs` |
| F68 | `Engine::apply_instrument(Piano)` 経路の内部状態（dsp-core 内部、wasm-audio 側は薄いラッパでテストなし） | D81 | `tests/instrument_tests.rs` 拡張 |
| F69 | WASM サイズと CPU の検証 | — | `cargo build` + `cargo test --release --test cpu_timing` |
| F70 | F38b 実機計測（Step 1 ベースライン + Step 20 完了後） | D85 | `__synthDev.measureProcessTime` |

### F59 — `n_strings(midi)` 関数の鍵盤位置依存（D69）

| サブタグ | テスト名（仕様） | 期待 |
|---|---|---|
| F59-a | `test_n_strings_for_midi` | `n_strings(21)=1, n_strings(33)=1, n_strings(34)=2, n_strings(47)=2, n_strings(48)=3, n_strings(108)=3`、および **範囲外 `n_strings(20) = 1` / `n_strings(127) = 3`** の clamp 動作 |
| F59-b | `test_string_detune_cents_3_strings` | 3 弦時に `[0.0, -1.5, +1.5]` 返却 |
| F59-c | `test_string_detune_cents_2_strings` | 2 弦時に `[0.0, +1.5]` |
| F59-d | `test_string_detune_cents_1_string` | 1 弦時に `[0.0]` |

達成ライン: F59-a〜d 全て pass、cargo test 緑。

### F60 — Multi-string per voice の note_on 動作（D70 / D72）

| サブタグ | テスト名 | 期待 |
|---|---|---|
| F60-a | `test_piano_n_strings_3_at_c4` | Piano kind で C4 (60) note_on → `n_strings_active = 3` |
| F60-b | `test_piano_n_strings_2_at_b2` | Piano kind で B2 (47) note_on → `n_strings_active = 2` |
| F60-c | `test_piano_n_strings_1_at_a1` | Piano kind で A1 (33) note_on → `n_strings_active = 1` |
| F60-d | `test_default_kind_always_1_string` | Default kind で C4 note_on → `n_strings_active = 1` |

達成ライン: F60-a〜d 全て pass。

### F61 — Phase 4a / 4b 互換性継承（D83）

| サブタグ | テスト名 | 期待 |
|---|---|---|
| F61-a | `test_default_n_strings_1_matches_phase4a` | Default kind で 256 frame × 2ch 出力が Phase 4a HEAD fixture と ε=1e-6 バイト一致（Phase 4b 継承） |
| F61-b | `test_piano_n_strings_diverges_from_phase4b_fixed_b` | Piano kind で出力が Phase 4b（固定 B=7.5e-4、n_strings=1）と意図的に異なる（負のテスト） |
| F61-c | `test_guitar_classical_phase4b_byte_match` | Guitar Classical kind で Phase 4b 出力と byte 一致（dispersion_active=false 経路、Phase 4c で変化なし） |
| F61-d | `test_all_non_piano_kinds_n_strings_1` | Default / Guitar / Ukulele / Mandolin / Bass / GuitarSteel / Sitar の 7 種で n_strings_active = 1 維持 |
| F61-e | `test_default_kind_bus_direct_mix_is_zero` | Default kind + CC#64 ON でも `bus_mix = feedback_gain / FEEDBACK_GAIN_MAX = 0` で modal_body 入力に bus が寄与しない（[`03-dsp-core-spec.md` §4.4](./03-dsp-core-spec.md#44-engineprocess-の-sympathetic-bus-統合per-sample-loop-内挿入既存責務を完全維持) 参照）。Piano → Default 切替後に bus buffer が `resonance_bus.reset()` でクリアされる |

達成ライン: F61-a〜e 全て pass。**Phase 4a HEAD との byte 一致は Phase 4c の中核保証**、bus 経路の gate (F61-e) と bus reset (F65-h/i) が成立して初めて F61-a が安定する。

### F62 — Multi-string detuning の動作（D72）

| サブタグ | テスト名 | 期待 |
|---|---|---|
| F62-a | `test_string_detune_produces_beating` | 3 弦で 1.5 cents detune した出力に beating（振幅変調、典型 0.5〜2 Hz の amplitude envelope）が観測される |
| F62-b | `test_string_independent_dispersion_a1` | 各弦の dispersion a1 が detune で異なる f_0 を反映 |
| F62-c | `test_two_stage_decay_observation` | 1 秒持続音の前半 (0-200ms) と後半 (500-1000ms) で減衰率が異なる（two-stage decay の代理が機能） |

達成ライン: F62-a〜c 全て pass。

### F63 — Multi-string で alloc ゼロ（Phase 1 D4 継承）

| サブタグ | テスト名 | 期待 |
|---|---|---|
| F63 | `test_no_allocation_in_process_multi_string` | `process_sample` で N=3 弦 active 時に alloc ゼロ |

達成ライン: F63 pass。

### F64 — ResonanceBus 単体動作（D76）

`ResonanceBus::process(bus_in) -> f32` は **feedback_gain と独立** に lossy delay + LPF で bus_out を返す（[`03-dsp-core-spec.md` §3.1](./03-dsp-core-spec.md#31-構造体) 参照）。bus が出力に到達するのは Engine 経由の **2 経路**:
1. **voice 注入経路**: `inject = bus_out_prev × feedback_gain` で `process_sample_with_feedback` 内の各 voice に注入
2. **直接ミックス経路**: `bus_out × BUS_DIRECT_MIX_GAIN × bus_mix` で modal_body 入力にミックス（`bus_mix = feedback_gain / FEEDBACK_GAIN_MAX`）

どちらの経路も `feedback_gain = 0` で出力 0 になる設計（F65-a / F65-g 参照）。bus 単体テスト（F64）は bus 内部の数値安定性のみを検証し、出力経路の gate は F65 で別に検証する。

| サブタグ | テスト名 | 期待 |
|---|---|---|
| F64-a | `test_resonance_bus_process_returns_filtered_signal` | bus.process(impulse) が LPF + lossy delay 後の有限非ゼロ信号を返す（feedback_gain とは独立、入力が非ゼロなら出力も非ゼロ） |
| F64-b | `test_resonance_bus_decay_after_impulse` | bus_in に 1 sample 分のインパルスを入れた後、ゼロ入力を継続すると数十 sample で振幅が `1e-6` 以下に減衰 |
| F64-c | `test_resonance_bus_stability_1024_samples` | `BUS_INTERNAL_DECAY = 0.95` で 1024 sample 連続インパルス入力しても max amplitude < 10.0（発散しない） |
| F64-d | `test_resonance_bus_lpf_attenuation` | 4 kHz と 200 Hz の正弦波を bus.process に入れ、低域出力 / 高域出力比 > 2.0 |

達成ライン: F64-a〜d 全て pass。

### F65 — Engine への Sympathetic 統合と voice 注入経路（D77）

voice 注入は **`pool.process_sample_with_feedback(bus_out_prev, feedback_gain)` 経路** で行われ（[`03-dsp-core-spec.md` §4.4 / §5.3](./03-dsp-core-spec.md#43-note_on-の-multi-string--bnote-連携) 参照）、`feedback_gain × bus_out_prev` を全 voice の `inject_feedback` に渡す。Default kind / Sustain OFF で `feedback_gain = 0` のときに **注入値が 0** になることをテスト対象とする。

| サブタグ | テスト名 | 期待 |
|---|---|---|
| F65-a | `test_engine_inject_zero_when_feedback_gain_zero` | Default kind + Sustain ON、または Piano kind + Sustain OFF（SmoothedValue が target=0 に収束後）で voice への inject 値が 0（`pool.process_sample_with_feedback` 直前の `inject = bus_out_prev × feedback_gain` が 0） |
| F65-b | `test_engine_sustain_on_activates_sympathetic_piano` | Piano kind + Sustain ON で `resonance_bus.feedback_gain.target() > 0`（target = `sympathetic_amount × FEEDBACK_GAIN_MAX`）、数 sample 後に `next_feedback_gain() > 0` |
| F65-c | `test_engine_sustain_on_no_sympathetic_default` | Default kind + Sustain ON で `feedback_gain.target() = 0` 維持 |
| F65-d | `test_engine_sustain_off_zeroes_sympathetic` | Piano kind で Sustain OFF → 数 sample 後に `next_feedback_gain() ≈ 0` |
| F65-e | `test_engine_apply_instrument_resets_sympathetic` | Piano → Default 切替で `feedback_gain.target() = 0` |
| F65-f | `test_no_allocation_in_resonance_bus_process` | bus.process(0.5) で alloc ゼロ |
| F65-g | `test_engine_bus_mix_zero_when_feedback_gain_zero` | Default kind + Sustain ON または Piano + Sustain OFF (SmoothedValue 収束後) で `bus_mix = feedback_gain / FEEDBACK_GAIN_MAX = 0` → modal_body 入力は `dry + bus_out * BUS_DIRECT_MIX_GAIN * 0 = dry`。`engine.resonance_feedback_target_for_test() == 0` で機械保証 |
| F65-h | `test_engine_apply_instrument_resets_bus_buffer` | Piano kind で数 sample 発音 → `apply_instrument(Default)` で `resonance_bus.reset()` + `bus_out_prev = 0.0` 実行、bus 内部 delay line が完全 zero クリアされていることを `#[doc(hidden)]` accessor (ResonanceBus に `buffer_max_amplitude_for_test()` 等を追加) で観測 |
| F65-i | `test_engine_all_notes_off_resets_bus_buffer` | 同上、`handle_midi_cc(CC_ALL_NOTES_OFF, 1.0)` で bus が完全リセットされる |

達成ライン: F65-a〜i 全て pass。

### F66 — Hertz hammer のパラメータ式（D75）

| サブタグ | テスト名 | 期待 |
|---|---|---|
| F66-a | `test_hammer_t_c_decreases_with_velocity` | velocity=0.1 で t_c=3.72 ms, velocity=1.0 で t_c=1.2 ms |
| F66-b | `test_hammer_f_c_increases_with_velocity` | velocity=0.1 で f_c=1270 Hz, velocity=1.0 で f_c=5500 Hz |
| F66-c | `test_hammer_amplitude_sqrt_velocity` | velocity=0.25 で amp=0.5、velocity=1.0 で amp=1.0 |
| F66-d | `test_hammer_raised_cosine_shape` | buffer[0..t_c_samples] が sin² で形成、ピークは中央 (i = t_c/2 近傍) |
| F66-e | `test_hammer_velocity_affects_brightness` | velocity=0.1 と 1.0 で出力スペクトル centroid が顕著に異なる（centroid_v10 > centroid_v01 × 1.5） |
| F66-f | `test_hammer_pluck_path_for_default` | Default kind の note_on で pluck 経路（noise burst）が走る、hammer 経路は走らない |

達成ライン: F66-a〜f 全て pass。

### F67 — B(note) LUT と MIDI clamp（D78 / D79）

`b_curve_piano(midi)` は `midi.clamp(21, 108)` で範囲外 (< 21 / > 108) を端値に丸めてから LUT を引く（[`03-dsp-core-spec.md` §2](./03-dsp-core-spec.md#2-dispersionrs-の-b-引数化戻り値は-phase-4b-同型の-tuple-を維持) 参照）。これにより Engine から渡される `u8` 全域 (0..=127) に対して未定義動作 / panic が発生しないことを機械保証する。

| サブタグ | テスト名 | 期待 |
|---|---|---|
| F67-a | `test_b_curve_length_88` | `INHARMONICITY_B_CURVE_PIANO.len() == 88` |
| F67-b | `test_b_curve_lookup_a0` | `b_curve_piano(21) ≈ 3.1e-4`（A0、低音） |
| F67-c | `test_b_curve_lookup_a4` | `b_curve_piano(69) ≈ 7.5e-4`（A4、Phase 4b 互換値と近似一致） |
| F67-d | `test_b_curve_lookup_c8` | `b_curve_piano(108) >= 0.05`（C8、高音） |
| F67-e | `test_b_curve_monotonic_increase_above_a3` | A3 (MIDI 57) 以上で LUT 値が単調増加 |
| F67-f | `test_b_curve_clamps_out_of_range` | `b_curve_piano(0) == LUT[0]` かつ `b_curve_piano(127) == LUT[87]`（範囲外 MIDI で端値 fallback） |
| F67-g | `test_b_curve_used_in_note_on_piano` | Piano kind の note_on で `set_instrument_params` 経由で渡される `inharmonicity_b` が `b_curve_piano(midi)` と一致 |
| F67-h | `test_b_curve_not_used_for_default` | Default kind の note_on で `inharmonicity_b = 0`（`b_curve_zero` が常に 0 返却） |

達成ライン: F67-a〜h 全て pass。

### F68 — `Engine::apply_instrument(Piano)` 経路の内部状態検証（D81）

**配置**: dsp-core 内部の `tests/instrument_tests.rs` 拡張で検証する。wasm-audio 側の `synth_apply_instrument(handle, 7)` は dsp-core への薄いラッパなので、内部状態検証は dsp-core で完結させる（04 章 §10 の「wasm-audio 側に Phase 4c 追加テストなし」と整合）。

**内部状態の観測**: `#[doc(hidden)] pub fn ..._for_test()` を 03 章 §7.5 の accessor 表に従って追加（`voice_pool.rs:165` の既存 `voice_index_for_note` / `voice_length_int` パターンを踏襲）。主要 accessor: `Engine::sustain_active_for_test`、`Engine::resonance_feedback_target_for_test`、`Engine::voice_n_strings_active_for_test(midi)`、`VoicePool::voice_inharmonicity_b_for_test(idx)`、`VoicePool::voice_unison_detune_cents_for_test(idx)`、`VoicePool::voice_dispersion_active_for_test(idx)`。

| サブタグ | テスト名 | 期待 |
|---|---|---|
| F68-a | `test_apply_instrument_piano_activates_all_features` | `engine.apply_instrument(Piano)` → `engine.note_on(60, 0.8)` 後、`engine.voice_n_strings_active_for_test(60) == Some(3)`、割当 voice の `voice_inharmonicity_b_for_test ≈ b_curve_piano(60)`、`voice_unison_detune_cents_for_test == 1.5`、`voice_dispersion_active_for_test == Some(true)` |
| F68-b | `test_apply_instrument_default_deactivates_all_features` | `engine.apply_instrument(Default)` → `engine.note_on(60, 0.8)` 後、`voice_n_strings_active_for_test(60) == Some(1)` + `voice_dispersion_active_for_test == Some(false)` + `voice_inharmonicity_b_for_test == 0.0` + `voice_unison_detune_cents_for_test == 0.0` |
| F68-c | `test_apply_instrument_piano_resets_sustain_and_bus_gain` | `engine.handle_midi_cc(CC_SUSTAIN_PEDAL, 1.0)` → `engine.apply_instrument(Piano)` 後、`engine.sustain_active_for_test() == false` かつ `engine.resonance_feedback_target_for_test() == 0.0` |
| F68-d | `test_apply_instrument_piano_preset_byte_diverges_from_phase4b` | Piano kind 出力が Phase 4b と byte 不一致（F61-b と独立、別の波形条件で確認） |

達成ライン: F68-a〜d 全て pass。

### F69 — WASM サイズと CPU の検証

| サブタグ | テスト名 / 検証手順 | 期待 |
|---|---|---|
| F69-a | `pnpm build:wasm` 後の gzip サイズ | < 22 KB（target 20 KB、警戒 25 KB） |
| F69-b | `cargo test --release -p dsp-core --test cpu_timing` (Piano) | < 0.15 ms / 128 frames |
| F69-c | `cargo test --release -p dsp-core --test cpu_timing` (非 Piano) | < 0.05 ms / 128 frames |
| F69-d | `wasm-opt --metrics` で binary 全体の関数数 / size breakdown を確認 | Phase 4b 比 raw +3 KB、gzip +1.2 KB 想定 |

達成ライン: F69-a〜d 全て達成、`check-wasm-exports.mjs` 緑（19 required exports 確認）。

### F70 — F38b 実機計測（Phase 4b 持ち越し + Phase 4c 後再計測、D85）

| サブタグ | 検証手順 | 期待 |
|---|---|---|
| F70-a (Step 1) | `pnpm dev` + Console で `window.__synthDev.measureProcessTime(5000)` 実行（Phase 4b Piano プリセット） | Phase 4b ベースライン値: Piano 0.047 ms / 非 Piano 0.029 ms 程度を確認 |
| F70-b (Step 20) | 同上 (Phase 4c 完成後の Piano プリセット) | Piano avg < 1.7 ms / max < 2.7 ms 達成 |
| F70-c | iPhone Safari 実機での Piano 演奏 | 音切れ・ノイズなしで Multi-string + Sympathetic 動作 |

達成ライン: F70-a (Step 1) でベースライン確認、F70-b (Step 20) で Phase 4c 完成後値を仕様書 retrospective に記録、F70-c で iPhone 実機動作確認（Phase 4a F9 継承）。

## リスク（R-tag）

Phase 1〜4b の R1〜R39 に加え、Phase 4c で **R40〜R44 の 5 件**を追加。

### R40 — WASM gzip サイズが 25 KB 警戒ライン超過

| 項目 | 内容 |
|---|---|
| 検知 | Step 9 (Multi-string KS 実装完成時) または Step 15 (Hertz hammer 完成時) で gzip > 25 KB |
| 影響 | 撤退ライン 30 KB に近づくと R32 (Modal 係数削減) を発動する必要 |
| 緩和 | B(note) LUT を 88 値 → 22 値 (4 半音ごと) に削減 / Modal M=16 拡張を撤回 / Multi-string buffer 案 2 移行 |
| 対応者 | 実装担当 |

### R41 — `process` per call が 0.25 ms 警戒ライン超過

| 項目 | 内容 |
|---|---|
| 検知 | Step 14 / Step 17 / Step 20 の cargo timing or `__synthDev.measureProcessTime` で Piano > 0.25 ms |
| 影響 | target 1.7 ms から見て依然余裕大、聴感問題なし。ただし将来の SIMD / 他楽器拡張で予算枯渇リスク |
| 緩和 | Multi-string buffer 案 2 (共有 buffer + 3 read 位置) で memory cache 圧迫を回避 / bridge coupling (案 B) を撤回 / Modal M=16 を撤回 |
| 対応者 | 実装担当、retrospective で R29 / R30 と統合判断 |

### R42 — WASM memory heap が 512 KB 超過

| 項目 | 内容 |
|---|---|
| 検知 | Step 9 で `wasm-memory` 計測 |
| 影響 | iPhone Safari の memory 制約に抵触する可能性、 `memory.grow()` で linear memory が線形拡大 |
| 緩和 | Multi-string buffer 案 1 → 案 2 (共有 buffer + 3 read 位置、+111.7 KB を回避) |
| 対応者 | 実装担当 |

### R43 — Sympathetic bus の数値発散

| 項目 | 内容 |
|---|---|
| 検知 | F64-c (`test_resonance_bus_stability_1024_samples`) or Step 14 実機聴感で「鳴り続ける」「クリップする」 |
| 影響 | 出力に NaN / Inf が混入、または unstable な振動 |
| 緩和 | `feedback_gain` の clamp を 0.05 → 0.03 に強化、bus 内部の `BUS_INTERNAL_DECAY` を 0.95 → 0.90 に強化、LPF cutoff を低下 |
| 対応者 | 実装担当、§5.4 (pre-research) の安定性条件を再確認 |

### R44 — Piano 聴感が「Phase 4b より本物のピアノに近づいた」と確認できない

| 項目 | 内容 |
|---|---|
| 検知 | Step 19 のユーザー実機聴感で「変化はあるが本物のピアノとは違う」「むしろ Phase 4b の方が良い」と評価 |
| 影響 | Phase 4c の主目的未達、retrospective §5 (Phase 4b と同じ「弦楽器寄り」問題) を踏襲してしまう |
| 緩和 (順番に試す) | (1) Piano プリセット聴感調整（damping / brightness / bodyWet / unison_detune_cents / sympathetic_amount / hammer_cutoff の反復）、(2) Modal Body M=16 拡張 (Step 15)、(3) Bridge coupling (案 B) 追加 (Step 15)、(4) B(note) LUT 値の見直し (Young 1952 / Conklin 1996 fitting の精密化)、(5) これでも未達なら Phase 4d で「Two-stage decay 明示実装」「複数 Piano 機種プリセット」「Una corda」等の追加検討 |
| 対応者 | ユーザー + 実装担当、retrospective §5 に記録 |

## 計測方法（Phase 4c で追加）

### `__synthDev.measureProcessTime` の使用（D85、Phase 4b 完成済 API）

```javascript
// 1. pnpm dev でブラウザを開く
// 2. Start ボタンを押す
// 3. PresetSelector で Piano を選択
// 4. Console で以下を実行
const result = await window.__synthDev.measureProcessTime(5000);
console.log(result);
// 期待出力: { avg: 0.047, max: 0.063, min: 0.038, samples: [...], bufferOverflow: false }
```

Phase 4c Step 1 でベースライン (Phase 4b 0.047 ms)、Step 20 で Phase 4c 後を取得。

### `wasm-opt --metrics` の使用（Phase 4b で確立、Phase 4c で継続）

```bash
wasm-opt --metrics target/wasm32-unknown-unknown/release/wasm_audio.wasm > wasm-metrics-phase4c.txt
```

binary 全体の関数数 / size breakdown を確認、Phase 4b との差分を retrospective §8 に記録。

### `cargo test --release` での cpu timing（F69-b, F69-c）

```bash
cargo test --release -p dsp-core --test cpu_timing -- --nocapture
```

`tests/cpu_timing.rs` を新規作成 or Phase 4b の既存テストを拡張、Piano / 非 Piano で 100 回 process_block を測定して平均値を出力。

## CI / GitHub Actions への影響

Phase 4b の CI (`.github/workflows/ci.yml` / `deploy.yml`) に変更なし。Phase 4c で追加するもの:
- `cargo test -p dsp-core` の実行時間が増える（Phase 4b 148 PASS + Phase 4c 新規 ~30 件 = ~178 PASS、CI 全体時間 +10〜20 秒想定）
- `check-wasm-exports.mjs` で 19 exports 確認（Phase 4a / 4b と同じ）

## Phase 4c 完了後の更新ドキュメント

| ドキュメント | 更新内容 |
|---|---|
| `CLAUDE.md` | 「完了済みイテレーション」に Phase 4c エントリ追加（05 章 §11 参照） |
| `docs/retrospective/2026-05-13-006-phase4c.md` | Phase 4c retrospective 新規作成、§1 概要 / §2 達成と未達 / §3 主要な設計判断 / §4 躓きと教訓 / §5 既存負債 / §6 開発フロー上の改善 / §7 次イテレーション (Phase 4d) への引き継ぎ / §8 メトリクス / §9 メモリ更新案 |
| `README.md` | 必要なら Phase 4c の概要を追加（既存 README が音色概要を含む場合のみ） |

## まとめ

Phase 4c のビルドと検証は **F59〜F70 の 12 件 + R40〜R44 の 5 件** で構成。性能目標は WASM gzip ~20 KB / Piano process < 0.15 ms / 非 Piano < 0.05 ms / heap ~233 KB、Phase 4a HEAD byte 一致 (`n_strings = 1`) と Phase 4b 互換 (Piano 以外 7 楽器) の機械保証を中核とする。**Piano 聴感達成 (R44 を回避し D82 を満たす) が完了条件**、cargo / clippy 全 green に加え Step 19 のユーザー実機聴感確認が必須。F38b 実機計測は Step 1 (ベースライン) + Step 20 (Phase 4c 後) でユーザー操作必須、Auto mode 完結不可（Phase 4b と同じ持ち越し）。
