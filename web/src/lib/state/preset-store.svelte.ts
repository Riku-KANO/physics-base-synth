import { isValidPresetV1, type PresetV1 } from './preset-schema';
import { FACTORY_PRESETS } from './factory-presets';
import type { SynthEngine } from '$lib/audio/engine';

const STORAGE_KEY_LIST = 'physbase.preset.v1.list';
const STORAGE_KEY_PREFIX = 'physbase.preset.v1.';
const STORAGE_KEY_LAST = 'physbase.preset.v1.last';
export const MAX_USER_PRESETS = 32;

class PresetStore {
	readonly factoryPresets: ReadonlyArray<PresetV1> = FACTORY_PRESETS;
	userPresets = $state<PresetV1[]>([]);
	currentPresetName = $state<string>('Default');
	errorMessage = $state<string | null>(null);

	/** localStorage から User Preset を読み込み。bad data は skip。
	 *  STORAGE_KEY_LIST 不在時 (User preset 未保存) でも STORAGE_KEY_LAST は読む
	 *  (Factory preset だけを選択して保存しているケースに対応)。
	 *  stale な lastName (削除済み User preset / 存在しない名前) は findByName で検証し、
	 *  なければ 'Default' に fallback。
	 *  load() が再実行されても古い userPresets が残らないよう、`loaded` を常に新規配列で
	 *  作り、最後に必ず `this.userPresets = loaded` で上書きする。 */
	load(): void {
		const loaded: PresetV1[] = [];
		try {
			const listJson = localStorage.getItem(STORAGE_KEY_LIST);
			if (listJson) {
				let names: unknown = null;
				try {
					names = JSON.parse(listJson);
				} catch (e) {
					console.warn('[PresetStore] STORAGE_KEY_LIST JSON parse failed:', e);
				}
				if (Array.isArray(names)) {
					for (const name of names) {
						if (typeof name !== 'string') continue;
						const presetJson = localStorage.getItem(STORAGE_KEY_PREFIX + name);
						if (!presetJson) continue;
						try {
							const obj = JSON.parse(presetJson);
							if (isValidPresetV1(obj)) {
								loaded.push(obj);
							} else {
								console.warn(`[PresetStore] Invalid preset ${name}, skipping`);
							}
						} catch (e) {
							console.warn(`[PresetStore] Failed to parse preset ${name}:`, e);
						}
					}
				} else {
					console.warn('[PresetStore] STORAGE_KEY_LIST is not an array, ignoring');
				}
			}
			this.userPresets = loaded;

			const lastName = localStorage.getItem(STORAGE_KEY_LAST);
			this.currentPresetName = lastName && this.findByName(lastName) ? lastName : 'Default';
		} catch (e) {
			console.error('[PresetStore] load failed:', e);
			this.errorMessage = 'Failed to load presets from storage';
			this.userPresets = loaded;
			this.currentPresetName = 'Default';
		}
	}

	/** Factory + User すべてのプリセット名を返す */
	allPresetNames(): { factory: string[]; user: string[] } {
		return {
			factory: this.factoryPresets.map((p) => p.name),
			user: this.userPresets.map((p) => p.name)
		};
	}

	/** 名前から PresetV1 を取得 (Factory 優先) */
	findByName(name: string): PresetV1 | undefined {
		return (
			this.factoryPresets.find((p) => p.name === name) ??
			this.userPresets.find((p) => p.name === name)
		);
	}

	/** 現在の synth state からプリセット作成 (UI 操作値を参照) */
	capturePreset(
		name: string,
		snapshot: Omit<PresetV1, 'version' | 'name' | 'createdAt'>
	): PresetV1 {
		return {
			version: 1,
			name,
			createdAt: new Date().toISOString(),
			...snapshot
		};
	}

	/** User Preset を保存。
	 *  バリデーションは isValidPresetV1 に集約 (空名 / name.length > 64 / 値域外 / NaN /
	 *  Infinity / 不正な instrument / waveform を一括 reject)。
	 *  Store-specific の制約 (Factory 名衝突 / User 上限) は本メソッド内で別途チェック。 */
	save(preset: PresetV1): void {
		if (!isValidPresetV1(preset)) {
			this.errorMessage = 'Invalid preset (name length, range, or schema violation)';
			return;
		}
		if (this.factoryPresets.some((p) => p.name === preset.name)) {
			this.errorMessage = `Cannot use factory preset name: ${preset.name}`;
			return;
		}
		const existingIdx = this.userPresets.findIndex((p) => p.name === preset.name);
		if (existingIdx === -1 && this.userPresets.length >= MAX_USER_PRESETS) {
			this.errorMessage = `Preset slot full (max ${MAX_USER_PRESETS})`;
			return;
		}
		try {
			localStorage.setItem(STORAGE_KEY_PREFIX + preset.name, JSON.stringify(preset));
			if (existingIdx === -1) {
				this.userPresets = [...this.userPresets, preset];
			} else {
				this.userPresets = this.userPresets.map((p, i) => (i === existingIdx ? preset : p));
			}
			const names = this.userPresets.map((p) => p.name);
			localStorage.setItem(STORAGE_KEY_LIST, JSON.stringify(names));
			this.errorMessage = null;
		} catch (e) {
			console.error('[PresetStore] save failed:', e);
			this.errorMessage = 'Failed to save preset (storage quota exceeded?)';
		}
	}

	/** User Preset を削除 (Factory は削除不可) */
	delete(name: string): void {
		if (this.factoryPresets.some((p) => p.name === name)) {
			this.errorMessage = 'Cannot delete factory preset';
			return;
		}
		try {
			localStorage.removeItem(STORAGE_KEY_PREFIX + name);
			this.userPresets = this.userPresets.filter((p) => p.name !== name);
			const names = this.userPresets.map((p) => p.name);
			localStorage.setItem(STORAGE_KEY_LIST, JSON.stringify(names));
			this.errorMessage = null;
		} catch (e) {
			console.error('[PresetStore] delete failed:', e);
		}
	}

	/** プリセットを engine に適用 */
	apply(name: string, engine: SynthEngine): void {
		const preset = this.findByName(name);
		if (!preset) {
			this.errorMessage = `Preset not found: ${name}`;
			return;
		}
		engine.applyPreset(preset);
		this.currentPresetName = name;
		try {
			localStorage.setItem(STORAGE_KEY_LAST, name);
		} catch {
			/* localStorage failure は無視 (apply 自体は成功) */
		}
	}
}

export const presetStore = new PresetStore();
