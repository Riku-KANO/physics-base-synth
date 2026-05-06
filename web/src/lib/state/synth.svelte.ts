import { SynthEngine } from '$lib/audio/engine';
import { PARAM_DESCRIPTORS, PARAM_IDS } from '$lib/audio/messages';

class SynthState {
	readonly engine = new SynthEngine();
	ready = $state(false);
	damping = $state(PARAM_DESCRIPTORS[PARAM_IDS.Damping].default);
	brightness = $state(PARAM_DESCRIPTORS[PARAM_IDS.Brightness].default);
	outputGain = $state(PARAM_DESCRIPTORS[PARAM_IDS.OutputGain].default);
}

export const synth = new SynthState();

// F18 / F20 の DevTools Console 検証用に mono/poly 切替を露出する。
// `import.meta.env.DEV` は本番ビルドで定数 false に解決されブロック全体が tree-shake で
// 除去されるため、本番バンドルに __synthDev は出ない。
if (import.meta.env.DEV) {
	type DevDiagnostics = {
		setMode: (mode: 'poly' | 'mono') => void;
	};
	(globalThis as unknown as { __synthDev?: DevDiagnostics }).__synthDev = {
		setMode: (mode) => synth.engine.setMode(mode)
	};
}
