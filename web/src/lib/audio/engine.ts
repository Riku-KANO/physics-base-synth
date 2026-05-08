import { base } from '$app/paths';
import type {
	ToWorkletMessage,
	FromWorkletMessage,
	LfoWaveformKey,
	LfoDestinationKey,
	InstrumentKindKey
} from './messages';
import { PARAM_IDS } from './messages';
import type { PresetV1 } from '$lib/state/preset-schema';
import wasmUrl from '$lib/wasm/wasm_audio.wasm?url';
import { voiceState } from './voice-state.svelte';

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

	// Phase 4a: LFO / 楽器の現在値を保持 (Worklet 再起動時に再送するため)
	private currentLfo = {
		rate: 5.0,
		waveform: 'sine' as LfoWaveformKey,
		pitchDepth: 0,
		brightnessDepth: 0,
		volumeDepth: 0
	};
	private currentInstrument: InstrumentKindKey = 'default';

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
				const data = e.data;
				if (data.type === 'voiceState') {
					voiceState.activeMask = data.activeMask;
					voiceState.amplitudes = data.amplitudes;
					return;
				}
				const h = this._readyHandlers;
				if (data.type === 'debug') {
					console.warn('[Worklet]', data.message);
					return;
				}
				if (!h) return;
				if (data.type === 'ready' && !h.settled) {
					h.markSettled();
					this.ready = true;
					h.resolve();
				} else if (data.type === 'error' && !h.settled) {
					h.markSettled();
					console.error('[Worklet]', data.message);
					h.reject(new Error(data.message));
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
		// Phase 4a: LFO / 楽器選択も Worklet 初期化後に再送 (Phase 3 の Param 再送と同位置)
		this.resendPhase4aState();
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

	/**
	 * mono / poly 切替 (D17 / D21 / D42)。離散的なイベントなので rAF スロットルせず即時送信する。
	 */
	setMode(mode: 'poly' | 'mono'): void {
		if (!this.ready) return;
		this.post({ type: 'setMode', mode });
	}

	sendMidiCc(cc: number, value: number): void {
		if (!this.ready) return;
		const normalized = Math.max(0, Math.min(1, value / 127));
		this.post({ type: 'midiCC', cc, value: normalized });
	}

	sendPitchBend(semitones: number): void {
		if (!this.ready) return;
		this.post({ type: 'pitchBend', semitones });
	}

	// Phase 4a D46-D49: LFO setter 群。各 setter は state を保持してから ready チェック
	// (start 前に呼ばれた場合は currentLfo にだけ書いて、start 後の resendPhase4aState で送られる)。
	lfoSetRate(hz: number): void {
		this.currentLfo.rate = hz;
		if (!this.ready) return;
		this.post({ type: 'lfoSetRate', hz });
	}

	lfoSetWaveform(kind: LfoWaveformKey): void {
		this.currentLfo.waveform = kind;
		if (!this.ready) return;
		this.post({ type: 'lfoSetWaveform', kind });
	}

	lfoSetDepth(dest: LfoDestinationKey, depth: number): void {
		if (dest === 'pitch') this.currentLfo.pitchDepth = depth;
		else if (dest === 'brightness') this.currentLfo.brightnessDepth = depth;
		else if (dest === 'volume') this.currentLfo.volumeDepth = depth;
		if (!this.ready) return;
		this.post({ type: 'lfoSetDepth', dest, depth });
	}

	// Phase 4a D52: 楽器プリセット切替 (内部で全 voice release + Modal 再構築)
	applyInstrument(kind: InstrumentKindKey): void {
		this.currentInstrument = kind;
		if (!this.ready) return;
		this.post({ type: 'applyInstrument', kind });
	}

	// Phase 4a D50: プリセット一括適用。各個別 setter は ready 前でも state を保持し、
	// start() 後の resendPhase4aState で再送される。よって applyPreset 自体は早期 return しない。
	applyPreset(preset: PresetV1): void {
		// 1. 楽器切替 (全 voice release を伴うため最初)
		this.applyInstrument(preset.instrument);

		// 2. パラメータ適用 (既存の setParam 経路を流用)
		this.setParam(PARAM_IDS.Damping, preset.params.damping);
		this.setParam(PARAM_IDS.Brightness, preset.params.brightness);
		this.setParam(PARAM_IDS.OutputGain, preset.params.outputGain);
		this.setParam(PARAM_IDS.PickPosition, preset.params.pickPosition);
		this.setParam(PARAM_IDS.BodyWet, preset.params.bodyWet);

		// 3. LFO 適用
		this.lfoSetRate(preset.lfo.rate);
		this.lfoSetWaveform(preset.lfo.waveform);
		this.lfoSetDepth('pitch', preset.lfo.pitchDepth);
		this.lfoSetDepth('brightness', preset.lfo.brightnessDepth);
		this.lfoSetDepth('volume', preset.lfo.volumeDepth);
	}

	// Phase 4a: start() 成功後に LFO / 楽器の状態を Worklet に再送する。
	// Worklet 再初期化 / retry 時の整合性確保。
	private resendPhase4aState(): void {
		// 楽器を最初に送る (内部で all_notes_off + Modal 再構築)
		this.post({ type: 'applyInstrument', kind: this.currentInstrument });
		this.post({ type: 'lfoSetRate', hz: this.currentLfo.rate });
		this.post({ type: 'lfoSetWaveform', kind: this.currentLfo.waveform });
		this.post({ type: 'lfoSetDepth', dest: 'pitch', depth: this.currentLfo.pitchDepth });
		this.post({
			type: 'lfoSetDepth',
			dest: 'brightness',
			depth: this.currentLfo.brightnessDepth
		});
		this.post({ type: 'lfoSetDepth', dest: 'volume', depth: this.currentLfo.volumeDepth });
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
