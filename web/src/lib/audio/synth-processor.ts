import type { ToWorkletMessage, FromWorkletMessage } from './messages';

declare const sampleRate: number;
declare const registerProcessor: (name: string, processor: new () => AudioWorkletProcessor) => void;

declare class AudioWorkletProcessor {
	readonly port: MessagePort;
	constructor();
	process(
		inputs: Float32Array[][],
		outputs: Float32Array[][],
		parameters: Record<string, Float32Array>
	): boolean;
}

interface WasmExports {
	memory: WebAssembly.Memory;
	synth_new: (sr: number, maxBlock: number) => number;
	synth_free: (ptr: number) => void;
	synth_note_on: (ptr: number, midi: number, vel: number) => void;
	synth_note_off: (ptr: number, midi: number) => void;
	synth_set_param: (ptr: number, id: number, value: number) => void;
	synth_reset: (ptr: number) => void;
	synth_out_l_ptr: (ptr: number) => number;
	synth_out_r_ptr: (ptr: number) => number;
	synth_capacity: (ptr: number) => number;
	synth_process_block: (ptr: number, frames: number) => void;
	synth_set_polyphony_mode: (ptr: number, mode: number) => void;
	// Phase 3 追加 (D38 / D39 / D41)
	synth_midi_cc: (ptr: number, cc: number, value: number) => void;
	synth_pitch_bend: (ptr: number, semitones: number) => void;
	synth_voice_state_ptr: (ptr: number) => number;
}

const FRAMES = 128;
const VOICE_STATE_BYTES = 33; // 1 byte mask + 8 voice × f32 (4 bytes)
const VOICE_STATE_STRIDE_FRAMES = 1024; // ≈ 21 ms @ 48kHz (D41)
const NUM_VOICES = 8;

class SynthProcessor extends AudioWorkletProcessor {
	private exports: WasmExports | null = null;
	private handlePtr = 0;
	private lPtr = 0;
	private rPtr = 0;
	private voiceStatePtr = 0;
	private cachedMemBuf: ArrayBuffer | SharedArrayBuffer | null = null;
	private leftView: Float32Array | null = null;
	private rightView: Float32Array | null = null;
	private voiceStateView: Uint8Array | null = null;
	private generation = 0;
	private warnedFrameLength = false;
	private framesSinceVoiceStatePush = 0;

	constructor() {
		super();
		this.port.onmessage = (e: MessageEvent<ToWorkletMessage>) => {
			void this.onMessage(e.data);
		};
	}

	private async onMessage(msg: ToWorkletMessage): Promise<void> {
		switch (msg.type) {
			case 'init':
				await this.initWasm(msg.wasmBytes, msg.sampleRate);
				break;
			case 'noteOn':
				this.exports?.synth_note_on(this.handlePtr, msg.midi, msg.velocity);
				break;
			case 'noteOff':
				this.exports?.synth_note_off(this.handlePtr, msg.midi);
				break;
			case 'setParam':
				this.exports?.synth_set_param(this.handlePtr, msg.id, msg.value);
				break;
			case 'setMode':
				this.exports?.synth_set_polyphony_mode(this.handlePtr, msg.mode === 'mono' ? 1 : 0);
				break;
			case 'midiCC':
				this.exports?.synth_midi_cc(this.handlePtr, msg.cc, msg.value);
				break;
			case 'pitchBend':
				this.exports?.synth_pitch_bend(this.handlePtr, msg.semitones);
				break;
			case 'reset':
				this.exports?.synth_reset(this.handlePtr);
				break;
			case 'dispose':
				this.disposeWasm();
				break;
		}
	}

	private disposeWasm(): void {
		this.generation++;
		if (this.exports && this.handlePtr !== 0) {
			this.exports.synth_free(this.handlePtr);
		}
		this.handlePtr = 0;
		this.lPtr = 0;
		this.rPtr = 0;
		this.voiceStatePtr = 0;
		this.cachedMemBuf = null;
		this.leftView = null;
		this.rightView = null;
		this.voiceStateView = null;
		this.exports = null;
		this.framesSinceVoiceStatePush = 0;
	}

