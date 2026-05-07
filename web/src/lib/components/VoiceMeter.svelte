<script lang="ts">
	import { voiceState } from '$lib/audio/voice-state.svelte';

	const NUM_VOICES = 8;
	const cells = Array.from({ length: NUM_VOICES }, (_, i) => i);
</script>

<div class="voice-meter" aria-label="Voice activity meter">
	{#each cells as i (i)}
		{@const active = voiceState.isActive(i)}
		{@const amp = voiceState.amplitudes[i] ?? 0}
		{@const brightness = active ? Math.min(1, amp * 4) : 0}
		<span
			class="cell"
			class:active
			style="background: rgba(80, 200, 120, {brightness.toFixed(3)});"
			title={`voice ${i}: ${active ? 'on' : 'off'} (${amp.toFixed(3)})`}
		></span>
	{/each}
</div>

<style>
	.voice-meter {
		display: inline-flex;
		gap: 2px;
		padding: 2px;
		border: 1px solid #ccc;
		border-radius: 4px;
		background: #1a1a1a;
	}
	.cell {
		width: 12px;
		height: 12px;
		display: inline-block;
		border-radius: 2px;
		background: rgba(80, 200, 120, 0);
		transition: background-color 50ms linear;
	}
	.cell.active {
		outline: 1px solid rgba(80, 200, 120, 0.6);
	}
</style>
