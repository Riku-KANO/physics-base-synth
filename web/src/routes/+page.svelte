<script lang="ts">
	import { onDestroy } from 'svelte';
	import StartButton from '$lib/components/StartButton.svelte';
	import Keyboard from '$lib/components/Keyboard.svelte';
	import MidiSelect from '$lib/components/MidiSelect.svelte';
	import ParamSlider from '$lib/components/ParamSlider.svelte';
	import { pcKeyboard } from '$lib/actions/pc-keyboard.svelte';
	import { PARAM_IDS } from '$lib/audio/messages';
	import { synth } from '$lib/state/synth.svelte';

	let testNoteTimer: ReturnType<typeof setTimeout> | null = null;

	function testNoteC4() {
		synth.engine.noteOn(60, 0.8);
		if (testNoteTimer !== null) clearTimeout(testNoteTimer);
		testNoteTimer = setTimeout(() => {
			synth.engine.noteOff(60);
			testNoteTimer = null;
		}, 500);
	}

	onDestroy(() => {
		if (testNoteTimer !== null) clearTimeout(testNoteTimer);
		void synth.engine.dispose();
		synth.ready = false;
	});
</script>

<main
	use:pcKeyboard={{
		onNote: (m) => {
			if (m.type === 'on') synth.engine.noteOn(m.midi, m.velocity);
			else synth.engine.noteOff(m.midi);
		}
	}}
>
	<h1>Physics-Base Synth</h1>
	<StartButton />
	<MidiSelect />
	<button onclick={testNoteC4} disabled={!synth.ready}>Play C4 (test)</button>

	<section class="params">
		<ParamSlider
			label="Damping"
			paramId={PARAM_IDS.Damping}
			step={0.0001}
			bind:value={synth.damping}
		/>
		<ParamSlider
			label="Brightness"
			paramId={PARAM_IDS.Brightness}
			step={0.01}
			bind:value={synth.brightness}
		/>
		<ParamSlider
			label="Output Gain"
			paramId={PARAM_IDS.OutputGain}
			step={0.01}
			bind:value={synth.outputGain}
		/>
	</section>

	<Keyboard />
	<small class="hint">PC keyboard: A S D F G H J K (white) / W E T Y U O (black)</small>
</main>

<style>
	main {
		max-width: 720px;
		margin: 2rem auto;
		padding: 0 1rem;
		font-family: system-ui, sans-serif;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.hint {
		color: #666;
	}
	.params {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
</style>
