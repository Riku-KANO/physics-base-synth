//! Engine の MIDI CC dispatch + Sustain Pedal 統合テスト (Phase 3 F31 / F33)

use dsp_core::engine::{Engine, SynthMode};
use dsp_core::params::ParamId;
use dsp_core::traits::AudioProcessor;

const SAMPLE_RATE: f32 = 48_000.0;

fn fresh_engine() -> Engine {
    let mut e = Engine::new();
    e.prepare(SAMPLE_RATE, 128);
    e
}

#[test]
fn test_engine_midi_cc_volume() {
    // CC#7 で channel_volume が変わり、output_gain は不変 (D38b 直交)
    let mut e = fresh_engine();
    let initial_gain = ParamId::OutputGain.descriptor().default;
    e.handle_midi_cc(7, 0.5);
    assert!(
        (e.channel_volume_target() - 0.5).abs() < 1e-6,
        "channel_volume target should be 0.5"
    );
    // output_gain は touch されない（OutputGain param で制御、D38b）
    e.set_param(ParamId::OutputGain as u32, initial_gain);
    // process が finite (panic / NaN なし) で完了する
    let mut l = vec![0.0_f32; 128];
    let mut r = vec![0.0_f32; 128];
    e.note_on(60, 0.8);
    e.process(&mut l, &mut r);
    assert!(l.iter().all(|s| s.is_finite()));
}

#[test]
fn test_engine_midi_cc_volume_multiplied_in_output() {
    // channel_volume=0 で出力が抑制される
    let mut e = fresh_engine();
    e.note_on(60, 0.8);
    e.handle_midi_cc(7, 0.0);
    let mut l = vec![0.0_f32; 4800]; // 100ms 経過で SmoothedValue 収束
    let mut r = vec![0.0_f32; 4800];
    e.process(&mut l, &mut r);
    let tail_max = l[3000..].iter().map(|x| x.abs()).fold(0.0_f32, f32::max);
    assert!(
        tail_max < 0.05,
        "channel_volume=0 should silence output, got max={}",
        tail_max
    );
}

#[test]
fn test_engine_midi_cc_sustain_defers() {
    // Poly mode: CC#64 on → note_off は defer、CC#64 off で release
    let mut e = fresh_engine();
    e.note_on(60, 0.8);
    e.handle_midi_cc(64, 1.0); // sustain on
    assert!(e.sustain_active());

    e.note_off(60);
    // pending bit が立つ、voice はまだ active (note_off による damping は適用されていない)
    assert!(e.sustain_pending_bitmap() & (1u128 << 60) != 0);

    e.handle_midi_cc(64, 0.0); // sustain off → pending を release
    assert!(!e.sustain_active());
    assert_eq!(e.sustain_pending_bitmap(), 0);
}

#[test]
fn test_engine_midi_cc_sustain_clears_pending_on_retrigger() {
    // P1-3: sustain on → note_off (pending) → 同 note を note_on で再励振 → CC#64 off で
    // 再打鍵後の voice は release されない
    let mut e = fresh_engine();
    e.note_on(60, 0.8);
    e.handle_midi_cc(64, 1.0);
    e.note_off(60);
    assert!(e.sustain_pending_bitmap() & (1u128 << 60) != 0);

    e.note_on(60, 0.8); // 再打鍵 → clear_pending(60)
    assert_eq!(e.sustain_pending_bitmap() & (1u128 << 60), 0);

    // pedal off で 60 が release されない（再打鍵分は鳴り続ける）
    e.handle_midi_cc(64, 0.0);
    // voice 60 は active のままで damping_target は 0.95 (note_off) ではない
    let idx = e.voice_index_for_note(60).expect("voice 60 must be active");
    let v = e.pool().voice(idx).expect("voice must exist");
    let dt = v.damping_target();
    assert!(
        dt > 0.95,
        "voice 60 should not be in release after retrigger: damping_target={}",
        dt
    );
}

#[test]
fn test_engine_midi_cc_all_notes_off_clears_sustain() {
    // P1-1: CC#123 で sustain も reset
    let mut e = fresh_engine();
    e.note_on(60, 0.8);
    e.handle_midi_cc(64, 1.0);
    e.note_off(60);
    assert!(e.sustain_pending_bitmap() != 0);

    e.handle_midi_cc(123, 1.0); // All Notes Off
    assert!(!e.sustain_active());
    assert_eq!(e.sustain_pending_bitmap(), 0);
}

