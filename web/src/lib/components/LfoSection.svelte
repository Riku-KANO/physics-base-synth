<script lang="ts">
	import { synth } from '$lib/state/synth.svelte';
	import type { LfoWaveformKey } from '$lib/state/preset-schema';

	function setRate(e: Event) {
		const v = Number((e.target as HTMLInputElement).value);
		synth.lfoRate = v;
		if (synth.engine.isReady()) synth.engine.lfoSetRate(v);
	}

	function setWaveform(kind: LfoWaveformKey) {
		synth.lfoWaveform = kind;
		if (synth.engine.isReady()) synth.engine.lfoSetWaveform(kind);
	}

	function setDepth(dest: 'pitch' | 'brightness' | 'volume', e: Event) {
		const v = Number((e.target as HTMLInputElement).value);
		if (dest === 'pitch') synth.lfoPitchDepth = v;
		if (dest === 'brightness') synth.lfoBrightnessDepth = v;
		if (dest === 'volume') synth.lfoVolumeDepth = v;
		if (synth.engine.isReady()) synth.engine.lfoSetDepth(dest, v);
	}
</script>

<section class="lfo">
	<h2>LFO</h2>
	<label>
		<span>Rate</span>
		<input
			type="range"
			min="0.1"
			max="8.0"
			step="0.1"
			value={synth.lfoRate}
			oninput={setRate}
			disabled={!synth.ready}
		/>
		<span class="value">{synth.lfoRate.toFixed(1)} Hz</span>
	</label>

	<fieldset class="waveform">
		<legend>Waveform</legend>
		<label>
			<input
				type="radio"
				name="lfo-waveform"
				value="sine"
				checked={synth.lfoWaveform === 'sine'}
				onchange={() => setWaveform('sine')}
				disabled={!synth.ready}
			/>
			Sine
		</label>
		<label>
			<input
				type="radio"
				name="lfo-waveform"
				value="triangle"
				checked={synth.lfoWaveform === 'triangle'}
				onchange={() => setWaveform('triangle')}
				disabled={!synth.ready}
			/>
			Triangle
		</label>
	</fieldset>

	<label>
		<span>Pitch Depth</span>
		<input
			type="range"
			min="0"
			max="1"
			step="0.01"
			value={synth.lfoPitchDepth}
			oninput={(e) => setDepth('pitch', e)}
			disabled={!synth.ready}
		/>
		<span class="value">{synth.lfoPitchDepth.toFixed(2)}</span>
	</label>
	<label>
		<span>Brightness Depth</span>
		<input
			type="range"
			min="0"
			max="1"
			step="0.01"
			value={synth.lfoBrightnessDepth}
			oninput={(e) => setDepth('brightness', e)}
			disabled={!synth.ready}
		/>
		<span class="value">{synth.lfoBrightnessDepth.toFixed(2)}</span>
	</label>
	<label>
		<span>Volume Depth</span>
		<input
			type="range"
			min="0"
			max="1"
			step="0.01"
			value={synth.lfoVolumeDepth}
			oninput={(e) => setDepth('volume', e)}
			disabled={!synth.ready}
		/>
		<span class="value">{synth.lfoVolumeDepth.toFixed(2)}</span>
	</label>
</section>

<style>
	.lfo {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		border: 1px solid #ccc;
		border-radius: 4px;
		padding: 0.75rem;
	}
	.lfo h2 {
		margin: 0 0 0.5rem 0;
		font-size: 1rem;
	}
	.lfo label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	.lfo .value {
		min-width: 3rem;
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	.waveform {
		border: none;
		padding: 0;
		margin: 0;
		display: flex;
		gap: 1rem;
	}
	.waveform legend {
		margin-right: 0.5rem;
	}
</style>
