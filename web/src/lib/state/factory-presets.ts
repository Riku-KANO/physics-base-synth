import type { PresetV1 } from './preset-schema';

/**
 * Phase 4a D50 / D51: Factory Preset 7 種 (Default + 楽器 6 種)。
 * 編集不可、コードで管理 (再ビルドで更新)。
 * `createdAt` は const 値 (実機で時刻取得しない、再現性のため)。
 */
export const FACTORY_PRESETS: PresetV1[] = [
	{
		version: 1,
		name: 'Default',
		createdAt: '2026-05-08T00:00:00.000Z',
		instrument: 'default',
		params: { damping: 0.996, brightness: 0.5, outputGain: 0.8, pickPosition: 0.125, bodyWet: 0.5 },
		lfo: { rate: 5.0, waveform: 'sine', pitchDepth: 0.0, brightnessDepth: 0.0, volumeDepth: 0.0 }
	},
	{
		version: 1,
		name: 'Classical Guitar',
		createdAt: '2026-05-08T00:00:00.000Z',
		instrument: 'guitar_classical',
		params: { damping: 0.997, brightness: 0.45, outputGain: 0.8, pickPosition: 0.12, bodyWet: 0.6 },
		lfo: { rate: 5.0, waveform: 'sine', pitchDepth: 0.0, brightnessDepth: 0.0, volumeDepth: 0.0 }
	},
	{
		version: 1,
		name: 'Ukulele',
		createdAt: '2026-05-08T00:00:00.000Z',
		instrument: 'ukulele',
		params: {
			damping: 0.992,
			brightness: 0.65,
			outputGain: 0.85,
			pickPosition: 0.18,
			bodyWet: 0.55
		},
		lfo: { rate: 5.0, waveform: 'sine', pitchDepth: 0.0, brightnessDepth: 0.0, volumeDepth: 0.0 }
	},
	{
		version: 1,
		name: 'Mandolin',
		createdAt: '2026-05-08T00:00:00.000Z',
		instrument: 'mandolin',
		params: { damping: 0.994, brightness: 0.7, outputGain: 0.85, pickPosition: 0.1, bodyWet: 0.6 },
		lfo: { rate: 6.5, waveform: 'sine', pitchDepth: 0.3, brightnessDepth: 0.0, volumeDepth: 0.0 }
	},
	{
		version: 1,
		name: 'Acoustic Bass',
		createdAt: '2026-05-08T00:00:00.000Z',
		instrument: 'bass',
		params: { damping: 0.998, brightness: 0.3, outputGain: 0.9, pickPosition: 0.15, bodyWet: 0.5 },
		lfo: { rate: 4.0, waveform: 'sine', pitchDepth: 0.0, brightnessDepth: 0.0, volumeDepth: 0.0 }
	},
	{
		version: 1,
		name: 'Steel Guitar',
		createdAt: '2026-05-08T00:00:00.000Z',
		instrument: 'guitar_steel',
		params: { damping: 0.996, brightness: 0.6, outputGain: 0.8, pickPosition: 0.13, bodyWet: 0.55 },
		lfo: { rate: 5.0, waveform: 'sine', pitchDepth: 0.0, brightnessDepth: 0.0, volumeDepth: 0.0 }
	},
	{
		version: 1,
		name: 'Sitar',
		createdAt: '2026-05-08T00:00:00.000Z',
		instrument: 'sitar',
		params: {
			damping: 0.997,
			brightness: 0.55,
			outputGain: 0.85,
			pickPosition: 0.08,
			bodyWet: 0.7
		},
		lfo: { rate: 5.5, waveform: 'sine', pitchDepth: 0.4, brightnessDepth: 0.0, volumeDepth: 0.0 }
	},
	{
		// Phase 4c D72/D75/D77/D78 + R44 緩和策 1 (Step 18 pass 1): Piano
		// (Multi-string + Hertz hammer + Sympathetic + B(note) LUT + Modal Body M=16)。
		version: 1,
		name: 'Piano',
		createdAt: '2026-05-14T00:00:00.000Z',
		instrument: 'piano',
		params: {
			damping: 0.997,
			brightness: 0.6,
			outputGain: 0.7,
			pickPosition: 0.13,
			bodyWet: 0.55
		},
		lfo: { rate: 5.0, waveform: 'sine', pitchDepth: 0.0, brightnessDepth: 0.0, volumeDepth: 0.0 }
	}
];
