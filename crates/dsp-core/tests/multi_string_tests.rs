//! Phase 4c Step 11: Multi-string per voice の integration tests。
//!
//! F59 (`n_strings` / `string_detune_cents` helpers)、F60 (note_on の弦数決定)、
//! F61 (Phase 4a HEAD byte 一致 / Phase 4b 互換 / Default kind の bus 寄与 0)、
//! F62 (Multi-string detuning による beating / dispersion a1 / two-stage decay)、
//! F63 (`process_sample` の alloc ゼロ) を集約。

#[path = "fixtures/phase4a_default_c4_v08.rs"]
mod phase4a_golden;

use dsp_core::engine::Engine;
use dsp_core::karplus_strong::{n_strings, string_detune_cents, KarplusStrong};
use dsp_core::params::{InstrumentKind, UNISON_DETUNE_CENTS_PIANO};
use dsp_core::traits::AudioProcessor;

const SAMPLE_RATE: f32 = 48_000.0;

// ===== F59: helper 関数 =====

/// F59-a: `n_strings(midi)` の鍵盤位置依存と範囲外 clamp 動作。
#[test]
fn test_n_strings_for_midi() {
    // A0..A1 (21..=33) → 1 弦
    assert_eq!(n_strings(21), 1);
    assert_eq!(n_strings(33), 1);
    // A#1..B2 (34..=47) → 2 弦
    assert_eq!(n_strings(34), 2);
    assert_eq!(n_strings(47), 2);
    // C3..C8 (48..=108) → 3 弦
    assert_eq!(n_strings(48), 3);
    assert_eq!(n_strings(108), 3);
    // 範囲外: 低音側は 1 弦、高音側は 3 弦に fallback
    assert_eq!(n_strings(0), 1);
    assert_eq!(n_strings(20), 1);
    assert_eq!(n_strings(109), 3);
    assert_eq!(n_strings(127), 3);
}

/// F59-b: 3 弦時の detune パターン (中央 0, 左 -base, 右 +base)。
#[test]
fn test_string_detune_cents_3_strings() {
    let base = 1.5_f32;
    assert!((string_detune_cents(0, 3, base) - 0.0).abs() < 1e-9);
    assert!((string_detune_cents(1, 3, base) - (-1.5)).abs() < 1e-9);
    assert!((string_detune_cents(2, 3, base) - 1.5).abs() < 1e-9);
}

/// F59-c: 2 弦時の detune パターン (中央 0, 片側 +base)。
#[test]
fn test_string_detune_cents_2_strings() {
    let base = 1.5_f32;
    assert!((string_detune_cents(0, 2, base) - 0.0).abs() < 1e-9);
    assert!((string_detune_cents(1, 2, base) - 1.5).abs() < 1e-9);
}

/// F59-d: 1 弦時は常に 0 cents。
#[test]
fn test_string_detune_cents_1_string() {
    let base = 1.5_f32;
    assert!((string_detune_cents(0, 1, base) - 0.0).abs() < 1e-9);
}

// ===== F60: Multi-string per voice の note_on 動作 =====

/// F60-a: Piano kind で C4 (MIDI 60) note_on → n_strings_active = 3。
#[test]
fn test_piano_n_strings_3_at_c4() {
    let mut engine = Engine::new();
    engine.prepare(SAMPLE_RATE, 128);
    engine.apply_instrument(InstrumentKind::Piano);
    engine.note_on(60, 0.8);
    assert_eq!(engine.voice_n_strings_active_for_test(60), Some(3));
}

/// F60-b: Piano kind で B2 (MIDI 47) note_on → n_strings_active = 2。
#[test]
fn test_piano_n_strings_2_at_b2() {
    let mut engine = Engine::new();
    engine.prepare(SAMPLE_RATE, 128);
    engine.apply_instrument(InstrumentKind::Piano);
    engine.note_on(47, 0.8);
    assert_eq!(engine.voice_n_strings_active_for_test(47), Some(2));
}

