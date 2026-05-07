//! SoftClip (Phase 3 D43)
//!
//! 区間関数型 saturator: 安全域 (|x| ≤ 0.95) は完全 linear (誤差ゼロ)、
//! 超過分を rational mapping で [0, 0.05) に圧縮、|x| → ∞ で出力 ±1.0 に厳密漸近。
//! `tanh` Padé 近似は |x| → ∞ で発散するため不採用。

const SOFT_CLIP_THRESHOLD: f32 = 0.95;
const SOFT_CLIP_RANGE: f32 = 0.05; // = 1.0 - THRESHOLD

/// f32 精度で 1.0 ちょうどに丸まらないよう僅かに下げた厳密上限。
/// `0.95 + 0.05 = 1.0` (exact in f32) になるため、|x| → ∞ で
/// `(THRESHOLD + compressed) → 1.0` に達する前にクランプする。
const SOFT_CLIP_MAX_MAG: f32 = 1.0 - f32::EPSILON;

#[inline(always)]
pub fn soft_clip(x: f32) -> f32 {
    let abs_x = x.abs();
    if abs_x <= SOFT_CLIP_THRESHOLD {
        x
    } else {
        let e = abs_x - SOFT_CLIP_THRESHOLD;
        let compressed = SOFT_CLIP_RANGE * e / (e + SOFT_CLIP_RANGE);
        let mag = (SOFT_CLIP_THRESHOLD + compressed).min(SOFT_CLIP_MAX_MAG);
        x.signum() * mag
    }
}
