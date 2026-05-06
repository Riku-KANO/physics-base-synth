<script lang="ts">
	import { synth } from '$lib/state/synth.svelte';

	const startNote = 48;
	const endNote = 72;
	const notes = Array.from({ length: endNote - startNote + 1 }, (_, i) => startNote + i);

	const NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];

	function isBlack(midi: number): boolean {
		return [1, 3, 6, 8, 10].includes(midi % 12);
	}

	function noteName(midi: number): string {
		const octave = Math.floor(midi / 12) - 1;
		return `${NAMES[midi % 12]}${octave}`;
	}

	function down(e: PointerEvent, midi: number) {
		e.preventDefault();
		synth.engine.noteOn(midi, 0.8);
	}
	function up(e: PointerEvent, midi: number) {
		e.preventDefault();
		synth.engine.noteOff(midi);
	}
</script>

<div class="keyboard" role="group" aria-label="Onscreen keyboard">
	{#each notes as midi (midi)}
		<button
			type="button"
			class:black={isBlack(midi)}
			aria-label={noteName(midi)}
			onpointerdown={(e) => down(e, midi)}
			onpointerup={(e) => up(e, midi)}
			onpointerleave={(e) => up(e, midi)}
			oncontextmenu={(e) => e.preventDefault()}
		>
			<span>{noteName(midi)}</span>
		</button>
	{/each}
</div>

<style>
	.keyboard {
		display: flex;
		gap: 0;
		user-select: none;
		touch-action: none;
		position: relative;
		padding-bottom: 0.5rem;
	}
	button {
		flex: 1 1 0;
		height: 140px;
		min-width: 28px;
		background: #fff;
		border: 1px solid #888;
		border-radius: 0 0 4px 4px;
		font-size: 0.7rem;
		color: #333;
		cursor: pointer;
		display: flex;
		align-items: flex-end;
		justify-content: center;
		padding-bottom: 0.5rem;
		font-family: inherit;
	}
	button:active {
		background: #ffd;
	}
	button.black {
		background: #222;
		color: #fff;
		height: 90px;
		flex: 0 0 auto;
		width: 18px;
		margin: 0 -9px;
		z-index: 2;
		border: 1px solid #000;
		border-radius: 0 0 3px 3px;
	}
	button.black:active {
		background: #444;
	}
	button.black span {
		font-size: 0.55rem;
	}
</style>
