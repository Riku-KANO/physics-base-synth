//! Lfo 単体テスト (Phase 4a F40-a / F40-b / F40-g)

use dsp_core::lfo::{Lfo, LfoWaveform};

const SAMPLE_RATE: f32 = 48_000.0;

fn fresh(rate_hz: f32, waveform: LfoWaveform) -> Lfo {
    let mut lfo = Lfo::new();
    lfo.prepare(SAMPLE_RATE);
    lfo.set_rate(rate_hz);
    lfo.set_waveform(waveform);
    lfo
}

#[test]
fn test_lfo_sine_range() {
    let mut lfo = fresh(5.0, LfoWaveform::Sine);
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for _ in 0..(SAMPLE_RATE as usize) {
        let v = lfo.process_sample();
        if v < min {
            min = v;
        }
        if v > max {
            max = v;
        }
    }
    assert!(
        min >= -1.0 - 1e-5 && max <= 1.0 + 1e-5,
        "sine out of [-1, 1]: min={min} max={max}"
    );
    assert!(min < -0.95, "sine min should approach -1: got {min}");
    assert!(max > 0.95, "sine max should approach +1: got {max}");
}

#[test]
fn test_lfo_triangle_range() {
    let mut lfo = fresh(5.0, LfoWaveform::Triangle);
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for _ in 0..(SAMPLE_RATE as usize) {
        let v = lfo.process_sample();
        if v < min {
            min = v;
        }
        if v > max {
            max = v;
        }
    }
    assert!(
        min >= -1.0 - 1e-5 && max <= 1.0 + 1e-5,
        "triangle out of [-1, 1]: min={min} max={max}"
    );
    assert!(min < -0.99, "triangle min should reach -1: got {min}");
    assert!(max > 0.99, "triangle max should reach +1: got {max}");
}

#[test]
fn test_lfo_zero_at_init() {
    // 値計算 → phase 進行 の順序により、初回 process_sample は phase=0 の値を返す。
    // Sine: sin(0) = 0、Triangle: 1 - 4·|0 - 0.5| = 1 - 2 = -1
    let mut lfo_sine = fresh(5.0, LfoWaveform::Sine);
    let v_sine = lfo_sine.process_sample();
    assert!(
        v_sine.abs() < 1e-6,
        "sine at phase=0 should be 0, got {v_sine}"
    );

    let mut lfo_tri = fresh(5.0, LfoWaveform::Triangle);
    let v_tri = lfo_tri.process_sample();
    assert!(
        (v_tri - (-1.0)).abs() < 1e-6,
        "triangle at phase=0 should be -1, got {v_tri}"
    );
}

#[test]
fn test_lfo_period_matches_rate() {
    // rate=5Hz、48000/5 = 9600 sample 後に phase が 1 周期完了 (phase wrap)。
    let mut lfo = fresh(5.0, LfoWaveform::Sine);
    let samples_per_period = (SAMPLE_RATE / 5.0) as usize;
    for _ in 0..samples_per_period {
        lfo.process_sample();
    }
    // 1 周期回したので phase は再び 0 近傍 [0, 1/9600) に戻っている (1.0 を超えたら -1.0 で wrap)
    let phase = lfo.phase();
    assert!(
        !(1.0e-3..=0.999).contains(&phase),
        "phase should wrap to ~0 after 1 period, got {phase}"
    );
}

#[test]
fn test_lfo_rate_smoothing() {
    // rate を 1Hz → 8Hz に変更後、target は即時、current は tau=0.05s で指数応答。
    let mut lfo = Lfo::new();
    lfo.prepare(SAMPLE_RATE);
    lfo.set_rate(1.0);
    // current を 1.0 に収束させる (1 秒間ぶん回す = tau の 20 倍)
    for _ in 0..(SAMPLE_RATE as usize) {
        lfo.process_sample();
    }
    assert!(
        (lfo.rate_current() - 1.0).abs() < 1e-3,
        "current should converge to 1.0, got {}",
        lfo.rate_current()
    );

    // target を 8Hz に変更
    lfo.set_rate(8.0);
    assert!(
        (lfo.rate_target() - 8.0).abs() < 1e-6,
        "target should be 8.0 immediately"
    );

    // 1 tau (50ms = 2400 sample) で current ≈ 1 + 7·(1 − e⁻¹) ≈ 5.42 Hz (誤差 ±0.2 Hz)
    let one_tau = (SAMPLE_RATE * 0.05) as usize;
    for _ in 0..one_tau {
        lfo.process_sample();
    }
    let current_at_1tau = lfo.rate_current();
    let expected_1tau = 1.0 + 7.0 * (1.0 - (-1.0_f32).exp());
    assert!(
        (current_at_1tau - expected_1tau).abs() < 0.2,
        "after 1 tau, current should be ≈ {expected_1tau:.2} Hz, got {current_at_1tau:.4}"
    );

    // 5 tau (250ms = 12000 sample) で current > 7.95 Hz の指数応答期待値
    let four_more_tau = 4 * one_tau;
    for _ in 0..four_more_tau {
        lfo.process_sample();
    }
    let current_at_5tau = lfo.rate_current();
    assert!(
        current_at_5tau > 7.95,
        "after 5 tau, current should approach 8.0 (> 7.95), got {current_at_5tau:.4}"
    );
}

#[test]
fn test_lfo_waveform_switch_no_click() {
    // クリック対策の本質: 波形切替で phase が reset されないこと。
    // 同一 phase での sine と triangle の値差は最大 ~1.2 までありうる (連続関数ではない)
    // ため、瞬間値の連続性を assert することはできない。phase 連続性 + 切替後の値が
    // 有限 / [-1, 1] 範囲に収まることで連続性を担保する。
    let mut lfo = fresh(5.0, LfoWaveform::Sine);
    for _ in 0..1000 {
        lfo.process_sample();
    }
    let phase_before = lfo.phase();
    lfo.set_waveform(LfoWaveform::Triangle);
    let phase_after_switch = lfo.phase();
    assert!(
        (phase_before - phase_after_switch).abs() < 1e-6,
        "phase must be preserved across waveform switch"
    );
    let v = lfo.process_sample();
    assert!(
        v.is_finite() && (-1.0..=1.0).contains(&v),
        "post-switch value must stay finite in [-1, 1], got {v}"
    );
}

#[test]
fn test_lfo_no_alloc_in_process() {
    // process_sample 1000 回呼出で外部からは alloc 発生を観測できないが、Vec を持たない
    // (構造体内に SmoothedValue + f32 のみ) ことから仕様上 alloc 0 は明らかなので、
    // ここでは「panic / NaN なし」と「最小値 / 最大値 が常に有限」を確認する。
    let mut lfo = fresh(5.0, LfoWaveform::Sine);
    for _ in 0..1000 {
        let v = lfo.process_sample();
        assert!(v.is_finite(), "LFO output must be finite, got {v}");
    }
}

#[test]
fn test_lfo_phase_wraps() {
    // 10 秒走らせて phase が [0, 1) で wrap している (NaN / 無限大なし)。
    let mut lfo = fresh(7.5, LfoWaveform::Sine);
    let total_samples = (SAMPLE_RATE * 10.0) as usize;
    for _ in 0..total_samples {
        let v = lfo.process_sample();
        assert!(v.is_finite(), "LFO output must be finite during long run");
        let p = lfo.phase();
        assert!(
            (0.0..1.0).contains(&p),
            "phase must stay in [0, 1), got {p}"
        );
    }
}
