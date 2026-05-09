//! Phase 4b D60 / D61 / D67: KarplusStrong に統合された dispersion 経路のテスト。
//!
//! Step 6 では `note_on` 時の a1 算出と dispersion_active flag の挙動を検証する。
//! `process_sample` の cascade 経由検証 (D67 Phase 4a 互換性核心テスト) は Step 7 で追加、
//! hammer 経路 (D61) のテストは Step 8 で追加する。

use dsp_core::karplus_strong::KarplusStrong;
use dsp_core::params::INHARMONICITY_B_PIANO;
use dsp_core::{compute_dispersion_a1, DISPERSION_STAGES};

const SR: f32 = 48_000.0;

#[test]
fn test_dispersion_active_default_false() {
    let v = KarplusStrong::new();
    assert!(
        !v.dispersion_active(),
        "dispersion_active should default to false (Phase 4a 互換)"
    );
}

#[test]
fn test_set_dispersion_active_toggles_flag() {
    let mut v = KarplusStrong::new();
    v.prepare(SR, 128);
    v.set_dispersion_active(true);
    assert!(v.dispersion_active());
    v.set_dispersion_active(false);
    assert!(!v.dispersion_active());
}

#[test]
fn test_dispersion_a1_set_in_note_on() {
    let mut v = KarplusStrong::new();
    v.prepare(SR, 128);
    v.set_dispersion_active(true);
    let freq = 440.0_f32;
    v.note_on(freq, 0.8);

    let (expected_a1, _gd) =
        compute_dispersion_a1(DISPERSION_STAGES as u32, INHARMONICITY_B_PIANO, freq, SR);
    for idx in 0..DISPERSION_STAGES {
        let a1 = v.dispersion_stage_a1(idx);
        assert!(
            (a1 - expected_a1).abs() < 1.0e-7,
            "stage[{}].a1 should equal compute_dispersion_a1 result, got {} expected {}",
            idx,
            a1,
            expected_a1
        );
    }
}

#[test]
fn test_dispersion_a1_zero_when_inactive() {
    // dispersion_active = false で note_on しても a1 は 0 のまま (compute_dispersion_a1 を呼ばない)
    let mut v = KarplusStrong::new();
    v.prepare(SR, 128);
    assert!(!v.dispersion_active());
    v.note_on(440.0, 0.8);
    for idx in 0..DISPERSION_STAGES {
        let a1 = v.dispersion_stage_a1(idx);
        assert_eq!(a1, 0.0, "stage[{}].a1 should be 0 when inactive", idx);
    }
}

#[test]
fn test_set_dispersion_active_false_resets_stages() {
    let mut v = KarplusStrong::new();
    v.prepare(SR, 128);
    v.set_dispersion_active(true);
    v.note_on(440.0, 0.8);
    // a1 は note_on 後に値を持つ
    assert!(v.dispersion_stage_a1(0).abs() > 0.0);

    v.set_dispersion_active(false);
    // false 切替で z 状態 reset (a1 自体は次回 note_on で上書きされる前提のため残るが、
    // z1_in/out は 0 になっているべき)。a1 が変わらないことだけ確認する。
    let a1_after_off = v.dispersion_stage_a1(0);
    assert!(
        a1_after_off.abs() > 0.0,
        "a1 stays from previous note_on (will be overwritten on next active note_on)"
    );
}

#[test]
fn test_dispersion_compensation_shortens_length_int() {
    // Piano kind では `note_on` 時の adjusted_length が brightness 補正に加えて
    // dispersion 群遅延補正で更に短くなる。同じ周波数で dispersion_active の有無を比較し、
    // active のほうが length_int が小さい (もしくは同じ) ことを確認する。
    let mut v_off = KarplusStrong::new();
    v_off.prepare(SR, 128);
    v_off.set_brightness(1.0); // brightness LPF 群遅延 0 にして dispersion 補正だけ見る
    v_off.note_on(440.0, 0.8);
    let len_off = v_off.length_int();

    let mut v_on = KarplusStrong::new();
    v_on.prepare(SR, 128);
    v_on.set_brightness(1.0);
    v_on.set_dispersion_active(true);
    v_on.note_on(440.0, 0.8);
    let len_on = v_on.length_int();

    assert!(
        len_on < len_off,
        "dispersion 群遅延補正で length_int は dispersion_active=true の方が短いはず: off={}, on={}",
        len_off,
        len_on
    );
}
