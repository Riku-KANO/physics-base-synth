//! Voice stealing 戦略 (D13 / D28)。
//!
//! VoicePool::note_on の (1) same-note-replace と (2) 空きボイス検索で割当が決まらず、
//! 全ボイスがアクティブな状態で呼ばれる。戦略:
//!
//! 1. amplitude が `ENERGY_THRESHOLD_FOR_STEAL` 以下のボイスのうち最古 (age 最大) を選ぶ
//! 2. 該当なし (全ボイスがある程度の振幅で鳴っている) なら最古を選ぶ
//!
//! 閾値 + age の組み合わせを採るのは:
//!   (a) ほぼ無音のボイスを優先犠牲にすると知覚されにくい (D28)
//!   (b) 純粋な min amplitude だと「ごくわずかに静かなボイス」が連続して犠牲になり stealing
//!       偏りが起きる
//!   (c) 閾値以下を「ほぼ無音グループ」と扱い、その中で最古を選ぶことで知覚と公平性を両立

use crate::traits::Voice;

/// この値以下なら「ほぼ静か」とみなして優先的に犠牲にする
pub const ENERGY_THRESHOLD_FOR_STEAL: f32 = 1.0e-3;

pub fn select_voice_for_steal<V: Voice, const N: usize>(voices: &[V; N]) -> usize {
    let mut best_quiet: Option<(usize, u32)> = None;
    for (i, v) in voices.iter().enumerate() {
        if v.amplitude() < ENERGY_THRESHOLD_FOR_STEAL {
            match best_quiet {
                Some((_, best_age)) if v.age() <= best_age => {}
                _ => best_quiet = Some((i, v.age())),
            }
        }
    }
    if let Some((i, _)) = best_quiet {
        return i;
    }

    let mut best_oldest = (0_usize, voices[0].age());
    for (i, v) in voices.iter().enumerate().skip(1) {
        if v.age() > best_oldest.1 {
            best_oldest = (i, v.age());
        }
    }
    best_oldest.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用 Voice モック。stealing 戦略のロジックのみを検証する。
    #[derive(Default)]
    struct MockVoice {
        amp: f32,
        age_samples: u32,
        active: bool,
    }

    impl Voice for MockVoice {
        fn note_on(&mut self, _freq_hz: f32, _velocity: f32) {}
        fn note_off(&mut self) {}
        fn process_sample(&mut self) -> f32 {
            0.0
        }
        fn is_active(&self) -> bool {
            self.active
        }
        fn note_id(&self) -> Option<u8> {
            None
        }
        fn age(&self) -> u32 {
            self.age_samples
        }
        fn amplitude(&self) -> f32 {
            self.amp
        }
    }

    fn voice(amp: f32, age: u32) -> MockVoice {
        MockVoice {
            amp,
            age_samples: age,
            active: true,
        }
    }

    #[test]
    fn test_steal_picks_quietest_voice() {
        // 1 つだけ閾値以下、他は loud → 閾値以下を選ぶ
        let voices: [MockVoice; 4] = [
            voice(0.5, 100),
            voice(0.4, 200),
            voice(1.0e-5, 50), // ほぼ無音
            voice(0.6, 300),
        ];
        let i = select_voice_for_steal(&voices);
        assert_eq!(i, 2);
    }

    #[test]
    fn test_steal_falls_back_to_oldest() {
        // 全ボイスが閾値超 → 最古 (age 最大) を選ぶ
        let voices: [MockVoice; 4] = [
            voice(0.5, 100),
            voice(0.5, 500),
            voice(0.5, 200),
            voice(0.5, 300),
        ];
        let i = select_voice_for_steal(&voices);
        assert_eq!(i, 1);
    }

    #[test]
    fn test_steal_among_quiet_voices_picks_oldest() {
        // 複数ボイスが閾値以下 → そのうち age 最大を選ぶ
        let voices: [MockVoice; 4] = [
            voice(1.0e-5, 100),
            voice(1.0e-4, 800),
            voice(0.5, 200),
            voice(1.0e-6, 400),
        ];
        let i = select_voice_for_steal(&voices);
        assert_eq!(i, 1);
    }
}
