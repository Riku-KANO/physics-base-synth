//! LFO destinations 統合テスト (Phase 4a F40-c/d/e/f/g)
//!
//! - test_mod_wheel_zero_disables_lfo: Phase 3 互換性の最重要テスト
//!   (LFO depth が 1.0 でも Mod Wheel = 0 で LFO 効果ゼロ)
//! - test_mod_wheel_one_full_lfo: Mod Wheel=1.0 で LFO 効果が出力に反映
//! - test_lfo_pitch_destination_modulates_voice_length
//! - test_lfo_brightness_destination_modulates_filter
//! - test_lfo_volume_destination_modulates_output
//! - test_lfo_no_alloc_in_engine_process

use dsp_core::engine::Engine;
use dsp_core::lfo::{LfoDestination, LfoWaveform};
use dsp_core::traits::AudioProcessor;

const SAMPLE_RATE: f32 = 48_000.0;
const BLOCK_SIZE: usize = 128;

fn fresh_engine_with_lfo(rate_hz: f32, waveform: LfoWaveform) -> Engine {
    let mut e = Engine::new();
    e.prepare(SAMPLE_RATE, BLOCK_SIZE);
    e.lfo_set_rate(rate_hz);
    e.lfo_set_waveform(waveform);
    e
}

fn rms(samples: &[f32]) -> f32 {
    let sq: f64 = samples.iter().map(|x| (*x as f64).powi(2)).sum();
    (sq / samples.len() as f64).sqrt() as f32
}

#[test]
fn test_mod_wheel_zero_disables_lfo() {
    // Phase 3 互換性の最重要保証: LFO depth が全 destination で 1.0 でも、
    // Mod Wheel = 0 (デフォルト) なら LFO 効果はゼロで、Phase 3 互換動作と完全一致する。
    let mut engine = fresh_engine_with_lfo(5.0, LfoWaveform::Sine);
    engine.lfo_set_depth(LfoDestination::Pitch, 1.0);
    engine.lfo_set_depth(LfoDestination::Brightness, 1.0);
    engine.lfo_set_depth(LfoDestination::Volume, 1.0);
    // Mod Wheel は 0 (デフォルト) のまま

    engine.note_on(60, 0.8);
    let mut buf_l = vec![0.0_f32; 256];
    let mut buf_r = vec![0.0_f32; 256];
    engine.process(&mut buf_l, &mut buf_r);

    // 比較対象: 同条件で LFO depth = 0 の engine (Phase 3 互換)
    let mut engine_ref = fresh_engine_with_lfo(5.0, LfoWaveform::Sine);
    engine_ref.note_on(60, 0.8);
    let mut buf_l2 = vec![0.0_f32; 256];
    let mut buf_r2 = vec![0.0_f32; 256];
    engine_ref.process(&mut buf_l2, &mut buf_r2);

    for i in 0..256 {
        assert!(
            (buf_l[i] - buf_l2[i]).abs() < 1e-6,
            "L: LFO depth=1.0 + Mod Wheel=0 should match no-LFO at frame {i} (got {} vs {})",
            buf_l[i],
            buf_l2[i]
        );
        assert!(
            (buf_r[i] - buf_r2[i]).abs() < 1e-6,
            "R: LFO depth=1.0 + Mod Wheel=0 should match no-LFO at frame {i} (got {} vs {})",
            buf_r[i],
            buf_r2[i]
        );
    }
}

#[test]
fn test_mod_wheel_one_full_lfo() {
    // Mod Wheel=1.0 + LFO Volume depth=1.0 で出力に LFO 効果が現れる
    // (depth=0 の参照と RMS / 振幅エンベロープが異なる)。
    let mut engine_with = fresh_engine_with_lfo(5.0, LfoWaveform::Sine);
    engine_with.lfo_set_depth(LfoDestination::Volume, 1.0);
    engine_with.handle_midi_cc(1, 1.0); // CC#1 = 1.0 → Mod Wheel = 1.0
    engine_with.note_on(60, 0.8);
    let mut buf_l = vec![0.0_f32; 9600];
    let mut buf_r = vec![0.0_f32; 9600];
    engine_with.process(&mut buf_l, &mut buf_r);

    let mut engine_off = fresh_engine_with_lfo(5.0, LfoWaveform::Sine);
    // depth = 0 (デフォルト) のまま
    engine_off.note_on(60, 0.8);
    let mut buf_l2 = vec![0.0_f32; 9600];
    let mut buf_r2 = vec![0.0_f32; 9600];
    engine_off.process(&mut buf_l2, &mut buf_r2);

    // 出力波形が異なる
    let mut differs = false;
    for i in 0..buf_l.len() {
        if (buf_l[i] - buf_l2[i]).abs() > 1e-4 {
            differs = true;
            break;
        }
    }
    assert!(
        differs,
        "LFO Volume depth=1.0 + Mod Wheel=1.0 should differ from depth=0 output"
    );
}

