// Phase 3 D38 / D42: WebMIDI の Control Change / Pitch Bend を SynthEngine 経路に橋渡し。
// `MidiSelect.svelte` の `onmidimessage` から呼ばれる薄いハンドラ。

import type { SynthEngine } from '$lib/audio/engine';

const PITCH_BEND_RANGE_SEMITONES = 2;

let lastPitchBend14: number | null = null;

/**
 * MIDI バイト列を SynthEngine の MIDI CC / Pitch Bend / Note 経路に分配する。
 * Note On/Off は呼び元 (MidiSelect) が引き続き直接担当するため、ここでは
 * Control Change (0xb0) と Pitch Bend (0xe0) のみ処理する。
 */
export function handleMidiMessage(data: Uint8Array, engine: SynthEngine): boolean {
	if (data.length === 0) return false;
	const status = data[0];
	const cmd = status & 0xf0;
	if (cmd === 0xb0 && data.length >= 3) {
		const cc = data[1];
		const value = data[2];
		// CC#1 (Mod Wheel) は Phase 4 送り、CC#7 / #64 / #123 のみ WASM 側で処理
		engine.sendMidiCc(cc, value);
		return true;
	}
	if (cmd === 0xe0 && data.length >= 3) {
		const lsb = data[1] & 0x7f;
		const msb = data[2] & 0x7f;
		const combined14 = (msb << 7) | lsb; // 0..16383、center = 8192
		if (combined14 === lastPitchBend14) return true;
		lastPitchBend14 = combined14;
		const normalized = (combined14 - 8192) / 8192; // -1..+1（境界で僅かに非対称）
		engine.sendPitchBend(normalized * PITCH_BEND_RANGE_SEMITONES);
		return true;
	}
	return false;
}
