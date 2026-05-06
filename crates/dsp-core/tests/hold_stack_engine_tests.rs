//! Engine の mono モード hold_stack 連携テスト (F18 / F19 / F20)

use dsp_core::engine::{Engine, SynthMode};
use dsp_core::traits::AudioProcessor;

const SAMPLE_RATE: f32 = 48_000.0;
const C: u8 = 60;
const D: u8 = 62;

fn fresh_engine_mono() -> Engine {
    let mut e = Engine::new();
    e.prepare(SAMPLE_RATE, 128);
    e.set_mode(SynthMode::Mono);
    e
}

#[test]
fn test_hold_stack_last_note_priority() {
    // F18: mono モード C 押 → D 押 → D 離 → C 復帰 → C 離 → 無音
    let mut e = fresh_engine_mono();

    e.note_on(C, 0.8);
    assert!(e.voice_index_for_note(C).is_some());

    e.note_on(D, 0.8);
    assert!(e.voice_index_for_note(D).is_some());

    // D 離した時点で C が VoicePool 上に再励振される (same-note-replace で同じ slot)
    e.note_off(D);
    assert!(
        e.voice_index_for_note(C).is_some(),
        "C should be revived after releasing D in mono mode"
    );

    // C 離 → どこにも C / D がいない
    e.note_off(C);
    assert!(e.voice_index_for_note(C).is_none(), "C should be released");
    assert!(e.voice_index_for_note(D).is_none(), "D should be released");
}

#[test]
fn test_hold_stack_overflow_in_engine() {
    // F19: mono モードで 17 鍵を順次押下 → 最古 (60) が破棄、現在押下中のキーは残る。
    // MAX_HELD = 16
    let mut e = fresh_engine_mono();
    for n in 60..(60 + 17_u8) {
        e.note_on(n, 0.8);
    }
    // 最新 (76) が top: 離すと 75 が復帰
    e.note_off(76);
    assert!(
        e.voice_index_for_note(75).is_some(),
        "key 75 should revive after releasing 76"
    );

    // 最古 (60) はスタックから消えているので note_off しても 75 は active のまま
    e.note_off(60);
    assert!(
        e.voice_index_for_note(75).is_some(),
        "key 75 should still be active after note_off(60) (60 was overflow-dropped)"
    );
}

#[test]
fn test_synth_mode_switch_no_break() {
    // F20: Poly → Mono → Poly 切替時に hold_stack はクリアされるが、進行中の VoicePool
    // ボイスは消音されず process もクラッシュしない。
    let mut e = Engine::new();
    e.prepare(SAMPLE_RATE, 128);

    e.note_on(C, 0.8);
    e.note_on(D, 0.8);
    assert_eq!(e.active_voice_count(), 2);

    e.set_mode(SynthMode::Mono);
    assert_eq!(e.mode(), SynthMode::Mono);
    assert_eq!(
        e.active_voice_count(),
        2,
        "voices should remain active across mode switch"
    );

    e.set_mode(SynthMode::Poly);
    assert_eq!(e.mode(), SynthMode::Poly);
    assert_eq!(e.active_voice_count(), 2);

    let mut l = [0.0_f32; 128];
    let mut r = [0.0_f32; 128];
    e.process(&mut l, &mut r);
    for s in l.iter() {
        assert!(s.is_finite(), "non-finite sample after mode switch");
    }
}
