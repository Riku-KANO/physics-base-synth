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
