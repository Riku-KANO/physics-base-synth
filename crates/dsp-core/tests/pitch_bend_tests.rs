//! Pitch Bend テスト (Phase 3 F32、03 章 §Pitch Bend)

use dsp_core::engine::{midi_to_freq, Engine};
use dsp_core::karplus_strong::KarplusStrong;
use dsp_core::traits::AudioProcessor;

const SAMPLE_RATE: f32 = 48_000.0;

#[test]
fn test_pitch_bend_smooth_transition() {
    // bend +2 → 0 → -2 で finite かつ過大振幅にならないことを確認 (5ms tau で滑らか)
    let mut e = Engine::new();
    e.prepare(SAMPLE_RATE, 128);
    e.set_param(0, 0.999); // damping
    e.note_on(69, 0.8);
    e.handle_pitch_bend(2.0);

    let mut l = vec![0.0_f32; 4800];
    let mut r = vec![0.0_f32; 4800];
    e.process(&mut l, &mut r);

    e.handle_pitch_bend(-2.0);
    let mut l2 = vec![0.0_f32; 4800];
    let mut r2 = vec![0.0_f32; 4800];
    e.process(&mut l2, &mut r2);

    for &s in l.iter().chain(r.iter()).chain(l2.iter()).chain(r2.iter()) {
        assert!(s.is_finite(), "non-finite during pitch bend: {}", s);
        assert!(s.abs() < 1.5, "amplitude blow-up during pitch bend: {}", s);
    }
}

#[test]
fn test_pitch_bend_clamps_to_range() {
    // ±10 半音を渡しても ±2 に clamp、panic なし
    let mut v = KarplusStrong::new();
    v.prepare(SAMPLE_RATE, 128);
    v.note_on(440.0, 0.8);

    v.set_pitch_bend(10.0);
    for _ in 0..1000 {
        let s = v.process_sample();
        assert!(s.is_finite());
    }

    v.set_pitch_bend(-10.0);
    for _ in 0..1000 {
        let s = v.process_sample();
        assert!(s.is_finite());
    }
}

#[test]
fn test_pitch_bend_ring_buffer_invariant() {
    // pitch bend で length_int が動的変化、buffer.len() (容量) は不変、ring buffer 整合
    let mut v = KarplusStrong::new();
    v.prepare(SAMPLE_RATE, 128);
    let baseline = v.buffer_capacity();
    v.note_on(440.0, 0.8);
    let len_init = v.length_int();

    v.set_pitch_bend(1.5);
    // 5ms tau で transition、十分時間をかける (4800 sample = 100ms)
    for _ in 0..4800 {
        let s = v.process_sample();
        assert!(s.is_finite());
    }
    let len_after = v.length_int();
    assert_ne!(len_init, len_after, "pitch bend should change length_int");
    assert_eq!(
        v.buffer_capacity(),
        baseline,
        "ring buffer capacity must be unchanged during pitch bend"
    );
}

#[test]
fn test_pitch_bend_zero_returns_to_baseline() {
    // bend → 0 で base_length に近い length へ戻る
    let mut v = KarplusStrong::new();
    v.prepare(SAMPLE_RATE, 128);
    v.set_brightness(1.0); // τ_g=0、base_length = raw_len そのまま
    v.note_on(midi_to_freq(69), 0.8);
    let baseline_length = v.length_int();

    v.set_pitch_bend(2.0);
    for _ in 0..4800 {
        v.process_sample();
    }
    let bent_length = v.length_int();
    assert!(bent_length < baseline_length); // pitch up = length down

    v.set_pitch_bend(0.0);
    for _ in 0..4800 {
        v.process_sample();
    }
    let restored_length = v.length_int();
    assert!(
        (restored_length as i32 - baseline_length as i32).abs() <= 1,
        "restored length {} should be close to baseline {}",
        restored_length,
        baseline_length
    );
}
