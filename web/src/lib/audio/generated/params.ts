// AUTO-GENERATED FROM params.json — DO NOT EDIT
// Run `pnpm gen:params` to regenerate.

export interface ParamDescriptor {
	readonly id: number;
	readonly name: string;
	readonly min: number;
	readonly max: number;
	readonly default: number;
	readonly smoothingTau: number;
}

export const PARAM_IDS = {
	Damping: 0,
	Brightness: 1,
	OutputGain: 2,
	PickPosition: 3,
	BodyWet: 4
} as const;

export type ParamIdValue = (typeof PARAM_IDS)[keyof typeof PARAM_IDS];

export const PARAM_DESCRIPTORS: readonly ParamDescriptor[] = [
	{ id: 0, name: 'Damping', min: 0.9, max: 0.9999, default: 0.996, smoothingTau: 0.02 },
	{ id: 1, name: 'Brightness', min: 0, max: 1, default: 0.5, smoothingTau: 0.02 },
	{ id: 2, name: 'OutputGain', min: 0, max: 1.5, default: 0.8, smoothingTau: 0.01 },
	{ id: 3, name: 'PickPosition', min: 0.05, max: 0.5, default: 0.125, smoothingTau: 0.05 },
	{ id: 4, name: 'BodyWet', min: 0, max: 1, default: 0.5, smoothingTau: 0.02 }
] as const;

export function getDescriptor(id: ParamIdValue): ParamDescriptor {
	return PARAM_DESCRIPTORS[id];
}

export function clampValue(id: ParamIdValue, value: number): number {
	const d = PARAM_DESCRIPTORS[id];
	return value < d.min ? d.min : value > d.max ? d.max : value;
}

// Phase 3 D30 / D32 + Phase 4a D52 / D54: ModalBodyResonator の係数テーブル
export interface BodyMode {
	readonly freq: number;
	readonly q: number;
	readonly gain: number;
}

export const STEREO_SPREAD_DEFAULT = 0.05;

export const BODY_MODES_DEFAULT_L: readonly BodyMode[] = [
	{ freq: 105, q: 30, gain: 1 },
	{ freq: 200, q: 25, gain: 0.8 },
	{ freq: 280, q: 20, gain: 0.5 },
	{ freq: 420, q: 35, gain: 0.4 },
	{ freq: 580, q: 40, gain: 0.35 },
	{ freq: 850, q: 45, gain: 0.25 },
	{ freq: 1400, q: 50, gain: 0.2 },
	{ freq: 2300, q: 60, gain: 0.15 }
] as const;

export const BODY_MODES_DEFAULT_R: readonly BodyMode[] = [
	{ freq: 110.25, q: 28.5, gain: 1.05 },
	{ freq: 190, q: 26.25, gain: 0.8400000000000001 },
	{ freq: 294, q: 19, gain: 0.525 },
	{ freq: 399, q: 36.75, gain: 0.42000000000000004 },
	{ freq: 609, q: 38, gain: 0.3675 },
	{ freq: 807.5, q: 47.25, gain: 0.2625 },
	{ freq: 1470, q: 47.5, gain: 0.21000000000000002 },
	{ freq: 2185, q: 63, gain: 0.1575 }
] as const;

export const STEREO_SPREAD_GUITAR_CLASSICAL = 0.05;

export const BODY_MODES_GUITAR_CLASSICAL_L: readonly BodyMode[] = [
	{ freq: 105, q: 30, gain: 1 },
	{ freq: 200, q: 25, gain: 0.8 },
	{ freq: 280, q: 20, gain: 0.5 },
	{ freq: 420, q: 35, gain: 0.4 },
	{ freq: 580, q: 40, gain: 0.35 },
	{ freq: 850, q: 45, gain: 0.25 },
	{ freq: 1400, q: 50, gain: 0.2 },
	{ freq: 2300, q: 60, gain: 0.15 }
] as const;

export const BODY_MODES_GUITAR_CLASSICAL_R: readonly BodyMode[] = [
	{ freq: 110.25, q: 28.5, gain: 1.05 },
	{ freq: 190, q: 26.25, gain: 0.8400000000000001 },
	{ freq: 294, q: 19, gain: 0.525 },
	{ freq: 399, q: 36.75, gain: 0.42000000000000004 },
	{ freq: 609, q: 38, gain: 0.3675 },
	{ freq: 807.5, q: 47.25, gain: 0.2625 },
	{ freq: 1470, q: 47.5, gain: 0.21000000000000002 },
	{ freq: 2185, q: 63, gain: 0.1575 }
] as const;

export const STEREO_SPREAD_UKULELE = 0.04;

export const BODY_MODES_UKULELE_L: readonly BodyMode[] = [
	{ freq: 200, q: 18, gain: 0.9 },
	{ freq: 380, q: 20, gain: 0.7 },
	{ freq: 540, q: 22, gain: 0.45 },
	{ freq: 780, q: 28, gain: 0.35 },
	{ freq: 1100, q: 32, gain: 0.3 },
	{ freq: 1600, q: 38, gain: 0.22 },
	{ freq: 2200, q: 42, gain: 0.18 },
	{ freq: 3100, q: 50, gain: 0.12 }
] as const;

