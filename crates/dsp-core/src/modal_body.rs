//! ModalBodyResonator (Phase 3 D30 / D31 / D32 / R24)
//!
//! 楽器ボディ共鳴を 8 つの並列 bandpass biquad で再現。`Engine::process` で
//! `pool.process_sample()` 後・`output_gain` 前に挿入。stereo は左右で独立した
//! 係数（`STEREO_SPREAD = 0.05` で偶奇 index 反転）。
//!
//! biquad 形（RBJ "Audio EQ Cookbook" の constant peak gain Q bandpass）:
//!   H(z) = (b0 + b2·z⁻²) / (1 + a1·z⁻¹ + a2·z⁻²)
//!   ω = 2π · freq / Fs、α = sin(ω) / (2Q)
//!   a0_raw = 1 + α、b0 = α·gain / a0_raw、b2 = -α·gain / a0_raw
//!   a1 = -2·cos(ω) / a0_raw、a2 = (1 - α) / a0_raw
//!
//! 特性:
//! - DC ゲイン = 0 (b0 + b2 = 0)
//! - ピークゲイン = mode.gain (constant peak gain Q 形)
//! - −3dB 帯域幅 = freq / Q

use crate::params::{BodyMode, BODY_MODES_L, BODY_MODES_R};

const NUM_MODES: usize = 8;

#[derive(Debug, Clone, Copy)]
struct ModeCoeffs {
    b0: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

#[derive(Debug, Clone, Copy)]
struct ModeState {
    z1: f32,
    z2: f32,
}

pub struct ModalBodyResonator {
    coeffs_l: [ModeCoeffs; NUM_MODES],
    coeffs_r: [ModeCoeffs; NUM_MODES],
    states_l: [ModeState; NUM_MODES],
    states_r: [ModeState; NUM_MODES],
    sample_rate: f32,
}

impl ModalBodyResonator {
    pub fn new() -> Self {
        const ZERO_C: ModeCoeffs = ModeCoeffs {
            b0: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        };
        const ZERO_S: ModeState = ModeState { z1: 0.0, z2: 0.0 };
        Self {
            coeffs_l: [ZERO_C; NUM_MODES],
            coeffs_r: [ZERO_C; NUM_MODES],
            states_l: [ZERO_S; NUM_MODES],
            states_r: [ZERO_S; NUM_MODES],
            sample_rate: 48000.0,
        }
    }

    pub fn prepare(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        for i in 0..NUM_MODES {
            self.coeffs_l[i] = calc_coeffs(BODY_MODES_L[i], sample_rate);
            self.coeffs_r[i] = calc_coeffs(BODY_MODES_R[i], sample_rate);
        }
        self.reset();
    }

    pub fn reset(&mut self) {
        for i in 0..NUM_MODES {
            self.states_l[i] = ModeState { z1: 0.0, z2: 0.0 };
            self.states_r[i] = ModeState { z1: 0.0, z2: 0.0 };
        }
    }

    /// 1 サンプル入力に対し左右 2 サンプル出力（並列加算、Direct Form II Transposed）。
    /// y = b0·x + z1; z1 = z2 - a1·y; z2 = b2·x - a2·y
    #[inline(always)]
    pub fn process_sample(&mut self, x: f32) -> (f32, f32) {
        let mut y_l = 0.0_f32;
        let mut y_r = 0.0_f32;
        for i in 0..NUM_MODES {
            let c = self.coeffs_l[i];
            let s = &mut self.states_l[i];
            let y = c.b0 * x + s.z1;
            s.z1 = s.z2 - c.a1 * y;
            s.z2 = c.b2 * x - c.a2 * y;
            y_l += y;

            let c = self.coeffs_r[i];
            let s = &mut self.states_r[i];
            let y = c.b0 * x + s.z1;
            s.z1 = s.z2 - c.a1 * y;
            s.z2 = c.b2 * x - c.a2 * y;
            y_r += y;
        }
        // denormal flush (R24 対策、D6 拡張)
        (y_l + 1e-25 - 1e-25, y_r + 1e-25 - 1e-25)
    }
}

impl Default for ModalBodyResonator {
    fn default() -> Self {
        Self::new()
    }
}

/// 単体 biquad テスト (`tests/modal_body_biquad_tests.rs`) で 1 段ずつの仕様検証に使う。
/// ライブラリ内部から呼ぶ用途では `prepare` 経由で間接呼び出しのため `pub` 露出は不要だが、
/// 単体係数テストでアクセスするため公開する。
pub fn calc_coeffs_for_test(mode: BodyMode, sample_rate: f32) -> (f32, f32, f32, f32) {
    let c = calc_coeffs(mode, sample_rate);
    (c.b0, c.b2, c.a1, c.a2)
}

/// 1 段 biquad の処理関数（テスト用、内部の本体ループと同じ式）。
#[inline(always)]
pub fn process_single_biquad_for_test(
    coeffs: (f32, f32, f32, f32),
    state: (f32, f32),
    x: f32,
) -> (f32, (f32, f32)) {
    let (b0, b2, a1, a2) = coeffs;
    let (z1, z2) = state;
    let y = b0 * x + z1;
    let new_z1 = z2 - a1 * y;
    let new_z2 = b2 * x - a2 * y;
    (y, (new_z1, new_z2))
}

fn calc_coeffs(mode: BodyMode, sr: f32) -> ModeCoeffs {
    let omega = 2.0 * core::f32::consts::PI * mode.freq / sr;
    let cos_w = omega.cos();
    let sin_w = omega.sin();
    let alpha = sin_w / (2.0 * mode.q);
    let a0 = 1.0 + alpha;
    let inv_a0 = 1.0 / a0;
    ModeCoeffs {
        b0: alpha * mode.gain * inv_a0,
        b2: -alpha * mode.gain * inv_a0,
        a1: -2.0 * cos_w * inv_a0,
        a2: (1.0 - alpha) * inv_a0,
    }
}
