//! Engine::apply_instrument 統合テスト (Phase 4a D52 / D53 / D54、F43-b/c/e)

use dsp_core::engine::Engine;
use dsp_core::params::{
    stereo_spread_for_instrument, InstrumentKind, BODY_MODES_DEFAULT_L, STEREO_SPREAD_DEFAULT,
};
use dsp_core::traits::AudioProcessor;

const SAMPLE_RATE: f32 = 48_000.0;
const BLOCK_SIZE: usize = 128;

fn fresh_engine() -> Engine {
    let mut e = Engine::new();
    e.prepare(SAMPLE_RATE, BLOCK_SIZE);
    e
}

#[test]
fn test_apply_instrument_changes_modal_coeffs() {
    // Default → Ukulele で modal_body の係数が変わる
    let mut e = fresh_engine();
    let coeff_default = e.modal_body().coeff_l_b0(0);
    e.apply_instrument(InstrumentKind::Ukulele);
    let coeff_uke = e.modal_body().coeff_l_b0(0);
    assert!(
        (coeff_default - coeff_uke).abs() > 1e-6,
        "apply_instrument(Ukulele) should change modal coeff: default={coeff_default} ukulele={coeff_uke}"
    );
}

#[test]
fn test_apply_instrument_releases_all_voices() {
    // 8 voice active → apply_instrument → active_count == 0
    let mut e = fresh_engine();
    for n in 60..=67 {
        e.note_on(n, 0.8);
    }
    assert_eq!(e.active_voice_count(), 8, "8 voices must be active");

    e.apply_instrument(InstrumentKind::Mandolin);

    // note_off の damping 適用は process が走る前は damping_target=0.95 だが、
    // active_count 判定は voice の active flag で、note_off 直後は active のまま。
    // 仕様書 D53: pool.all_notes_off() → 全 voice の damping_target が 0.95 に変わる。
    // 「active_count==0」を満たすには process を走らせて energy 減衰させる必要があるが、
    // ここでは「全 voice の damping_target が note-off と同じ 0.95」を確認する。
    let pool = e.pool();
    for i in 0..8 {
        if let Some(v) = pool.voice(i) {
            assert!(
                (v.damping_target() - 0.95).abs() < 1e-6,
                "voice {i} damping_target should be 0.95 (note_off) after apply_instrument: got {}",
                v.damping_target()
            );
        }
    }
}

#[test]
fn test_apply_instrument_clears_sustain_state() {
    // sustain pending あり → apply_instrument → pending bitmap = 0
    let mut e = fresh_engine();
    e.note_on(60, 0.8);
    e.handle_midi_cc(64, 1.0); // sustain on
    e.note_off(60);
    assert_ne!(e.sustain_pending_bitmap(), 0, "pending bitmap must be set");

    e.apply_instrument(InstrumentKind::Bass);
    assert_eq!(
        e.sustain_pending_bitmap(),
        0,
        "pending bitmap must be 0 after apply_instrument"
    );
    assert!(
        !e.sustain_active(),
        "sustain must be inactive after apply_instrument"
    );
}

#[test]
fn test_apply_instrument_resets_modal_state() {
    // modal_body の z1/z2 state がクリアされる
    let mut e = fresh_engine();
    e.note_on(60, 0.8);
    let mut buf_l = vec![0.0_f32; 4800];
    let mut buf_r = vec![0.0_f32; 4800];
    e.process(&mut buf_l, &mut buf_r);
    // modal state は累積している
    let z1_before = e.modal_body().state_l_z1(0).abs();
    assert!(
        z1_before > 1e-12,
        "modal state must accumulate before apply_instrument"
    );

    e.apply_instrument(InstrumentKind::Sitar);
    assert_eq!(
        e.modal_body().state_l_z1(0),
        0.0,
        "modal state must be cleared after apply_instrument"
    );
}