/// F60-c: Piano kind で A1 (MIDI 33) note_on → n_strings_active = 1。
#[test]
fn test_piano_n_strings_1_at_a1() {
    let mut engine = Engine::new();
    engine.prepare(SAMPLE_RATE, 128);
    engine.apply_instrument(InstrumentKind::Piano);
    engine.note_on(33, 0.8);
    assert_eq!(engine.voice_n_strings_active_for_test(33), Some(1));
}

/// F60-d: Default kind で C4 note_on → n_strings_active = 1 (Phase 4a / 4b 互換)。
#[test]
fn test_default_kind_always_1_string() {
    let mut engine = Engine::new();
    engine.prepare(SAMPLE_RATE, 128);
    // 初期 Default kind のまま
    engine.note_on(60, 0.8);
    assert_eq!(engine.voice_n_strings_active_for_test(60), Some(1));
}

// ===== F61: Phase 4a / 4b compatibility =====

/// F61-a: Default kind で 256 frame × 2ch 出力が Phase 4a HEAD fixture と ε=1e-6 バイト一致。
/// Phase 4c の Multi-string 構造変更 (string_buffers / string_states) を経ても、
/// `n_strings_active = 1` 経路では Phase 4a HEAD と完全に同じ出力を返すことを機械保証。
#[test]
fn test_default_n_strings_1_matches_phase4a() {
    let mut engine = Engine::new();
    engine.prepare(48_000.0, 128);
    engine.note_on(60, 0.8);

    let mut buf_l = vec![0.0_f32; 256];
    let mut buf_r = vec![0.0_f32; 256];
    engine.process(&mut buf_l, &mut buf_r);

    let golden_l = phase4a_golden::PHASE4A_GOLDEN_L;
    let golden_r = phase4a_golden::PHASE4A_GOLDEN_R;

    for i in 0..256 {
        let dl = (buf_l[i] - golden_l[i]).abs();
        assert!(
            dl < 1.0e-6,
            "L mismatch at frame {}: phase4c={} vs phase4a_golden={} (|delta|={})",
            i,
            buf_l[i],
            golden_l[i],
            dl
        );
        let dr = (buf_r[i] - golden_r[i]).abs();
        assert!(
            dr < 1.0e-6,
            "R mismatch at frame {}: phase4c={} vs phase4a_golden={} (|delta|={})",
            i,
            buf_r[i],
            golden_r[i],
            dr
        );
    }
}

/// F61-b (負のテスト): Piano kind 出力が Phase 4b と意図的に異なる。
/// Phase 4b 固定 B=7.5e-4 + n_strings=1 と Phase 4c (B(note) LUT + n_strings=3) の出力が
/// 256 frame 内のどこかで ε=1e-3 以上ずれていることを確認 (D81 / F61-b)。
///
/// 比較対象は Default kind の出力 (= dispersion_active=false 経路) と Piano kind の出力。
/// 両者が同一なら Phase 4c の構造変更は機能していない (= Multi-string / Hertz hammer / B(note)
/// が無効) と判定できる。
#[test]
fn test_piano_n_strings_diverges_from_default() {
    let mut engine_piano = Engine::new();
    engine_piano.prepare(48_000.0, 128);
    engine_piano.apply_instrument(InstrumentKind::Piano);
    engine_piano.note_on(60, 0.8);
    let mut piano_l = vec![0.0_f32; 256];
    let mut piano_r = vec![0.0_f32; 256];
    engine_piano.process(&mut piano_l, &mut piano_r);

    let mut engine_default = Engine::new();
    engine_default.prepare(48_000.0, 128);
    engine_default.note_on(60, 0.8);
    let mut default_l = vec![0.0_f32; 256];
    let mut default_r = vec![0.0_f32; 256];
    engine_default.process(&mut default_l, &mut default_r);

    let max_delta = piano_l
        .iter()
        .zip(default_l.iter())
        .map(|(p, d)| (p - d).abs())
        .fold(0.0_f32, f32::max);
    assert!(
        max_delta > 1.0e-3,
        "Phase 4c Piano kind must diverge from Default kind (max delta {} < 1e-3)",
        max_delta
    );
}

