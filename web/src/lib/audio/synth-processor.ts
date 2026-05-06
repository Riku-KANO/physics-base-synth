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
}

const FRAMES = 128;

class SynthProcessor extends AudioWorkletProcessor {
	private exports: WasmExports | null = null;
	private handlePtr = 0;
	private lPtr = 0;
	private rPtr = 0;
	private cachedMemBuf: ArrayBuffer | SharedArrayBuffer | null = null;
	private leftView: Float32Array | null = null;
	private rightView: Float32Array | null = null;
	private generation = 0;
	private warnedFrameLength = false;

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
		this.cachedMemBuf = null;
		this.leftView = null;
		this.rightView = null;
		this.exports = null;
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
			const memBuf = localExports.memory.buffer;
			const localLeftView = new Float32Array(memBuf, localLPtr, FRAMES);
			const localRightView = new Float32Array(memBuf, localRPtr, FRAMES);

			if (myGen !== this.generation) {
				localExports.synth_free(localHandle);
				return;
			}

			this.exports = localExports;
			this.handlePtr = localHandle;
			this.lPtr = localLPtr;
			this.rPtr = localRPtr;
			this.cachedMemBuf = memBuf;
			this.leftView = localLeftView;
			this.rightView = localRightView;

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
	}

	private silence(out: Float32Array[]): void {
		out[0].fill(0);
		if (out[1]) out[1].fill(0);
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
		return true;
	}
}

// 開発時のみコンパイラに sampleRate / registerProcessor を露出するために参照（実行時は no-op）
void sampleRate;

registerProcessor('synth-processor', SynthProcessor);
