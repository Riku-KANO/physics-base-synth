// AUTO-GENERATED FROM params.json — DO NOT EDIT
// Run `pnpm gen:params` to regenerate.

#[derive(Debug, Clone, Copy)]
pub struct ParamDescriptor {
    pub id: u32,
    pub name: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub smoothing_tau: f32,
}

impl ParamDescriptor {
    pub const fn clamp(&self, value: f32) -> f32 {
        if value < self.min {
            self.min
        } else if value > self.max {
            self.max
        } else {
            value
        }
    }
}

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

    pub fn descriptor(&self) -> &'static ParamDescriptor {
        &PARAM_DESCRIPTORS[*self as usize]
    }
}

pub const DAMPING_DESCRIPTOR: ParamDescriptor = ParamDescriptor {
    id: 0,
    name: "Damping",
    min: 0.9,
    max: 0.9999,
    default: 0.996,
    smoothing_tau: 0.02,
};

pub const BRIGHTNESS_DESCRIPTOR: ParamDescriptor = ParamDescriptor {
    id: 1,
    name: "Brightness",
    min: 0.0,
    max: 1.0,
    default: 0.5,
    smoothing_tau: 0.02,
};

pub const OUTPUT_GAIN_DESCRIPTOR: ParamDescriptor = ParamDescriptor {
    id: 2,
    name: "OutputGain",
    min: 0.0,
    max: 1.5,
    default: 0.8,
    smoothing_tau: 0.01,
};

pub const PARAM_DESCRIPTORS: [ParamDescriptor; 3] = [
    DAMPING_DESCRIPTOR,
    BRIGHTNESS_DESCRIPTOR,
    OUTPUT_GAIN_DESCRIPTOR,
];

// Phase 1 互換の範囲定数（既存コードからの参照のため維持）
pub const DAMPING_MIN: f32 = DAMPING_DESCRIPTOR.min;
pub const DAMPING_MAX: f32 = DAMPING_DESCRIPTOR.max;
pub const DAMPING_DEFAULT: f32 = DAMPING_DESCRIPTOR.default;

pub const BRIGHTNESS_MIN: f32 = BRIGHTNESS_DESCRIPTOR.min;
pub const BRIGHTNESS_MAX: f32 = BRIGHTNESS_DESCRIPTOR.max;
pub const BRIGHTNESS_DEFAULT: f32 = BRIGHTNESS_DESCRIPTOR.default;

pub const OUTPUT_GAIN_MIN: f32 = OUTPUT_GAIN_DESCRIPTOR.min;
pub const OUTPUT_GAIN_MAX: f32 = OUTPUT_GAIN_DESCRIPTOR.max;
pub const OUTPUT_GAIN_DEFAULT: f32 = OUTPUT_GAIN_DESCRIPTOR.default;
