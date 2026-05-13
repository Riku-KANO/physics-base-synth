// AUTO-GENERATED FROM params.json — DO NOT EDIT
// Run `pnpm gen:params` to regenerate.

#![allow(clippy::approx_constant)]

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

// Phase 3 D30 / D32 + Phase 4a D52 / D54: ModalBodyResonator の係数テーブル
#[derive(Debug, Clone, Copy)]
pub struct BodyMode {
    pub freq: f32,
    pub q: f32,
    pub gain: f32,
}

pub const STEREO_SPREAD_DEFAULT: f32 = 0.05;
pub const STEREO_SPREAD_GUITAR_CLASSICAL: f32 = 0.05;
pub const STEREO_SPREAD_UKULELE: f32 = 0.04;
pub const STEREO_SPREAD_MANDOLIN: f32 = 0.06;
pub const STEREO_SPREAD_BASS: f32 = 0.03;
pub const STEREO_SPREAD_GUITAR_STEEL: f32 = 0.05;
pub const STEREO_SPREAD_SITAR: f32 = 0.08;
pub const STEREO_SPREAD_PIANO: f32 = 0.05;

#[rustfmt::skip]
pub const BODY_MODES_DEFAULT_L: [BodyMode; 8] = [
    BodyMode { freq: 105.0, q: 30.0, gain: 1.0 },
    BodyMode { freq: 200.0, q: 25.0, gain: 0.8 },
    BodyMode { freq: 280.0, q: 20.0, gain: 0.5 },
    BodyMode { freq: 420.0, q: 35.0, gain: 0.4 },
    BodyMode { freq: 580.0, q: 40.0, gain: 0.35 },
    BodyMode { freq: 850.0, q: 45.0, gain: 0.25 },
    BodyMode { freq: 1400.0, q: 50.0, gain: 0.2 },
    BodyMode { freq: 2300.0, q: 60.0, gain: 0.15 },
];

#[rustfmt::skip]
pub const BODY_MODES_DEFAULT_R: [BodyMode; 8] = [
    BodyMode { freq: 110.25, q: 28.5, gain: 1.05 },
    BodyMode { freq: 190.0, q: 26.25, gain: 0.84 },
    BodyMode { freq: 294.0, q: 19.0, gain: 0.525 },
    BodyMode { freq: 399.0, q: 36.75, gain: 0.42 },
    BodyMode { freq: 609.0, q: 38.0, gain: 0.3675 },
    BodyMode { freq: 807.5, q: 47.25, gain: 0.2625 },
    BodyMode { freq: 1470.0, q: 47.5, gain: 0.21 },
    BodyMode { freq: 2185.0, q: 63.0, gain: 0.1575 },
];

#[rustfmt::skip]
pub const BODY_MODES_GUITAR_CLASSICAL_L: [BodyMode; 8] = [
    BodyMode { freq: 105.0, q: 30.0, gain: 1.0 },
    BodyMode { freq: 200.0, q: 25.0, gain: 0.8 },
    BodyMode { freq: 280.0, q: 20.0, gain: 0.5 },
    BodyMode { freq: 420.0, q: 35.0, gain: 0.4 },
    BodyMode { freq: 580.0, q: 40.0, gain: 0.35 },
    BodyMode { freq: 850.0, q: 45.0, gain: 0.25 },
    BodyMode { freq: 1400.0, q: 50.0, gain: 0.2 },
    BodyMode { freq: 2300.0, q: 60.0, gain: 0.15 },
];

#[rustfmt::skip]
pub const BODY_MODES_GUITAR_CLASSICAL_R: [BodyMode; 8] = [
    BodyMode { freq: 110.25, q: 28.5, gain: 1.05 },
    BodyMode { freq: 190.0, q: 26.25, gain: 0.84 },
    BodyMode { freq: 294.0, q: 19.0, gain: 0.525 },
    BodyMode { freq: 399.0, q: 36.75, gain: 0.42 },
    BodyMode { freq: 609.0, q: 38.0, gain: 0.3675 },
    BodyMode { freq: 807.5, q: 47.25, gain: 0.2625 },
    BodyMode { freq: 1470.0, q: 47.5, gain: 0.21 },
    BodyMode { freq: 2185.0, q: 63.0, gain: 0.1575 },
];

