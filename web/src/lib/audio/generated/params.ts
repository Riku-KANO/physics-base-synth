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

// Phase 3 D30 / D32: ModalBodyResonator の係数テーブル
export interface BodyMode {
	readonly freq: number;
	readonly q: number;
	readonly gain: number;
}

export const STEREO_SPREAD = 0.05;

export const BODY_MODES_L: readonly BodyMode[] = [
	{ freq: 105, q: 30, gain: 1 },
	{ freq: 200, q: 25, gain: 0.8 },
	{ freq: 280, q: 20, gain: 0.5 },
	{ freq: 420, q: 35, gain: 0.4 },
	{ freq: 580, q: 40, gain: 0.35 },
	{ freq: 850, q: 45, gain: 0.25 },
	{ freq: 1400, q: 50, gain: 0.2 },
	{ freq: 2300, q: 60, gain: 0.15 }
] as const;

export const BODY_MODES_R: readonly BodyMode[] = [
	{ freq: 110.25, q: 28.5, gain: 1.05 },
	{ freq: 190, q: 26.25, gain: 0.8400000000000001 },
	{ freq: 294, q: 19, gain: 0.525 },
	{ freq: 399, q: 36.75, gain: 0.42000000000000004 },
	{ freq: 609, q: 38, gain: 0.3675 },
	{ freq: 807.5, q: 47.25, gain: 0.2625 },
	{ freq: 1470, q: 47.5, gain: 0.21000000000000002 },
	{ freq: 2185, q: 63, gain: 0.1575 }
] as const;
