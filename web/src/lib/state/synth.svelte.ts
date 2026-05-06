import { SynthEngine } from '$lib/audio/engine';

class SynthState {
	readonly engine = new SynthEngine();
	ready = $state(false);
	damping = $state(0.996);
	brightness = $state(0.5);
	outputGain = $state(0.8);
}

export const synth = new SynthState();
