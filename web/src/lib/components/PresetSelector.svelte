<script lang="ts">
	import { onMount } from 'svelte';
	import { synth } from '$lib/state/synth.svelte';
	import { presetStore } from '$lib/state/preset-store.svelte';
	import type { PresetV1 } from '$lib/state/preset-schema';

	let saveName = $state('');

	onMount(() => {
		presetStore.load();
		// 起動時に最後に選択された Preset を復元 (UI state を無条件で同期)。
		// engine.applyPreset は ready 前でも currentParams / currentLfo / currentInstrument
		// を更新する設計のため、起動前から呼んで OK。Start Audio 後の start() 末尾で
		// resendPhase4aState() が走り、Worklet にも反映される。
		const lastPreset = presetStore.findByName(presetStore.currentPresetName);
		if (lastPreset) {
			synth.engine.applyPreset(lastPreset);
			applyPresetToUiState(lastPreset);
		}
	});

	function applyPresetToUiState(preset: PresetV1): void {
		synth.damping = preset.params.damping;
		synth.brightness = preset.params.brightness;
		synth.outputGain = preset.params.outputGain;
		synth.pickPosition = preset.params.pickPosition;
		synth.bodyWet = preset.params.bodyWet;
		synth.lfoRate = preset.lfo.rate;
		synth.lfoWaveform = preset.lfo.waveform;
		synth.lfoPitchDepth = preset.lfo.pitchDepth;
		synth.lfoBrightnessDepth = preset.lfo.brightnessDepth;
		synth.lfoVolumeDepth = preset.lfo.volumeDepth;
		synth.instrument = preset.instrument;
	}

	function handleSelect(e: Event) {
		const name = (e.target as HTMLSelectElement).value;
		if (name) {
			presetStore.apply(name, synth.engine);
			const preset = presetStore.findByName(name);
			if (preset) applyPresetToUiState(preset);
		}
	}

	function handleSave() {
		if (saveName.trim().length === 0) return;
		const preset = presetStore.capturePreset(saveName.trim(), {
			instrument: synth.instrument,
			params: {
				damping: synth.damping,
				brightness: synth.brightness,
				outputGain: synth.outputGain,
				pickPosition: synth.pickPosition,
				bodyWet: synth.bodyWet
			},
			lfo: {
				rate: synth.lfoRate,
				waveform: synth.lfoWaveform,
				pitchDepth: synth.lfoPitchDepth,
				brightnessDepth: synth.lfoBrightnessDepth,
				volumeDepth: synth.lfoVolumeDepth
			}
		});
		presetStore.save(preset);
		saveName = '';
	}

	function handleDelete() {
		if (confirm(`Delete preset "${presetStore.currentPresetName}"?`)) {
			presetStore.delete(presetStore.currentPresetName);
			// 削除後はデフォルトに戻す
			presetStore.apply('Default', synth.engine);
			const defaultPreset = presetStore.findByName('Default');
			if (defaultPreset) applyPresetToUiState(defaultPreset);
		}
	}
</script>

<section class="preset">
	<label>
		<span>Preset</span>
		<select value={presetStore.currentPresetName} onchange={handleSelect} disabled={!synth.ready}>
			<optgroup label="Factory">
				{#each presetStore.factoryPresets as p (p.name)}
					<option value={p.name}>{p.name}</option>
				{/each}
			</optgroup>
			{#if presetStore.userPresets.length > 0}
				<optgroup label="User">
					{#each presetStore.userPresets as p (p.name)}
						<option value={p.name}>{p.name}</option>
					{/each}
				</optgroup>
			{/if}
		</select>
	</label>

	<div class="actions">
		<input
			type="text"
			placeholder="New preset name"
			bind:value={saveName}
			maxlength="32"
			disabled={!synth.ready}
		/>
		<button onclick={handleSave} disabled={!synth.ready || saveName.trim().length === 0}>
			Save
		</button>
		<button
			onclick={handleDelete}
			disabled={!synth.ready ||
				presetStore.factoryPresets.some((p) => p.name === presetStore.currentPresetName)}
		>
			Delete
		</button>
	</div>

	{#if presetStore.errorMessage}
		<p class="error">{presetStore.errorMessage}</p>
	{/if}
	<small class="hint">楽器を切り替えると現在の音は止まります。</small>
</section>

<style>
	.preset {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		border: 1px solid #ccc;
		border-radius: 4px;
		padding: 0.75rem;
	}
	.preset label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	.actions {
		display: flex;
		gap: 0.5rem;
	}
	.actions input {
		flex: 1;
	}
	.error {
		color: #c00;
		font-size: 0.85rem;
		margin: 0;
	}
	.hint {
		color: #666;
		font-size: 0.8rem;
	}
</style>
