<script lang="ts">
	import { synth } from '$lib/state/synth.svelte';

	let starting = $state(false);
	let error = $state<string | null>(null);

	async function start() {
		if (synth.ready) return;
		starting = true;
		error = null;
		try {
			await synth.engine.start();
			synth.ready = true;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
			synth.ready = false;
		} finally {
			starting = false;
		}
	}
</script>

<div class="start-button">
	<button onclick={start} disabled={starting || synth.ready}>
		{#if starting}
			Starting...
		{:else if synth.ready}
			✓ Audio Ready
		{:else}
			▶ Start Audio
		{/if}
	</button>
	{#if error}
		<small class="error">{error}</small>
	{/if}
</div>

<style>
	.start-button {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	button {
		padding: 0.5rem 1rem;
		font-size: 1rem;
		cursor: pointer;
	}
	button:disabled {
		cursor: default;
	}
	.error {
		color: #c00;
	}
</style>
