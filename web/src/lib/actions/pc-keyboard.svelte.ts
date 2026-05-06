import type { Action } from 'svelte/action';

const MAPPING: Record<string, number> = {
	KeyA: 60,
	KeyW: 61,
	KeyS: 62,
	KeyE: 63,
	KeyD: 64,
	KeyF: 65,
	KeyT: 66,
	KeyG: 67,
	KeyY: 68,
	KeyH: 69,
	KeyU: 70,
	KeyJ: 71,
	KeyK: 72,
	KeyO: 73,
	KeyL: 74
};

export type PcKeyboardNote =
	| { type: 'on'; midi: number; velocity: number }
	| { type: 'off'; midi: number };

export interface PcKeyboardParams {
	onNote: (msg: PcKeyboardNote) => void;
}

export const pcKeyboard: Action<HTMLElement, PcKeyboardParams> = (_node, params) => {
	$effect(() => {
		// heldKeys is effect-local, never read by reactive context — plain Set is intentional
		// eslint-disable-next-line svelte/prefer-svelte-reactivity
		const heldKeys = new Set<string>();

		const onDown = (e: KeyboardEvent) => {
			if (e.repeat) return;
			const midi = MAPPING[e.code];
			if (midi === undefined) return;
			if (heldKeys.has(e.code)) return;
			heldKeys.add(e.code);
			params.onNote({ type: 'on', midi, velocity: 0.8 });
		};
		const onUp = (e: KeyboardEvent) => {
			const midi = MAPPING[e.code];
			if (midi === undefined) return;
			heldKeys.delete(e.code);
			params.onNote({ type: 'off', midi });
		};

		window.addEventListener('keydown', onDown);
		window.addEventListener('keyup', onUp);

		return () => {
			window.removeEventListener('keydown', onDown);
			window.removeEventListener('keyup', onUp);
			heldKeys.clear();
		};
	});
};
