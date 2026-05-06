export {
	PARAM_IDS,
	PARAM_DESCRIPTORS,
	getDescriptor,
	clampValue,
	type ParamIdValue,
	type ParamDescriptor
} from './generated/params';

export type ToWorkletMessage =
	| { type: 'init'; wasmBytes: ArrayBuffer; sampleRate: number }
	| { type: 'noteOn'; midi: number; velocity: number }
	| { type: 'noteOff'; midi: number }
	| { type: 'setParam'; id: number; value: number }
	| { type: 'setMode'; mode: 'poly' | 'mono' }
	| { type: 'reset' }
	| { type: 'dispose' };

export type FromWorkletMessage =
	| { type: 'ready' }
	| { type: 'error'; message: string }
	| { type: 'debug'; message: string };
