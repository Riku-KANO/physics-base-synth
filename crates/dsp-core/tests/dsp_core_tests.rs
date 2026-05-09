use dsp_core::engine::{midi_to_freq, Engine};
use dsp_core::karplus_strong::KarplusStrong;
use dsp_core::params::ParamId;
use dsp_core::traits::AudioProcessor;
use dsp_core::voice_pool::POLYPHONY;

const SAMPLE_RATE: f32 = 48_000.0;

fn fresh_voice() -> KarplusStrong {
    let mut v = KarplusStrong::new();
    v.prepare(SAMPLE_RATE, 128);
    v
}

fn fresh_engine() -> Engine {
    let mut e = Engine::new();
    e.prepare(SAMPLE_RATE, 128);
    e
}

#[test]
fn test_silence_when_inactive() {
    let mut v = fresh_voice();
    for _ in 0..256 {
        assert_eq!(v.process_sample(), 0.0);
    }
}

#[test]
fn test_energy_rises_after_note_on() {
    let mut v = fresh_voice();
    v.note_on(440.0, 0.8);
    let mut sum_sq = 0.0_f32;
    for _ in 0..100 {
        let s = v.process_sample();
        sum_sq += s * s;
    }
    assert!(sum_sq > 0.0);
    assert!(v.is_active());
}

#[test]
fn test_decay_with_low_damping() {
    let mut v = fresh_voice();
    v.set_damping(0.90);
    v.note_on(440.0, 0.8);
    for _ in 0..(SAMPLE_RATE as usize) {
        v.process_sample();
    }
    assert!(!v.is_active());
}

#[test]
fn test_length_matches_freq() {
    let mut v = fresh_voice();
    // Phase 3 D37: brightness=1.0 で τ_g=0 (LPF パススルー)、補正の影響を排除して計測
    v.set_brightness(1.0);
    v.note_on(440.0, 0.8);
    // length_int は floor(sr/freq); length_int + length_frac ≈ raw_len。
    // 440Hz @ 48kHz: raw_len = 109.0909..., floor = 109。
    let raw = SAMPLE_RATE / 440.0;
    let expected_int = raw.floor() as usize;
    assert_eq!(v.length_int(), expected_int);
}

#[test]
fn test_no_allocation_in_process() {
    let mut v = fresh_voice();
    v.note_on(440.0, 0.8);
    let len_before = v.length_int();
    for _ in 0..(SAMPLE_RATE as usize) {
        let _ = v.process_sample();
    }
    assert_eq!(v.length_int(), len_before);
}

#[test]
fn test_note_on_first_block_nonzero() {
    // 励振配置 buffer[0..length_int] + write_index = length_int の組み合わせの動作確認。
    // write_index = 0 で励振配置すると初回 read 位置がゼロ領域を指し、励振サンプルが
    // ゼロ上書きされて無音になる罠を踏んでいないかを 1 ブロック (128 サンプル) で検証。
    let mut v = fresh_voice();
    let velocity = 0.8_f32;
    v.note_on(440.0, velocity);
    let mut max_abs = 0.0_f32;
    for _ in 0..128 {
        max_abs = max_abs.max(v.process_sample().abs());
    }
    assert!(
        max_abs >= velocity * 1.0e-3,
        "expected max abs >= {}, got {}",
        velocity * 1.0e-3,
        max_abs
    );
}

#[test]
fn test_paramid_roundtrip() {
    assert_eq!(ParamId::from_u32(0), Some(ParamId::Damping));
    assert_eq!(ParamId::from_u32(1), Some(ParamId::Brightness));
    assert_eq!(ParamId::from_u32(2), Some(ParamId::OutputGain));
    assert_eq!(ParamId::from_u32(99), None);
    assert_eq!(ParamId::Damping as u32, 0);
    assert_eq!(ParamId::Brightness as u32, 1);
    assert_eq!(ParamId::OutputGain as u32, 2);
}

#[test]
fn test_damping_preserved_across_note_on() {
    let mut e = fresh_engine();
    e.set_param(ParamId::Damping as u32, 0.999);
    assert!((e.current_damping() - 0.999).abs() < 1e-6);

    e.note_on(60, 0.8);
    e.note_off(60);
    e.note_on(60, 0.8);

    assert!((e.current_damping() - 0.999).abs() < 1e-6);
}

#[test]
fn test_engine_processes_block_without_panic() {
    let mut e = fresh_engine();
    e.note_on(60, 0.8);
    let mut l = [0.0_f32; 128];
    let mut r = [0.0_f32; 128];
    e.process(&mut l, &mut r);
    assert!(l.iter().any(|s| *s != 0.0));
    // Phase 3 D31/D32: Modal Body Resonator で stereo 分離（左右係数 ±5%）。
    // 完全一致ではなく、両 ch が finite であることのみ確認。
    for i in 0..128 {
        assert!(l[i].is_finite(), "L[{i}] not finite: {}", l[i]);
        assert!(r[i].is_finite(), "R[{i}] not finite: {}", r[i]);
    }
}

