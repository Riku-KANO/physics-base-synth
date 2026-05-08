// Phase 4a D50: Preset v1 schema 型定義 (Step 11 先行追加)。
// Step 12 で `isValidPresetV1` / `getDefaultPreset` 等の validator / factory 関数を追加する。
// LfoWaveformKey / InstrumentKindKey は messages.ts で定義済み (Worklet との型統一)。

export type { LfoWaveformKey, InstrumentKindKey } from '$lib/audio/messages';
import type { LfoWaveformKey, InstrumentKindKey } from '$lib/audio/messages';

export interface PresetV1 {
	version: 1;
	name: string;
	/** ISO 8601, e.g. "2026-05-08T12:34:56.789Z" */
	createdAt: string;
	instrument: InstrumentKindKey;
	params: {
		damping: number;
		brightness: number;
		outputGain: number;
		pickPosition: number;
		bodyWet: number;
	};
	lfo: {
		rate: number;
		waveform: LfoWaveformKey;
		pitchDepth: number;
		brightnessDepth: number;
		volumeDepth: number;
	};
}