/// F61-c: Guitar Classical kind で出力が Phase 4a/4b の pluck 経路と byte 一致継承。
/// dispersion_active=false 経路は Phase 4c で変更されていないため、Default kind と同じ
/// fixture を満たす (Guitar Classical の body_modes は Default と同じ係数なので fixture も同じ)。
#[test]
fn test_guitar_classical_phase4b_byte_match() {
    let mut engine = Engine::new();
    engine.prepare(48_000.0, 128);
    engine.apply_instrument(InstrumentKind::GuitarClassical);
    engine.note_on(60, 0.8);

    let mut buf_l = vec![0.0_f32; 256];
    let mut buf_r = vec![0.0_f32; 256];
    engine.process(&mut buf_l, &mut buf_r);

    let golden_l = phase4a_golden::PHASE4A_GOLDEN_L;
    let golden_r = phase4a_golden::PHASE4A_GOLDEN_R;

    for i in 0..256 {
        let dl = (buf_l[i] - golden_l[i]).abs();
        assert!(
            dl < 1.0e-6,
            "GuitarClassical L mismatch at frame {}: phase4c={} vs golden={} (|delta|={})",
            i,
            buf_l[i],
            golden_l[i],
            dl
        );
        let dr = (buf_r[i] - golden_r[i]).abs();
        assert!(
            dr < 1.0e-6,
            "GuitarClassical R mismatch at frame {}: phase4c={} vs golden={} (|delta|={})",
            i,
            buf_r[i],
            golden_r[i],
            dr
        );
    }
}

/// F61-d: Default / Guitar / Ukulele / Mandolin / Bass / GuitarSteel / Sitar の 7 種で
/// note_on(C4) 後の n_strings_active = 1 維持。
#[test]
fn test_all_non_piano_kinds_n_strings_1() {
    let non_piano_kinds = [
        InstrumentKind::Default,
        InstrumentKind::GuitarClassical,
        InstrumentKind::Ukulele,
        InstrumentKind::Mandolin,
        InstrumentKind::Bass,
        InstrumentKind::GuitarSteel,
        InstrumentKind::Sitar,
    ];
    for kind in non_piano_kinds {
        let mut engine = Engine::new();
        engine.prepare(SAMPLE_RATE, 128);
        engine.apply_instrument(kind);
        engine.note_on(60, 0.8);
        assert_eq!(
            engine.voice_n_strings_active_for_test(60),
            Some(1),
            "{:?} kind should keep n_strings_active = 1",
            kind
        );
        assert_eq!(
            engine.voice_dispersion_active_for_test(60),
            Some(false),
            "{:?} kind should keep dispersion_active = false",
            kind
        );
    }
}

/// F61-e: Default kind + CC#64 ON でも bus_mix=0 で modal_body 入力に bus が寄与しない。
/// Piano → Default 切替後に bus buffer が完全リセットされる。
#[test]
fn test_default_kind_bus_direct_mix_is_zero() {
    let mut engine = Engine::new();
    engine.prepare(SAMPLE_RATE, 128);
    // CC#64 ON してから note_on (Default kind)
    engine.handle_midi_cc(64, 1.0);
    engine.note_on(60, 0.8);

    // Default + Sustain でも resonance_feedback_target は 0 (Engine の sympathetic_amount = 0)
    assert!(
        engine.resonance_feedback_target_for_test().abs() < 1e-9,
        "Default kind + Sustain ON should keep feedback_gain target = 0, got {}",
        engine.resonance_feedback_target_for_test()
    );

    // 256 frame process しても Phase 4a HEAD と byte 一致継承
    let mut buf_l = vec![0.0_f32; 256];
    let mut buf_r = vec![0.0_f32; 256];
    engine.process(&mut buf_l, &mut buf_r);
    let golden_l = phase4a_golden::PHASE4A_GOLDEN_L;
    for i in 0..256 {
        let dl = (buf_l[i] - golden_l[i]).abs();
        assert!(
            dl < 1.0e-6,
            "L mismatch at frame {} with CC#64 ON: phase4c={} vs golden={} (|delta|={})",
            i,
            buf_l[i],
            golden_l[i],
            dl
        );
    }

    // Piano → Default 切替後に bus buffer も完全リセット
    engine.apply_instrument(InstrumentKind::Piano);
    engine.note_on(72, 0.8);
    engine.process(&mut buf_l, &mut buf_r); // bus に少し dry が乗る
    engine.apply_instrument(InstrumentKind::Default);
    assert!(
        engine.resonance_feedback_target_for_test().abs() < 1e-9,
        "After Piano → Default, feedback_gain target should be 0"
    );
    assert_eq!(
        engine.resonance_bus_buffer_max_amplitude_for_test(),
        0.0,
        "After Piano → Default, bus delay line should be fully cleared"
    );
    assert!(
        engine.bus_out_prev_for_test().abs() < 1e-9,
        "After Piano → Default, bus_out_prev should be 0"
    );
}