#[rustfmt::skip]
pub const BODY_MODES_UKULELE_L: [BodyMode; 8] = [
    BodyMode { freq: 200.0, q: 18.0, gain: 0.9 },
    BodyMode { freq: 380.0, q: 20.0, gain: 0.7 },
    BodyMode { freq: 540.0, q: 22.0, gain: 0.45 },
    BodyMode { freq: 780.0, q: 28.0, gain: 0.35 },
    BodyMode { freq: 1100.0, q: 32.0, gain: 0.3 },
    BodyMode { freq: 1600.0, q: 38.0, gain: 0.22 },
    BodyMode { freq: 2200.0, q: 42.0, gain: 0.18 },
    BodyMode { freq: 3100.0, q: 50.0, gain: 0.12 },
];

#[rustfmt::skip]
pub const BODY_MODES_UKULELE_R: [BodyMode; 8] = [
    BodyMode { freq: 208.0, q: 17.28, gain: 0.936 },
    BodyMode { freq: 364.8, q: 20.8, gain: 0.728 },
    BodyMode { freq: 561.6, q: 21.12, gain: 0.468 },
    BodyMode { freq: 748.8, q: 29.12, gain: 0.364 },
    BodyMode { freq: 1144.0, q: 30.72, gain: 0.312 },
    BodyMode { freq: 1536.0, q: 39.52, gain: 0.2288 },
    BodyMode { freq: 2288.0, q: 40.32, gain: 0.1872 },
    BodyMode { freq: 2976.0, q: 52.0, gain: 0.1248 },
];

#[rustfmt::skip]
pub const BODY_MODES_MANDOLIN_L: [BodyMode; 8] = [
    BodyMode { freq: 145.0, q: 25.0, gain: 0.85 },
    BodyMode { freq: 260.0, q: 28.0, gain: 0.7 },
    BodyMode { freq: 410.0, q: 32.0, gain: 0.5 },
    BodyMode { freq: 620.0, q: 40.0, gain: 0.4 },
    BodyMode { freq: 920.0, q: 48.0, gain: 0.35 },
    BodyMode { freq: 1450.0, q: 60.0, gain: 0.3 },
    BodyMode { freq: 2100.0, q: 70.0, gain: 0.25 },
    BodyMode { freq: 2900.0, q: 75.0, gain: 0.2 },
];

#[rustfmt::skip]
pub const BODY_MODES_MANDOLIN_R: [BodyMode; 8] = [
    BodyMode { freq: 153.7, q: 23.5, gain: 0.901 },
    BodyMode { freq: 244.4, q: 29.68, gain: 0.742 },
    BodyMode { freq: 434.6, q: 30.08, gain: 0.53 },
    BodyMode { freq: 582.8, q: 42.4, gain: 0.424 },
    BodyMode { freq: 975.2, q: 45.12, gain: 0.371 },
    BodyMode { freq: 1363.0, q: 63.6, gain: 0.318 },
    BodyMode { freq: 2226.0, q: 65.8, gain: 0.265 },
    BodyMode { freq: 2726.0, q: 79.5, gain: 0.212 },
];

#[rustfmt::skip]
pub const BODY_MODES_BASS_L: [BodyMode; 8] = [
    BodyMode { freq: 60.0, q: 25.0, gain: 1.2 },
    BodyMode { freq: 120.0, q: 22.0, gain: 0.9 },
    BodyMode { freq: 195.0, q: 25.0, gain: 0.6 },
    BodyMode { freq: 290.0, q: 30.0, gain: 0.4 },
    BodyMode { freq: 420.0, q: 35.0, gain: 0.3 },
    BodyMode { freq: 650.0, q: 40.0, gain: 0.22 },
    BodyMode { freq: 980.0, q: 45.0, gain: 0.16 },
    BodyMode { freq: 1500.0, q: 50.0, gain: 0.1 },
];