export const BODY_MODES_UKULELE_R: readonly BodyMode[] = [
	{ freq: 208, q: 17.28, gain: 0.936 },
	{ freq: 364.8, q: 20.8, gain: 0.728 },
	{ freq: 561.6, q: 21.119999999999997, gain: 0.468 },
	{ freq: 748.8, q: 29.12, gain: 0.364 },
	{ freq: 1144, q: 30.72, gain: 0.312 },
	{ freq: 1536, q: 39.52, gain: 0.2288 },
	{ freq: 2288, q: 40.32, gain: 0.1872 },
	{ freq: 2976, q: 52, gain: 0.1248 }
] as const;

export const STEREO_SPREAD_MANDOLIN = 0.06;

export const BODY_MODES_MANDOLIN_L: readonly BodyMode[] = [
	{ freq: 145, q: 25, gain: 0.85 },
	{ freq: 260, q: 28, gain: 0.7 },
	{ freq: 410, q: 32, gain: 0.5 },
	{ freq: 620, q: 40, gain: 0.4 },
	{ freq: 920, q: 48, gain: 0.35 },
	{ freq: 1450, q: 60, gain: 0.3 },
	{ freq: 2100, q: 70, gain: 0.25 },
	{ freq: 2900, q: 75, gain: 0.2 }
] as const;

export const BODY_MODES_MANDOLIN_R: readonly BodyMode[] = [
	{ freq: 153.70000000000002, q: 23.5, gain: 0.901 },
	{ freq: 244.39999999999998, q: 29.68, gain: 0.742 },
	{ freq: 434.6, q: 30.08, gain: 0.53 },
	{ freq: 582.8, q: 42.400000000000006, gain: 0.42400000000000004 },
	{ freq: 975.2, q: 45.12, gain: 0.371 },
	{ freq: 1363, q: 63.6, gain: 0.318 },
	{ freq: 2226, q: 65.8, gain: 0.265 },
	{ freq: 2726, q: 79.5, gain: 0.21200000000000002 }
] as const;

export const STEREO_SPREAD_BASS = 0.03;

export const BODY_MODES_BASS_L: readonly BodyMode[] = [
	{ freq: 60, q: 25, gain: 1.2 },
	{ freq: 120, q: 22, gain: 0.9 },
	{ freq: 195, q: 25, gain: 0.6 },
	{ freq: 290, q: 30, gain: 0.4 },
	{ freq: 420, q: 35, gain: 0.3 },
	{ freq: 650, q: 40, gain: 0.22 },
	{ freq: 980, q: 45, gain: 0.16 },
	{ freq: 1500, q: 50, gain: 0.1 }
] as const;

export const BODY_MODES_BASS_R: readonly BodyMode[] = [
	{ freq: 61.800000000000004, q: 24.25, gain: 1.236 },
	{ freq: 116.39999999999999, q: 22.66, gain: 0.927 },
	{ freq: 200.85, q: 24.25, gain: 0.618 },
	{ freq: 281.3, q: 30.900000000000002, gain: 0.41200000000000003 },
	{ freq: 432.6, q: 33.949999999999996, gain: 0.309 },
	{ freq: 630.5, q: 41.2, gain: 0.2266 },
	{ freq: 1009.4, q: 43.65, gain: 0.1648 },
	{ freq: 1455, q: 51.5, gain: 0.10300000000000001 }
] as const;

export const STEREO_SPREAD_GUITAR_STEEL = 0.05;

export const BODY_MODES_GUITAR_STEEL_L: readonly BodyMode[] = [
	{ freq: 100, q: 32, gain: 1 },
	{ freq: 215, q: 28, gain: 0.85 },
	{ freq: 300, q: 22, gain: 0.55 },
	{ freq: 440, q: 38, gain: 0.45 },
	{ freq: 620, q: 42, gain: 0.4 },
	{ freq: 920, q: 48, gain: 0.32 },
	{ freq: 1500, q: 55, gain: 0.28 },
	{ freq: 2500, q: 65, gain: 0.22 }
] as const;

export const BODY_MODES_GUITAR_STEEL_R: readonly BodyMode[] = [
	{ freq: 105, q: 30.4, gain: 1.05 },
	{ freq: 204.25, q: 29.400000000000002, gain: 0.8925 },
	{ freq: 315, q: 20.9, gain: 0.5775000000000001 },
	{ freq: 418, q: 39.9, gain: 0.47250000000000003 },
	{ freq: 651, q: 39.9, gain: 0.42000000000000004 },
	{ freq: 874, q: 50.400000000000006, gain: 0.336 },
	{ freq: 1575, q: 52.25, gain: 0.29400000000000004 },
	{ freq: 2375, q: 68.25, gain: 0.231 }
] as const;

export const STEREO_SPREAD_SITAR = 0.08;

