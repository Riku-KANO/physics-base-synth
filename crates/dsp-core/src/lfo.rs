//! Lfo (Phase 4a D46 / D47)
//!
//! グローバル LFO（Engine 内 1 個）。Sine / Triangle 切替、レンジ 0.1〜8.0 Hz、
//! SmoothedValue tau=0.05s で rate 平滑化（クリック防止）。
//! denormal flush は phase が [0, 1) で常に有限のため不要。
//!
//! 配置: `Engine::process` の per-sample loop 冒頭で `process_sample()` を呼び、
//! 戻り値 ∈ [-1, 1] を destinations の offset として伝播。

use crate::smoothing::SmoothedValue;

/// LFO 波形種。
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LfoWaveform {
    Sine = 0,
    Triangle = 1,
}

impl LfoWaveform {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Sine),
            1 => Some(Self::Triangle),
            _ => None,
        }
    }
}

/// LFO destination 種。Phase 4a D48 で 3 つ確定。
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LfoDestination {
    Pitch = 0,
    Brightness = 1,
    Volume = 2,
}

impl LfoDestination {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Pitch),
            1 => Some(Self::Brightness),
            2 => Some(Self::Volume),
            _ => None,
        }
    }
}

pub const LFO_RATE_DEFAULT: f32 = 5.0;
pub const LFO_RATE_MIN: f32 = 0.1;
pub const LFO_RATE_MAX: f32 = 8.0;
const LFO_RATE_TAU: f32 = 0.05;

pub struct Lfo {
    /// 0..1 で正規化された phase。`process_sample` で += rate / sample_rate。
    phase: f32,
    /// SmoothedValue 化された rate (Hz)。target は `set_rate` で更新、`process_sample` で 1 サンプル毎に next_sample。
    rate_hz: SmoothedValue,
    waveform: LfoWaveform,
    sample_rate: f32,
}

impl Lfo {
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            rate_hz: SmoothedValue::new(LFO_RATE_DEFAULT),
            waveform: LfoWaveform::Sine,
            sample_rate: 48000.0,
        }
    }

    pub fn prepare(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.rate_hz.set_time_constant(sample_rate, LFO_RATE_TAU);
        self.reset();
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.rate_hz.set_immediate(LFO_RATE_DEFAULT);
        self.waveform = LfoWaveform::Sine;
    }

    pub fn set_rate(&mut self, hz: f32) {
        let v = hz.clamp(LFO_RATE_MIN, LFO_RATE_MAX);
        self.rate_hz.set_target(v);
    }

    pub fn set_waveform(&mut self, kind: LfoWaveform) {
        self.waveform = kind;
    }

    /// 現在 phase に対応する [-1, 1] の LFO 値を返してから 1 サンプル分 phase を進める。
    /// Sine: `f32::sin(2π · phase)`、Triangle: `1 − 4·|phase − 0.5|`
    /// (phase=0 で -1、phase=0.5 で +1、phase=1 へ戻る三角波、03 章テキストの定義に従う)。
    /// 値計算 → phase 進行の順にすることで、初回呼出は phase=0 での値（sine=0、triangle=-1）を返す。
    #[inline(always)]
    pub fn process_sample(&mut self) -> f32 {
        let value = match self.waveform {
            LfoWaveform::Sine => {
                use core::f32::consts::TAU;
                (TAU * self.phase).sin()
            }
            LfoWaveform::Triangle => {
                let centered = self.phase - 0.5;
                1.0 - 4.0 * centered.abs()
            }
        };
        let rate = self.rate_hz.next_sample();
        self.phase += rate / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        value
    }

    #[doc(hidden)]
    pub fn phase(&self) -> f32 {
        self.phase
    }

    #[doc(hidden)]
    pub fn rate_target(&self) -> f32 {
        self.rate_hz.target()
    }

    #[doc(hidden)]
    pub fn rate_current(&self) -> f32 {
        self.rate_hz.current()
    }
}

impl Default for Lfo {
    fn default() -> Self {
        Self::new()
    }
}
