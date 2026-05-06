#[repr(u32)]
#[non_exhaustive]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ParamId {
    Damping = 0,
    Brightness = 1,
    OutputGain = 2,
}

impl ParamId {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Damping),
            1 => Some(Self::Brightness),
            2 => Some(Self::OutputGain),
            _ => None,
        }
    }
}

pub const DAMPING_MIN: f32 = 0.90;
pub const DAMPING_MAX: f32 = 0.9999;
pub const DAMPING_DEFAULT: f32 = 0.996;

pub const BRIGHTNESS_MIN: f32 = 0.0;
pub const BRIGHTNESS_MAX: f32 = 1.0;
pub const BRIGHTNESS_DEFAULT: f32 = 0.5;

pub const OUTPUT_GAIN_MIN: f32 = 0.0;
pub const OUTPUT_GAIN_MAX: f32 = 1.5;
pub const OUTPUT_GAIN_DEFAULT: f32 = 0.8;
