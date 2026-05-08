<script lang="ts">
	import { synth } from '$lib/state/synth.svelte';
	import { ui } from '$lib/state/ui.svelte';

	function setMode(mode: 'poly' | 'mono') {
		ui.polyphonyMode = mode;
		synth.engine.setMode(mode);
	}
</script>

<fieldset class="polyphony-toggle">
	<legend>Mode</legend>
	<label>
		<input
			type="radio"
			name="polyphony"
			value="poly"
			checked={ui.polyphonyMode === 'poly'}
			onchange={() => setMode('poly')}
		/>
		Poly
	</label>
	<label>
		<input
			type="radio"
			name="polyphony"
			value="mono"
			checked={ui.polyphonyMode === 'mono'}
			onchange={() => setMode('mono')}
		/>
		Mono
	</label>
</fieldset>

<style>
	.polyphony-toggle {
		display: inline-flex;
		gap: 0.5rem;
		align-items: center;
		border: 1px solid #ccc;
		padding: 0.25rem 0.5rem;
		border-radius: 4px;
	}
	legend {
		padding: 0 0.25rem;
		font-size: 0.85em;
		color: #666;
	}
	label {
		display: inline-flex;
		gap: 0.25rem;
		align-items: center;
		font-size: 0.9em;
	}
</style>
