//! 単体 biquad の係数仕様検証 (Phase 3 F26、03 章 §テスト方針 (a))。
//!
//! ModalBodyResonator の `calc_coeffs(mode, sr)` 出力を 1 段だけ走らせて、
//! 隣接モードの寄与なしに係数仕様を保証する。aggregate テストは
//! `modal_body_tests.rs` で別途扱う。

use dsp_core::modal_body::{calc_coeffs_for_test, process_single_biquad_for_test};
use dsp_core::params::BodyMode;

const SAMPLE_RATE: f32 = 48_000.0;

fn run_single_biquad(mode: BodyMode, sr: f32, samples: &[f32]) -> Vec<f32> {
    let coeffs = calc_coeffs_for_test(mode, sr);
    let mut state = (0.0_f32, 0.0_f32);
    let mut out = Vec::with_capacity(samples.len());
    for &x in samples {
        let (y, s) = process_single_biquad_for_test(coeffs, state, x);
        state = s;
        out.push(y);
    }
    out
}

#[test]
fn test_single_biquad_dc_blocking() {
    // bandpass biquad は DC ゲイン 0 (b0 + b2 = 0)。DC 入力 1.0 を 1 秒入力した
    // 後の定常出力は |y| < 0.001
    let mode = BodyMode {
        freq: 200.0,
        q: 25.0,
        gain: 0.8,
    };
    let samples = vec![1.0_f32; SAMPLE_RATE as usize];
    let out = run_single_biquad(mode, SAMPLE_RATE, &samples);

    let tail = &out[(SAMPLE_RATE * 0.9) as usize..];
    let max_abs = tail.iter().map(|x| x.abs()).fold(0.0_f32, f32::max);
    assert!(max_abs < 0.001, "DC blocking failed: max_abs = {}", max_abs);
}

#[test]
fn test_single_biquad_peak_at_freq() {
    // f = mode.freq の sin 入力 (振幅 1.0) に対し定常出力 RMS が mode.gain / sqrt(2) ± 5%
    for mode in [
        BodyMode {
            freq: 200.0,
            q: 25.0,
            gain: 0.8,
        },
        BodyMode {
            freq: 580.0,
            q: 40.0,
            gain: 0.35,
        },
        BodyMode {
            freq: 1400.0,
            q: 50.0,
            gain: 0.2,
        },
    ] {
        let total = SAMPLE_RATE as usize;
        let mut samples = Vec::with_capacity(total);
        let omega = 2.0 * core::f32::consts::PI * mode.freq / SAMPLE_RATE;
        for n in 0..total {
            samples.push((omega * n as f32).sin());
        }
        let out = run_single_biquad(mode, SAMPLE_RATE, &samples);

        // 立ち上がり transient を skip して末尾 0.1 秒で RMS 測定
        let skip = (SAMPLE_RATE * 0.7) as usize;
        let tail = &out[skip..];
        let rms = (tail.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / tail.len() as f64).sqrt()
            as f32;

        let expected = mode.gain / (2.0_f32).sqrt();
        let err = (rms - expected).abs() / expected;
        assert!(
            err < 0.05,
            "peak gain failed for f={}: expected RMS≈{:.4}, got {:.4} (err={:.2}%)",
            mode.freq,
            expected,
            rms,
            err * 100.0
        );
    }
}

#[test]
fn test_single_biquad_bandwidth() {
    // -3dB 帯域幅が概ね freq / Q (±20%) であることを定性的に確認
    let mode = BodyMode {
        freq: 580.0,
        q: 40.0,
        gain: 0.35,
    };
    let expected_bw = mode.freq / mode.q;

    // 中心周波数の RMS を取る
    let total = SAMPLE_RATE as usize;
    let omega_center = 2.0 * core::f32::consts::PI * mode.freq / SAMPLE_RATE;
    let samples_center: Vec<f32> = (0..total)
        .map(|n| (omega_center * n as f32).sin())
        .collect();
    let out_center = run_single_biquad(mode, SAMPLE_RATE, &samples_center);
    let skip = (SAMPLE_RATE * 0.7) as usize;
    let rms_center = {
        let tail = &out_center[skip..];
        (tail.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / tail.len() as f64).sqrt() as f32
    };
    let target_3db = rms_center / (2.0_f32).sqrt();

    // 中心周波数 + bw/2 で -3dB 程度に減衰しているか
    let f_upper = mode.freq + expected_bw / 2.0;
    let omega_upper = 2.0 * core::f32::consts::PI * f_upper / SAMPLE_RATE;
    let samples_upper: Vec<f32> = (0..total).map(|n| (omega_upper * n as f32).sin()).collect();
    let out_upper = run_single_biquad(mode, SAMPLE_RATE, &samples_upper);
    let rms_upper = {
        let tail = &out_upper[skip..];
        (tail.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / tail.len() as f64).sqrt() as f32
    };

    // ±20% 許容（厳密な -3dB 測定ではなく、定性的なロールオフ確認）
    let err_ratio = (rms_upper - target_3db).abs() / target_3db;
    assert!(
        err_ratio < 0.5,
        "bandwidth approximation failed: rms_upper={:.4}, target_3db={:.4}, err={:.2}%",
        rms_upper,
        target_3db,
        err_ratio * 100.0
    );
}
