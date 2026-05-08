<script lang="ts">
	import { synth } from '$lib/state/synth.svelte';

	// 0..127 の MIDI CC value で UI 表示。内部的には 0..1 で SmoothedValue
	let cc127 = $derived(Math.round(synth.modWheel * 127));

	function handleInput(e: Event) {
		const v = Number((e.target as HTMLInputElement).value);
		synth.modWheel = Math.max(0, Math.min(127, v)) / 127;
		if (synth.engine.isReady()) {
			synth.engine.sendMidiCc(1, v); // CC#1 = Mod Wheel
		}
	}
</script>

<label class="mod-wheel">
	<span>Mod Wheel</span>
	<input
		type="range"
		min="0"
		max="127"
		step="1"
		value={cc127}
		oninput={handleInput}
		disabled={!synth.ready}
	/>
	<span class="value">{cc127}</span>
</label>

<style>
	.mod-wheel {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	.value {
		min-width: 2rem;
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
</style>
