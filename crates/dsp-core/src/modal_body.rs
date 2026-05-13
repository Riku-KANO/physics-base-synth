//! ModalBodyResonator (Phase 3 D30 / D31 / D32 / R24 + Phase 4c R44 緩和策 2)
//!
//! 楽器ボディ共鳴を最大 16 の並列 bandpass biquad で再現。`Engine::process` で
//! `pool.process_sample()` 後・`output_gain` 前に挿入。stereo は左右で独立した
//! 係数（`STEREO_SPREAD` で偶奇 index 反転）。
//!
//! 楽器ごとの mode 数:
//! - Default / 6 楽器 (Phase 4a) + Piano (Phase 4b 8 modes) = 8 mode 構成
//! - Piano (Phase 4c R44 緩和策 2): 16 mode に拡張、追加 8 modes は 3.2-19 kHz の brilliance/sparkle 帯
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

use crate::params::{
    body_modes_for_instrument, BodyMode, InstrumentKind, BODY_MODES_L, BODY_MODES_R,
};

/// Phase 4c R44 緩和策 2: 最大 mode 数 (Piano の 16 に合わせて拡張)。Default 等の 8 mode
/// 楽器はこの array の 0..8 のみ使用、`num_modes_l/r` でループ範囲を絞る。
pub const MAX_MODES: usize = 16;

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
    coeffs_l: [ModeCoeffs; MAX_MODES],
    coeffs_r: [ModeCoeffs; MAX_MODES],
    states_l: [ModeState; MAX_MODES],
    states_r: [ModeState; MAX_MODES],
    /// Phase 4c R44 緩和策 2: 現在 active な mode 数 (Default 等 = 8, Piano = 16)。
    /// L/R は常に同じ slice 長 (`body_modes_for_instrument` 経由) のため単一フィールドで持つ。
    num_modes: usize,
    sample_rate: f32,
}

impl ModalBodyResonator {
    pub fn new() -> Self {
        const ZERO_S: ModeState = ModeState { z1: 0.0, z2: 0.0 };
        Self {
            coeffs_l: [ZERO_COEFFS; MAX_MODES],
            coeffs_r: [ZERO_COEFFS; MAX_MODES],
            states_l: [ZERO_S; MAX_MODES],
            states_r: [ZERO_S; MAX_MODES],
            num_modes: BODY_MODES_L.len(),
            sample_rate: 48000.0,
        }
    }

    pub fn prepare(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        // BODY_MODES_L / R は Default kind 8 modes の alias (Phase 3 互換)。
        self.num_modes = load_modes(&mut self.coeffs_l, BODY_MODES_L.as_slice(), sample_rate);
        let r_len = load_modes(&mut self.coeffs_r, BODY_MODES_R.as_slice(), sample_rate);
        debug_assert_eq!(self.num_modes, r_len, "BODY_MODES_L / R length mismatch");
        self.reset();
    }

    pub fn reset(&mut self) {
        // active / inactive 全 slot をクリア (楽器切替で前の状態が残らないため)。
        for s in self.states_l.iter_mut() {
            *s = ModeState { z1: 0.0, z2: 0.0 };
        }
        for s in self.states_r.iter_mut() {
            *s = ModeState { z1: 0.0, z2: 0.0 };
        }
    }

    /// Phase 4a D52 / D53 + Phase 4c R44 緩和策 2: 楽器切替で Modal 係数を差し替え、
    /// state をクリア。`body_modes_for_instrument(kind)` の slice 長を `num_modes` に反映
    /// (Default 等 = 8、Piano = 16)。
    pub fn set_instrument(&mut self, kind: InstrumentKind, sample_rate: f32) {
        self.sample_rate = sample_rate;
        let (l_modes, r_modes) = body_modes_for_instrument(kind);
        let l_len = load_modes(&mut self.coeffs_l, l_modes, sample_rate);
        let r_len = load_modes(&mut self.coeffs_r, r_modes, sample_rate);
        debug_assert_eq!(l_len, r_len, "L / R modes length mismatch");
        self.num_modes = l_len;
        self.reset();
    }

    /// テスト用: 指定モードの L 側 biquad 係数 b0 を取得。
    #[doc(hidden)]
    pub fn coeff_l_b0(&self, mode_index: usize) -> f32 {
        self.coeffs_l[mode_index].b0
    }

    /// テスト用: 指定モードの R 側 biquad 係数 b0 を取得。
    #[doc(hidden)]
    pub fn coeff_r_b0(&self, mode_index: usize) -> f32 {
        self.coeffs_r[mode_index].b0
    }

    /// テスト用: state の z1 を取得 (state クリア検証用)。
    #[doc(hidden)]
    pub fn state_l_z1(&self, mode_index: usize) -> f32 {
        self.states_l[mode_index].z1
    }

    /// Phase 4c test-only: 現在の active mode 数 (Default = 8, Piano = 16)。
    #[doc(hidden)]
    pub fn num_modes_l(&self) -> usize {
        self.num_modes
    }

    /// Phase 4c test-only: 互換維持 (L/R は同じ `num_modes`)。
    #[doc(hidden)]
    pub fn num_modes_r(&self) -> usize {
        self.num_modes
    }

    /// 1 サンプル入力に対し左右 2 サンプル出力（並列加算、Direct Form II Transposed）。
    /// y = b0·x + z1; z1 = z2 - a1·y; z2 = b2·x - a2·y
    ///
    /// L / R を 1 つのループに interleave して `x` の cache locality を保つ
    /// (Phase 4b までと同じ構造)。
    #[inline(always)]
    pub fn process_sample(&mut self, x: f32) -> (f32, f32) {
        let mut y_l = 0.0_f32;
        let mut y_r = 0.0_f32;
        for i in 0..self.num_modes {
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

const ZERO_COEFFS: ModeCoeffs = ModeCoeffs {
    b0: 0.0,
    b2: 0.0,
    a1: 0.0,
    a2: 0.0,
};

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

/// `src` の各 mode を biquad 係数へ展開して `dst[0..src.len()]` に書き込む。
/// `process_sample` は `0..num_modes` しか読まないため、末尾のスロットは前回値のまま残しても
/// 害は無いが、検証 / デバッグ容易性のためゼロ化する経路は呼出側で実施しない方針 (process
/// が読まないので残留 coeffs は dead state)。
fn load_modes(dst: &mut [ModeCoeffs; MAX_MODES], src: &[BodyMode], sr: f32) -> usize {
    debug_assert!(src.len() <= MAX_MODES);
    for (slot, mode) in dst.iter_mut().zip(src.iter()) {
        *slot = calc_coeffs(*mode, sr);
    }
    src.len()
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
