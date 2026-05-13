//! Fractional delay interpolation.
//!
//! `KarplusStrong` は `ThiranCoeffs` (1 次 allpass) を採用。`LagrangeCoeffs` は
//! 将来の再評価用に残置。

/// Lagrange 3 次補間係数。fractional 部 d ∈ [0, 1) に対する 4 つの重み。
///
/// 4 サンプルの引数順は時間的に新しい → 古い:
///   x_minus   = x[n - D_int + 1] （最新側、h0 重み）
///   x_zero    = x[n - D_int]     （中央、h1 重み）
///   x_plus_1  = x[n - D_int - 1] （h2 重み）
///   x_plus_2  = x[n - D_int - 2] （最古側、h3 重み）
#[derive(Debug, Clone, Copy)]
pub struct LagrangeCoeffs {
    pub h0: f32,
    pub h1: f32,
    pub h2: f32,
    pub h3: f32,
}

impl LagrangeCoeffs {
    pub fn new(d: f32) -> Self {
        let dc = d.clamp(0.0, 1.0);
        let dm1 = dc - 1.0;
        let dm2 = dc - 2.0;
        let dp1 = dc + 1.0;
        Self {
            h0: -dc * dm1 * dm2 / 6.0,
            h1: dp1 * dm1 * dm2 / 2.0,
            h2: -dp1 * dc * dm2 / 2.0,
            h3: dp1 * dc * dm1 / 6.0,
        }
    }

    pub fn set_fractional(&mut self, d: f32) {
        *self = Self::new(d);
    }

    #[inline(always)]
    pub fn apply(&self, x_minus: f32, x_zero: f32, x_plus_1: f32, x_plus_2: f32) -> f32 {
        self.h0 * x_minus + self.h1 * x_zero + self.h2 * x_plus_1 + self.h3 * x_plus_2
    }
}

impl Default for LagrangeCoeffs {
    fn default() -> Self {
        Self::new(0.0)
    }
}

/// 1 次 Thiran allpass。`H(z) = (a₁ + z⁻¹)/(1 + a₁·z⁻¹)`、`a₁ = (1 - d)/(1 + d)`。
/// `|H(ω)| = 1` を厳密に保つため、Lagrange 4 点 FIR の高域減衰がない。
#[derive(Debug, Clone, Copy)]
pub struct ThiranCoeffs {
    pub a1: f32,
    z1_in: f32,
    z1_out: f32,
}

impl ThiranCoeffs {
    pub const fn new() -> Self {
        Self {
            a1: 0.0,
            z1_in: 0.0,
            z1_out: 0.0,
        }
    }

    /// d=0 は a₁=1.0 で極が単位円上 z=-1 に乗る境界ケースのため、`[1e-4, 0.999]` に clamp。
    /// d=1e-4 → a₁ ≈ 0.9998、d=0.999 → a₁ ≈ 5e-4、いずれも極が単位円内で安定。
    pub fn set_fractional(&mut self, d: f32) {
        let d_safe = d.clamp(1e-4, 0.999);
        self.a1 = (1.0 - d_safe) / (1.0 + d_safe);
    }

    pub fn reset(&mut self) {
        self.z1_in = 0.0;
        self.z1_out = 0.0;
    }

    /// 整数 delay D_int サンプル後の値に対し allpass を通す。
    /// y[n] = a₁·x[n] + x[n-1] - a₁·y[n-1]
    #[inline(always)]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.a1 * x + self.z1_in - self.a1 * self.z1_out;
        self.z1_in = x;
        self.z1_out = y;
        y
    }
}

impl Default for ThiranCoeffs {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lagrange_d_zero_gives_x_zero() {
        let c = LagrangeCoeffs::new(0.0);
        let y = c.apply(7.0, 11.0, 13.0, 17.0);
        assert!(
            (y - 11.0).abs() < 1.0e-6,
            "expected x_zero=11.0, got {y} (coeffs={:?})",
            c
        );
    }

    #[test]
    fn test_lagrange_d_one_gives_x_plus_1() {
        let c = LagrangeCoeffs::new(1.0);
        let y = c.apply(7.0, 11.0, 13.0, 17.0);
        assert!(
            (y - 13.0).abs() < 1.0e-6,
            "expected x_plus_1=13.0, got {y} (coeffs={:?})",
            c
        );
    }

    #[test]
    fn test_lagrange_coeffs_sum_to_one() {
        for k in 0..=10 {
            let d = k as f32 / 10.0;
            let c = LagrangeCoeffs::new(d);
            let sum = c.h0 + c.h1 + c.h2 + c.h3;
            assert!(
                (sum - 1.0).abs() < 1.0e-5,
                "sum = {sum} for d = {d} (coeffs={:?})",
                c
            );
        }
    }

