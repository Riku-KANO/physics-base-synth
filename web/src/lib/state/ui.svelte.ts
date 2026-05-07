// Phase 3 D42: mono / poly トグル用の UI state。
// `polyphonyMode` は `engine.setMode` で送る側（PolyphonyToggle）と
// 表示側（VoiceMeter / 他コンポーネント）で参照する。

class UiState {
	polyphonyMode = $state<'poly' | 'mono'>('poly');
}

export const ui = new UiState();