#[rustfmt::skip]
pub const BODY_MODES_BASS_R: [BodyMode; 8] = [
    BodyMode { freq: 61.8, q: 24.25, gain: 1.236 },
    BodyMode { freq: 116.4, q: 22.66, gain: 0.927 },
    BodyMode { freq: 200.85, q: 24.25, gain: 0.618 },
    BodyMode { freq: 281.3, q: 30.9, gain: 0.412 },
    BodyMode { freq: 432.6, q: 33.95, gain: 0.309 },
    BodyMode { freq: 630.5, q: 41.2, gain: 0.2266 },
    BodyMode { freq: 1009.4, q: 43.65, gain: 0.1648 },
    BodyMode { freq: 1455.0, q: 51.5, gain: 0.103 },
];

#[rustfmt::skip]
pub const BODY_MODES_GUITAR_STEEL_L: [BodyMode; 8] = [
    BodyMode { freq: 100.0, q: 32.0, gain: 1.0 },
    BodyMode { freq: 215.0, q: 28.0, gain: 0.85 },
    BodyMode { freq: 300.0, q: 22.0, gain: 0.55 },
    BodyMode { freq: 440.0, q: 38.0, gain: 0.45 },
    BodyMode { freq: 620.0, q: 42.0, gain: 0.4 },
    BodyMode { freq: 920.0, q: 48.0, gain: 0.32 },
    BodyMode { freq: 1500.0, q: 55.0, gain: 0.28 },
    BodyMode { freq: 2500.0, q: 65.0, gain: 0.22 },
];

#[rustfmt::skip]
pub const BODY_MODES_GUITAR_STEEL_R: [BodyMode; 8] = [
    BodyMode { freq: 105.0, q: 30.4, gain: 1.05 },
    BodyMode { freq: 204.25, q: 29.4, gain: 0.8925 },
    BodyMode { freq: 315.0, q: 20.9, gain: 0.5775 },
    BodyMode { freq: 418.0, q: 39.9, gain: 0.4725 },
    BodyMode { freq: 651.0, q: 39.9, gain: 0.42 },
    BodyMode { freq: 874.0, q: 50.4, gain: 0.336 },
    BodyMode { freq: 1575.0, q: 52.25, gain: 0.294 },
    BodyMode { freq: 2375.0, q: 68.25, gain: 0.231 },
];

#[rustfmt::skip]
pub const BODY_MODES_SITAR_L: [BodyMode; 8] = [
    BodyMode { freq: 130.0, q: 30.0, gain: 0.7 },
    BodyMode { freq: 240.0, q: 35.0, gain: 0.6 },
    BodyMode { freq: 380.0, q: 60.0, gain: 0.5 },
    BodyMode { freq: 560.0, q: 70.0, gain: 0.45 },
    BodyMode { freq: 820.0, q: 80.0, gain: 0.4 },
    BodyMode { freq: 1200.0, q: 90.0, gain: 0.35 },
    BodyMode { freq: 1750.0, q: 100.0, gain: 0.3 },
    BodyMode { freq: 2500.0, q: 110.0, gain: 0.25 },
];

#[rustfmt::skip]
pub const BODY_MODES_SITAR_R: [BodyMode; 8] = [
    BodyMode { freq: 140.4, q: 27.6, gain: 0.756 },
    BodyMode { freq: 220.8, q: 37.8, gain: 0.648 },
    BodyMode { freq: 410.4, q: 55.2, gain: 0.54 },
    BodyMode { freq: 515.2, q: 75.6, gain: 0.486 },
    BodyMode { freq: 885.6, q: 73.6, gain: 0.432 },
    BodyMode { freq: 1104.0, q: 97.2, gain: 0.378 },
    BodyMode { freq: 1890.0, q: 92.0, gain: 0.324 },
    BodyMode { freq: 2300.0, q: 118.8, gain: 0.27 },
];

