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
	OutputGain: 2
} as const;

export type ParamIdValue = (typeof PARAM_IDS)[keyof typeof PARAM_IDS];

export const PARAM_DESCRIPTORS: readonly ParamDescriptor[] = [
	{ id: 0, name: 'Damping', min: 0.9, max: 0.9999, default: 0.996, smoothingTau: 0.02 },
	{ id: 1, name: 'Brightness', min: 0, max: 1, default: 0.5, smoothingTau: 0.02 },
	{ id: 2, name: 'OutputGain', min: 0, max: 1.5, default: 0.8, smoothingTau: 0.01 }
] as const;

export function getDescriptor(id: ParamIdValue): ParamDescriptor {
	return PARAM_DESCRIPTORS[id];
}

export function clampValue(id: ParamIdValue, value: number): number {
	const d = PARAM_DESCRIPTORS[id];
	return value < d.min ? d.min : value > d.max ? d.max : value;
}
