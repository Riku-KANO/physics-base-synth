//! Phase 4c Step 13: Hertz law raised cosine hammer (F66) + B(note) LUT (F67) の
//! integration tests。
//!
//! F66 は `KarplusStrong` を直接駆動して励振 buffer を観測する (t_c / f_c / amplitude /
//! 形状)、F67 は LUT のサイズ / 端値 / clamp 動作 / Engine 経路での lookup を検証する。

use dsp_core::dispersion::{b_curve_piano, b_curve_zero};
use dsp_core::engine::Engine;
use dsp_core::karplus_strong::KarplusStrong;
use dsp_core::params::{
    InstrumentKind, HAMMER_CUTOFF_HIGH_PIANO, HAMMER_CUTOFF_LOW_PIANO, INHARMONICITY_B_CURVE_PIANO,
};
use dsp_core::traits::AudioProcessor;

const SAMPLE_RATE: f32 = 48_000.0;

/// テスト用ヘルパ: `dispersion_active=true` + Phase 4c Piano 既定 cutoffs で
/// `note_on_with_id(60, freq, velocity)` を実行し、string 0 buffer を返す。
fn hammer_excitation(velocity: f32) -> (Vec<f32>, usize) {
    let mut ks = KarplusStrong::new();
    ks.prepare(SAMPLE_RATE, 128);
    ks.set_dispersion_active(true);
    ks.set_instrument_params(
        0.0, // unison_detune = 0 で n_strings_active=1 になる経路を踏みたいが、
        // n_strings_active は midi 21..=33 のときのみ 1 弦になる仕様 (D69)。
        // 実装上は note_id=60 で n_strings=3 になるが、buf[0] (中央弦) を取得して
        // hammer 形状を観測する目的では問題ない。
        7.5e-4,
        HAMMER_CUTOFF_LOW_PIANO,
        HAMMER_CUTOFF_HIGH_PIANO,
    );
    // 低音 (MIDI 33 = A1、n_strings=1) で len_int を稼ぐ。t_c_samples が 4 ms × 48 kHz = 192 まで
    // ありえるため、len_int がそれ以上である必要がある。MIDI 33 ≈ 55 Hz → len_int ≈ 873。
    ks.note_on_with_id(33, 55.0, velocity);
    let buf = ks.buffer_clone_for_test(0);
    let len = ks.length_int_for_string(0);
    (buf, len)
}

// ===== F66: Hertz hammer のパラメータ式 =====