    #[test]
    fn test_lagrange_clamps_out_of_range() {
        let c_neg = LagrangeCoeffs::new(-0.5);
        let c_zero = LagrangeCoeffs::new(0.0);
        assert!((c_neg.h0 - c_zero.h0).abs() < 1.0e-6);
        assert!((c_neg.h1 - c_zero.h1).abs() < 1.0e-6);
        assert!((c_neg.h2 - c_zero.h2).abs() < 1.0e-6);
        assert!((c_neg.h3 - c_zero.h3).abs() < 1.0e-6);

        let c_big = LagrangeCoeffs::new(1.7);
        let c_one = LagrangeCoeffs::new(1.0);
        assert!((c_big.h0 - c_one.h0).abs() < 1.0e-6);
        assert!((c_big.h1 - c_one.h1).abs() < 1.0e-6);
        assert!((c_big.h2 - c_one.h2).abs() < 1.0e-6);
        assert!((c_big.h3 - c_one.h3).abs() < 1.0e-6);
    }

    #[test]
    fn test_lagrange_set_fractional_matches_new() {
        let mut c = LagrangeCoeffs::default();
        c.set_fractional(0.37);
        let expected = LagrangeCoeffs::new(0.37);
        assert!((c.h0 - expected.h0).abs() < 1.0e-7);
        assert!((c.h1 - expected.h1).abs() < 1.0e-7);
        assert!((c.h2 - expected.h2).abs() < 1.0e-7);
        assert!((c.h3 - expected.h3).abs() < 1.0e-7);
    }

    #[test]
    fn test_thiran_a1_formula() {
        let mut t = ThiranCoeffs::new();
        t.set_fractional(0.5);
        // d=0.5 → a₁ = 0.5/1.5 = 1/3
        assert!((t.a1 - 1.0 / 3.0).abs() < 1.0e-6, "a1 = {}", t.a1);
    }

    #[test]
    fn test_thiran_clamps_d_to_safe_range() {
        let mut t = ThiranCoeffs::new();
        t.set_fractional(0.0);
        assert!(t.a1 < 1.0, "a1 must be < 1.0, got {}", t.a1);
        assert!(
            t.a1 > 0.999,
            "a1 should be near 1.0 for d ≈ 0, got {}",
            t.a1
        );

        t.set_fractional(1.0);
        assert!(
            t.a1.abs() < 1.0e-3,
            "a1 should be near 0 for d ≈ 1, got {}",
            t.a1
        );

        t.set_fractional(2.0);
        assert!(t.a1.abs() < 1.0e-3);

        t.set_fractional(-0.5);
        assert!(t.a1 < 1.0 && t.a1 > 0.999);
    }

    #[test]
    fn test_thiran_pole_stability() {
        for &d in &[0.0_f32, 0.5, 0.999, 1.0] {
            let mut t = ThiranCoeffs::new();
            t.set_fractional(d);
            assert!(
                t.a1.abs() < 1.0,
                "pole on/outside unit circle for d={}: |a1|={}",
                d,
                t.a1.abs()
            );

            let mut max_abs = 0.0_f32;
            t.reset();
            let mut x = 1.0_f32;
            for _ in 0..1024 {
                let y = t.process(x);
                assert!(y.is_finite(), "non-finite output for d={}", d);
                max_abs = max_abs.max(y.abs());
                x = 0.0;
            }
            assert!(
                max_abs < 100.0,
                "thiran response too large for d={}: max={}",
                d,
                max_abs
            );
        }
    }

    #[test]
    fn test_thiran_d_one_passthrough_like() {
        let mut t = ThiranCoeffs::new();
        t.set_fractional(1.0);
        let y0 = t.process(1.0);
        let y1 = t.process(0.0);
        assert!(y0.abs() < 0.01, "y0 should be small (≈ a1), got {}", y0);
        assert!(
            (y1 - 1.0).abs() < 0.01,
            "y1 should be close to 1.0, got {}",
            y1
        );
    }

    #[test]
    fn test_thiran_reset_clears_state() {
        let mut t = ThiranCoeffs::new();
        t.set_fractional(0.5);
        let _ = t.process(1.0);
        let _ = t.process(0.5);
        t.reset();
        let y_after_reset = t.process(0.0);
        assert!(
            y_after_reset.abs() < 1.0e-7,
            "expected 0 after reset, got {}",
            y_after_reset
        );
    }
}
