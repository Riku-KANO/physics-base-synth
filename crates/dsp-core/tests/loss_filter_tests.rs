//! LossFilter テスト (Phase 3 F27)

use dsp_core::loss_filter::LossFilter;

#[test]
fn test_loss_filter_dc_gain() {
    // DC ゲイン = 1.0 (保存)
    let mut f = LossFilter::new();
    f.set_for_frequency(440.0);
    let mut last = 0.0;
    for _ in 0..1000 {
        last = f.process_sample(1.0);
    }
    assert!((last - 1.0).abs() < 0.001, "DC gain not 1.0: got {}", last);
}

#[test]
fn test_loss_filter_nyquist_attenuation() {
    // Nyquist 入力（交互 ±1.0）に対し定常出力 RMS が (1-rho)/(1+rho) に近い
    let mut f = LossFilter::new();
    f.set_for_frequency(440.0);
    let rho = f.rho();
    let expected = (1.0 - rho) / (1.0 + rho);

    let mut sign = 1.0_f32;
    let mut tail_sum_sq = 0.0_f64;
    let total = 2000;
    let tail_start = 1000;
    for i in 0..total {
        let y = f.process_sample(sign);
        sign = -sign;
        if i >= tail_start {
            tail_sum_sq += (y as f64).powi(2);
        }
    }
    let rms = (tail_sum_sq / (total - tail_start) as f64).sqrt() as f32;
    let err = (rms - expected).abs() / expected;
    assert!(
        err < 0.05,
        "Nyquist attenuation off: got rms={:.4}, expected≈{:.4} ({}% err)",
        rms,
        expected,
        err * 100.0
    );
}

#[test]
fn test_loss_filter_high_freq_more_loss() {
    // 高周波数で ρ がより大きい（clamp 範囲 [0.5, 2.0] 内で比較するため A2 vs A4）。
    // 注: 440Hz 以上は scale=2.0 に saturate するため A4 vs A6 では差が出ない。
    let mut f1 = LossFilter::new();
    f1.set_for_frequency(110.0); // A2 → scale = 110/220 = 0.5
    let rho_a2 = f1.rho();

    let mut f2 = LossFilter::new();
    f2.set_for_frequency(440.0); // A4 → scale = 2.0 (clamp 上限)
    let rho_a4 = f2.rho();

    assert!(
        rho_a4 > rho_a2,
        "high freq should have more loss: rho_a2={}, rho_a4={}",
        rho_a2,
        rho_a4
    );
}

#[test]
fn test_loss_filter_freq_clamps() {
    // freq < 110 (≒ A2) で scale=0.5 に clamp、freq > 440 (= A4) で 2.0 まで成長
    let mut f_low = LossFilter::new();
    f_low.set_for_frequency(50.0); // 27.5 Hz 等の低音は scale=0.5 に clamp
    assert!((f_low.rho() - LossFilter::RHO_BASE * 0.5).abs() < 1e-6);

    let mut f_high = LossFilter::new();
    f_high.set_for_frequency(5000.0); // C8 等の高音は scale=2.0 に clamp
    assert!((f_high.rho() - LossFilter::RHO_BASE * 2.0).abs() < 1e-6);
}
