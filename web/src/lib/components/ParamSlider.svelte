<script lang="ts">
	import { synth } from '$lib/state/synth.svelte';
	import { getDescriptor, type ParamIdValue } from '$lib/audio/messages';

	type Props = {
		label: string;
		paramId: ParamIdValue;
		step: number;
		value: number;
	};

	let { label, paramId, step, value = $bindable() }: Props = $props();

	const descriptor = $derived(getDescriptor(paramId));

	function onInput(e: Event) {
		const v = parseFloat((e.target as HTMLInputElement).value);
		value = v;
		synth.engine.setParam(paramId, v);
	}
</script>

<label class="param-slider">
	<span class="label">{label}</span>
	<input type="range" min={descriptor.min} max={descriptor.max} {step} {value} oninput={onInput} />
	<span class="value">{value.toFixed(3)}</span>
</label>

<style>
	.param-slider {
		display: grid;
		grid-template-columns: 8rem 1fr 4rem;
		align-items: center;
		gap: 0.5rem;
	}
	.label {
		font-weight: 500;
	}
	input[type='range'] {
		width: 100%;
	}
	.value {
		font-variant-numeric: tabular-nums;
		text-align: right;
		color: #555;
	}
</style>