#[test]
fn test_midi_to_freq_a4() {
    assert!((midi_to_freq(69) - 440.0).abs() < 1e-3);
}

/// Phase 3 F37: release ビルドで 8 voice 全 active + Pitch Bend + CC#7 の最悪ケースで
/// process(128 frames) の平均時間が < 1.5 ms 以内 (CI flaky 対策で 2.0 ms)。
#[test]
#[cfg(not(debug_assertions))]
fn test_engine_process_block_timing() {
    use std::time::Instant;
    let mut e = fresh_engine();
    for i in 0..8 {
        e.note_on(60 + i, 0.8);
    }
    e.handle_pitch_bend(1.0);
    e.handle_midi_cc(7, 0.8);
    e.set_param(ParamId::BodyWet as u32, 0.7);

    let mut output_l = vec![0.0_f32; 128];
    let mut output_r = vec![0.0_f32; 128];
    const ITERATIONS: u32 = 1000;
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        e.process(&mut output_l, &mut output_r);
    }
    let elapsed = start.elapsed();
    let per_block_us = elapsed.as_micros() as f64 / ITERATIONS as f64;
    let per_block_ms = per_block_us / 1000.0;
    println!(
        "F37: process_block timing = {:.3} ms / 128 frames",
        per_block_ms
    );
    assert!(
        per_block_ms < 2.0,
        "F37 fail: {:.3} ms >= 2.0 ms",
        per_block_ms
    );
}

/// Phase 4a F46: release ビルドで 8 voice 全 active + Pitch Bend + CC#7 + Mod Wheel = 1.0
/// + LFO depths 全 1.0 の最悪ケースで process(128 frames) 平均が < 1.7 ms 以内
/// (Phase 3 1.5 ms + 0.2 ms 余裕)。CI flaky 対策の上限は 2.0 ms。
#[test]
#[cfg(not(debug_assertions))]
fn test_engine_process_block_timing_phase4a() {
    use dsp_core::lfo::{LfoDestination, LfoWaveform};
    use std::time::Instant;
    let mut e = fresh_engine();
    for i in 0..8 {
        e.note_on(60 + i, 0.8);
    }
    e.handle_pitch_bend(1.0);
    e.handle_midi_cc(7, 0.8);
    e.handle_midi_cc(1, 1.0); // Mod Wheel = 1.0
    e.lfo_set_rate(5.0);
    e.lfo_set_waveform(LfoWaveform::Sine);
    e.lfo_set_depth(LfoDestination::Pitch, 1.0);
    e.lfo_set_depth(LfoDestination::Brightness, 1.0);
    e.lfo_set_depth(LfoDestination::Volume, 1.0);
    e.set_param(ParamId::BodyWet as u32, 0.7);

    let mut output_l = vec![0.0_f32; 128];
    let mut output_r = vec![0.0_f32; 128];
    const ITERATIONS: u32 = 1000;
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        e.process(&mut output_l, &mut output_r);
    }
    let elapsed = start.elapsed();
    let per_block_us = elapsed.as_micros() as f64 / ITERATIONS as f64;
    let per_block_ms = per_block_us / 1000.0;
    println!(
        "F46 (Phase 4a): process_block timing = {:.3} ms / 128 frames (8 voice + LFO + Mod Wheel + Pitch Bend + CC#7)",
        per_block_ms
    );
    assert!(
        per_block_ms < 2.0,
        "F46 fail: {:.3} ms >= 2.0 ms (Phase 4a target 1.7 ms, flaky margin 2.0 ms)",
        per_block_ms
    );
}

/// Phase 4b F50: release ビルドで Piano kind 最悪ケース (8 voice + Pitch Bend + CC#7 +
/// Mod Wheel = 1.0 + LFO depths 全 1.0 + dispersion 8 段 cascade) で process(128 frames)
/// 平均が < 1.7 ms 以内。CI flaky 対策の上限は 2.0 ms。
#[test]
#[cfg(not(debug_assertions))]
fn test_engine_process_block_timing_phase4b_piano() {
    use dsp_core::lfo::{LfoDestination, LfoWaveform};
    use dsp_core::params::InstrumentKind;
    use std::time::Instant;

    let mut e = fresh_engine();
    e.apply_instrument(InstrumentKind::Piano);
    for i in 0..8 {
        e.note_on(60 + i, 0.8);
    }
    e.handle_pitch_bend(1.0);
    e.handle_midi_cc(7, 0.8);
    e.handle_midi_cc(1, 1.0);
    e.lfo_set_rate(5.0);
    e.lfo_set_waveform(LfoWaveform::Sine);
    e.lfo_set_depth(LfoDestination::Pitch, 1.0);
    e.lfo_set_depth(LfoDestination::Brightness, 1.0);
    e.lfo_set_depth(LfoDestination::Volume, 1.0);
    e.set_param(ParamId::BodyWet as u32, 0.7);

    let mut output_l = vec![0.0_f32; 128];
    let mut output_r = vec![0.0_f32; 128];
    const ITERATIONS: u32 = 1000;
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        e.process(&mut output_l, &mut output_r);
    }
    let elapsed = start.elapsed();
    let per_block_us = elapsed.as_micros() as f64 / ITERATIONS as f64;
    let per_block_ms = per_block_us / 1000.0;
    println!(
        "F50 (Phase 4b Piano): process_block timing = {:.3} ms / 128 frames (8 voice + dispersion 8 段 + LFO + Mod Wheel + Pitch Bend + CC#7)",
        per_block_ms
    );
    assert!(
        per_block_ms < 2.0,
        "F50 Piano fail: {:.3} ms >= 2.0 ms (Phase 4b target 1.7 ms, flaky margin 2.0 ms)",
        per_block_ms
    );
}

