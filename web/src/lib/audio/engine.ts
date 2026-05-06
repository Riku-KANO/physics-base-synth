import { base } from '$app/paths';
import type { ToWorkletMessage, FromWorkletMessage } from './messages';
import wasmUrl from '$lib/wasm/wasm_audio.wasm?url';

interface ReadyHandlers {
	resolve: () => void;
	reject: (e: Error) => void;
	readonly settled: boolean;
	markSettled: () => void;
}

const READY_TIMEOUT_MS = 5000;

export class SynthEngine {
	private ctx: AudioContext | null = null;
	private node: AudioWorkletNode | null = null;
	private ready = false;

	private currentParams = new Map<number, number>();
	private pendingParams = new Map<number, number>();
	private rafHandle: number | null = null;

	private _readyHandlers: ReadyHandlers | null = null;

	async start(): Promise<void> {
		if (this.ready) return;

		if (typeof window !== 'undefined' && !window.isSecureContext) {
			throw new Error('AudioWorklet requires a secure context. Use HTTPS or localhost.');
		}

		let timer: ReturnType<typeof setTimeout> | null = null;

		try {
			if (!this.ctx) {
				this.ctx = new AudioContext({ latencyHint: 'interactive' });
			}
			if (!this.ctx.audioWorklet) {
				throw new Error('AudioWorklet is not supported in this browser.');
			}
			if (this.ctx.state === 'suspended') {
				await this.ctx.resume();
			}

			if (!this.node) {
				await this.ctx.audioWorklet.addModule(`${base}/worklet/synth-processor.js`);
				this.node = new AudioWorkletNode(this.ctx, 'synth-processor', {
					numberOfInputs: 0,
					numberOfOutputs: 1,
					outputChannelCount: [2]
				});
				this.node.connect(this.ctx.destination);
			}

			const res = await fetch(wasmUrl);
			if (!res.ok) {
				throw new Error(`Failed to fetch ${wasmUrl}: HTTP ${res.status}`);
			}
			const wasmBytes = await res.arrayBuffer();

			let settled = false;
			const readyPromise = new Promise<void>((resolve, reject) => {
				this._readyHandlers = {
					resolve,
					reject,
					get settled() {
						return settled;
					},
					markSettled: () => {
						settled = true;
					}
				};
			});

			this.node.port.onmessage = (e: MessageEvent<FromWorkletMessage>) => {
				const h = this._readyHandlers;
				if (e.data.type === 'debug') {
					console.warn('[Worklet]', e.data.message);
					return;
				}
				if (!h) return;
				if (e.data.type === 'ready' && !h.settled) {
					h.markSettled();
					this.ready = true;
					h.resolve();
				} else if (e.data.type === 'error' && !h.settled) {
					h.markSettled();
					console.error('[Worklet]', e.data.message);
					h.reject(new Error(e.data.message));
				}
			};

			timer = setTimeout(() => {
				const h = this._readyHandlers;
				if (h && !h.settled) {
					h.markSettled();
					h.reject(new Error(`Worklet did not become ready within ${READY_TIMEOUT_MS}ms`));
				}
			}, READY_TIMEOUT_MS);

			this.post({ type: 'init', wasmBytes, sampleRate: this.ctx.sampleRate }, [wasmBytes]);

			await readyPromise;
		} catch (err) {
			if (timer !== null) clearTimeout(timer);
			this._readyHandlers = null;
			await this.resetForRetry();
			throw err;
		}

		if (timer !== null) clearTimeout(timer);
		this._readyHandlers = null;

		for (const [id, value] of this.currentParams) {
			this.post({ type: 'setParam', id, value });
		}
	}

	noteOn(midi: number, velocity: number): void {
		if (!this.ready) return;
		this.post({ type: 'noteOn', midi, velocity });
	}

	noteOff(midi: number): void {
		if (!this.ready) return;
		this.post({ type: 'noteOff', midi });
	}

	setParam(id: number, value: number): void {
		this.currentParams.set(id, value);
		if (!this.ready) return;
		this.pendingParams.set(id, value);
		if (this.rafHandle === null) {
			this.rafHandle = requestAnimationFrame(() => this.flushParams());
		}
	}

	private flushParams(): void {
		this.rafHandle = null;
		for (const [id, value] of this.pendingParams) {
			this.post({ type: 'setParam', id, value });
		}
		this.pendingParams.clear();
	}

	private post(msg: ToWorkletMessage, transfer: Transferable[] = []): void {
		if (!this.node) return;
		this.node.port.postMessage(msg, transfer);
	}

	isReady(): boolean {
		return this.ready;
	}

	async dispose(): Promise<void> {
		if (this.rafHandle !== null) {
			cancelAnimationFrame(this.rafHandle);
			this.rafHandle = null;
		}
		this.pendingParams.clear();

		if (this.node) {
			try {
				this.node.port.postMessage({ type: 'dispose' } satisfies ToWorkletMessage);
			} catch {
				/* */
			}
			this.node.port.onmessage = null;
			this.node.disconnect();
			this.node = null;
		}
		if (this.ctx && this.ctx.state !== 'closed') {
			try {
				await this.ctx.close();
			} catch {
				/* */
			}
		}
		this.ctx = null;
		this.ready = false;
	}

	private async resetForRetry(): Promise<void> {
		await this.dispose();
	}
}
