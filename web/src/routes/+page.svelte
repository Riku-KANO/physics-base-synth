<script lang="ts">
	import { onDestroy } from 'svelte';
	import StartButton from '$lib/components/StartButton.svelte';
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

<main>
	<h1>Physics-Base Synth</h1>
	<StartButton />
	<button onclick={testNoteC4} disabled={!synth.ready}>Play C4 (test)</button>
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
</style>