#[test]
fn test_lfo_pitch_destination_modulates_voice_length() {
    // LFO Pitch depth=1.0 + Mod Wheel=1.0 で voice の length_int が周期変動する。
    let mut engine = fresh_engine_with_lfo(5.0, LfoWaveform::Sine);
    engine.lfo_set_depth(LfoDestination::Pitch, 1.0);
    engine.handle_midi_cc(1, 1.0);
    engine.note_on(60, 0.8);
    let voice_idx = engine
        .voice_index_for_note(60)
        .expect("voice 60 must be active");

    // 1 LFO 周期 (5Hz = 200ms = 9600 sample) を 50 段階で観測 → length_int の min/max を取る
    let chunk = 192; // ≈ 4ms
    let steps = 50;
    let mut lengths = Vec::with_capacity(steps);
    let mut buf_l = vec![0.0_f32; chunk];
    let mut buf_r = vec![0.0_f32; chunk];
    for _ in 0..steps {
        engine.process(&mut buf_l, &mut buf_r);
        if let Some(len) = engine.pool().voice_length_int(voice_idx) {
            lengths.push(len);
        }
    }
    let min_len = *lengths.iter().min().unwrap();
    let max_len = *lengths.iter().max().unwrap();
    assert!(
        max_len > min_len,
        "LFO Pitch should modulate voice length: min={min_len} max={max_len} samples={lengths:?}"
    );
}

#[test]
fn test_lfo_brightness_destination_modulates_filter() {
    // LFO Brightness depth=1.0 で出力波形が depth=0 と異なる (filter 出力が変調)
    let mut engine_with = fresh_engine_with_lfo(2.0, LfoWaveform::Sine);
    engine_with.lfo_set_depth(LfoDestination::Brightness, 1.0);
    engine_with.handle_midi_cc(1, 1.0);
    engine_with.note_on(60, 0.8);
    let mut buf_l = vec![0.0_f32; 24000];
    let mut buf_r = vec![0.0_f32; 24000];
    engine_with.process(&mut buf_l, &mut buf_r);

    let mut engine_off = fresh_engine_with_lfo(2.0, LfoWaveform::Sine);
    engine_off.note_on(60, 0.8);
    let mut buf_l2 = vec![0.0_f32; 24000];
    let mut buf_r2 = vec![0.0_f32; 24000];
    engine_off.process(&mut buf_l2, &mut buf_r2);

    let mut total_abs_diff = 0.0_f64;
    for i in 0..buf_l.len() {
        total_abs_diff += ((buf_l[i] - buf_l2[i]) as f64).abs();
    }
    let avg_diff = total_abs_diff / buf_l.len() as f64;
    assert!(
        avg_diff > 1e-4,
        "LFO Brightness should produce noticeable filter modulation: avg_diff={avg_diff:.6}"
    );
}

#[test]
fn test_lfo_volume_destination_modulates_output() {
    // LFO Volume depth=1.0 で output の RMS がチャンクごとに変動する。
    let mut engine = fresh_engine_with_lfo(5.0, LfoWaveform::Sine);
    engine.lfo_set_depth(LfoDestination::Volume, 1.0);
    engine.handle_midi_cc(1, 1.0);
    engine.note_on(60, 0.8);

    // 5Hz LFO 1 周期 = 9600 sample。4 つのチャンク (位相 0, 0.25, 0.5, 0.75) で RMS 計測。
    let chunk_len = 2400;
    let mut chunk_rms = Vec::with_capacity(4);
    for _ in 0..4 {
        let mut buf_l = vec![0.0_f32; chunk_len];
        let mut buf_r = vec![0.0_f32; chunk_len];
        engine.process(&mut buf_l, &mut buf_r);
        chunk_rms.push(rms(&buf_l));
    }
    let max_rms = chunk_rms.iter().cloned().fold(0.0_f32, f32::max);
    let min_rms = chunk_rms.iter().cloned().fold(f32::INFINITY, f32::min);
    assert!(
        max_rms > min_rms * 1.1,
        "LFO Volume should produce time-varying RMS across phases: rms={chunk_rms:?}"
    );
}

#[test]
fn test_lfo_no_alloc_in_engine_process() {
    // 8 voice + LFO active + Pitch Bend + CC#7 + Mod Wheel で voice buffer capacity 不変
    let mut engine = fresh_engine_with_lfo(5.0, LfoWaveform::Sine);
    engine.lfo_set_depth(LfoDestination::Pitch, 0.5);
    engine.lfo_set_depth(LfoDestination::Brightness, 0.5);
    engine.lfo_set_depth(LfoDestination::Volume, 0.5);
    engine.handle_midi_cc(1, 1.0);

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
    for _ in 0..10 {
        engine.handle_pitch_bend(0.5);
        engine.handle_midi_cc(7, 0.7);
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
        "voice buffer capacity must not change"
    );
}
