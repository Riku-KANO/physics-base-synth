//! Phase 4b D57 / D58 / D59: Stretching all-pass cascade の単体テスト。
//!
//! Faust `piano_dispersion_filter` の Rust 移植 (`compute_dispersion_a1`) と
//! `DispersionStage::process` の挙動を検証する。

use dsp_core::{compute_dispersion_a1, DispersionStage, DISPERSION_STAGES};

const SR: f32 = 48000.0;
const PIANO_B: f32 = 7.5e-4;

#[test]
fn test_dispersion_a1_in_safe_range() {
    let (a1, _gd) = compute_dispersion_a1(DISPERSION_STAGES as u32, PIANO_B, 440.0, SR);
    assert!(a1.is_finite(), "a1 must be finite, got {}", a1);
    assert!(a1.abs() < 1.0, "a1 must be inside unit circle, got {}", a1);
}

#[test]
fn test_dispersion_a1_increases_with_b() {
    // B が小さい → a1 ≈ 0 (passthrough)、B が大きい → |a1| 増加
    let (a1_small, _) = compute_dispersion_a1(8, 1.0e-6, 440.0, SR);
    let (a1_mid, _) = compute_dispersion_a1(8, 1.0e-3, 440.0, SR);
    let (a1_large, _) = compute_dispersion_a1(8, 1.0e-2, 440.0, SR);
    assert!(
        a1_small.abs() < a1_mid.abs(),
        "|a1| should increase with B: small={}, mid={}",
        a1_small.abs(),
        a1_mid.abs()
    );
    assert!(
        a1_mid.abs() < a1_large.abs(),
        "|a1| should increase with B: mid={}, large={}",
        a1_mid.abs(),
        a1_large.abs()
    );
}

#[test]
fn test_dispersion_b_zero_limit() {
    // Faust closed-form は内部で `b.max(1.0e-6)` のフロアを掛けているため
    // 「B → 0+ で a1 → 0」の漸近極限には実数値で到達しないが、Piano B より
    // 充分小さい B では |a1| が単調に小さくなることを確認する (well-behaved)。
    let (a1_tiny, _) = compute_dispersion_a1(8, 1.0e-6, 440.0, SR);
    let (a1_piano, _) = compute_dispersion_a1(8, PIANO_B, 440.0, SR);
    assert!(
        a1_tiny.is_finite(),
        "a1 must be finite at tiny B, got {}",
        a1_tiny
    );
    assert!(
        a1_tiny.abs() < 1.0,
        "a1 must stay inside unit circle, got {}",
        a1_tiny
    );
    assert!(
        a1_tiny.abs() < a1_piano.abs(),
        "|a1| at tiny B should be smaller than at Piano B, tiny={}, piano={}",
        a1_tiny.abs(),
        a1_piano.abs()
    );
}

#[test]
fn test_dispersion_a1_keyboard_dependence() {
    // 同じ B でも note 位置 (Ikey(f0)) で a1 が異なることを確認
    let (a1_low, _) = compute_dispersion_a1(8, PIANO_B, 27.5, SR); // A0
    let (a1_high, _) = compute_dispersion_a1(8, PIANO_B, 4186.0, SR); // C8
    assert!(
        (a1_low - a1_high).abs() > 1.0e-3,
        "a1 should differ between A0 and C8 (Ikey effect), got low={}, high={}",
        a1_low,
        a1_high
    );
}

#[test]
fn test_dispersion_stage_reset() {
    let mut stage = DispersionStage::new();
    stage.a1 = 0.3;
    // 適当な信号を流して内部状態を更新
    for _ in 0..10 {
        stage.process(0.5);
    }
    assert!(
        stage.z1_in.abs() > 0.0 || stage.z1_out.abs() > 0.0,
        "internal state should be non-zero after processing"
    );
    stage.reset();
    assert_eq!(stage.z1_in, 0.0);
    assert_eq!(stage.z1_out, 0.0);
    // a1 は reset で消えない (note_on 時に上書きされるため)
    assert_eq!(stage.a1, 0.3);
}

#[test]
fn test_dispersion_stage_passthrough_when_a1_zero() {
    // a1 = 0.0 で y = z1_in (1 サンプル遅延 passthrough)
    let mut stage = DispersionStage::new();
    stage.a1 = 0.0;
    let y0 = stage.process(1.0);
    assert!(y0.abs() < 1.0e-7, "first sample y0 = z1_in (init 0)");
    let y1 = stage.process(0.0);
    assert!(
        (y1 - 1.0).abs() < 1.0e-7,
        "second sample y1 = previous x = 1.0"
    );
}

#[test]
fn test_dispersion_cascade_8_stages_stable() {
    // M=8 段カスケードで 1024 サンプル走らせて発散しないこと
    let (a1, _) = compute_dispersion_a1(DISPERSION_STAGES as u32, PIANO_B, 440.0, SR);
    let mut stages = [DispersionStage::new(); DISPERSION_STAGES];
    for s in stages.iter_mut() {
        s.a1 = a1;
    }
    // インパルス入力
    let mut max_abs = 0.0_f32;
    for n in 0..1024 {
        let x = if n == 0 { 1.0 } else { 0.0 };
        let mut y = x;
        for s in stages.iter_mut() {
            y = s.process(y);
        }
        assert!(y.is_finite(), "output must be finite at n={}, got {}", n, y);
        if y.abs() > max_abs {
            max_abs = y.abs();
        }
    }
    assert!(
        max_abs < 100.0,
        "cascade output should be bounded, max_abs={}",
        max_abs
    );
}

#[test]
fn test_dispersion_group_delay_positive() {
    // A4 / B=7.5e-4 / M=8 で group_delay_per_stage > 0
    let (_, gd) = compute_dispersion_a1(DISPERSION_STAGES as u32, PIANO_B, 440.0, SR);
    assert!(gd.is_finite(), "group delay must be finite, got {}", gd);
    assert!(
        gd > 0.0,
        "dispersion group delay should be positive at A4, got {}",
        gd
    );
}