#[test]
fn test_engine_mono_sustain_no_op() {
    // Mono mode では Sustain は無視（Phase 2 D29 既存挙動継承、P1-2）
    let mut e = fresh_engine();
    e.set_mode(SynthMode::Mono);
    e.note_on(60, 0.8);
    e.handle_midi_cc(64, 1.0);
    e.note_off(60);
    // Mono では note_off が即座に release を発火（pending には積まない、Phase 2 既存）
    // 注: try_defer_note_off は Poly でのみ呼ばれる実装のため、Mono では sustain_active 中でも
    //     pending には積まれず Phase 2 の hold_stack ロジックが働く
    let idx_opt = e.voice_index_for_note(60);
    if let Some(idx) = idx_opt {
        let v = e.pool().voice(idx).expect("voice must exist");
        // damping target が 0.95 (note_off) になっている
        assert!(
            (v.damping_target() - 0.95).abs() < 1e-6,
            "Mono: note_off should release voice immediately even with sustain on"
        );
    }
}

#[test]
fn test_engine_mode_switch_clears_sustain() {
    // P2-1: Poly + pending → set_mode(Mono) で pending 全 release
    let mut e = fresh_engine();
    e.note_on(60, 0.8);
    e.handle_midi_cc(64, 1.0);
    e.note_off(60);
    assert!(e.sustain_pending_bitmap() != 0);

    e.set_mode(SynthMode::Mono);
    assert_eq!(e.sustain_pending_bitmap(), 0);
    assert!(!e.sustain_active());
}

#[test]
fn test_engine_mode_switch_no_pending_passes_through() {
    // pending なしの set_mode は Phase 2 既存挙動と等価
    let mut e = fresh_engine();
    e.note_on(60, 0.8);
    e.set_mode(SynthMode::Mono);
    // Mono に切替後も既存の voice 60 は自然減衰、hold_stack はクリアされる
    assert_eq!(e.sustain_pending_bitmap(), 0);
}

#[test]
fn test_engine_midi_cc_unknown_ignored() {
    // 未対応 CC は no-op (panic / alloc なし)。
    // Phase 4a D49 で CC#1 (Mod Wheel) は no-op から実装済へ移行。
    let mut e = fresh_engine();
    e.handle_midi_cc(2, 0.5);
    e.handle_midi_cc(11, 0.5);
    e.handle_midi_cc(127, 0.5);
    // process が finite で完了
    e.note_on(60, 0.8);
    let mut l = vec![0.0_f32; 128];
    let mut r = vec![0.0_f32; 128];
    e.process(&mut l, &mut r);
    assert!(l.iter().all(|s| s.is_finite()));
}

#[test]
fn test_midi_cc_mod_wheel_sets_target() {
    // Phase 4a D49 / F41: CC#1 で mod_wheel.target() が更新される
    let mut e = fresh_engine();
    e.handle_midi_cc(1, 0.5);
    assert!(
        (e.mod_wheel_target() - 0.5).abs() < 1e-6,
        "CC#1 = 0.5 should set mod_wheel target to 0.5"
    );
    e.handle_midi_cc(1, 1.0);
    assert!(
        (e.mod_wheel_target() - 1.0).abs() < 1e-6,
        "CC#1 = 1.0 should set mod_wheel target to 1.0"
    );
    e.handle_midi_cc(1, 0.0);
    assert!(
        e.mod_wheel_target().abs() < 1e-6,
        "CC#1 = 0.0 should set mod_wheel target to 0.0"
    );
}

#[test]
fn test_midi_cc_mod_wheel_clamps_to_range() {
    // Phase 4a F41-b: CC#1 値が [0, 1] 範囲外でも 0..1 に clamp される (handle_midi_cc 入口で clamp)
    let mut e = fresh_engine();
    e.handle_midi_cc(1, 1.5);
    assert!(
        (e.mod_wheel_target() - 1.0).abs() < 1e-6,
        "CC#1 = 1.5 should clamp to 1.0"
    );
    e.handle_midi_cc(1, -0.5);
    assert!(
        e.mod_wheel_target().abs() < 1e-6,
        "CC#1 = -0.5 should clamp to 0.0"
    );
}
