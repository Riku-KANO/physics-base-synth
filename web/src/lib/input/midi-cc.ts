import type { SynthEngine } from '$lib/audio/engine';
import { synth } from '$lib/state/synth.svelte';

const STATUS_MASK = 0xf0;
const STATUS_CONTROL_CHANGE = 0xb0;
const STATUS_PITCH_BEND = 0xe0;
const PITCH_BEND_CENTER = 8192;
const PITCH_BEND_RANGE_SEMITONES = 2;

// Phase 4a D49 / F41: WebMIDI 物理 Mod Wheel と UI スライダー同期用
const CC_MOD_WHEEL = 1;

let lastPitchBend14: number | null = null;

export function handleMidiMessage(data: Uint8Array, engine: SynthEngine): boolean {
	if (data.length === 0) return false;
	const cmd = data[0] & STATUS_MASK;
	if (cmd === STATUS_CONTROL_CHANGE && data.length >= 3) {
		const ccNum = data[1];
		const ccValue = data[2];
		engine.sendMidiCc(ccNum, ccValue);
		// Phase 4a: 物理 Mod Wheel を UI スライダーと同期 (D49 / F41)
		if (ccNum === CC_MOD_WHEEL) {
			synth.modWheel = ccValue / 127;
		}
		return true;
	}
	if (cmd === STATUS_PITCH_BEND && data.length >= 3) {
		const lsb = data[1] & 0x7f;
		const msb = data[2] & 0x7f;
		const combined14 = (msb << 7) | lsb;
		if (combined14 === lastPitchBend14) return true;
		lastPitchBend14 = combined14;
		const normalized = (combined14 - PITCH_BEND_CENTER) / PITCH_BEND_CENTER;
		engine.sendPitchBend(normalized * PITCH_BEND_RANGE_SEMITONES);
		return true;
	}
	return false;
}
