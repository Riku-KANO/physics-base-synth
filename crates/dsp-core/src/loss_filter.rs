//! LossFilter (Phase 3 D33)
//!
//! 弦の周波数依存損失を `(1 + ρ·z⁻¹)/(1 + ρ)` の 1 段 FIR で再現。
//! `KarplusStrong::process_sample` の brightness LPF 直後・damping 乗算前に挿入。
//!
//! 特性:
//! - DC ゲイン = 1.0（保存）
//! - Nyquist ゲイン = (1 - ρ) / (1 + ρ)
//! - ρ は周波数依存式で `note_on` 時に算出: ρ = ρ_base · clamp(freq/220, 0.5, 2.0)

pub struct LossFilter {
    rho: f32,
    norm: f32,
    z1: f32,
}

impl LossFilter {
    pub const RHO_BASE: f32 = 0.05;

    pub fn new() -> Self {
        Self {
            rho: Self::RHO_BASE,
            norm: 1.0 / (1.0 + Self::RHO_BASE),
            z1: 0.0,
        }
    }

    /// note_on 時に呼ぶ。ρ = ρ_base · clamp(freq/220, 0.5, 2.0)、上限 0.5 で安定保証。
    pub fn set_for_frequency(&mut self, freq_hz: f32) {
        let scale = (freq_hz / 220.0).clamp(0.5, 2.0);
        self.rho = (Self::RHO_BASE * scale).clamp(0.0, 0.5);
        self.norm = 1.0 / (1.0 + self.rho);
    }

    pub fn reset(&mut self) {
        self.z1 = 0.0;
    }

    pub fn rho(&self) -> f32 {
        self.rho
    }

    /// y[n] = (x[n] + ρ · x[n-1]) / (1 + ρ)
    #[inline(always)]
    pub fn process_sample(&mut self, x: f32) -> f32 {
        let y = (x + self.rho * self.z1) * self.norm;
        self.z1 = x;
        y
    }
}

impl Default for LossFilter {
    fn default() -> Self {
        Self::new()
    }
}
