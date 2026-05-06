/// Lagrange 3 次補間係数。fractional 部 d ∈ [0, 1) に対する 4 つの重み。
///
/// d = 0 のとき (h0, h1, h2, h3) = (0, 1, 0, 0)、つまり apply は中央サンプル `x_zero` を返す。
/// d = 1 のとき (h0, h1, h2, h3) = (0, 0, 1, 0)、apply は隣 `x_plus_1` を返す。
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
    /// d ∈ [0, 1) に対する Lagrange 3 次補間係数を計算する。
    /// 範囲外は `[0, 1]` に clamp して防御。
    ///
    /// h_0(d) = -d(d-1)(d-2) / 6
    /// h_1(d) = (d+1)(d-1)(d-2) / 2
    /// h_2(d) = -(d+1)d(d-2) / 2
    /// h_3(d) = (d+1)d(d-1) / 6
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
}
