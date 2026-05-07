//! SoftClip テスト (Phase 3 F35)

use dsp_core::soft_clip::soft_clip;

#[test]
fn test_soft_clip_linear_in_safe_range() {
    // |x| ≤ 0.95 で soft_clip(x) ≡ x (完全 linear、誤差ゼロ)
    for raw in [-0.95_f32, -0.5, -0.1, -0.001, 0.0, 0.001, 0.1, 0.5, 0.95] {
        assert_eq!(soft_clip(raw), raw, "linear range violation at x={}", raw);
    }
}

#[test]
fn test_soft_clip_bounded() {
    // 任意の x に対し |y| < 1.0
    for x_raw in [-1e6_f32, -1.5, -1.0, -0.96, 0.96, 1.0, 1.5, 1e6] {
        let y = soft_clip(x_raw);
        assert!(
            y.abs() < 1.0,
            "soft_clip({}) = {} not bounded < 1.0",
            x_raw,
            y
        );
    }
}

#[test]
fn test_soft_clip_continuous_at_threshold() {
    // x=0.95 ± 1e-6 で連続 (kink なし、左右の値がほぼ等しい)
    let y_left = soft_clip(0.95 - 1e-6);
    let y_right = soft_clip(0.95 + 1e-6);
    let diff = (y_left - y_right).abs();
    assert!(
        diff < 1e-4,
        "discontinuity at threshold: y_left={}, y_right={}, diff={}",
        y_left,
        y_right,
        diff
    );
}

#[test]
fn test_soft_clip_extreme() {
    // |x| → ∞ で |y| → 1.0 だが strictly less than 1
    for x in [1e3_f32, 1e6, 1e9] {
        let y = soft_clip(x);
        assert!(
            y > 0.99 && y < 1.0,
            "extreme x={}: y={} not in (0.99, 1.0)",
            x,
            y
        );
        let y_neg = soft_clip(-x);
        assert!(
            y_neg < -0.99 && y_neg > -1.0,
            "extreme -x={}: y={} not in (-1.0, -0.99)",
            x,
            y_neg
        );
    }
}

#[test]
fn test_soft_clip_at_one() {
    // x=1.0 → e=0.05 → compressed = 0.05·0.05/(0.05+0.05) = 0.025、出力 = 0.975
    let y = soft_clip(1.0);
    assert!((y - 0.975).abs() < 1e-5, "soft_clip(1.0) = {}", y);
}

#[test]
fn test_soft_clip_signum_preserved() {
    // 入力符号が出力に保持される
    assert!(soft_clip(0.5) > 0.0);
    assert!(soft_clip(-0.5) < 0.0);
    assert!(soft_clip(2.0) > 0.0);
    assert!(soft_clip(-2.0) < 0.0);
    assert_eq!(soft_clip(0.0), 0.0);
}
