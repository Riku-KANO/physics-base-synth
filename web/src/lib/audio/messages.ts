export type ToWorkletMessage =
	| { type: 'init'; wasmBytes: ArrayBuffer; sampleRate: number }
	| { type: 'noteOn'; midi: number; velocity: number }
	| { type: 'noteOff'; midi: number }
	| { type: 'setParam'; id: number; value: number }
	| { type: 'reset' }
	| { type: 'dispose' };

export type FromWorkletMessage =
	| { type: 'ready' }
	| { type: 'error'; message: string }
	| { type: 'debug'; message: string };

export const PARAM_IDS = {
	Damping: 0,
	Brightness: 1,
	OutputGain: 2
} as const;

export type ParamIdValue = (typeof PARAM_IDS)[keyof typeof PARAM_IDS];
