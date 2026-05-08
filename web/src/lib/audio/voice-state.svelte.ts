const NUM_VOICES = 8;

// `Float32Array<ArrayBufferLike>` を明示することで、postMessage で構造化複製された
// `Float32Array<ArrayBuffer>` も `Float32Array<SharedArrayBuffer>` も代入可能にする。
type AmpView = Float32Array<ArrayBufferLike>;

export class VoiceState {
	activeMask = $state(0);
	amplitudes: AmpView = $state(new Float32Array(NUM_VOICES));

	isActive(index: number): boolean {
		return ((this.activeMask >> index) & 1) === 1;
	}
}

export const voiceState = new VoiceState();
