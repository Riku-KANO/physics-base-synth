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
    PickPosition = 3,
    BodyWet = 4,
}

impl ParamId {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Damping),
            1 => Some(Self::Brightness),
            2 => Some(Self::OutputGain),
            3 => Some(Self::PickPosition),
            4 => Some(Self::BodyWet),
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

pub const PICK_POSITION_DESCRIPTOR: ParamDescriptor = ParamDescriptor {
    id: 3,
    name: "PickPosition",
    min: 0.05,
    max: 0.5,
    default: 0.125,
    smoothing_tau: 0.05,
};

pub const BODY_WET_DESCRIPTOR: ParamDescriptor = ParamDescriptor {
    id: 4,
    name: "BodyWet",
    min: 0.0,
    max: 1.0,
    default: 0.5,
    smoothing_tau: 0.02,
};

pub const PARAM_DESCRIPTORS: [ParamDescriptor; 5] = [
    DAMPING_DESCRIPTOR,
    BRIGHTNESS_DESCRIPTOR,
    OUTPUT_GAIN_DESCRIPTOR,
    PICK_POSITION_DESCRIPTOR,
    BODY_WET_DESCRIPTOR,
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

pub const PICK_POSITION_MIN: f32 = PICK_POSITION_DESCRIPTOR.min;
pub const PICK_POSITION_MAX: f32 = PICK_POSITION_DESCRIPTOR.max;
pub const PICK_POSITION_DEFAULT: f32 = PICK_POSITION_DESCRIPTOR.default;

pub const BODY_WET_MIN: f32 = BODY_WET_DESCRIPTOR.min;
pub const BODY_WET_MAX: f32 = BODY_WET_DESCRIPTOR.max;
pub const BODY_WET_DEFAULT: f32 = BODY_WET_DESCRIPTOR.default;

// Phase 3 D30 / D32: ModalBodyResonator の係数テーブル
#[derive(Debug, Clone, Copy)]
pub struct BodyMode {
    pub freq: f32,
    pub q: f32,
    pub gain: f32,
}

pub const STEREO_SPREAD: f32 = 0.05;

pub const BODY_MODES_L: [BodyMode; 8] = [
    BodyMode { freq: 105.0, q: 30.0, gain: 1.0 },
    BodyMode { freq: 200.0, q: 25.0, gain: 0.8 },
    BodyMode { freq: 280.0, q: 20.0, gain: 0.5 },
    BodyMode { freq: 420.0, q: 35.0, gain: 0.4 },
    BodyMode { freq: 580.0, q: 40.0, gain: 0.35 },
    BodyMode { freq: 850.0, q: 45.0, gain: 0.25 },
    BodyMode { freq: 1400.0, q: 50.0, gain: 0.2 },
    BodyMode { freq: 2300.0, q: 60.0, gain: 0.15 },
];

pub const BODY_MODES_R: [BodyMode; 8] = [
    BodyMode { freq: 110.25, q: 28.5, gain: 1.05 },
    BodyMode { freq: 190.0, q: 26.25, gain: 0.84 },
    BodyMode { freq: 294.0, q: 19.0, gain: 0.525 },
    BodyMode { freq: 399.0, q: 36.75, gain: 0.42 },
    BodyMode { freq: 609.0, q: 38.0, gain: 0.3675 },
    BodyMode { freq: 807.5, q: 47.25, gain: 0.2625 },
    BodyMode { freq: 1470.0, q: 47.5, gain: 0.21 },
    BodyMode { freq: 2185.0, q: 63.0, gain: 0.1575 },
];