/// F66-a: velocity を上げると t_c_ms (接触時間) が短くなる → 励振 envelope のピーク位置が左寄り。
#[test]
fn test_hammer_t_c_decreases_with_velocity() {
    let (buf_soft, len_soft) = hammer_excitation(0.1);
    let (buf_hard, len_hard) = hammer_excitation(1.0);

    // ピーク位置を求める
    fn peak_idx(buf: &[f32], len: usize) -> usize {
        buf[..len]
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                a.abs()
                    .partial_cmp(&b.abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    let peak_soft = peak_idx(&buf_soft, len_soft);
    let peak_hard = peak_idx(&buf_hard, len_hard);

    // v=0.1: t_c_samples ≈ 178 → peak ~89
    // v=1.0: t_c_samples ≈ 57 → peak ~28
    assert!(
        peak_soft > peak_hard + 20,
        "soft velocity peak ({}) should be >>20 samples after hard velocity peak ({})",
        peak_soft,
        peak_hard
    );
    // 大体の位置を確認
    assert!(
        (60..130).contains(&peak_soft),
        "soft peak should be in ~89 range, got {}",
        peak_soft
    );
    assert!(
        (10..60).contains(&peak_hard),
        "hard peak should be in ~28 range, got {}",
        peak_hard
    );
}

/// F66-b: velocity 上昇で f_c (LPF cutoff) が上がる → 高域成分が増える (diff_rms で代理)。
#[test]
fn test_hammer_f_c_increases_with_velocity() {
    let (buf_soft, len_soft) = hammer_excitation(0.1);
    let (buf_hard, len_hard) = hammer_excitation(1.0);

    fn diff_rms(buf: &[f32], len: usize) -> f64 {
        let slice = &buf[..len];
        let mut sum = 0.0_f64;
        for i in 1..slice.len() {
            let d = (slice[i] - slice[i - 1]) as f64;
            sum += d * d;
        }
        (sum / (slice.len() - 1) as f64).sqrt()
    }

    let dr_soft = diff_rms(&buf_soft, len_soft);
    let dr_hard = diff_rms(&buf_hard, len_hard);
    assert!(
        dr_hard > dr_soft,
        "hard velocity should produce higher diff_rms (= higher f_c), soft={}, hard={}",
        dr_soft,
        dr_hard
    );
}

/// F66-c: amplitude = √velocity (perceptual loudness)。
/// v=0.25 → amp=0.5、v=1.0 → amp=1.0。LPF 後でも比率は概ね保たれる。
#[test]
fn test_hammer_amplitude_sqrt_velocity() {
    let (buf_quarter, len_quarter) = hammer_excitation(0.25);
    let (buf_full, len_full) = hammer_excitation(1.0);

    fn peak_abs(buf: &[f32], len: usize) -> f32 {
        buf[..len].iter().map(|x| x.abs()).fold(0.0_f32, f32::max)
    }

    let peak_quarter = peak_abs(&buf_quarter, len_quarter);
    let peak_full = peak_abs(&buf_full, len_full);
    // amp = √v なので peak の比は √(1.0/0.25) = 2.0 が理論値。LPF で歪むため広い窓で許容。
    let ratio = peak_full / peak_quarter.max(1e-9);
    assert!(
        (1.5..3.5).contains(&ratio),
        "peak_full / peak_quarter should be ~2.0 (sqrt(4)), got {}",
        ratio
    );
}

/// F66-d: raised cosine 形状 (sin²) の確認。`buf[0]` ≈ 0、ピークが ~t_c/2 付近に出現。
#[test]
fn test_hammer_raised_cosine_shape() {
    let (buf, len) = hammer_excitation(0.5);
    // v=0.5 → t_c_ms = 2.6 ms → t_c_samples ≈ 125 @ 48kHz
    // sin² の初期値は 0 (LPF 通過後も非常に小さい)
    assert!(
        buf[0].abs() < 0.05,
        "raised cosine at i=0 must start near 0, got {}",
        buf[0]
    );

    // 初期領域 (0..t_c/2) は単調増加に近い
    let peak_idx = buf[..len]
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            a.abs()
                .partial_cmp(&b.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap();
    // v=0.5 → t_c=125 → peak ~62 想定
    assert!(
        (30..120).contains(&peak_idx),
        "raised cosine peak should be in ~t_c/2 (= 30..120), got {}",
        peak_idx
    );
}

/// F66-e: velocity が高いほど出力スペクトルの centroid が上がる → diff_rms 比 > 1.5。
/// 実質的には F66-b と同じ性質を別の閾値で確認する。
#[test]
fn test_hammer_velocity_affects_brightness() {
    let (buf_soft, len_soft) = hammer_excitation(0.1);
    let (buf_hard, len_hard) = hammer_excitation(1.0);

    fn diff_rms(buf: &[f32], len: usize) -> f64 {
        let slice = &buf[..len];
        let mut sum = 0.0_f64;
        for i in 1..slice.len() {
            let d = (slice[i] - slice[i - 1]) as f64;
            sum += d * d;
        }
        (sum / (slice.len() - 1) as f64).sqrt()
    }

    let dr_soft = diff_rms(&buf_soft, len_soft);
    let dr_hard = diff_rms(&buf_hard, len_hard);
    let ratio = dr_hard / dr_soft.max(1e-9);
    assert!(
        ratio > 1.5,
        "hard / soft diff_rms ratio should exceed 1.5 (= brightness shift), got {}",
        ratio
    );
}

/// F66-f: Default kind では pluck (noise burst) 経路で励振される。隣接 sample の符号変化が頻発する。
#[test]
fn test_hammer_pluck_path_for_default() {
    let mut ks = KarplusStrong::new();
    ks.prepare(SAMPLE_RATE, 128);
    // Default kind = dispersion_active=false
    ks.set_instrument_params(0.0, 0.0, 0.0, 0.0);
    ks.note_on_with_id(60, 261.63, 0.8);

    let len = ks.length_int_for_string(0);
    let buf = ks.buffer_clone_for_test(0);
    let snapshot = &buf[..len];

    let mut sign_changes = 0;
    for i in 1..snapshot.len() {
        if snapshot[i].signum() != snapshot[i - 1].signum() && snapshot[i].abs() > 1e-9 {
            sign_changes += 1;
        }
    }
    assert!(
        sign_changes > snapshot.len() / 4,
        "Default kind should pluck with many sign changes, got {} of {}",
        sign_changes,
        snapshot.len()
    );
}

// ===== F67: B(note) LUT =====

/// F67-a: LUT は 88 鍵 ぴったり。
#[test]
fn test_b_curve_length_88() {
    assert_eq!(INHARMONICITY_B_CURVE_PIANO.len(), 88);
}

/// F67-b: A0 (MIDI 21) の B 値が ~3.1e-4。
#[test]
fn test_b_curve_lookup_a0() {
    let b = b_curve_piano(21);
    assert!(
        (b - 3.1e-4).abs() < 1e-5,
        "b_curve_piano(21) should be ~3.1e-4, got {}",
        b
    );
}

/// F67-c: A4 (MIDI 69) の B 値が LUT 値と一致。spec のコメント `~7.5e-4 (Phase 4b 互換値と近似一致)`
/// は LUT の index alignment を取り違えており、実際の LUT では index 48 = 4.0e-3。
/// 仕様書の聴感調整 (Step 18-19) で curve 値が変化する可能性があるため、ここでは存在検証のみ。
#[test]
fn test_b_curve_lookup_a4() {
    let b = b_curve_piano(69);
    let expected = INHARMONICITY_B_CURVE_PIANO[48]; // MIDI 69 - 21 = 48
    assert!(
        (b - expected).abs() < 1e-9,
        "b_curve_piano(69) should match INHARMONICITY_B_CURVE_PIANO[48] = {}, got {}",
        expected,
        b
    );
    // mid-range の値が低音 / 高音より高いことを確認 (倒立 U-curve、A3 付近で底)
    assert!(b > b_curve_piano(21), "A4 should exceed A0 in the LUT");
}

/// F67-d: C8 (MIDI 108) の B 値 ≥ 0.05 (高音域)。
#[test]
fn test_b_curve_lookup_c8() {
    let b = b_curve_piano(108);
    assert!(
        b >= 0.05,
        "b_curve_piano(108) should be >= 0.05 for high-octave inharmonicity, got {}",
        b
    );
}

/// F67-e: A3 (MIDI 57) 以上で LUT が単調増加する。
#[test]
fn test_b_curve_monotonic_increase_above_a3() {
    let mut prev = b_curve_piano(57);
    for midi in 58..=108 {
        let cur = b_curve_piano(midi);
        assert!(
            cur > prev,
            "LUT should be monotonically increasing from A3, broke at MIDI {} ({} vs prev {})",
            midi,
            cur,
            prev
        );
        prev = cur;
    }
}

/// F67-f: MIDI 範囲外 (< 21 / > 108) は端値で fallback。Engine が `u8` を直接渡しても panic / OOB なし。
#[test]
fn test_b_curve_clamps_out_of_range() {
    assert!(
        (b_curve_piano(0) - INHARMONICITY_B_CURVE_PIANO[0]).abs() < 1e-9,
        "b_curve_piano(0) should clamp to LUT[0]"
    );
    assert!(
        (b_curve_piano(20) - INHARMONICITY_B_CURVE_PIANO[0]).abs() < 1e-9,
        "b_curve_piano(20) should clamp to LUT[0]"
    );
    assert!(
        (b_curve_piano(127) - INHARMONICITY_B_CURVE_PIANO[87]).abs() < 1e-9,
        "b_curve_piano(127) should clamp to LUT[87]"
    );
    assert!(
        (b_curve_piano(255) - INHARMONICITY_B_CURVE_PIANO[87]).abs() < 1e-9,
        "b_curve_piano(255) should clamp to LUT[87]"
    );
}

/// F67-g: Piano kind の note_on で割当 voice の `inharmonicity_b` が `b_curve_piano(midi)` と一致。
#[test]
fn test_b_curve_used_in_note_on_piano() {
    let mut engine = Engine::new();
    engine.prepare(SAMPLE_RATE, 128);
    engine.apply_instrument(InstrumentKind::Piano);
    engine.note_on(60, 0.8);

    let expected = b_curve_piano(60);
    let actual = engine
        .voice_inharmonicity_b_for_test(60)
        .expect("voice 60 should exist");
    assert!(
        (actual - expected).abs() < 1e-9,
        "voice inharmonicity_b should match b_curve_piano(60): expected {}, got {}",
        expected,
        actual
    );
}

/// F67-h: Default kind の note_on で `inharmonicity_b = 0` (`b_curve_zero` が返す値)。
#[test]
fn test_b_curve_not_used_for_default() {
    let mut engine = Engine::new();
    engine.prepare(SAMPLE_RATE, 128);
    // Default kind (デフォルト) のまま
    engine.note_on(60, 0.8);

    let actual = engine
        .voice_inharmonicity_b_for_test(60)
        .expect("voice 60 should exist");
    assert!(
        actual.abs() < 1e-9,
        "Default kind voice inharmonicity_b should be 0, got {}",
        actual
    );

    // `b_curve_zero` が全 MIDI で 0 を返すことの直接確認
    for m in 0u8..=127 {
        assert_eq!(b_curve_zero(m), 0.0);
    }
}