#[rustfmt::skip]
pub const BODY_MODES_PIANO_L: [BodyMode; 16] = [
    BodyMode { freq: 55.0, q: 10.0, gain: 1.0 },
    BodyMode { freq: 110.0, q: 12.0, gain: 0.85 },
    BodyMode { freq: 175.0, q: 15.0, gain: 0.7 },
    BodyMode { freq: 280.0, q: 18.0, gain: 0.55 },
    BodyMode { freq: 460.0, q: 22.0, gain: 0.45 },
    BodyMode { freq: 750.0, q: 28.0, gain: 0.35 },
    BodyMode { freq: 1300.0, q: 35.0, gain: 0.28 },
    BodyMode { freq: 2200.0, q: 40.0, gain: 0.22 },
    BodyMode { freq: 3200.0, q: 45.0, gain: 0.36 },
    BodyMode { freq: 4500.0, q: 50.0, gain: 0.28 },
    BodyMode { freq: 6200.0, q: 55.0, gain: 0.22 },
    BodyMode { freq: 8500.0, q: 60.0, gain: 0.18 },
    BodyMode { freq: 11000.0, q: 65.0, gain: 0.14 },
    BodyMode { freq: 13500.0, q: 70.0, gain: 0.12 },
    BodyMode { freq: 16000.0, q: 75.0, gain: 0.1 },
    BodyMode { freq: 19000.0, q: 80.0, gain: 0.08 },
];

#[rustfmt::skip]
pub const BODY_MODES_PIANO_R: [BodyMode; 16] = [
    BodyMode { freq: 57.75, q: 9.5, gain: 1.05 },
    BodyMode { freq: 104.5, q: 12.6, gain: 0.8925 },
    BodyMode { freq: 183.75, q: 14.25, gain: 0.735 },
    BodyMode { freq: 266.0, q: 18.9, gain: 0.5775 },
    BodyMode { freq: 483.0, q: 20.9, gain: 0.4725 },
    BodyMode { freq: 712.5, q: 29.4, gain: 0.3675 },
    BodyMode { freq: 1365.0, q: 33.25, gain: 0.294 },
    BodyMode { freq: 2090.0, q: 42.0, gain: 0.231 },
    BodyMode { freq: 3360.0, q: 42.75, gain: 0.378 },
    BodyMode { freq: 4275.0, q: 52.5, gain: 0.294 },
    BodyMode { freq: 6510.0, q: 52.25, gain: 0.231 },
    BodyMode { freq: 8075.0, q: 63.0, gain: 0.189 },
    BodyMode { freq: 11550.0, q: 61.75, gain: 0.147 },
    BodyMode { freq: 12825.0, q: 73.5, gain: 0.126 },
    BodyMode { freq: 16800.0, q: 71.25, gain: 0.105 },
    BodyMode { freq: 18050.0, q: 84.0, gain: 0.084 },
];

// Phase 3 互換: Default kind の alias
pub const BODY_MODES_L: [BodyMode; 8] = BODY_MODES_DEFAULT_L;
pub const BODY_MODES_R: [BodyMode; 8] = BODY_MODES_DEFAULT_R;
pub const STEREO_SPREAD: f32 = STEREO_SPREAD_DEFAULT;

#[repr(u32)]
#[non_exhaustive]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum InstrumentKind {
    Default = 0,
    GuitarClassical = 1,
    Ukulele = 2,
    Mandolin = 3,
    Bass = 4,
    GuitarSteel = 5,
    Sitar = 6,
    Piano = 7,
}

impl InstrumentKind {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Default),
            1 => Some(Self::GuitarClassical),
            2 => Some(Self::Ukulele),
            3 => Some(Self::Mandolin),
            4 => Some(Self::Bass),
            5 => Some(Self::GuitarSteel),
            6 => Some(Self::Sitar),
            7 => Some(Self::Piano),
            _ => None,
        }
    }
}

