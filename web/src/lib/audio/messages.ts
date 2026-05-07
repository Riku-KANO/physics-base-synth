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
	// Phase 3 D38: 汎用 MIDI CC dispatch (CC#7 / #64 / #123 のみ Worklet/WASM 側で処理)。
	// `value` は 0..1 へ正規化済み (呼び元で `cc_value / 127.0`)。
	| { type: 'midiCC'; cc: number; value: number }
	// Phase 3 D39: Pitch Bend を半音単位 (±2 まで) で送信。
	| { type: 'pitchBend'; semitones: number }
	| { type: 'reset' }
	| { type: 'dispose' };

export type FromWorkletMessage =
	| { type: 'ready' }
	| { type: 'error'; message: string }
	| { type: 'debug'; message: string }
	// Phase 3 D41: Voice State (active mask + 8 振幅、1024 sample 毎に push)。
	| { type: 'voiceState'; activeMask: number; amplitudes: Float32Array };