#[test]
fn test_apply_instrument_no_alloc() {
    // apply_instrument 100 回連打で voice / modal capacity 不変
    let mut e = fresh_engine();
    for n in 60..=67 {
        e.note_on(n, 0.8);
    }
    let cap_before: Vec<usize> = (0..8)
        .map(|i| e.pool().voice(i).map(|v| v.buffer_capacity()).unwrap_or(0))
        .collect();

    let kinds = [
        InstrumentKind::Default,
        InstrumentKind::GuitarClassical,
        InstrumentKind::Ukulele,
        InstrumentKind::Mandolin,
        InstrumentKind::Bass,
        InstrumentKind::GuitarSteel,
        InstrumentKind::Sitar,
    ];
    for _ in 0..15 {
        for &k in &kinds {
            e.apply_instrument(k);
        }
    }

    let cap_after: Vec<usize> = (0..8)
        .map(|i| e.pool().voice(i).map(|v| v.buffer_capacity()).unwrap_or(0))
        .collect();
    assert_eq!(
        cap_before, cap_after,
        "voice buffer capacity unchanged across 105 instrument changes"
    );
}

#[test]
fn test_stereo_spread_per_instrument() {
    // 各楽器で stereo_spread が pre-research §7.3 と一致 (params.rs 経由で機械化済)
    let mut e = fresh_engine();

    e.apply_instrument(InstrumentKind::Default);
    assert!((e.stereo_spread() - STEREO_SPREAD_DEFAULT).abs() < 1e-9);

    e.apply_instrument(InstrumentKind::Ukulele);
    assert!((e.stereo_spread() - 0.04).abs() < 1e-6);

    e.apply_instrument(InstrumentKind::Mandolin);
    assert!((e.stereo_spread() - 0.06).abs() < 1e-6);

    e.apply_instrument(InstrumentKind::Bass);
    assert!((e.stereo_spread() - 0.03).abs() < 1e-6);

    e.apply_instrument(InstrumentKind::Sitar);
    assert!((e.stereo_spread() - 0.08).abs() < 1e-6);

    // ヘルパが直接関数として返す値とも一致
    assert!((stereo_spread_for_instrument(InstrumentKind::Sitar) - 0.08).abs() < 1e-6);
}

// ===== Phase 4b D67 / F54: apply_instrument で dispersion_active を fan-out =====

#[test]
fn test_apply_instrument_piano_enables_dispersion() {
    let mut e = fresh_engine();
    e.apply_instrument(InstrumentKind::Piano);
    let pool = e.pool();
    for i in 0..8 {
        let v = pool.voice(i).expect("voice exists");
        assert!(
            v.dispersion_active(),
            "voice {i} should have dispersion_active=true after apply_instrument(Piano)"
        );
    }
}

#[test]
fn test_apply_instrument_default_disables_dispersion() {
    let mut e = fresh_engine();
    // Piano を一度有効にしてから Default に戻す
    e.apply_instrument(InstrumentKind::Piano);
    e.apply_instrument(InstrumentKind::Default);
    let pool = e.pool();
    for i in 0..8 {
        let v = pool.voice(i).expect("voice exists");
        assert!(
            !v.dispersion_active(),
            "voice {i} should have dispersion_active=false after apply_instrument(Default)"
        );
    }
}

#[test]
fn test_apply_instrument_other_kinds_disable_dispersion() {
    // Piano 以外のすべての楽器で dispersion_active = false
    let mut e = fresh_engine();
    let kinds = [
        InstrumentKind::Default,
        InstrumentKind::GuitarClassical,
        InstrumentKind::Ukulele,
        InstrumentKind::Mandolin,
        InstrumentKind::Bass,
        InstrumentKind::GuitarSteel,
        InstrumentKind::Sitar,
    ];
    for kind in kinds {
        e.apply_instrument(InstrumentKind::Piano);
        e.apply_instrument(kind);
        let pool = e.pool();
        for i in 0..8 {
            assert!(
                !pool.voice(i).unwrap().dispersion_active(),
                "{kind:?} should disable dispersion on voice {i}"
            );
        }
    }
}

