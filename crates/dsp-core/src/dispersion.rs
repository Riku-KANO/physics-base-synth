//! Stretching / Dispersion all-pass cascade (Phase 4b D57 / D58 / D59)
//!
//! ピアノ stiff string の inharmonicity (`f_n = n·f_0·√(1+B·n²)`) を、
//! M 段の 1 次 allpass cascade で再現する。係数 a1 は Rauhala-Välimäki 2006 の
//! closed-form 式（Faust `piano_dispersion_filter` の Rust 移植）で算出。
//!
//! 配置: `KarplusStrong::process_sample` の `buffer[read_z]` 値を 8 段に通してから
//! 既存 Thiran allpass に渡す。`KarplusStrong::note_on` で `compute_dispersion_a1`
//! を呼び、各 stage の a1 + 状態を初期化する。
//!
//! Phase 4a 互換性: `dispersion_active = false` の楽器（Default 〜 Sitar）では
//! `process_sample` で skip、CPU 影響ゼロ。`Engine::apply_instrument(Piano)` で
//! 全 voice に `set_dispersion_active(true)` を fan-out。

#![allow(clippy::approx_constant)]

/// Phase 4b D57: Dispersion all-pass の段数（M=8 固定、Faust 標準）。
/// 増減する場合は `KarplusStrong::dispersion_stages` の配列長と同期させること。
pub const DISPERSION_STAGES: usize = 8;

/// Phase 4b D59: Rauhala-Välimäki 2006 closed-form の magic constants (文献値)。
/// `compute_dispersion_a1` で以下の式に対応する:
///   `kd = exp(K1·log²(B) + K2·log(B) + K3)`
///   `Cd = exp((M1·log(M) + M2)·log(B) + M3·log(M) + M4)`
///   `D = exp(Cd - Ikey·kd)`、`a1 = (1 - D) / (1 + D)`
/// `approx_constant` lint は module-level allow で抑止。
const K1: f32 = -0.00179;
const K2: f32 = -0.0233;
const K3: f32 = -2.93;
const M1: f32 = 0.0126;
const M2: f32 = 0.0606;
const M3: f32 = -0.00825;
const M4: f32 = 1.97;

/// 1 段の dispersion allpass。`H(z) = (a1 + z⁻¹)/(1 + a1·z⁻¹)`。
/// `KarplusStrong::dispersion_stages: [DispersionStage; 8]` で inline 保持。
#[derive(Debug, Clone, Copy)]
pub struct DispersionStage {
    pub a1: f32,
    pub z1_in: f32,
    pub z1_out: f32,
}

impl DispersionStage {
    pub const fn new() -> Self {
        Self {
            a1: 0.0,
            z1_in: 0.0,
            z1_out: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.z1_in = 0.0;
        self.z1_out = 0.0;
    }

    /// 1 サンプル処理: `y = a1·(x − z1_out) + z1_in`、状態更新。
    /// `y = a1·x + z1_in − a1·z1_out` を 1 mul に括った数学的等価形 (FMUL を 8×8 voice で
    /// ~12% 削減)。`KarplusStrong::process_sample` のホットパスで 8 段直列呼出される
    /// 前提のため `#[inline(always)]` で関数呼出オーバーヘッドを除去。
    #[inline(always)]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.a1 * (x - self.z1_out) + self.z1_in;
        self.z1_in = x;
        self.z1_out = y;
        y
    }
}

impl Default for DispersionStage {
    fn default() -> Self {
        Self::new()
    }
}

/// Phase 4c D78 / D79: MIDI ノートを 21..=108 に clamp してから 88 鍵 B(note) LUT を引く。
/// 範囲外 (< 21 / > 108) は端値で fallback、Engine から渡される `u8` 全域 (0..=127) に対し
/// 未定義動作 / panic が発生しないことを保証する。`Engine::inharmonicity_b_for_note` 関数
/// ポインタ経由で Piano kind の note_on 時に呼ばれる。
#[inline]
pub fn b_curve_piano(midi: u8) -> f32 {
    let clamped = midi.clamp(21, 108);
    let idx = (clamped - 21) as usize;
    crate::params::INHARMONICITY_B_CURVE_PIANO[idx]
}

