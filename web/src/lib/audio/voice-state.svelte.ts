// Phase 3 D41 / F34: Voice Meter UI 表示用の共有 state。
// SynthEngine が Worklet からの `voiceState` メッセージを受信して更新、
// VoiceMeter コンポーネントが `$state` 経由で再描画する。

const NUM_VOICES = 8;

export class VoiceState {
	activeMask = $state(0);
	amplitudes = $state(new Float32Array(NUM_VOICES));

	isActive(index: number): boolean {
		return ((this.activeMask >> index) & 1) === 1;
	}
}

export const voiceState = new VoiceState();