	private async initWasm(bytes: ArrayBuffer, sr: number): Promise<void> {
		const myGen = ++this.generation;
		let localExports: WasmExports | null = null;
		let localHandle = 0;

		try {
			const imports: WebAssembly.Imports = { env: {} };
			const { instance } = await WebAssembly.instantiate(bytes, imports);
			if (myGen !== this.generation) return;

			localExports = instance.exports as unknown as WasmExports;
			localHandle = localExports.synth_new(sr, FRAMES);
			if (myGen !== this.generation) {
				localExports.synth_free(localHandle);
				return;
			}

			const localLPtr = localExports.synth_out_l_ptr(localHandle);
			const localRPtr = localExports.synth_out_r_ptr(localHandle);
			const localVsPtr = localExports.synth_voice_state_ptr(localHandle);
			const memBuf = localExports.memory.buffer;
			const localLeftView = new Float32Array(memBuf, localLPtr, FRAMES);
			const localRightView = new Float32Array(memBuf, localRPtr, FRAMES);
			const localVoiceStateView = new Uint8Array(memBuf, localVsPtr, VOICE_STATE_BYTES);

			if (myGen !== this.generation) {
				localExports.synth_free(localHandle);
				return;
			}

			this.exports = localExports;
			this.handlePtr = localHandle;
			this.lPtr = localLPtr;
			this.rPtr = localRPtr;
			this.voiceStatePtr = localVsPtr;
			this.cachedMemBuf = memBuf;
			this.leftView = localLeftView;
			this.rightView = localRightView;
			this.voiceStateView = localVoiceStateView;

			const ready: FromWorkletMessage = { type: 'ready' };
			this.port.postMessage(ready);
		} catch (e: unknown) {
			if (localExports && localHandle !== 0) {
				try {
					localExports.synth_free(localHandle);
				} catch {
					/* */
				}
			}
			if (myGen !== this.generation) return;
			const err: FromWorkletMessage = { type: 'error', message: String(e) };
			this.port.postMessage(err);
		}
	}

	private refreshViews(): void {
		if (!this.exports) return;
		const memBuf = this.exports.memory.buffer;
		this.cachedMemBuf = memBuf;
		this.leftView = new Float32Array(memBuf, this.lPtr, FRAMES);
		this.rightView = new Float32Array(memBuf, this.rPtr, FRAMES);
		this.voiceStateView = new Uint8Array(memBuf, this.voiceStatePtr, VOICE_STATE_BYTES);
	}

	private silence(out: Float32Array[]): void {
		out[0].fill(0);
		if (out[1]) out[1].fill(0);
	}

	private maybePushVoiceState(): void {
		this.framesSinceVoiceStatePush += FRAMES;
		if (this.framesSinceVoiceStatePush < VOICE_STATE_STRIDE_FRAMES) return;
		this.framesSinceVoiceStatePush = 0;
		if (!this.voiceStateView) return;
		// view が detach されていれば 0 になる
		if (this.voiceStateView.byteLength === 0) {
			this.refreshViews();
			if (!this.voiceStateView) return;
		}
		const mask = this.voiceStateView[0];
		// 8 振幅を Float32 として読む。view を新規確保せずに DataView 経由で読む。
		const amps = new Float32Array(NUM_VOICES);
		for (let i = 0; i < NUM_VOICES; i++) {
			const off = 1 + i * 4;
			// little-endian f32 を組み立て
			const b0 = this.voiceStateView[off];
			const b1 = this.voiceStateView[off + 1];
			const b2 = this.voiceStateView[off + 2];
			const b3 = this.voiceStateView[off + 3];
			const u32 = (b3 << 24) | (b2 << 16) | (b1 << 8) | b0;
			amps[i] = new Float32Array(new Uint32Array([u32 >>> 0]).buffer)[0];
		}
		const msg: FromWorkletMessage = {
			type: 'voiceState',
			activeMask: mask,
			amplitudes: amps
		};
		this.port.postMessage(msg);
	}

	process(_inputs: Float32Array[][], outputs: Float32Array[][]): boolean {
		const out = outputs[0];
		if (!out || !out[0]) return true;

		if (!this.exports || this.handlePtr === 0) {
			this.silence(out);
			return true;
		}
		const exports = this.exports;

		if (out[0].length !== FRAMES) {
			if (!this.warnedFrameLength) {
				this.warnedFrameLength = true;
				const warn: FromWorkletMessage = {
					type: 'debug',
					message: `Unexpected render quantum: ${out[0].length} (expected ${FRAMES}). Output silenced.`
				};
				this.port.postMessage(warn);
			}
			this.silence(out);
			return true;
		}

		exports.synth_process_block(this.handlePtr, FRAMES);

		if (this.cachedMemBuf !== exports.memory.buffer) {
			this.refreshViews();
		}

		if (this.leftView) out[0].set(this.leftView);
		if (out[1] && this.rightView) out[1].set(this.rightView);

		this.maybePushVoiceState();
		return true;
	}
}

void sampleRate;

registerProcessor('synth-processor', SynthProcessor);
