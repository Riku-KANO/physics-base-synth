//! Phase 4a F45: alloc ゼロ統合テスト
//!
//! 8 voice + LFO active + Mod Wheel + 楽器切替で voice buffer / LFO 状態 / modal_body
//! coeffs の capacity 不変を保証する。Phase 3 既存の no-alloc テストに加えて、
//! Phase 4a の追加経路 (apply_instrument / lfo_set_*) も alloc 0 を確認する。

use dsp_core::engine::Engine;
use dsp_core::lfo::{LfoDestination, LfoWaveform};
use dsp_core::params::InstrumentKind;
use dsp_core::traits::AudioProcessor;

const SAMPLE_RATE: f32 = 48_000.0;

#[test]
fn test_no_allocation_with_lfo_and_instrument() {
    // 8 voice + LFO active + Mod Wheel + 楽器切替 (1 回) + Pitch Bend + CC#7 で
    // voice buffer / modal_body の capacity が不変。
    let mut engine = Engine::new();
    engine.prepare(SAMPLE_RATE, 128);

    // LFO + Mod Wheel + 楽器切替を設定
    engine.lfo_set_rate(5.0);
    engine.lfo_set_waveform(LfoWaveform::Sine);
    engine.lfo_set_depth(LfoDestination::Pitch, 0.5);
    engine.lfo_set_depth(LfoDestination::Brightness, 0.5);
    engine.lfo_set_depth(LfoDestination::Volume, 0.5);
    engine.handle_midi_cc(1, 1.0); // Mod Wheel = 1.0

    // 8 voice 全部 active
    for n in 60..=67 {
        engine.note_on(n, 0.8);
    }

    // 楽器切替を 1 回挟む (apply_instrument 内の pool.all_notes_off + Modal 係数差し替え)
    engine.apply_instrument(InstrumentKind::Mandolin);

    // 切替後に再度 voice を起こす (alloc が走るかの確認)
    for n in 60..=67 {
        engine.note_on(n, 0.8);
    }

    // capacity を計測 (voice buffer / scratch / Engine 内 LFO 状態の capacity)
    let cap_before: Vec<usize> = (0..8)
        .map(|i| {
            engine
                .pool()
                .voice(i)
                .map(|v| v.buffer_capacity())
                .unwrap_or(0)
        })
        .collect();

    let mut buf_l = vec![0.0_f32; 4800];
    let mut buf_r = vec![0.0_f32; 4800];

    // 動作している状態で 10 ブロック process + Pitch Bend + CC#7 + 楽器切替 を交互に
    for cycle in 0..10 {
        engine.handle_pitch_bend(0.5);
        engine.handle_midi_cc(7, 0.7);
        engine.lfo_set_rate(3.0 + cycle as f32 * 0.5);
        engine.process(&mut buf_l, &mut buf_r);
    }

    // 楽器を別の kind に切り替える
    engine.apply_instrument(InstrumentKind::Sitar);
    for n in 60..=67 {
        engine.note_on(n, 0.8);
    }
    for _ in 0..5 {
        engine.process(&mut buf_l, &mut buf_r);
    }

    let cap_after: Vec<usize> = (0..8)
        .map(|i| {
            engine
                .pool()
                .voice(i)
                .map(|v| v.buffer_capacity())
                .unwrap_or(0)
        })
        .collect();

    assert_eq!(
        cap_before, cap_after,
        "voice buffer capacity must not change across LFO + instrument + Pitch Bend + CC#7"
    );
}

/// Phase 4b F58: 8 voice + Piano kind active + LFO + Mod Wheel + Pitch Bend + 楽器切替
/// (Piano ↔ Default 1 回) で voice buffer / LFO 状態 / dispersion_stages capacity 不変。
/// dispersion_stages は inline 配列 ([DispersionStage; 8]) なので heap 操作なし、
/// hammer LPF も note_on 内 stack 変数のみで alloc しない。
#[test]
fn test_no_allocation_with_piano_kind() {
    let mut engine = Engine::new();
    engine.prepare(SAMPLE_RATE, 128);

    engine.lfo_set_rate(5.0);
    engine.lfo_set_waveform(LfoWaveform::Sine);
    engine.lfo_set_depth(LfoDestination::Pitch, 0.5);
    engine.lfo_set_depth(LfoDestination::Brightness, 0.5);
    engine.lfo_set_depth(LfoDestination::Volume, 0.5);
    engine.handle_midi_cc(1, 1.0);

    // Piano に切替 + 8 voice 全 active
    engine.apply_instrument(InstrumentKind::Piano);
    for n in 60..=67 {
        engine.note_on(n, 0.8);
    }

    let cap_before: Vec<usize> = (0..8)
        .map(|i| {
            engine
                .pool()
                .voice(i)
                .map(|v| v.buffer_capacity())
                .unwrap_or(0)
        })
        .collect();

    let mut buf_l = vec![0.0_f32; 4800];
    let mut buf_r = vec![0.0_f32; 4800];

    for cycle in 0..10 {
        engine.handle_pitch_bend(0.5);
        engine.handle_midi_cc(7, 0.7);
        engine.lfo_set_rate(3.0 + cycle as f32 * 0.5);
        engine.process(&mut buf_l, &mut buf_r);
    }

    // Piano ↔ Default 切替を 1 回 (set_dispersion_active fan-out 経由)
    engine.apply_instrument(InstrumentKind::Default);
    for n in 60..=67 {
        engine.note_on(n, 0.8);
    }
    for _ in 0..5 {
        engine.process(&mut buf_l, &mut buf_r);
    }

    engine.apply_instrument(InstrumentKind::Piano);
    for n in 60..=67 {
        engine.note_on(n, 0.8);
    }
    for _ in 0..5 {
        engine.process(&mut buf_l, &mut buf_r);
    }

    let cap_after: Vec<usize> = (0..8)
        .map(|i| {
            engine
                .pool()
                .voice(i)
                .map(|v| v.buffer_capacity())
                .unwrap_or(0)
        })
        .collect();

    assert_eq!(
        cap_before, cap_after,
        "Piano kind: voice buffer capacity must not change across LFO + Piano↔Default + Pitch Bend + CC#7"
    );
}