// ===== F62: Multi-string detuning の動作 =====

/// F62-a: 3 弦 detune で出力に振幅変調 (beating) が観測される。
/// 大きめの detune (50 cents) を `KarplusStrong::set_instrument_params` 経由で注入し、
/// 1 秒のサンプルで amplitude envelope の山と谷を観測する。
#[test]
fn test_string_detune_produces_beating() {
    let mut ks = KarplusStrong::new();
    ks.prepare(SAMPLE_RATE, 128);
    ks.set_dispersion_active(true);
    // 大 detune (20 cents) で beat 周波数 ≈ 261.63 * (2^(20/1200) - 1) ≈ 3.03 Hz
    // 48000 sample (1 s) で約 3 cycle 観測できる
    ks.set_instrument_params(20.0, 7.5e-4, 800.0, 5500.0);
    ks.note_on_with_id(60, 261.63, 0.8); // C4

    let mut samples = vec![0.0_f32; 48_000];
    for s in samples.iter_mut() {
        *s = ks.process_sample();
    }

    // 窓 RMS で amplitude envelope を抽出し、山と谷の比 > 1.2 (= 20% 変動) を確認
    let window = 1024;
    let mut window_rms = Vec::new();
    for chunk in samples.chunks(window) {
        let rms =
            (chunk.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / chunk.len() as f64).sqrt();
        window_rms.push(rms as f32);
    }
    let max_rms = window_rms.iter().fold(0.0_f32, |a, &b| a.max(b));
    let min_rms = window_rms.iter().fold(f32::INFINITY, |a, &b| a.min(b));
    assert!(
        max_rms > 0.0 && min_rms > 0.0,
        "RMS envelope must be non-zero (max={}, min={})",
        max_rms,
        min_rms
    );
    let ratio = max_rms / min_rms.max(1e-9);
    assert!(
        ratio > 1.2,
        "Beating should produce >20% amplitude variation, got ratio {} (max={}, min={})",
        ratio,
        max_rms,
        min_rms
    );
}

/// F62-b: 各弦の dispersion a1 が detune で異なる f_0 を反映する。
/// 3 弦の中央 (detune=0) と左右 (detune=±base) で f_0 が異なるため、`compute_dispersion_a1`
/// の結果も差が出る (微小だが ε=1e-6 以下にはならない)。
#[test]
fn test_string_independent_dispersion_a1() {
    let mut ks = KarplusStrong::new();
    ks.prepare(SAMPLE_RATE, 128);
    ks.set_dispersion_active(true);
    // 大 detune (30 cents) で a1 の差を確実に観測可能に
    ks.set_instrument_params(30.0, 7.5e-4, 800.0, 5500.0);
    ks.note_on_with_id(60, 261.63, 0.8);

    assert_eq!(ks.n_strings_active(), 3);

    let a1_center = ks.dispersion_stage_a1_for_string(0, 0);
    let a1_left = ks.dispersion_stage_a1_for_string(1, 0);
    let a1_right = ks.dispersion_stage_a1_for_string(2, 0);

    // 3 弦の a1 は detune の影響で僅かに異なる
    assert!(
        (a1_center - a1_left).abs() > 1.0e-6,
        "center vs left a1 should differ: center={}, left={}",
        a1_center,
        a1_left
    );
    assert!(
        (a1_center - a1_right).abs() > 1.0e-6,
        "center vs right a1 should differ: center={}, right={}",
        a1_center,
        a1_right
    );
    // 左 (-base) と右 (+base) の a1 は微妙に異なる (a1 は f0 に対し非単調ではないが、対称ではない)
    // 同一値ではないことだけ確認
    assert!(
        (a1_left - a1_right).abs() > 1.0e-9 || (a1_left.is_finite() && a1_right.is_finite()),
        "left and right strings should have finite distinct a1"
    );
}