export const BODY_MODES_SITAR_L: readonly BodyMode[] = [
	{ freq: 130, q: 30, gain: 0.7 },
	{ freq: 240, q: 35, gain: 0.6 },
	{ freq: 380, q: 60, gain: 0.5 },
	{ freq: 560, q: 70, gain: 0.45 },
	{ freq: 820, q: 80, gain: 0.4 },
	{ freq: 1200, q: 90, gain: 0.35 },
	{ freq: 1750, q: 100, gain: 0.3 },
	{ freq: 2500, q: 110, gain: 0.25 }
] as const;

export const BODY_MODES_SITAR_R: readonly BodyMode[] = [
	{ freq: 140.4, q: 27.6, gain: 0.756 },
	{ freq: 220.8, q: 37.800000000000004, gain: 0.648 },
	{ freq: 410.40000000000003, q: 55.2, gain: 0.54 },
	{ freq: 515.2, q: 75.60000000000001, gain: 0.48600000000000004 },
	{ freq: 885.6, q: 73.60000000000001, gain: 0.43200000000000005 },
	{ freq: 1104, q: 97.2, gain: 0.378 },
	{ freq: 1890.0000000000002, q: 92, gain: 0.324 },
	{ freq: 2300, q: 118.80000000000001, gain: 0.27 }
] as const;

export const STEREO_SPREAD_PIANO = 0.05;

export const BODY_MODES_PIANO_L: readonly BodyMode[] = [
	{ freq: 55, q: 10, gain: 1 },
	{ freq: 110, q: 12, gain: 0.85 },
	{ freq: 175, q: 15, gain: 0.7 },
	{ freq: 280, q: 18, gain: 0.55 },
	{ freq: 460, q: 22, gain: 0.45 },
	{ freq: 750, q: 28, gain: 0.35 },
	{ freq: 1300, q: 35, gain: 0.28 },
	{ freq: 2200, q: 40, gain: 0.22 }
] as const;

export const BODY_MODES_PIANO_R: readonly BodyMode[] = [
	{ freq: 57.75, q: 9.5, gain: 1.05 },
	{ freq: 104.5, q: 12.600000000000001, gain: 0.8925 },
	{ freq: 183.75, q: 14.25, gain: 0.735 },
	{ freq: 266, q: 18.900000000000002, gain: 0.5775000000000001 },
	{ freq: 483, q: 20.9, gain: 0.47250000000000003 },
	{ freq: 712.5, q: 29.400000000000002, gain: 0.3675 },
	{ freq: 1365, q: 33.25, gain: 0.29400000000000004 },
	{ freq: 2090, q: 42, gain: 0.231 }
] as const;

// Phase 3 互換: Default kind の alias
export const BODY_MODES_L = BODY_MODES_DEFAULT_L;
export const BODY_MODES_R = BODY_MODES_DEFAULT_R;
export const STEREO_SPREAD = STEREO_SPREAD_DEFAULT;

export const INSTRUMENT_KIND = {
	Default: 0,
	GuitarClassical: 1,
	Ukulele: 2,
	Mandolin: 3,
	Bass: 4,
	GuitarSteel: 5,
	Sitar: 6,
	Piano: 7
} as const;

export type InstrumentKindKey = keyof typeof INSTRUMENT_KIND;
export type InstrumentKindValue = (typeof INSTRUMENT_KIND)[InstrumentKindKey];

export const INSTRUMENT_KIND_COUNT = 8;

export const INHARMONICITY_B_PIANO = 0.00075;
export const HAMMER_CUTOFF_LOW_PIANO = 800;
export const HAMMER_CUTOFF_HIGH_PIANO = 5500;
export const UNISON_DETUNE_CENTS_PIANO = 1.5;
export const SYMPATHETIC_AMOUNT_PIANO = 1;

export const INHARMONICITY_B_CURVE_PIANO: readonly number[] = [
	0.00031, 0.00029, 0.00027, 0.00025, 0.00023, 0.00022, 0.00021, 0.0002, 0.0002, 0.0002, 0.0002,
	0.0002, 0.0002, 0.0002, 0.0002, 0.0002, 0.00021, 0.00021, 0.00022, 0.00023, 0.00024, 0.00025,
	0.00027, 0.00029, 0.00031, 0.00034, 0.00037, 0.00041, 0.00045, 0.0005, 0.00055, 0.00061, 0.00067,
	0.00075, 0.00083, 0.00092, 0.001, 0.0011, 0.0013, 0.0014, 0.0016, 0.0018, 0.002, 0.0023, 0.0026,
	0.0029, 0.0032, 0.0036, 0.004, 0.0045, 0.005, 0.0056, 0.0063, 0.007, 0.0079, 0.0088, 0.0099,
	0.011, 0.012, 0.014, 0.016, 0.018, 0.02, 0.022, 0.025, 0.028, 0.032, 0.035, 0.04, 0.045, 0.05,
	0.056, 0.063, 0.071, 0.08, 0.09, 0.1, 0.11, 0.13, 0.14, 0.16, 0.18, 0.2, 0.22, 0.25, 0.28, 0.32,
	0.36
] as const;