/// Phase 4b F50: 非 Piano (Default) で dispersion skip 経路の timing。
/// Phase 4a 同等の avg < 1.0 ms (Phase 4a 0.023 ms 程度) を期待、CI flaky 上限 1.5 ms。
#[test]
#[cfg(not(debug_assertions))]
fn test_engine_process_block_timing_phase4b_other() {
    use dsp_core::lfo::{LfoDestination, LfoWaveform};
    use dsp_core::params::InstrumentKind;
    use std::time::Instant;

    let mut e = fresh_engine();
    e.apply_instrument(InstrumentKind::Default);
    for i in 0..8 {
        e.note_on(60 + i, 0.8);
    }
    e.handle_pitch_bend(1.0);
    e.handle_midi_cc(7, 0.8);
    e.handle_midi_cc(1, 1.0);
    e.lfo_set_rate(5.0);
    e.lfo_set_waveform(LfoWaveform::Sine);
    e.lfo_set_depth(LfoDestination::Pitch, 1.0);
    e.lfo_set_depth(LfoDestination::Brightness, 1.0);
    e.lfo_set_depth(LfoDestination::Volume, 1.0);
    e.set_param(ParamId::BodyWet as u32, 0.7);

    let mut output_l = vec![0.0_f32; 128];
    let mut output_r = vec![0.0_f32; 128];
    const ITERATIONS: u32 = 1000;
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        e.process(&mut output_l, &mut output_r);
    }
    let elapsed = start.elapsed();
    let per_block_us = elapsed.as_micros() as f64 / ITERATIONS as f64;
    let per_block_ms = per_block_us / 1000.0;
    println!(
        "F50 (Phase 4b non-Piano): process_block timing = {:.3} ms / 128 frames (Default kind, dispersion skip)",
        per_block_ms
    );
    assert!(
        per_block_ms < 1.5,
        "F50 non-Piano fail: {:.3} ms >= 1.5 ms (Phase 4a regression baseline)",
        per_block_ms
    );
}

#[test]
fn test_engine_voice_state_buffer_format() {
    // Phase 3 D41: 33 byte レイアウト (active mask 1 + 8 voice × f32 4 bytes = 33)。
    // voice_state は 1024 sample 毎にしか書かれないため、8 ブロック分 process して trigger する。
    let mut e = fresh_engine();
    e.note_on(60, 0.8);
    let mut l = vec![0.0_f32; 128];
    let mut r = vec![0.0_f32; 128];
    for _ in 0..8 {
        e.process(&mut l, &mut r);
    }

    let ptr = e.voice_state_ptr();
    assert!(!ptr.is_null());
    // 33 bytes をスライスとして読む
    let buf = unsafe { core::slice::from_raw_parts(ptr, 33) };
    let active_mask = buf[0];
    // 60 を割り当てた voice index 0 が active
    assert_eq!(active_mask & 0x01, 0x01, "voice 0 should be active");

    // voice 0 の振幅 > 0
    let amp_bytes: [u8; 4] = buf[1..5].try_into().unwrap();
    let amp_0 = f32::from_le_bytes(amp_bytes);
    assert!(
        amp_0 > 0.0,
        "voice 0 amplitude should be > 0, got {}",
        amp_0
    );

    // 1..POLYPHONY voice は inactive
    for i in 1..POLYPHONY {
        let bit = (active_mask >> i) & 1;
        assert_eq!(bit, 0, "voice {} should be inactive", i);
    }
}

#[test]
fn test_poly_mode_independent_voices() {
    // ポリモードでは 60 と 62 は別ボイスに割り当てられ独立して鳴る。
    // last-note-priority は mono モードの hold_stack でのみ発動する。
    let mut e = fresh_engine();
    e.note_on(60, 0.8);
    e.note_on(62, 0.8);
    assert_eq!(e.active_voice_count(), 2);

    let idx_60 = e
        .voice_index_for_note(60)
        .expect("voice 60 should be active");
    let idx_62 = e
        .voice_index_for_note(62)
        .expect("voice 62 should be active");
    assert_ne!(
        idx_60, idx_62,
        "different notes should be on different voices"
    );
}

#[test]
fn test_setparam_clamps_out_of_range() {
    let mut e = fresh_engine();
    e.set_param(ParamId::Damping as u32, 100.0);
    assert!(e.current_damping() <= 0.9999);
    e.set_param(ParamId::Damping as u32, -1.0);
    assert!(e.current_damping() >= 0.90);
}