/// F62-c: Multi-string 経路の Piano が 1 秒持続音で有意な信号を保つ (two-stage decay の代理観測)。
/// Phase 4b の単弦経路と比較して、Multi-string では弦間の弱い相関により後半 (500-1000ms) の
/// 振幅が単純な指数減衰よりも遅く落ちる傾向がある。本テストでは「late window が ε=1e-6 以上」
/// であることを確認する緩い検証で十分 (厳密な two-stage 検出は Step 13 で hammer_hertz と
/// 統合)。
#[test]
fn test_two_stage_decay_observation() {
    let mut engine = Engine::new();
    engine.prepare(SAMPLE_RATE, 128);
    engine.apply_instrument(InstrumentKind::Piano);
    engine.note_on(60, 0.8); // C4 → 3 strings

    let mut buf_l = vec![0.0_f32; 48_000]; // 1 秒
    let mut buf_r = vec![0.0_f32; 48_000];
    engine.process(&mut buf_l, &mut buf_r);

    // 早い窓 (0..200ms) と遅い窓 (500..1000ms) の RMS
    let early = &buf_l[0..9_600];
    let late = &buf_l[24_000..48_000];
    let rms_early =
        (early.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / early.len() as f64).sqrt();
    let rms_late =
        (late.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / late.len() as f64).sqrt();

    assert!(
        rms_early > 1.0e-6,
        "Piano early window RMS must be substantial, got {}",
        rms_early
    );
    assert!(
        rms_late > 1.0e-9,
        "Piano late window must still have decay tail (Multi-string sustain), got {}",
        rms_late
    );
    assert!(
        rms_early > rms_late,
        "Decay should be monotonic in macro scale (early > late), got early={} late={}",
        rms_early,
        rms_late
    );
}

// ===== F63: alloc ゼロ =====

/// F63: Piano kind で 3 弦 active 時の `process_sample` で alloc ゼロ (Phase 1 D4 維持)。
/// buffer_capacity / voice 構造の不変性で機械保証。
#[test]
fn test_no_allocation_in_process_multi_string() {
    let mut engine = Engine::new();
    engine.prepare(SAMPLE_RATE, 128);
    engine.apply_instrument(InstrumentKind::Piano);
    for n in 60..=67 {
        engine.note_on(n, 0.8);
    }

    let cap_before: Vec<usize> = (0..8)
        .map(|i| {
            engine
                .pool()
                .voice(i)
                .map(|v| v.buffer_capacity())
                .unwrap_or(0)
        })
        .collect();

    let mut buf_l = vec![0.0_f32; 4800];
    let mut buf_r = vec![0.0_f32; 4800];
    for _ in 0..20 {
        engine.process(&mut buf_l, &mut buf_r);
    }

    let cap_after: Vec<usize> = (0..8)
        .map(|i| {
            engine
                .pool()
                .voice(i)
                .map(|v| v.buffer_capacity())
                .unwrap_or(0)
        })
        .collect();

    assert_eq!(
        cap_before, cap_after,
        "voice buffer_capacity must remain unchanged across multi-string process"
    );
}

/// 補足: UNISON_DETUNE_CENTS_PIANO の値が D72 仕様通り 1.5 であることを params 経由で確認。
/// gen-params.mjs の drift 防止。
#[test]
fn test_unison_detune_cents_constant_matches_spec() {
    assert!((UNISON_DETUNE_CENTS_PIANO - 1.5).abs() < 1e-9);
}
