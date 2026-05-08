export type MidiInput = { id: string; name: string };

export type MidiNoteMessage =
	| { type: 'on'; midi: number; velocity: number }
	| { type: 'off'; midi: number };

let access: MIDIAccess | null = null;
let listener: ((msg: MidiNoteMessage) => void) | null = null;
let rawListener: ((data: Uint8Array) => void) | null = null;
let activeInputId: string | null = null;

export async function initMidi(onNote: (msg: MidiNoteMessage) => void): Promise<void> {
	if (!('requestMIDIAccess' in navigator)) {
		throw new Error('Web MIDI not supported');
	}
	listener = onNote;
	access = await navigator.requestMIDIAccess({ sysex: false });
	for (const input of access.inputs.values()) {
		input.onmidimessage = handleMidi;
	}
	access.onstatechange = (e) => {
		const port = e.port;
		if (port?.type === 'input' && port.state === 'connected') {
			(port as MIDIInput).onmidimessage = handleMidi;
		}
	};
}

export function disposeMidi(): void {
	if (!access) return;
	for (const input of access.inputs.values()) {
		input.onmidimessage = null;
	}
	access.onstatechange = null;
	access = null;
	listener = null;
	rawListener = null;
	activeInputId = null;
}

/**
 * Phase 3 D38 / D42: Note 以外の MIDI CC / Pitch Bend を受け取る生バイトリスナー。
 * `setRawListener(null)` で解除。
 */
export function setRawListener(cb: ((data: Uint8Array) => void) | null): void {
	rawListener = cb;
}

export function setActiveInput(id: string | null): void {
	activeInputId = id;
}

function handleMidi(e: MIDIMessageEvent) {
	if (!e.data) return;
	const port = e.target as MIDIInput | null;
	if (activeInputId !== null && port?.id !== activeInputId) return;

	// Phase 3 D38: 生バイトを CC / Pitch Bend ハンドラに転送 (Note 以外を処理)
	if (rawListener) {
		rawListener(e.data);
	}

	if (!listener) return;
	const status = e.data[0];
	const data1 = e.data[1] ?? 0;
	const data2 = e.data[2] ?? 0;
	const cmd = status & 0xf0;
	if (cmd === 0x90 && data2 > 0) {
		listener({ type: 'on', midi: data1, velocity: data2 / 127 });
	} else if (cmd === 0x80 || (cmd === 0x90 && data2 === 0)) {
		listener({ type: 'off', midi: data1 });
	}
}

export function listInputs(): MidiInput[] {
	if (!access) return [];
	return Array.from(access.inputs.values()).map((i) => ({
		id: i.id,
		name: i.name ?? 'Unknown'
	}));
}