#[test]
fn test_apply_instrument_piano_no_alloc() {
    // apply_instrument(Piano) を 100 連打で voice buffer / dispersion_stages 容量不変。
    // dispersion_stages は inline 配列なので heap 操作なし、buffer も Phase 1 で確保済み。
    let mut e = fresh_engine();
    let cap_before: Vec<usize> = (0..8)
        .map(|i| e.pool().voice(i).map(|v| v.buffer_capacity()).unwrap_or(0))
        .collect();

    for _ in 0..50 {
        e.apply_instrument(InstrumentKind::Piano);
        e.apply_instrument(InstrumentKind::Default);
    }

    let cap_after: Vec<usize> = (0..8)
        .map(|i| e.pool().voice(i).map(|v| v.buffer_capacity()).unwrap_or(0))
        .collect();
    assert_eq!(
        cap_before, cap_after,
        "voice buffer capacity unchanged across 100 Piano↔Default toggles"
    );
}

// ===== Phase 4b D62 / F53-c/d/e: Piano kind の Engine 経由動作確認 =====

#[test]
fn test_apply_instrument_piano_modal_coeffs() {
    // Default → Piano で modal_body の係数が Piano 値ベースに変わることを確認
    let mut e = fresh_engine();
    let coeff_default = e.modal_body().coeff_l_b0(0);
    e.apply_instrument(InstrumentKind::Piano);
    let coeff_piano = e.modal_body().coeff_l_b0(0);
    assert!(
        (coeff_default - coeff_piano).abs() > 1e-6,
        "apply_instrument(Piano) should change modal coeff: default={coeff_default} piano={coeff_piano}"
    );
}

#[test]
fn test_body_modes_for_instrument_piano() {
    use dsp_core::params::{body_modes_for_instrument, BODY_MODES_PIANO_L, BODY_MODES_PIANO_R};
    // const は inline 展開され同一アドレスにならないため、値で比較する
    let (l, r) = body_modes_for_instrument(InstrumentKind::Piano);
    for i in 0..8 {
        assert!((l[i].freq - BODY_MODES_PIANO_L[i].freq).abs() < 1e-6);
        assert!((l[i].q - BODY_MODES_PIANO_L[i].q).abs() < 1e-6);
        assert!((l[i].gain - BODY_MODES_PIANO_L[i].gain).abs() < 1e-6);
        assert!((r[i].freq - BODY_MODES_PIANO_R[i].freq).abs() < 1e-6);
    }
}

#[test]
fn test_instrument_kind_count_includes_piano() {
    use dsp_core::params::INSTRUMENT_KIND_COUNT;
    // Phase 4a 7 → Phase 4b 8 に拡張
    assert_eq!(INSTRUMENT_KIND_COUNT, 8);
}

#[test]
fn test_piano_specific_constants() {
    use dsp_core::params::{
        HAMMER_CUTOFF_HIGH_PIANO, HAMMER_CUTOFF_LOW_PIANO, INHARMONICITY_B_PIANO,
    };
    assert!((INHARMONICITY_B_PIANO - 7.5e-4).abs() < 1e-9);
    assert!((HAMMER_CUTOFF_LOW_PIANO - 800.0).abs() < 1e-3);
    assert!((HAMMER_CUTOFF_HIGH_PIANO - 4000.0).abs() < 1e-3);
}

#[test]
fn test_default_instrument_matches_phase3_modes() {
    // Phase 3 既存 BODY_MODES_DEFAULT_L (= BODY_MODES_L alias) の各値が Default kind と一致。
    // 機械的保証: params.rs の生成式により Default kind の係数 = Phase 3 既存値。
    let m0 = BODY_MODES_DEFAULT_L[0];
    assert!((m0.freq - 105.0).abs() < 1e-6);
    assert!((m0.q - 30.0).abs() < 1e-6);
    assert!((m0.gain - 1.0).abs() < 1e-6);
    let m7 = BODY_MODES_DEFAULT_L[7];
    assert!((m7.freq - 2300.0).abs() < 1e-6);
    assert!((m7.q - 60.0).abs() < 1e-6);
    assert!((m7.gain - 0.15).abs() < 1e-6);
}