pub const INSTRUMENT_KIND_COUNT: usize = 8;

pub const INHARMONICITY_B_PIANO: f32 = 0.00075;
pub const HAMMER_CUTOFF_LOW_PIANO: f32 = 800.0;
pub const HAMMER_CUTOFF_HIGH_PIANO: f32 = 6500.0;
pub const UNISON_DETUNE_CENTS_PIANO: f32 = 1.5;
pub const SYMPATHETIC_AMOUNT_PIANO: f32 = 1.0;

#[rustfmt::skip]
pub const INHARMONICITY_B_CURVE_PIANO: [f32; 88] = [
    0.00031, 0.00029, 0.00027, 0.00025, 0.00023, 0.00022, 0.00021, 0.0002,
    0.0002, 0.0002, 0.0002, 0.0002, 0.0002, 0.0002, 0.0002, 0.0002,
    0.00021, 0.00021, 0.00022, 0.00023, 0.00024, 0.00025, 0.00027, 0.00029,
    0.00031, 0.00034, 0.00037, 0.00041, 0.00045, 0.0005, 0.00055, 0.00061,
    0.00067, 0.00075, 0.00083, 0.00092, 0.001, 0.0011, 0.0013, 0.0014,
    0.0016, 0.0018, 0.002, 0.0023, 0.0026, 0.0029, 0.0032, 0.0036,
    0.004, 0.0045, 0.005, 0.0056, 0.0063, 0.007, 0.0079, 0.0088,
    0.0099, 0.011, 0.012, 0.014, 0.016, 0.018, 0.02, 0.022,
    0.025, 0.028, 0.032, 0.035, 0.04, 0.045, 0.05, 0.056,
    0.063, 0.071, 0.08, 0.09, 0.1, 0.11, 0.13, 0.14,
    0.16, 0.18, 0.2, 0.22, 0.25, 0.28, 0.32, 0.36,
];

#[rustfmt::skip]
pub fn body_modes_for_instrument(
    kind: InstrumentKind,
) -> (&'static [BodyMode], &'static [BodyMode]) {
    match kind {
        InstrumentKind::Default => (&BODY_MODES_DEFAULT_L, &BODY_MODES_DEFAULT_R),
        InstrumentKind::GuitarClassical => (&BODY_MODES_GUITAR_CLASSICAL_L, &BODY_MODES_GUITAR_CLASSICAL_R),
        InstrumentKind::Ukulele => (&BODY_MODES_UKULELE_L, &BODY_MODES_UKULELE_R),
        InstrumentKind::Mandolin => (&BODY_MODES_MANDOLIN_L, &BODY_MODES_MANDOLIN_R),
        InstrumentKind::Bass => (&BODY_MODES_BASS_L, &BODY_MODES_BASS_R),
        InstrumentKind::GuitarSteel => (&BODY_MODES_GUITAR_STEEL_L, &BODY_MODES_GUITAR_STEEL_R),
        InstrumentKind::Sitar => (&BODY_MODES_SITAR_L, &BODY_MODES_SITAR_R),
        InstrumentKind::Piano => (&BODY_MODES_PIANO_L, &BODY_MODES_PIANO_R),
    }
}

pub fn stereo_spread_for_instrument(kind: InstrumentKind) -> f32 {
    match kind {
        InstrumentKind::Default => STEREO_SPREAD_DEFAULT,
        InstrumentKind::GuitarClassical => STEREO_SPREAD_GUITAR_CLASSICAL,
        InstrumentKind::Ukulele => STEREO_SPREAD_UKULELE,
        InstrumentKind::Mandolin => STEREO_SPREAD_MANDOLIN,
        InstrumentKind::Bass => STEREO_SPREAD_BASS,
        InstrumentKind::GuitarSteel => STEREO_SPREAD_GUITAR_STEEL,
        InstrumentKind::Sitar => STEREO_SPREAD_SITAR,
        InstrumentKind::Piano => STEREO_SPREAD_PIANO,
    }
}
