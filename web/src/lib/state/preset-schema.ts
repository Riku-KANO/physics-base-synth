// Phase 4a D50: Preset v1 schema 型定義 + validator + factory。
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

const VALID_INSTRUMENTS: InstrumentKindKey[] = [
	'default',
	'guitar_classical',
	'ukulele',
	'mandolin',
	'bass',
	'guitar_steel',
	'sitar',
	'piano' // Phase 4b D62
];
const VALID_WAVEFORMS: LfoWaveformKey[] = ['sine', 'triangle'];

// 各 Param の値域 (params.json と同期)
const PARAM_RANGES = {
	damping: { min: 0.9, max: 0.9999 },
	brightness: { min: 0.0, max: 1.0 },
	outputGain: { min: 0.0, max: 1.5 },
	pickPosition: { min: 0.05, max: 0.5 },
	bodyWet: { min: 0.0, max: 1.0 }
} as const;

const LFO_RANGES = {
	rate: { min: 0.1, max: 8.0 },
	pitchDepth: { min: 0.0, max: 1.0 },
	brightnessDepth: { min: 0.0, max: 1.0 },
	volumeDepth: { min: 0.0, max: 1.0 }
} as const;

const MAX_NAME_LENGTH = 64;

function isFiniteInRange(v: unknown, min: number, max: number): boolean {
	return typeof v === 'number' && Number.isFinite(v) && v >= min && v <= max;
}

/** 受信した unknown オブジェクトが PresetV1 として valid か検証 (型 + 有限性 + 値域)。
 *  Schema-level の制約のみを担当。Factory 名衝突 / User 上限などの store-specific
 *  制約は呼び元 (PresetStore.save) でチェック。
 */
export function isValidPresetV1(obj: unknown): obj is PresetV1 {
	if (!obj || typeof obj !== 'object') return false;
	const p = obj as Record<string, unknown>;
	if (p.version !== 1) return false;
	if (typeof p.name !== 'string' || p.name.length === 0 || p.name.length > MAX_NAME_LENGTH)
		return false;
	if (typeof p.createdAt !== 'string') return false;
	if (typeof p.instrument !== 'string') return false;
	if (!VALID_INSTRUMENTS.includes(p.instrument as InstrumentKindKey)) return false;

	if (!p.params || typeof p.params !== 'object') return false;
	const pp = p.params as Record<string, unknown>;
	if (!isFiniteInRange(pp.damping, PARAM_RANGES.damping.min, PARAM_RANGES.damping.max))
		return false;
	if (!isFiniteInRange(pp.brightness, PARAM_RANGES.brightness.min, PARAM_RANGES.brightness.max))
		return false;
	if (!isFiniteInRange(pp.outputGain, PARAM_RANGES.outputGain.min, PARAM_RANGES.outputGain.max))
		return false;
	if (
		!isFiniteInRange(pp.pickPosition, PARAM_RANGES.pickPosition.min, PARAM_RANGES.pickPosition.max)
	)
		return false;
	if (!isFiniteInRange(pp.bodyWet, PARAM_RANGES.bodyWet.min, PARAM_RANGES.bodyWet.max))
		return false;

	if (!p.lfo || typeof p.lfo !== 'object') return false;
	const pl = p.lfo as Record<string, unknown>;
	if (!isFiniteInRange(pl.rate, LFO_RANGES.rate.min, LFO_RANGES.rate.max)) return false;
	if (typeof pl.waveform !== 'string' || !VALID_WAVEFORMS.includes(pl.waveform as LfoWaveformKey))
		return false;
	if (!isFiniteInRange(pl.pitchDepth, LFO_RANGES.pitchDepth.min, LFO_RANGES.pitchDepth.max))
		return false;
	if (
		!isFiniteInRange(
			pl.brightnessDepth,
			LFO_RANGES.brightnessDepth.min,
			LFO_RANGES.brightnessDepth.max
		)
	)
		return false;
	if (!isFiniteInRange(pl.volumeDepth, LFO_RANGES.volumeDepth.min, LFO_RANGES.volumeDepth.max))
		return false;

	return true;
}

export function getDefaultPreset(): PresetV1 {
	return {
		version: 1,
		name: 'Default',
		createdAt: new Date().toISOString(),
		instrument: 'default',
		params: {
			damping: 0.996,
			brightness: 0.5,
			outputGain: 0.8,
			pickPosition: 0.125,
			bodyWet: 0.5
		},
		lfo: {
			rate: 5.0,
			waveform: 'sine',
			pitchDepth: 0.0,
			brightnessDepth: 0.0,
			volumeDepth: 0.0
		}
	};
}