/// Phase 4c D77 / D78: 非 Piano 楽器用の B 関数ポインタ。常に 0 を返すことで
/// `compute_dispersion_a1` の `b.max(1e-6)` 分岐を通り a1 が極小 (= dispersion 実質 disable)
/// になる。Phase 4a / 4b の `dispersion_active = false` 経路と二重保証で互換性を維持する。
#[inline]
pub fn b_curve_zero(_midi: u8) -> f32 {
    0.0
}

/// Phase 4b D59: Rauhala-Välimäki 2006 closed-form で a1 + 群遅延を算出。
///
/// # 引数
/// - `m`: 段数（典型 8、`DISPERSION_STAGES`）
/// - `b`: inharmonicity coefficient（典型 1e-4〜1e-1、Phase 4b は Piano 固定 7.5e-4）
/// - `f0`: 基音周波数 (Hz)
/// - `fs`: サンプリングレート (Hz)
///
/// # 戻り値
/// - `(a1, group_delay_per_stage)`: a1 は各段共通、group_delay_per_stage は基音 f0 における 1 段の群遅延（sample 単位、`adjusted_length` 補正に使用）
///
/// # 数値安定性
/// - B → 0 で a1 → 0（allpass = passthrough）
/// - B 大 → a1 大、|a1| < 1.0 で極が単位円内
/// - 念のため `a1.clamp(-0.999, 0.999)` で安全側に制限（C8 / 高 B 値での発散防止）
pub fn compute_dispersion_a1(m: u32, b: f32, f0: f32, fs: f32) -> (f32, f32) {
    use core::f32::consts::PI;

    let m_f32 = m as f32;
    let trt = 2.0_f32.powf(1.0 / 12.0);
    let bc = b.max(1.0e-6);
    let log_bc = bc.ln();

    // 鍵盤位置 Ikey(f0) = log_(2^(1/12))(f0 · 2^(1/12) / 27.5)
    // A0 = 27.5 Hz を 0 とする半音単位インデックス（A4 = 48）
    let ikey = ((f0 * trt) / 27.5_f32).ln() / trt.ln();

    // kd = exp(K1 · log²(B) + K2 · log(B) + K3)
    let kd = (K1 * log_bc * log_bc + K2 * log_bc + K3).exp();

    // Cd = exp((M1 · log(M) + M2) · log(B) + M3 · log(M) + M4)
    let m_log = m_f32.ln();
    let cd = ((M1 * m_log + M2) * log_bc + M3 * m_log + M4).exp();

    // D = exp(Cd - Ikey · kd)
    let d = (cd - ikey * kd).exp();

    // a1 = (1 - D) / (1 + D)
    let a1 = ((1.0 - d) / (1.0 + d)).clamp(-0.999, 0.999);

    // 群遅延（基音 f0 における 1 段の delay）
    // polydel(a) = atan(sin(wT) / (a + cos(wT))) / wT
    let wt = 2.0 * PI * f0 / fs;
    let sin_wt = wt.sin();
    let cos_wt = wt.cos();
    let polydel = |a: f32| -> f32 { (sin_wt / (a + cos_wt)).atan() / wt };
    let group_delay_per_stage = polydel(a1) - polydel(1.0 / a1);

    (a1, group_delay_per_stage)
}

#[cfg(test)]
mod b_curve_tests {
    use super::*;
    use crate::params::INHARMONICITY_B_CURVE_PIANO;

    /// F67-a (provisional pass at Step 3, full F67-a 再掲 in Step 13).
    #[test]
    fn test_b_curve_length_88() {
        assert_eq!(INHARMONICITY_B_CURVE_PIANO.len(), 88);
    }

    /// F67-f (provisional pass at Step 3). MIDI 0 and 127 must map to LUT
    /// endpoints rather than panic / OOB, so Engine can hand any `u8`.
    #[test]
    fn test_b_curve_clamps_out_of_range() {
        assert!((b_curve_piano(0) - INHARMONICITY_B_CURVE_PIANO[0]).abs() < 1e-9);
        assert!((b_curve_piano(127) - INHARMONICITY_B_CURVE_PIANO[87]).abs() < 1e-9);
    }

    #[test]
    fn test_b_curve_zero_returns_zero() {
        for midi in 0u8..=127 {
            assert_eq!(b_curve_zero(midi), 0.0);
        }
    }
}
