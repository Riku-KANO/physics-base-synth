<script lang="ts">
	import { onMount } from 'svelte';
	import {
		initMidi,
		disposeMidi,
		listInputs,
		setActiveInput,
		setRawListener,
		type MidiInput
	} from '$lib/input/midi';
	import { handleMidiMessage } from '$lib/input/midi-cc';
	import { synth } from '$lib/state/synth.svelte';

	let supported = $state(false);
	let inputs: MidiInput[] = $state([]);
	let selectedId: string | null = $state(null);

	onMount(() => {
		supported = 'requestMIDIAccess' in navigator && (window?.isSecureContext ?? false);
		if (!supported) return;

		let alive = true;

		initMidi((msg) => {
			if (msg.type === 'on') synth.engine.noteOn(msg.midi, msg.velocity);
			else synth.engine.noteOff(msg.midi);
		})
			.then(() => {
				if (!alive) return;
				// Phase 3 D38 / D42: MIDI CC / Pitch Bend を SynthEngine に橋渡し
				setRawListener((data) => handleMidiMessage(data, synth.engine));
				inputs = listInputs();
			})
			.catch((e: unknown) => {
				if (!alive) return;
				console.warn('[MIDI] init failed:', e);
				supported = false;
			});

		return () => {
			alive = false;
			disposeMidi();
		};
	});

	$effect(() => {
		setActiveInput(selectedId);
	});
</script>

{#if supported}
	<label class="midi-select">
		MIDI Device:
		<select bind:value={selectedId}>
			<option value={null}>(all inputs)</option>
			{#each inputs as i (i.id)}
				<option value={i.id}>{i.name}</option>
			{/each}
		</select>
	</label>
{:else}
	<small>
		Web MIDI is unavailable. Requires HTTPS/localhost and Chrome/Edge (or Firefox 126+).
	</small>
{/if}

<style>
	.midi-select {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	select {
		padding: 0.25rem;
	}
</style>
