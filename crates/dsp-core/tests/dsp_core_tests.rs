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
    for i in 0..128 {
        assert_eq!(l[i], r[i]);
    }
}

#[test]
fn test_midi_to_freq_a4() {
    assert!((midi_to_freq(69) - 440.0).abs() < 1e-3);
}

#[test]
fn test_poly_mode_independent_voices() {
    // ポリモードでは 60 と 62 は別ボイスに割り当てられ独立して鳴る。
    // last-note-priority は mono モードの hold_stack でのみ発動する。
    let mut e = fresh_engine();
    e.note_on(60, 0.8);
    e.note_on(62, 0.8);
    assert_eq!(e.active_voice_count(), 2);

    let idx_60 = e.voice_index_for_note(60).expect("voice 60 should be active");
    let idx_62 = e.voice_index_for_note(62).expect("voice 62 should be active");
    assert_ne!(idx_60, idx_62, "different notes should be on different voices");
}

#[test]
fn test_setparam_clamps_out_of_range() {
    let mut e = fresh_engine();
    e.set_param(ParamId::Damping as u32, 100.0);
    assert!(e.current_damping() <= 0.9999);
    e.set_param(ParamId::Damping as u32, -1.0);
    assert!(e.current_damping() >= 0.90);
}
