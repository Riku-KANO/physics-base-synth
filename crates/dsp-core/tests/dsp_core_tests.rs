use dsp_core::engine::{midi_to_freq, Engine};
use dsp_core::karplus_strong::KarplusStrong;
use dsp_core::params::ParamId;
use dsp_core::traits::AudioProcessor;

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
    v.note_on(440.0, 0.8);
    // Phase 2: length_int は floor(sr/freq), length_int + length_frac ≈ raw_len。
    // 440Hz @ 48kHz は raw_len = 109.0909...、floor = 109。
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
    // Phase 2 (Step 6 High 修正): note_on 直後 1 ブロックの出力絶対値最大が velocity * 1e-3 以上。
    // 励振配置 buffer[0..length_int] + write_index = length_int の組み合わせが
    // 機能していることを確認する（write_index = 0 + 励振配置だと初回 read 位置が
    // ゼロ領域を指して励振サンプルがゼロ上書きされ無音になる罠への防壁）。
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
    for i in 0..128 {
        assert_eq!(l[i], r[i]);
    }
}

#[test]
fn test_midi_to_freq_a4() {
    assert!((midi_to_freq(69) - 440.0).abs() < 1e-3);
}

#[test]
fn test_last_note_priority_simple() {
    let mut e = fresh_engine();
    e.note_on(60, 0.8);
    e.note_on(62, 0.8);
    assert_eq!(e.current_note(), Some(62));

    e.note_off(60);
    assert_eq!(e.current_note(), Some(62));

    e.note_off(62);
    assert_eq!(e.current_note(), None);
}

#[test]
fn test_setparam_clamps_out_of_range() {
    let mut e = fresh_engine();
    e.set_param(ParamId::Damping as u32, 100.0);
    assert!(e.current_damping() <= 0.9999);
    e.set_param(ParamId::Damping as u32, -1.0);
    assert!(e.current_damping() >= 0.90);
}
